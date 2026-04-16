use serde::Serialize;

use agentchrome::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, SkillArgs, SkillCommand, ToolName};
use crate::output::print_output;

// =============================================================================
// Types
// =============================================================================

/// How a skill is installed for a given tool.
enum InstallMode {
    /// Write a standalone file (create dirs as needed).
    Standalone { path_template: &'static str },
    /// Append a delimited section to a shared file.
    AppendSection { path_template: &'static str },
    /// Write standalone file + patch a config file.
    StandaloneWithConfig {
        skill_path_template: &'static str,
        config_path_template: &'static str,
    },
}

/// Metadata for a supported agentic tool.
struct ToolInfo {
    name: &'static str,
    detection: &'static str,
    install_mode: InstallMode,
}

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct SkillResult {
    tool: String,
    path: String,
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}

#[derive(Serialize)]
struct SkillListOutput {
    tools: Vec<ToolListEntry>,
}

#[derive(Serialize)]
struct ToolListEntry {
    name: String,
    detection: String,
    path: String,
    installed: bool,
}

// =============================================================================
// Tool registry
// =============================================================================

static TOOLS: &[ToolInfo] = &[
    ToolInfo {
        name: "claude-code",
        detection: "CLAUDE_CODE env var or 'claude' in parent process",
        install_mode: InstallMode::Standalone {
            path_template: "~/.claude/skills/agentchrome/SKILL.md",
        },
    },
    ToolInfo {
        name: "windsurf",
        detection: "WINDSURF_* env var or ~/.codeium/ directory exists",
        install_mode: InstallMode::AppendSection {
            path_template: "~/.codeium/windsurf/memories/global_rules.md",
        },
    },
    ToolInfo {
        name: "aider",
        detection: "AIDER_* env var or 'aider' in parent process",
        install_mode: InstallMode::StandaloneWithConfig {
            skill_path_template: "~/.aider/agentchrome.md",
            config_path_template: "~/.aider.conf.yml",
        },
    },
    ToolInfo {
        name: "continue",
        detection: "~/.continue/ directory exists",
        install_mode: InstallMode::Standalone {
            path_template: "~/.continue/rules/agentchrome.md",
        },
    },
    ToolInfo {
        name: "copilot-jb",
        detection: "~/.config/github-copilot/ directory exists",
        install_mode: InstallMode::AppendSection {
            path_template: "~/.config/github-copilot/intellij/global-copilot-instructions.md",
        },
    },
    ToolInfo {
        name: "cursor",
        detection: "CURSOR_* env var or ~/.cursor/ directory exists",
        install_mode: InstallMode::Standalone {
            path_template: ".cursor/rules/agentchrome.mdc",
        },
    },
];

// =============================================================================
// Skill content template
// =============================================================================

const SKILL_TEMPLATE: &str = "\
# agentchrome — Browser Automation CLI

agentchrome gives you browser superpowers via the Chrome DevTools Protocol.

## When to Use

Use agentchrome when you need to:
- Navigate to URLs, inspect pages, fill forms, click elements
- Take screenshots or capture accessibility trees
- Monitor console output or network requests
- Automate browser workflows (testing, scraping, verification)

## How to Discover Commands

agentchrome is self-documenting. Use these commands to learn what it can do:

- `agentchrome --help` — overview of all commands
- `agentchrome <command> --help` — detailed help for any command
- `agentchrome capabilities` — machine-readable JSON manifest of all commands
- `agentchrome examples` — practical usage examples for every command
- `agentchrome man <command>` — full man page for any command

## Quick Start

```sh
agentchrome connect --launch --headless
agentchrome navigate <url>
agentchrome page snapshot
```

Version: {version}
";

fn skill_content() -> String {
    SKILL_TEMPLATE.replace("{version}", env!("CARGO_PKG_VERSION"))
}

// =============================================================================
// Detection heuristic
// =============================================================================

fn detect_tool() -> Option<&'static ToolInfo> {
    // Tier 1: Environment variables (highest priority)
    if std::env::var("CLAUDE_CODE").is_ok() {
        return find_tool("claude-code");
    }
    if has_env_prefix("WINDSURF_") {
        return find_tool("windsurf");
    }
    if has_env_prefix("AIDER_") {
        return find_tool("aider");
    }
    if has_env_prefix("CURSOR_") {
        return find_tool("cursor");
    }

    // Tier 2: Parent process name
    if let Ok(parent) = std::env::var("_") {
        let parent_lower = parent.to_lowercase();
        if parent_lower.contains("claude") {
            return find_tool("claude-code");
        }
        if parent_lower.contains("aider") {
            return find_tool("aider");
        }
    }

    // Tier 3: Config directory existence (lowest priority)
    let home = home_dir()?;
    if home.join(".codeium").is_dir() {
        return find_tool("windsurf");
    }
    if home.join(".continue").is_dir() {
        return find_tool("continue");
    }
    if home.join(".config/github-copilot").is_dir() {
        return find_tool("copilot-jb");
    }
    if home.join(".cursor").is_dir() {
        return find_tool("cursor");
    }

    None
}

fn has_env_prefix(prefix: &str) -> bool {
    std::env::vars().any(|(key, _)| key.starts_with(prefix))
}

fn find_tool(name: &str) -> Option<&'static ToolInfo> {
    TOOLS.iter().find(|t| t.name == name)
}

fn tool_for_name(name: &ToolName) -> &'static ToolInfo {
    let key = match name {
        ToolName::ClaudeCode => "claude-code",
        ToolName::Windsurf => "windsurf",
        ToolName::Aider => "aider",
        ToolName::Continue => "continue",
        ToolName::CopilotJb => "copilot-jb",
        ToolName::Cursor => "cursor",
    };
    find_tool(key).expect("all ToolName variants have a matching ToolInfo entry")
}

// =============================================================================
// Path resolution
// =============================================================================

fn home_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir()
}

fn resolve_path(template: &str) -> Result<std::path::PathBuf, AppError> {
    if let Some(rest) = template.strip_prefix("~/") {
        let home = home_dir().ok_or_else(|| AppError {
            message: "could not determine home directory".into(),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;
        Ok(home.join(rest))
    } else {
        Ok(std::path::PathBuf::from(template))
    }
}

fn path_template(tool: &ToolInfo) -> &'static str {
    match &tool.install_mode {
        InstallMode::Standalone { path_template }
        | InstallMode::AppendSection { path_template } => path_template,
        InstallMode::StandaloneWithConfig {
            skill_path_template,
            ..
        } => skill_path_template,
    }
}

// =============================================================================
// Resolve tool from --tool flag or detection
// =============================================================================

fn resolve_tool(tool_flag: Option<&ToolName>) -> Result<&'static ToolInfo, AppError> {
    match tool_flag {
        Some(name) => Ok(tool_for_name(name)),
        None => detect_tool().ok_or_else(|| {
            let supported: Vec<&str> = TOOLS.iter().map(|t| t.name).collect();
            let custom = serde_json::json!({
                "error": "no supported agentic tool detected",
                "supported_tools": supported
            });
            AppError {
                message: "no supported agentic tool detected".into(),
                code: ExitCode::GeneralError,
                custom_json: Some(custom.to_string()),
            }
        }),
    }
}

// =============================================================================
// Install logic
// =============================================================================

const SECTION_START: &str = "<!-- agentchrome:start -->";
const SECTION_END: &str = "<!-- agentchrome:end -->";

fn install_skill(tool: &ToolInfo) -> Result<SkillResult, AppError> {
    let content = skill_content();

    match &tool.install_mode {
        InstallMode::Standalone { path_template } => {
            let path = resolve_path(path_template)?;
            write_file(&path, &content)?;
            Ok(SkillResult {
                tool: tool.name.into(),
                path: path.display().to_string(),
                action: "installed".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            })
        }
        InstallMode::AppendSection { path_template } => {
            let path = resolve_path(path_template)?;
            write_section(&path, &content)?;
            Ok(SkillResult {
                tool: tool.name.into(),
                path: path.display().to_string(),
                action: "installed".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            })
        }
        InstallMode::StandaloneWithConfig {
            skill_path_template,
            config_path_template,
        } => {
            let skill_path = resolve_path(skill_path_template)?;
            let config_path = resolve_path(config_path_template)?;
            write_file(&skill_path, &content)?;
            patch_aider_config(&config_path, skill_path_template)?;
            Ok(SkillResult {
                tool: tool.name.into(),
                path: skill_path.display().to_string(),
                action: "installed".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            })
        }
    }
}

// =============================================================================
// Uninstall logic
// =============================================================================

fn uninstall_skill(tool: &ToolInfo) -> Result<SkillResult, AppError> {
    match &tool.install_mode {
        InstallMode::Standalone { path_template } => {
            let path = resolve_path(path_template)?;
            remove_file_and_empty_parents(&path);
            Ok(SkillResult {
                tool: tool.name.into(),
                path: path.display().to_string(),
                action: "uninstalled".into(),
                version: None,
            })
        }
        InstallMode::AppendSection { path_template } => {
            let path = resolve_path(path_template)?;
            remove_section(&path);
            Ok(SkillResult {
                tool: tool.name.into(),
                path: path.display().to_string(),
                action: "uninstalled".into(),
                version: None,
            })
        }
        InstallMode::StandaloneWithConfig {
            skill_path_template,
            config_path_template,
        } => {
            let skill_path = resolve_path(skill_path_template)?;
            let config_path = resolve_path(config_path_template)?;
            remove_file_and_empty_parents(&skill_path);
            unpatch_aider_config(&config_path, skill_path_template);
            Ok(SkillResult {
                tool: tool.name.into(),
                path: skill_path.display().to_string(),
                action: "uninstalled".into(),
                version: None,
            })
        }
    }
}

// =============================================================================
// Update logic
// =============================================================================

fn update_skill(tool: &ToolInfo) -> Result<SkillResult, AppError> {
    // Check that the skill is currently installed
    let template = path_template(tool);
    let path = resolve_path(template)?;
    let installed = match &tool.install_mode {
        InstallMode::Standalone { .. } | InstallMode::StandaloneWithConfig { .. } => path.exists(),
        InstallMode::AppendSection { .. } => {
            path.exists()
                && std::fs::read_to_string(&path)
                    .map(|c| c.contains(SECTION_START))
                    .unwrap_or(false)
        }
    };

    if !installed {
        return Err(AppError {
            message: format!(
                "no skill currently installed for {}. Run 'agentchrome skill install' first.",
                tool.name
            ),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }

    // Delegate to install logic
    let mut result = install_skill(tool)?;
    result.action = "updated".into();
    Ok(result)
}

// =============================================================================
// List logic
// =============================================================================

fn list_tools() -> Result<SkillListOutput, AppError> {
    let mut entries = Vec::with_capacity(TOOLS.len());

    for tool in TOOLS {
        let template = path_template(tool);
        let path = resolve_path(template)?;
        let installed = match &tool.install_mode {
            InstallMode::Standalone { .. } | InstallMode::StandaloneWithConfig { .. } => {
                path.exists()
            }
            InstallMode::AppendSection { .. } => {
                path.exists()
                    && std::fs::read_to_string(&path)
                        .map(|c| c.contains(SECTION_START))
                        .unwrap_or(false)
            }
        };
        entries.push(ToolListEntry {
            name: tool.name.into(),
            detection: tool.detection.into(),
            path: template.into(),
            installed,
        });
    }

    Ok(SkillListOutput { tools: entries })
}

// =============================================================================
// File I/O helpers
// =============================================================================

fn write_file(path: &std::path::Path, content: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError {
            message: format!("failed to create directory {}: {e}", parent.display()),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;
    }
    std::fs::write(path, content).map_err(|e| AppError {
        message: format!("failed to write {}: {e}", path.display()),
        code: ExitCode::GeneralError,
        custom_json: None,
    })
}

fn write_section(path: &std::path::Path, content: &str) -> Result<(), AppError> {
    let section = format!("{SECTION_START}\n{content}{SECTION_END}\n");

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError {
            message: format!("failed to create directory {}: {e}", parent.display()),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;
    }

    if path.exists() {
        let existing = std::fs::read_to_string(path).map_err(|e| AppError {
            message: format!("failed to read {}: {e}", path.display()),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

        if let (Some(start), Some(end)) = (existing.find(SECTION_START), existing.find(SECTION_END))
        {
            // Replace existing section
            let end_of_marker = end + SECTION_END.len();
            // Skip trailing newline after end marker if present
            let end_of_marker = if existing[end_of_marker..].starts_with('\n') {
                end_of_marker + 1
            } else {
                end_of_marker
            };
            let mut new_content = String::with_capacity(existing.len());
            new_content.push_str(&existing[..start]);
            new_content.push_str(&section);
            new_content.push_str(&existing[end_of_marker..]);
            std::fs::write(path, new_content).map_err(|e| AppError {
                message: format!("failed to write {}: {e}", path.display()),
                code: ExitCode::GeneralError,
                custom_json: None,
            })?;
        } else {
            // Append section
            let mut new_content = existing;
            if !new_content.ends_with('\n') && !new_content.is_empty() {
                new_content.push('\n');
            }
            new_content.push_str(&section);
            std::fs::write(path, new_content).map_err(|e| AppError {
                message: format!("failed to write {}: {e}", path.display()),
                code: ExitCode::GeneralError,
                custom_json: None,
            })?;
        }
    } else {
        std::fs::write(path, section).map_err(|e| AppError {
            message: format!("failed to write {}: {e}", path.display()),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;
    }

    Ok(())
}

fn remove_section(path: &std::path::Path) {
    if !path.exists() {
        return;
    }
    let Ok(existing) = std::fs::read_to_string(path) else {
        return;
    };

    if let (Some(start), Some(end)) = (existing.find(SECTION_START), existing.find(SECTION_END)) {
        let end_of_marker = end + SECTION_END.len();
        let end_of_marker = if existing[end_of_marker..].starts_with('\n') {
            end_of_marker + 1
        } else {
            end_of_marker
        };
        let remaining = format!("{}{}", &existing[..start], &existing[end_of_marker..]);
        let trimmed = remaining.trim();
        if trimmed.is_empty() {
            let _ = std::fs::remove_file(path);
        } else {
            let _ = std::fs::write(path, remaining);
        }
    }
}

fn remove_file_and_empty_parents(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    // Walk up and remove empty parent directories
    let mut dir = path.parent();
    while let Some(parent) = dir {
        // Stop at home or root
        if parent == home_dir().as_deref().unwrap_or(std::path::Path::new("/")) {
            break;
        }
        if std::fs::remove_dir(parent).is_err() {
            break; // Not empty or permission denied
        }
        dir = parent.parent();
    }
}

// =============================================================================
// Aider config helpers
// =============================================================================

fn patch_aider_config(config_path: &std::path::Path, skill_path: &str) -> Result<(), AppError> {
    if config_path.exists() {
        let content = std::fs::read_to_string(config_path).map_err(|e| AppError {
            message: format!("failed to read {}: {e}", config_path.display()),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

        // Check if the entry is already present
        if content.contains(skill_path) {
            return Ok(());
        }

        // Check if `read:` section exists
        if let Some(read_pos) = content.find("\nread:") {
            // Find the end of the read section's entries and append
            let after_read = read_pos + "\nread:".len();
            let mut new_content = String::with_capacity(content.len() + skill_path.len() + 10);
            new_content.push_str(&content[..after_read]);
            new_content.push_str(&format!("\n  - {skill_path}"));
            new_content.push_str(&content[after_read..]);
            std::fs::write(config_path, new_content).map_err(|e| AppError {
                message: format!("failed to write {}: {e}", config_path.display()),
                code: ExitCode::GeneralError,
                custom_json: None,
            })?;
        } else if content.starts_with("read:") {
            // read: is the first line
            let after_read = "read:".len();
            let mut new_content = String::with_capacity(content.len() + skill_path.len() + 10);
            new_content.push_str(&content[..after_read]);
            new_content.push_str(&format!("\n  - {skill_path}"));
            new_content.push_str(&content[after_read..]);
            std::fs::write(config_path, new_content).map_err(|e| AppError {
                message: format!("failed to write {}: {e}", config_path.display()),
                code: ExitCode::GeneralError,
                custom_json: None,
            })?;
        } else {
            // No read section — append one
            let mut new_content = content;
            if !new_content.ends_with('\n') && !new_content.is_empty() {
                new_content.push('\n');
            }
            new_content.push_str(&format!("read:\n  - {skill_path}\n"));
            std::fs::write(config_path, new_content).map_err(|e| AppError {
                message: format!("failed to write {}: {e}", config_path.display()),
                code: ExitCode::GeneralError,
                custom_json: None,
            })?;
        }
    } else {
        // Create config file with read entry
        if let Some(parent) = config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = format!("read:\n  - {skill_path}\n");
        std::fs::write(config_path, content).map_err(|e| AppError {
            message: format!("failed to write {}: {e}", config_path.display()),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;
    }

    Ok(())
}

fn unpatch_aider_config(config_path: &std::path::Path, skill_path: &str) {
    if !config_path.exists() {
        return;
    }
    let Ok(content) = std::fs::read_to_string(config_path) else {
        return;
    };

    // Remove lines containing the skill path
    let new_lines: Vec<&str> = content
        .lines()
        .filter(|line| !line.contains(skill_path))
        .collect();
    let new_content = new_lines.join("\n") + "\n";
    let _ = std::fs::write(config_path, new_content);
}

// =============================================================================
// Dispatcher
// =============================================================================

pub fn execute_skill(global: &GlobalOpts, args: &SkillArgs) -> Result<(), AppError> {
    match &args.command {
        SkillCommand::Install(install_args) => {
            let tool = resolve_tool(install_args.tool.as_ref())?;
            let result = install_skill(tool)?;
            print_output(&result, &global.output)
        }
        SkillCommand::Uninstall(tool_args) => {
            let tool = resolve_tool(tool_args.tool.as_ref())?;
            let result = uninstall_skill(tool)?;
            print_output(&result, &global.output)
        }
        SkillCommand::Update(tool_args) => {
            let tool = resolve_tool(tool_args.tool.as_ref())?;
            let result = update_skill(tool)?;
            print_output(&result, &global.output)
        }
        SkillCommand::List => {
            let result = list_tools()?;
            print_output(&result, &global.output)
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_registry_has_six_tools() {
        assert_eq!(TOOLS.len(), 6);
    }

    #[test]
    fn all_tools_have_non_empty_fields() {
        for tool in TOOLS {
            assert!(!tool.name.is_empty(), "tool name is empty");
            assert!(
                !tool.detection.is_empty(),
                "detection is empty for {}",
                tool.name
            );
            assert!(
                !path_template(tool).is_empty(),
                "path template is empty for {}",
                tool.name
            );
        }
    }

    #[test]
    fn find_tool_returns_correct_tool() {
        assert_eq!(find_tool("claude-code").unwrap().name, "claude-code");
        assert_eq!(find_tool("cursor").unwrap().name, "cursor");
        assert!(find_tool("nonexistent").is_none());
    }

    #[test]
    fn tool_for_name_maps_all_variants() {
        assert_eq!(tool_for_name(&ToolName::ClaudeCode).name, "claude-code");
        assert_eq!(tool_for_name(&ToolName::Windsurf).name, "windsurf");
        assert_eq!(tool_for_name(&ToolName::Aider).name, "aider");
        assert_eq!(tool_for_name(&ToolName::Continue).name, "continue");
        assert_eq!(tool_for_name(&ToolName::CopilotJb).name, "copilot-jb");
        assert_eq!(tool_for_name(&ToolName::Cursor).name, "cursor");
    }

    #[test]
    fn skill_content_contains_version() {
        let content = skill_content();
        assert!(content.contains(env!("CARGO_PKG_VERSION")));
        assert!(!content.contains("{version}"));
    }

    #[test]
    fn skill_content_contains_key_sections() {
        let content = skill_content();
        assert!(content.contains("# agentchrome"));
        assert!(content.contains("## When to Use"));
        assert!(content.contains("## How to Discover Commands"));
        assert!(content.contains("agentchrome capabilities"));
        assert!(content.contains("agentchrome examples"));
    }

    #[test]
    fn resolve_path_expands_tilde() {
        let path = resolve_path("~/.claude/test").unwrap();
        assert!(!path.to_str().unwrap().starts_with('~'));
        assert!(path.to_str().unwrap().ends_with(".claude/test"));
    }

    #[test]
    fn resolve_path_relative_stays_relative() {
        let path = resolve_path(".cursor/rules/test.mdc").unwrap();
        assert_eq!(path.to_str().unwrap(), ".cursor/rules/test.mdc");
    }

    #[test]
    fn resolve_tool_with_explicit_flag() {
        let tool = resolve_tool(Some(&ToolName::Cursor)).unwrap();
        assert_eq!(tool.name, "cursor");
    }

    #[test]
    fn resolve_tool_without_flag_and_no_env_returns_error() {
        // Clear any env vars that might trigger detection.
        // Since we can't guarantee no config dirs exist, this test just verifies
        // the error path works when detection fails.
        // In CI or minimal environments, this should return an error.
        let result = resolve_tool(None);
        // If detection happens to succeed (e.g., config dirs exist on dev machine),
        // that's fine — just verify it returns *something*.
        assert!(result.is_ok() || result.is_err());
        if let Err(err) = result {
            assert!(err.custom_json.is_some());
            let json: serde_json::Value =
                serde_json::from_str(err.custom_json.as_ref().unwrap()).unwrap();
            assert!(json["supported_tools"].is_array());
        }
    }

    #[test]
    fn install_standalone_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("skill/SKILL.md");
        write_file(&file_path, "test content").unwrap();
        assert!(file_path.exists());
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "test content");
    }

    #[test]
    fn write_section_creates_new_file_with_markers() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("rules.md");
        write_section(&file_path, "skill content\n").unwrap();
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains(SECTION_START));
        assert!(content.contains(SECTION_END));
        assert!(content.contains("skill content"));
    }

    #[test]
    fn write_section_appends_to_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("rules.md");
        std::fs::write(&file_path, "existing content\n").unwrap();
        write_section(&file_path, "skill content\n").unwrap();
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.starts_with("existing content\n"));
        assert!(content.contains(SECTION_START));
        assert!(content.contains("skill content"));
    }

    #[test]
    fn write_section_replaces_existing_section() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("rules.md");
        let initial = format!("before\n{SECTION_START}\nold content\n{SECTION_END}\nafter\n");
        std::fs::write(&file_path, initial).unwrap();
        write_section(&file_path, "new content\n").unwrap();
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("before\n"));
        assert!(content.contains("new content"));
        assert!(!content.contains("old content"));
        assert!(content.contains("after\n"));
    }

    #[test]
    fn remove_section_removes_markers_and_content() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("rules.md");
        let content = format!("before\n{SECTION_START}\nskill\n{SECTION_END}\nafter\n");
        std::fs::write(&file_path, content).unwrap();
        remove_section(&file_path);
        let result = std::fs::read_to_string(&file_path).unwrap();
        assert!(!result.contains(SECTION_START));
        assert!(!result.contains("skill"));
        assert!(result.contains("before"));
        assert!(result.contains("after"));
    }

    #[test]
    fn remove_section_deletes_file_if_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("rules.md");
        let content = format!("{SECTION_START}\nskill\n{SECTION_END}\n");
        std::fs::write(&file_path, content).unwrap();
        remove_section(&file_path);
        assert!(!file_path.exists());
    }

    #[test]
    fn patch_aider_config_creates_new_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join(".aider.conf.yml");
        patch_aider_config(&config, "~/.aider/agentchrome.md").unwrap();
        let content = std::fs::read_to_string(&config).unwrap();
        assert!(content.contains("read:"));
        assert!(content.contains("~/.aider/agentchrome.md"));
    }

    #[test]
    fn patch_aider_config_appends_to_existing_read() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join(".aider.conf.yml");
        std::fs::write(&config, "read:\n  - existing.md\n").unwrap();
        patch_aider_config(&config, "~/.aider/agentchrome.md").unwrap();
        let content = std::fs::read_to_string(&config).unwrap();
        assert!(content.contains("existing.md"));
        assert!(content.contains("~/.aider/agentchrome.md"));
    }

    #[test]
    fn patch_aider_config_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join(".aider.conf.yml");
        patch_aider_config(&config, "~/.aider/agentchrome.md").unwrap();
        patch_aider_config(&config, "~/.aider/agentchrome.md").unwrap();
        let content = std::fs::read_to_string(&config).unwrap();
        let count = content.matches("agentchrome.md").count();
        assert_eq!(count, 1, "entry should not be duplicated");
    }

    #[test]
    fn unpatch_aider_config_removes_entry() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join(".aider.conf.yml");
        std::fs::write(
            &config,
            "read:\n  - ~/.aider/agentchrome.md\n  - other.md\n",
        )
        .unwrap();
        unpatch_aider_config(&config, "~/.aider/agentchrome.md");
        let content = std::fs::read_to_string(&config).unwrap();
        assert!(!content.contains("agentchrome.md"));
        assert!(content.contains("other.md"));
    }

    #[test]
    fn list_output_has_all_tools() {
        let output = list_tools().unwrap();
        assert_eq!(output.tools.len(), 6);
        let names: Vec<&str> = output.tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"claude-code"));
        assert!(names.contains(&"windsurf"));
        assert!(names.contains(&"aider"));
        assert!(names.contains(&"continue"));
        assert!(names.contains(&"copilot-jb"));
        assert!(names.contains(&"cursor"));
    }

    #[test]
    fn list_output_serializes_correctly() {
        let output = list_tools().unwrap();
        let json = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let tools = parsed["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 6);
        for tool in tools {
            assert!(tool["name"].is_string());
            assert!(tool["detection"].is_string());
            assert!(tool["path"].is_string());
            assert!(tool["installed"].is_boolean());
        }
    }

    #[test]
    fn skill_result_serializes_correctly() {
        let result = SkillResult {
            tool: "claude-code".into(),
            path: "/home/user/.claude/skills/agentchrome/SKILL.md".into(),
            action: "installed".into(),
            version: Some("1.8.0".into()),
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["tool"], "claude-code");
        assert_eq!(parsed["action"], "installed");
        assert_eq!(parsed["version"], "1.8.0");
    }

    #[test]
    fn skill_result_omits_version_when_none() {
        let result = SkillResult {
            tool: "claude-code".into(),
            path: "/test".into(),
            action: "uninstalled".into(),
            version: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.contains("version"));
    }
}
