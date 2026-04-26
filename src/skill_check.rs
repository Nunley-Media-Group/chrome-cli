use std::path::Path;

use crate::config::ConfigFile;

// =============================================================================
// Version type
// =============================================================================

/// A parsed semantic version triple (major, minor, patch).
///
/// No `semver` crate dependency — parsed from raw text.
pub(crate) type Version = (u32, u32, u32);

// =============================================================================
// Stale tool descriptor
// =============================================================================

/// A tool whose installed skill file carries an older version marker than the
/// currently running binary.
pub(crate) struct StaleTool {
    pub(crate) tool: &'static crate::skill::ToolInfo,
    pub(crate) installed_version: Version,
}

// =============================================================================
// Version marker parsing
// =============================================================================

/// Line budget for scanning the version marker out of a skill file.
///
/// Version markers live in the YAML frontmatter or the first few lines of
/// standalone skill files. Append-section installs may live deeper in shared
/// instruction files, so they are also scanned inside `AgentChrome` section
/// markers wherever the section appears.
const VERSION_SCAN_LINES: usize = 20;
const SECTION_START: &str = "<!-- agentchrome:start -->";
const SECTION_END: &str = "<!-- agentchrome:end -->";

/// Try to parse a version triple out of the first few lines of a skill file.
///
/// Accepts three formats (in priority order):
/// 1. YAML frontmatter: `version: "X.Y.Z"` or `version: X.Y.Z`
/// 2. Legacy markdown heading: `Version: X.Y.Z`
/// 3. HTML comment marker: `<!-- agentchrome-version: X.Y.Z -->`
///
/// Returns `None` for any I/O error, parse error, or unrecognized content.
pub(crate) fn read_version_marker(path: &Path) -> Option<Version> {
    let content = std::fs::read_to_string(path).ok()?;
    parse_version_from_content(&content)
}

fn parse_version_from_content(content: &str) -> Option<Version> {
    parse_version_from_lines(content.lines().take(VERSION_SCAN_LINES)).or_else(|| {
        let start = content.find(SECTION_START)?;
        let after_start = start + SECTION_START.len();
        let end = content[after_start..]
            .find(SECTION_END)
            .map_or(content.len(), |offset| after_start + offset);
        parse_version_from_lines(content[after_start..end].lines())
    })
}

fn parse_version_from_lines<'a>(lines: impl Iterator<Item = &'a str>) -> Option<Version> {
    for line in lines {
        let trimmed = line.trim();

        // YAML frontmatter: version: "X.Y.Z"  or  version: X.Y.Z
        if let Some(rest) = trimmed.strip_prefix("version:") {
            let ver_str = rest.trim().trim_matches('"');
            if let Some(v) = parse_version(ver_str) {
                return Some(v);
            }
        }

        // Legacy: Version: X.Y.Z
        if let Some(rest) = trimmed.strip_prefix("Version:") {
            let ver_str = rest.trim();
            if let Some(v) = parse_version(ver_str) {
                return Some(v);
            }
        }

        // HTML comment: <!-- agentchrome-version: X.Y.Z -->
        if let Some(inner) = trimmed
            .strip_prefix("<!-- agentchrome-version:")
            .and_then(|s| s.strip_suffix("-->"))
        {
            let ver_str = inner.trim();
            if let Some(v) = parse_version(ver_str) {
                return Some(v);
            }
        }
    }
    None
}

/// Parse a `"X.Y.Z"` string into a `Version` triple.
///
/// Returns `None` for any non-conforming input.
fn parse_version(s: &str) -> Option<Version> {
    let mut parts = s.splitn(3, '.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next()?.parse::<u32>().ok()?;
    let patch = parts.next()?.trim().parse::<u32>().ok()?;
    Some((major, minor, patch))
}

// =============================================================================
// Binary version
// =============================================================================

/// Return the current binary version as a `Version` triple.
///
/// Sourced from `CARGO_PKG_VERSION` at compile time — always valid.
fn binary_version() -> Version {
    parse_version(env!("CARGO_PKG_VERSION"))
        .expect("CARGO_PKG_VERSION is always a valid X.Y.Z triple")
}

// =============================================================================
// Stale tool scan
// =============================================================================

/// Collect all tools whose installed skill file reports an older version than
/// the running binary.
///
/// I/O errors (missing file, unreadable) are silently skipped for that tool —
/// a missing install is not a stale install.
pub(crate) fn stale_tools() -> Vec<StaleTool> {
    let bin_ver = binary_version();
    let mut result = Vec::with_capacity(crate::skill::TOOLS.len());

    for tool in crate::skill::TOOLS {
        let template = crate::skill::path_template(tool);
        let Ok(path) = crate::skill::resolve_path(template) else {
            continue;
        };
        let Some(installed_ver) = read_version_marker(&path) else {
            continue;
        };
        if installed_ver < bin_ver {
            result.push(StaleTool {
                tool,
                installed_version: installed_ver,
            });
        }
    }

    result
}

// =============================================================================
// Notice formatting
// =============================================================================

fn format_version(v: Version) -> String {
    format!("{}.{}.{}", v.0, v.1, v.2)
}

fn format_notice(stale: &[StaleTool]) -> Option<String> {
    if stale.is_empty() {
        return None;
    }
    let bin_ver = format_version(binary_version());

    if stale.len() == 1 {
        let tool = &stale[0];
        let installed_ver = format_version(tool.installed_version);
        let name = tool.tool.name;
        Some(format!(
            "note: installed agentchrome skill for {name} is v{installed_ver} but binary is v{bin_ver} \
             — run 'agentchrome skill update' to refresh"
        ))
    } else {
        let names: Vec<&str> = stale.iter().map(|t| t.tool.name).collect();
        let name_list = names.join(", ");
        let oldest_ver = stale
            .iter()
            .map(|t| t.installed_version)
            .min()
            .expect("stale is non-empty");
        let oldest_str = format_version(oldest_ver);
        Some(format!(
            "note: installed agentchrome skills for {name_list} are stale (oldest v{oldest_str}, binary v{bin_ver}) \
             — run 'agentchrome skill update' to refresh"
        ))
    }
}

// =============================================================================
// Public entry point
// =============================================================================

/// Emit a staleness notice to stderr if any installed skill file is older than
/// the running binary.
///
/// Suppressed when:
/// - `AGENTCHROME_NO_SKILL_CHECK=1` is set in the environment
/// - `config.skill.check_enabled == Some(false)`
///
/// Never returns an error — any internal failure is silently swallowed so that
/// skill-check issues never break the main command path.
pub fn emit_stale_notice_if_any(config: &ConfigFile) {
    if std::env::var("AGENTCHROME_NO_SKILL_CHECK").as_deref() == Ok("1") {
        return;
    }

    if config.skill.check_enabled == Some(false) {
        return;
    }

    let stale = stale_tools();
    if let Some(notice) = format_notice(&stale) {
        eprintln!("{notice}");
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // parse_version_from_content — all three marker formats
    // =========================================================================

    #[test]
    fn parses_yaml_frontmatter_quoted() {
        let content = "---\nname: agentchrome\nversion: \"1.42.0\"\n---\n";
        assert_eq!(parse_version_from_content(content), Some((1, 42, 0)));
    }

    #[test]
    fn parses_yaml_frontmatter_unquoted() {
        let content = "---\nname: agentchrome\nversion: 1.42.0\n---\n";
        assert_eq!(parse_version_from_content(content), Some((1, 42, 0)));
    }

    #[test]
    fn parses_legacy_version_heading() {
        let content = "# agentchrome\n\nVersion: 1.40.0\n\nSome content.\n";
        assert_eq!(parse_version_from_content(content), Some((1, 40, 0)));
    }

    #[test]
    fn parses_html_comment_marker() {
        let content =
            "<!-- agentchrome:start -->\n<!-- agentchrome-version: 1.38.2 -->\n\nContent.\n";
        assert_eq!(parse_version_from_content(content), Some((1, 38, 2)));
    }

    #[test]
    fn garbage_input_returns_none() {
        let content = "no version here at all\nrandom text\n";
        assert_eq!(parse_version_from_content(content), None);
    }

    #[test]
    fn empty_content_returns_none() {
        assert_eq!(parse_version_from_content(""), None);
    }

    #[test]
    fn version_beyond_20_lines_is_ignored() {
        let mut lines: Vec<String> = (0..25).map(|i| format!("line {i}")).collect();
        lines.push("version: 1.0.0".to_string());
        let content = lines.join("\n");
        // Line 26 is past the 20-line limit
        assert_eq!(parse_version_from_content(&content), None);
    }

    #[test]
    fn parses_append_section_marker_after_long_shared_file_preamble() {
        let preamble = (0..25)
            .map(|i| format!("shared instruction line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let content = format!(
            "{preamble}\n{SECTION_START}\n<!-- agentchrome-version: 1.39.4 -->\n\n# agentchrome\n{SECTION_END}\n"
        );

        assert_eq!(parse_version_from_content(&content), Some((1, 39, 4)));
    }

    #[test]
    fn missing_file_returns_none() {
        let path = Path::new("/nonexistent/path/to/SKILL.md");
        assert_eq!(read_version_marker(path), None);
    }

    // =========================================================================
    // parse_version
    // =========================================================================

    #[test]
    fn parse_version_valid() {
        assert_eq!(parse_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("0.0.0"), Some((0, 0, 0)));
        assert_eq!(parse_version("100.200.300"), Some((100, 200, 300)));
    }

    #[test]
    fn parse_version_invalid() {
        assert_eq!(parse_version("not.a.version"), None);
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("1.2.3.4"), None); // too many parts — `.splitn(3, '.')` captures "3.4" for patch
    }

    // =========================================================================
    // format_notice — grammar variants
    // =========================================================================

    #[test]
    fn format_notice_empty_stale_returns_none() {
        let stale: Vec<StaleTool> = vec![];
        assert!(format_notice(&stale).is_none());
    }

    #[test]
    fn format_notice_single_tool() {
        let stale = vec![StaleTool {
            tool: crate::skill::find_tool("claude-code").unwrap(),
            installed_version: (1, 40, 0),
        }];
        let bin_ver = format_version(binary_version());
        let notice = format_notice(&stale).unwrap();
        assert!(
            notice.contains("claude-code"),
            "notice must name the stale tool"
        );
        assert!(
            notice.contains("v1.40.0"),
            "notice must show installed version"
        );
        assert!(
            notice.contains(&format!("v{bin_ver}")),
            "notice must show binary version"
        );
        assert!(
            notice.contains("agentchrome skill update"),
            "notice must mention the fix command"
        );
    }

    #[test]
    fn format_notice_multi_tool() {
        let stale = vec![
            StaleTool {
                tool: crate::skill::find_tool("claude-code").unwrap(),
                installed_version: (1, 40, 0),
            },
            StaleTool {
                tool: crate::skill::find_tool("cursor").unwrap(),
                installed_version: (1, 38, 0),
            },
        ];
        let notice = format_notice(&stale).unwrap();
        assert!(notice.contains("claude-code"), "must list tool 1");
        assert!(notice.contains("cursor"), "must list tool 2");
        assert!(notice.contains("stale"), "must use 'stale' grammar");
        // Oldest is 1.38.0
        assert!(
            notice.contains("v1.38.0"),
            "must report oldest installed version"
        );
    }

    // =========================================================================
    // emit_stale_notice_if_any — suppression gates
    // =========================================================================

    #[test]
    fn suppressed_by_config_flag() {
        use crate::config::{ConfigFile, SkillConfigFile};
        let config = ConfigFile {
            skill: SkillConfigFile {
                check_enabled: Some(false),
            },
            ..ConfigFile::default()
        };
        // With check_enabled=false, function must return early without panicking.
        // We cannot assert on stderr in unit tests, but verifying no panic is
        // sufficient for the suppression-gate unit test.
        emit_stale_notice_if_any(&config);
    }

    #[test]
    fn not_stale_when_installed_equals_binary() {
        let bin_ver = binary_version();
        let content = format!(
            "---\nversion: \"{}.{}.{}\"\n---\n",
            bin_ver.0, bin_ver.1, bin_ver.2
        );
        let parsed = parse_version_from_content(&content).expect("binary version parses");
        assert!(parsed >= bin_ver, "equal version must not be stale");
    }

    #[test]
    fn newer_installed_version_not_stale() {
        let bin_ver = binary_version();
        let newer = (bin_ver.0 + 1, 0, 0);
        assert!(newer >= bin_ver, "newer version must not be stale");
    }
}
