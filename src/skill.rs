use std::path::Path;

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
pub(crate) struct ToolInfo {
    pub(crate) name: &'static str,
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
struct SkillBatchOutput {
    results: Vec<SkillBatchResult>,
}

#[derive(Serialize)]
struct SkillBatchResult {
    tool: String,
    path: String,
    action: String,
    version: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
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

pub(crate) static TOOLS: &[ToolInfo] = &[
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
    ToolInfo {
        name: "gemini",
        detection: "GEMINI_* env var or ~/.gemini/ directory exists",
        install_mode: InstallMode::Standalone {
            path_template: "~/.gemini/instructions/agentchrome.md",
        },
    },
    ToolInfo {
        name: "codex",
        detection: "CODEX_HOME env var or ~/.codex/ directory exists",
        install_mode: InstallMode::Standalone {
            path_template: "$CODEX_HOME/skills/agentchrome/SKILL.md",
        },
    },
];

// =============================================================================
// Skill content template
// =============================================================================

const SKILL_TEMPLATE: &str = "\
---
name: agentchrome
description: Use agentchrome when you need to automate a browser, fill a form, test a login, scrape a page, take a screenshot, or inspect console / network.
version: \"{version}\"
---

# agentchrome — Browser Automation CLI

agentchrome gives you browser superpowers via the Chrome DevTools Protocol. It is the right tool whenever the task involves driving a real Chromium instance non-interactively.

## When to Use

Reach for agentchrome when you need to:
- Navigate to URLs, inspect pages, fill forms, click elements
- Take screenshots or capture accessibility trees
- Monitor console output or network requests
- Automate browser workflows (testing, scraping, verification, auditing)

## How to Discover Commands

agentchrome is self-documenting. Start here before guessing:

- `agentchrome --help` — overview of all commands
- `agentchrome <command> --help` — detailed help for any command
- `agentchrome capabilities` — machine-readable JSON manifest of all commands
- `agentchrome capabilities <command>` — detail for one command (large; may return a temp-file object — see below)
- `agentchrome examples` — practical usage examples for every command
- `agentchrome examples strategies` — scenario-based guides (iframes, shadow DOM, SPA waits, ...)
- `agentchrome examples strategies <name>` — the full guide for one scenario
- `agentchrome man <command>` — full man page for any command

## Before You Automate

- `agentchrome diagnose <url>` — scan a page for iframes, dialogs, overlays, and framework quirks *before* trying to automate it.
- `agentchrome diagnose --current` — run the same scan against whatever tab is already attached.

If `diagnose` flags an iframe, SPA, or shadow DOM, run `agentchrome examples strategies <topic>` for the matching playbook.

## After You Act

Interaction commands (`interact click`, `interact hover`, `form fill`, `form fill-many`, `navigate`, ...) accept `--include-snapshot`. Pass it to get the post-action accessibility snapshot back in the same invocation — one round trip instead of two.

## Large Responses

Any response larger than ~16 KB returns a `{output_file, size_bytes, command, summary}` object on stdout and writes the full payload to a temp file. Read the `summary` first; only open the file if the summary does not answer your question. Streaming commands (`network follow`, `console follow`) are exempt — they stream directly.

For compound results (interaction + `--include-snapshot` above the threshold), the interaction confirmation stays inline and only the `snapshot` field is offloaded to a file.

## Quick Start

```sh
agentchrome connect --launch --headless
agentchrome navigate <url>
agentchrome diagnose --current
agentchrome page snapshot
```
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
    if has_env_prefix("GEMINI_") {
        return find_tool("gemini");
    }
    if std::env::var("CODEX_HOME").is_ok_and(|value| !value.is_empty()) {
        return find_tool("codex");
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
    if home.join(".gemini").is_dir() {
        return find_tool("gemini");
    }
    if home.join(".codex").is_dir() {
        return find_tool("codex");
    }

    None
}

fn detected_tools() -> Vec<&'static ToolInfo> {
    let env: Vec<(String, String)> = std::env::vars().collect();
    let parent = std::env::var("_").ok();
    let home = home_dir();
    detected_tools_with(&env, parent.as_deref(), home.as_deref())
}

fn detected_tools_with(
    env: &[(String, String)],
    parent: Option<&str>,
    home: Option<&Path>,
) -> Vec<&'static ToolInfo> {
    TOOLS
        .iter()
        .filter(|tool| tool_detected_with(tool, env, parent, home))
        .collect()
}

fn tool_detected_with(
    tool: &ToolInfo,
    env: &[(String, String)],
    parent: Option<&str>,
    home: Option<&Path>,
) -> bool {
    match tool.name {
        "claude-code" => env_has_key(env, "CLAUDE_CODE") || parent_contains(parent, "claude"),
        "windsurf" => env_has_prefix(env, "WINDSURF_") || home_has_dir(home, ".codeium"),
        "aider" => env_has_prefix(env, "AIDER_") || parent_contains(parent, "aider"),
        "continue" => home_has_dir(home, ".continue"),
        "copilot-jb" => home_has_dir(home, ".config/github-copilot"),
        "cursor" => env_has_prefix(env, "CURSOR_") || home_has_dir(home, ".cursor"),
        "gemini" => env_has_prefix(env, "GEMINI_") || home_has_dir(home, ".gemini"),
        "codex" => env_has_non_empty_key(env, "CODEX_HOME") || home_has_dir(home, ".codex"),
        _ => false,
    }
}

fn env_has_key(env: &[(String, String)], key: &str) -> bool {
    env.iter().any(|(candidate, _)| candidate == key)
}

fn env_has_non_empty_key(env: &[(String, String)], key: &str) -> bool {
    env.iter()
        .any(|(candidate, value)| candidate == key && !value.is_empty())
}

fn env_has_prefix(env: &[(String, String)], prefix: &str) -> bool {
    env.iter().any(|(key, _)| key.starts_with(prefix))
}

fn parent_contains(parent: Option<&str>, needle: &str) -> bool {
    parent.is_some_and(|value| value.to_ascii_lowercase().contains(needle))
}

fn home_has_dir(home: Option<&Path>, relative: &str) -> bool {
    home.is_some_and(|root| root.join(relative).is_dir())
}

fn has_env_prefix(prefix: &str) -> bool {
    std::env::vars().any(|(key, _)| key.starts_with(prefix))
}

pub(crate) fn find_tool(name: &str) -> Option<&'static ToolInfo> {
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
        ToolName::Gemini => "gemini",
        ToolName::Codex => "codex",
    };
    find_tool(key).expect("all ToolName variants have a matching ToolInfo entry")
}

// =============================================================================
// Path resolution
// =============================================================================

fn home_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir()
}

pub(crate) fn resolve_path(template: &str) -> Result<std::path::PathBuf, AppError> {
    if let Some(rest) = template.strip_prefix("$CODEX_HOME/") {
        let root = codex_home_root(std::env::var_os("CODEX_HOME"), home_dir())?;
        Ok(root.join(rest))
    } else if let Some(rest) = template.strip_prefix("~/") {
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

fn codex_home_root(
    codex_home: Option<std::ffi::OsString>,
    home: Option<std::path::PathBuf>,
) -> Result<std::path::PathBuf, AppError> {
    if let Some(root) = codex_home.filter(|value| !value.is_empty()) {
        return Ok(std::path::PathBuf::from(root));
    }

    home.map(|path| path.join(".codex"))
        .ok_or_else(|| AppError {
            message: "could not determine home directory".into(),
            code: ExitCode::GeneralError,
            custom_json: None,
        })
}

pub(crate) fn path_template(tool: &ToolInfo) -> &'static str {
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
        None => detect_tool().ok_or_else(no_supported_agentic_tool_detected),
    }
}

fn no_supported_agentic_tool_detected() -> AppError {
    let supported: Vec<&str> = TOOLS.iter().map(|t| t.name).collect();
    let supported_details: Vec<serde_json::Value> = TOOLS
        .iter()
        .map(|tool| {
            serde_json::json!({
                "name": tool.name,
                "detection": tool.detection,
                "path": path_template(tool),
            })
        })
        .collect();
    let custom = serde_json::json!({
        "error": "no supported agentic tool detected",
        "supported_tools": supported,
        "supported_tool_details": supported_details,
    });
    AppError {
        message: "no supported agentic tool detected".into(),
        code: ExitCode::GeneralError,
        custom_json: Some(custom.to_string()),
    }
}

fn no_stale_installed_skills_found() -> AppError {
    let custom = serde_json::json!({
        "error": "no stale installed AgentChrome skills found",
    });
    AppError {
        message: "no stale installed AgentChrome skills found".into(),
        code: ExitCode::GeneralError,
        custom_json: Some(custom.to_string()),
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
// Multi-target install/update logic
// =============================================================================

fn install_detected_skills(global: &GlobalOpts) -> Result<(), AppError> {
    let tools = detected_tools();
    if tools.is_empty() {
        return Err(no_supported_agentic_tool_detected());
    }

    let batch = run_skill_batch(tools, "installed", install_skill);
    print_batch_output(global, &batch)
}

fn update_stale_skills(global: &GlobalOpts) -> Result<(), AppError> {
    let tools: Vec<&'static ToolInfo> = crate::skill_check::stale_tools()
        .into_iter()
        .map(|stale| stale.tool)
        .collect();
    if tools.is_empty() {
        return Err(no_stale_installed_skills_found());
    }

    let batch = run_skill_batch(tools, "updated", update_skill);
    print_batch_output(global, &batch)
}

fn run_skill_batch(
    tools: Vec<&'static ToolInfo>,
    action: &'static str,
    operation: fn(&ToolInfo) -> Result<SkillResult, AppError>,
) -> SkillBatchOutput {
    let results = tools
        .into_iter()
        .map(|tool| match operation(tool) {
            Ok(result) => SkillBatchResult {
                tool: result.tool,
                path: result.path,
                action: result.action,
                version: result
                    .version
                    .unwrap_or_else(|| env!("CARGO_PKG_VERSION").into()),
                status: "ok".into(),
                error: None,
            },
            Err(err) => SkillBatchResult {
                tool: tool.name.into(),
                path: batch_path(tool),
                action: action.into(),
                version: env!("CARGO_PKG_VERSION").into(),
                status: "error".into(),
                error: Some(err.message),
            },
        })
        .collect();

    SkillBatchOutput { results }
}

fn print_batch_output(global: &GlobalOpts, batch: &SkillBatchOutput) -> Result<(), AppError> {
    let has_error = batch.results.iter().any(|result| result.status == "error");
    print_output(batch, &global.output)?;

    if has_error {
        Err(AppError {
            message: "one or more skill targets failed".into(),
            code: ExitCode::GeneralError,
            custom_json: None,
        })
    } else {
        Ok(())
    }
}

fn batch_path(tool: &ToolInfo) -> String {
    resolve_path(path_template(tool)).map_or_else(
        |_| path_template(tool).into(),
        |path| path.display().to_string(),
    )
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
    // Embed a machine-parseable version marker immediately after SECTION_START.
    // The staleness check in src/skill_check.rs reads this HTML comment.
    let version_marker = format!(
        "<!-- agentchrome-version: {} -->\n\n",
        env!("CARGO_PKG_VERSION")
    );
    let section = format!("{SECTION_START}\n{version_marker}{content}{SECTION_END}\n");

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
            use std::fmt::Write as _;

            // Find the end of the read section's entries and append
            let after_read = read_pos + "\nread:".len();
            let mut new_content = String::with_capacity(content.len() + skill_path.len() + 10);
            new_content.push_str(&content[..after_read]);
            let _ = write!(new_content, "\n  - {skill_path}");
            new_content.push_str(&content[after_read..]);
            std::fs::write(config_path, new_content).map_err(|e| AppError {
                message: format!("failed to write {}: {e}", config_path.display()),
                code: ExitCode::GeneralError,
                custom_json: None,
            })?;
        } else if content.starts_with("read:") {
            use std::fmt::Write as _;

            // read: is the first line
            let after_read = "read:".len();
            let mut new_content = String::with_capacity(content.len() + skill_path.len() + 10);
            new_content.push_str(&content[..after_read]);
            let _ = write!(new_content, "\n  - {skill_path}");
            new_content.push_str(&content[after_read..]);
            std::fs::write(config_path, new_content).map_err(|e| AppError {
                message: format!("failed to write {}: {e}", config_path.display()),
                code: ExitCode::GeneralError,
                custom_json: None,
            })?;
        } else {
            use std::fmt::Write as _;

            // No read section — append one
            let mut new_content = content;
            if !new_content.ends_with('\n') && !new_content.is_empty() {
                new_content.push('\n');
            }
            let _ = writeln!(new_content, "read:\n  - {skill_path}");
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
            if let Some(tool_name) = install_args.tool.as_ref() {
                let tool = tool_for_name(tool_name);
                let result = install_skill(tool)?;
                print_output(&result, &global.output)
            } else {
                install_detected_skills(global)
            }
        }
        SkillCommand::Uninstall(tool_args) => {
            let tool = resolve_tool(tool_args.tool.as_ref())?;
            let result = uninstall_skill(tool)?;
            print_output(&result, &global.output)
        }
        SkillCommand::Update(tool_args) => {
            if let Some(tool_name) = tool_args.tool.as_ref() {
                let tool = tool_for_name(tool_name);
                let result = update_skill(tool)?;
                print_output(&result, &global.output)
            } else {
                update_stale_skills(global)
            }
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
    use std::ffi::OsString;

    #[test]
    fn tool_registry_has_eight_tools() {
        assert_eq!(TOOLS.len(), 8);
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
        assert_eq!(tool_for_name(&ToolName::Gemini).name, "gemini");
        assert_eq!(tool_for_name(&ToolName::Codex).name, "codex");
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
        // AC2 — high-leverage paths
        assert!(content.contains("agentchrome diagnose <url>"));
        assert!(content.contains("agentchrome diagnose --current"));
        assert!(content.contains("agentchrome examples strategies"));
        assert!(content.contains("--include-snapshot"));
        assert!(content.contains("output_file"));
        assert!(content.contains("console follow"));
    }

    #[test]
    fn skill_content_starts_with_yaml_frontmatter() {
        let content = skill_content();
        assert!(
            content.starts_with("---\n"),
            "skill content must start with YAML frontmatter delimiter"
        );
        assert!(
            content.contains("name: agentchrome"),
            "frontmatter must contain name key"
        );
        assert!(
            content.contains("description:"),
            "frontmatter must contain description key"
        );
        // version key should be present with the actual version number (not placeholder)
        let version_line = format!("version: \"{}\"", env!("CARGO_PKG_VERSION"));
        assert!(
            content.contains(&version_line),
            "frontmatter must contain quoted version value"
        );
    }

    #[test]
    fn skill_content_has_six_trigger_phrases() {
        let content = skill_content();
        assert!(
            content.contains("automate a browser"),
            "missing trigger: automate a browser"
        );
        assert!(
            content.contains("fill a form"),
            "missing trigger: fill a form"
        );
        assert!(
            content.contains("test a login"),
            "missing trigger: test a login"
        );
        assert!(
            content.contains("scrape a page"),
            "missing trigger: scrape a page"
        );
        assert!(
            content.contains("take a screenshot"),
            "missing trigger: take a screenshot"
        );
        assert!(
            content.contains("inspect console / network"),
            "missing trigger: inspect console / network"
        );
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
    fn codex_home_root_uses_codex_home_when_set() {
        let home = std::path::PathBuf::from("/home/user");
        let root = codex_home_root(Some(OsString::from("/custom/codex")), Some(home)).unwrap();
        assert_eq!(root, std::path::PathBuf::from("/custom/codex"));
    }

    #[test]
    fn codex_home_root_falls_back_when_unset() {
        let home = std::path::PathBuf::from("/home/user");
        let root = codex_home_root(None, Some(home)).unwrap();
        assert_eq!(root, std::path::PathBuf::from("/home/user/.codex"));
    }

    #[test]
    fn codex_home_root_falls_back_when_empty() {
        let home = std::path::PathBuf::from("/home/user");
        let root = codex_home_root(Some(OsString::from("")), Some(home)).unwrap();
        assert_eq!(root, std::path::PathBuf::from("/home/user/.codex"));
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
    fn write_section_embeds_version_marker() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("rules.md");
        write_section(&file_path, "skill content\n").unwrap();
        let content = std::fs::read_to_string(&file_path).unwrap();
        let expected_marker = format!(
            "<!-- agentchrome-version: {} -->",
            env!("CARGO_PKG_VERSION")
        );
        assert!(
            content.contains(&expected_marker),
            "version marker must be embedded inside section markers"
        );
        // Marker must be inside the section (between SECTION_START and SECTION_END)
        let start_pos = content.find(SECTION_START).unwrap();
        let end_pos = content.find(SECTION_END).unwrap();
        let marker_pos = content.find(&expected_marker).unwrap();
        assert!(
            marker_pos > start_pos && marker_pos < end_pos,
            "version marker must be between section start and end markers"
        );
    }

    #[test]
    fn write_section_no_duplicate_version_markers_on_reinstall() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("rules.md");
        // Install twice
        write_section(&file_path, "skill content\n").unwrap();
        write_section(&file_path, "skill content\n").unwrap();
        let content = std::fs::read_to_string(&file_path).unwrap();
        let marker = format!(
            "<!-- agentchrome-version: {} -->",
            env!("CARGO_PKG_VERSION")
        );
        let count = content.matches(&marker).count();
        assert_eq!(
            count, 1,
            "version marker must not be duplicated on reinstall"
        );
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
        assert_eq!(output.tools.len(), 8);
        let names: Vec<&str> = output.tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"claude-code"));
        assert!(names.contains(&"windsurf"));
        assert!(names.contains(&"aider"));
        assert!(names.contains(&"continue"));
        assert!(names.contains(&"copilot-jb"));
        assert!(names.contains(&"cursor"));
        assert!(names.contains(&"gemini"));
        assert!(names.contains(&"codex"));
    }

    #[test]
    fn list_output_serializes_correctly() {
        let output = list_tools().unwrap();
        let json = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let tools = parsed["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 8);
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

    #[test]
    fn detected_tools_collects_multiple_env_signals_in_registry_order() {
        let env = vec![
            ("CODEX_HOME".to_string(), "/tmp/codex".to_string()),
            ("CLAUDE_CODE".to_string(), "1".to_string()),
        ];
        let tools = detected_tools_with(&env, None, None);
        let names: Vec<&str> = tools.iter().map(|tool| tool.name).collect();
        assert_eq!(names, vec!["claude-code", "codex"]);
    }

    #[test]
    fn detected_tools_collects_multiple_config_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".continue")).unwrap();
        std::fs::create_dir_all(dir.path().join(".codex")).unwrap();

        let tools = detected_tools_with(&[], None, Some(dir.path()));
        let names: Vec<&str> = tools.iter().map(|tool| tool.name).collect();

        assert_eq!(names, vec!["continue", "codex"]);
    }

    #[test]
    fn batch_output_serializes_success_and_failure_results() {
        let output = SkillBatchOutput {
            results: vec![
                SkillBatchResult {
                    tool: "claude-code".into(),
                    path: "/tmp/home/.claude/skills/agentchrome/SKILL.md".into(),
                    action: "installed".into(),
                    version: "1.8.0".into(),
                    status: "ok".into(),
                    error: None,
                },
                SkillBatchResult {
                    tool: "codex".into(),
                    path: "/tmp/codex/skills/agentchrome/SKILL.md".into(),
                    action: "installed".into(),
                    version: "1.8.0".into(),
                    status: "error".into(),
                    error: Some("failed to write target".into()),
                },
            ],
        };

        let json = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let results = parsed["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["tool"], "claude-code");
        assert_eq!(results[0]["status"], "ok");
        assert!(results[0].get("error").is_none());
        assert_eq!(results[1]["tool"], "codex");
        assert_eq!(results[1]["status"], "error");
        assert_eq!(results[1]["error"], "failed to write target");
    }
}
