# Design: Add agentchrome skill Command Group

**Issues**: #172, #214, #263
**Date**: 2026-04-24
**Status**: Draft
**Author**: Claude (AI-assisted)

---

## Overview

This feature adds a new `agentchrome skill` command group that enables agentic coding tools to discover agentchrome through their native instruction/skill systems. The implementation follows the same pattern as `capabilities.rs` and `examples.rs` — a non-async command module that requires no Chrome/CDP connection. The module detects the user's agentic environment through a prioritized heuristic chain (env vars > parent process > config directories), then writes a minimal signpost skill file to the tool's user-level instruction path.

The skill content is a static, version-stamped template embedded in the binary. For tools that use dedicated files (Claude Code, Aider, Continue.dev, Cursor), the module writes a standalone file. For tools that share a single global rules file (Windsurf, GitHub Copilot JB), the module appends a delimited section. The README is updated to position `skill install` as the primary setup step for AI agent users.

---

## Architecture

### Component Diagram

```
CLI Layer (cli/mod.rs)
    ↓
    SkillArgs → SkillCommand enum
    ↓
main.rs dispatch (non-async, like examples/capabilities)
    ↓
┌───────────────────────────────────────────────────┐
│                   skill.rs                         │
├───────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐               │
│  │  Detection    │  │  Skill       │               │
│  │  (env, proc,  │  │  Content     │               │
│  │   config dir) │  │  (static     │               │
│  └──────┬───────┘  │   template)  │               │
│         │          └──────┬───────┘               │
│         ▼                 ▼                        │
│  ┌──────────────────────────────────┐             │
│  │  Install / Uninstall / Update    │             │
│  │  (file write, append-section,    │             │
│  │   config patching)               │             │
│  └──────────────────────────────────┘             │
│                    ↓                               │
│         JSON output on stdout                      │
└───────────────────────────────────────────────────┘
    ↓
File System (user home dirs)
```

### Data Flow

```
1. User runs `agentchrome skill install [--tool <name>]`
2. CLI layer parses args into SkillArgs / SkillCommand
3. main.rs dispatches to skill::execute_skill() (non-async)
4. If --tool provided, use that; otherwise run detection heuristic
5. Detection checks env vars → parent process name → config dir existence
6. Resolve install path for detected tool (expanding ~ to home dir)
7. Generate skill content from static template with version stamp
8. Write file (standalone) or append delimited section (shared file)
9. For Aider: also patch ~/.aider.conf.yml with read entry
10. Print JSON result on stdout
```

---

## API / Interface Changes

### New CLI Commands

| Command | Type | Purpose |
|---------|------|---------|
| `agentchrome skill install [--tool <name>]` | Non-async | Install skill to detected/specified tool |
| `agentchrome skill uninstall [--tool <name>]` | Non-async | Remove installed skill |
| `agentchrome skill update [--tool <name>]` | Non-async | Replace installed skill with current version |
| `agentchrome skill list` | Non-async | List supported tools and install status |

### CLI Args Structure

```rust
// In cli/mod.rs

/// Arguments for the `skill` subcommand group.
#[derive(Args)]
pub struct SkillArgs {
    #[command(subcommand)]
    pub command: SkillCommand,
}

/// Skill subcommands.
#[derive(Subcommand)]
pub enum SkillCommand {
    /// Install the agentchrome skill for an agentic coding tool
    Install(SkillInstallArgs),
    /// Remove a previously installed agentchrome skill
    Uninstall(SkillToolArgs),
    /// Update an installed skill to the current version
    Update(SkillToolArgs),
    /// List supported agentic tools and installation status
    List,
}

#[derive(Args)]
pub struct SkillInstallArgs {
    /// Target tool (auto-detected if omitted)
    #[arg(long, value_enum)]
    pub tool: Option<ToolName>,
}

#[derive(Args)]
pub struct SkillToolArgs {
    /// Target tool (auto-detected if omitted)
    #[arg(long, value_enum)]
    pub tool: Option<ToolName>,
}

#[derive(Clone, ValueEnum)]
pub enum ToolName {
    ClaudeCode,
    Windsurf,
    Aider,
    Continue,
    CopilotJb,
    Cursor,
    Gemini,
}
```

### Output Schemas

**install / uninstall / update (success — stdout):**
```json
{
  "tool": "claude-code",
  "path": "/Users/name/.claude/skills/agentchrome/SKILL.md",
  "action": "installed",
  "version": "1.8.0"
}
```

**list (success — stdout):**
```json
{
  "tools": [
    {
      "name": "claude-code",
      "detection": "CLAUDE_CODE env var or 'claude' in parent process",
      "path": "~/.claude/skills/agentchrome/SKILL.md",
      "installed": true
    }
  ]
}
```

**Error (stderr):**
```json
{
  "error": "no supported agentic tool detected",
  "supported_tools": ["claude-code", "windsurf", "aider", "continue", "copilot-jb", "cursor", "gemini"]
}
```

---

## Module Design: `src/skill.rs`

### Core Types

```rust
/// Metadata for a supported agentic tool.
struct ToolInfo {
    name: &'static str,           // e.g., "claude-code"
    detection: &'static str,      // human-readable detection method
    install_mode: InstallMode,    // how the skill is installed
}

enum InstallMode {
    /// Write a standalone file (create dirs as needed)
    Standalone { path_template: &'static str },
    /// Append a delimited section to a shared file
    AppendSection { path_template: &'static str },
    /// Write standalone file + patch a config file
    StandaloneWithConfig {
        skill_path_template: &'static str,
        config_path_template: &'static str,
    },
}
```

### Tool Registry

A static array of `ToolInfo` entries, one per supported tool. The registry is the single source of truth for tool names, detection descriptions, and install paths. Detection and install logic are driven by this data.

### Detection Heuristic

Detection follows a strict priority order, returning the first match:

1. **Environment variables** (highest priority):
   - `CLAUDE_CODE` → Claude Code
   - `WINDSURF_*` (any env var starting with `WINDSURF_`) → Windsurf
   - `AIDER_*` (any env var starting with `AIDER_`) → Aider
   - `CURSOR_*` (any env var starting with `CURSOR_`) → Cursor
   - `GEMINI_*` (any env var starting with `GEMINI_`) → Gemini CLI

2. **Parent process name**:
   - Process tree contains `claude` → Claude Code
   - Process tree contains `aider` → Aider

3. **Config directory existence** (lowest priority):
   - `~/.codeium/` exists → Windsurf
   - `~/.continue/` exists → Continue.dev
   - `~/.config/github-copilot/` exists → GitHub Copilot JB
   - `~/.cursor/` exists → Cursor
   - `~/.gemini/` exists → Gemini CLI

Parent process inspection uses `std::env::var("_")` on Unix (which contains the parent process path in many shells) or falls back to platform-specific APIs if needed. The detection is best-effort — the `--tool` flag exists for cases where auto-detection is insufficient.

### Install Strategies

**Standalone file** (Claude Code, Continue.dev, Cursor, Gemini CLI):
1. Resolve `~` to `$HOME`
2. Create parent directories if missing (`std::fs::create_dir_all`)
3. Write the skill content to the file (overwrite if exists)

**Append section** (Windsurf, GitHub Copilot JB):
1. Resolve path, create parent dirs
2. If file exists, read it
3. If agentchrome section markers already present, replace the section content
4. If no markers present, append section with markers
5. Section delimiters: `<!-- agentchrome:start -->` / `<!-- agentchrome:end -->`

**Standalone with config** (Aider):
1. Write standalone skill file to `~/.aider/agentchrome.md`
2. Read `~/.aider.conf.yml` (or create if missing)
3. If `read:` key exists, append `~/.aider/agentchrome.md` if not already listed
4. If `read:` key doesn't exist, add `read:\n  - ~/.aider/agentchrome.md`

### Uninstall Strategies

- **Standalone**: Delete the file. Delete empty parent dirs.
- **Append section**: Read file, remove content between markers (inclusive), write back. If file becomes empty, delete it.
- **Standalone with config**: Delete skill file. Remove the entry from `~/.aider.conf.yml` `read:` list.

### Skill Content Template

A static string compiled into the binary. Minimal content — signpost only:

```markdown
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
```

The `{version}` placeholder is replaced at runtime with the binary's version from `VERSION`.

---

## Design Amendment: Codex Skill Target (#263)

Issue #263 extends the existing registry-driven skill installer with Codex as the eighth supported tool. Codex fits the existing `Standalone` install model, but its install root is dynamic: `$CODEX_HOME` when set, otherwise `~/.codex`. The implementation should keep Codex in the same `TOOLS` registry used by install/list/update/uninstall and by `src/skill_check.rs`, while adding one narrow path-resolution branch for Codex's environment-sensitive root.

### CLI Surface Amendment

Add `Codex` to `src/cli/mod.rs`'s `ToolName` enum so `clap` accepts `--tool codex` across `skill install`, `skill update`, and `skill uninstall`.

```rust
#[derive(Debug, Clone, ValueEnum)]
pub enum ToolName {
    ClaudeCode,
    Windsurf,
    Aider,
    Continue,
    CopilotJb,
    Cursor,
    Gemini,
    Codex,
}
```

The command schemas stay unchanged. `codex` is an additional enum value, not a new command or output shape. Existing JSON stdout and stderr contracts remain unchanged.

### Registry Amendment

Add a Codex entry to `src/skill.rs::TOOLS`:

```rust
ToolInfo {
    name: "codex",
    detection: "CODEX_HOME env var or ~/.codex/ directory exists",
    install_mode: InstallMode::Standalone {
        path_template: "$CODEX_HOME/skills/agentchrome/SKILL.md",
    },
}
```

Because `$CODEX_HOME` has a fallback, the path resolver must not treat the literal `$CODEX_HOME` string as a filesystem segment. Two implementation options are acceptable:

1. Add a Codex-specific `InstallMode` variant such as `CodexStandalone { env_var, fallback_home_relative }`.
2. Keep `Standalone` and teach `resolve_path()` to recognize the exact `$CODEX_HOME/` prefix and fall back to `~/.codex/` when unset.

Option 2 is the smaller change and preserves the current registry shape. It is selected as long as the prefix handling is exact and covered by tests. Do not add general shell-style environment interpolation; the project only needs this one Codex root rule.

### Detection Amendment

Extend `detect_tool()` in `src/skill.rs` without changing the tier order:

1. Tier 1 environment variables:
   - `CODEX_HOME` -> Codex
2. Tier 2 parent process:
   - No Codex parent-process detection in this issue. Codex invocations may run through different host process names, and `CODEX_HOME` / config-dir signals are more reliable.
3. Tier 3 config directories:
   - `~/.codex/` exists -> Codex

Codex should be checked after the existing explicit env-var signals unless a code review finds a stronger project-local convention for inserting new tools. AC21 only requires Codex selection when no higher-priority explicit tool signal applies, so this preserves existing detection behavior.

### Path Resolution Amendment

`src/skill.rs::resolve_path()` currently expands `~/` against `dirs::home_dir()`. Add exact support for the Codex template:

```text
$CODEX_HOME/skills/agentchrome/SKILL.md
```

Resolution rules:

| Condition | Resolved path |
|-----------|---------------|
| `CODEX_HOME=/custom/codex` | `/custom/codex/skills/agentchrome/SKILL.md` |
| `CODEX_HOME` unset | `~/.codex/skills/agentchrome/SKILL.md` |
| `CODEX_HOME` set to empty string | Treat as unset and use `~/.codex/skills/agentchrome/SKILL.md` |

This resolver is shared by `install`, `update`, `uninstall`, `list`, and `skill_check`, so one implementation point keeps lifecycle behavior and staleness checks aligned.

### Skill Check Amendment

`src/skill_check.rs::stale_tools()` already iterates `crate::skill::TOOLS` and calls `path_template()` plus `resolve_path()`. After Codex is in `TOOLS` and `resolve_path()` handles `$CODEX_HOME`, Codex becomes part of the stale-skill scan automatically. Tests must cover:

- Codex-only stale notice names `codex`.
- Multi-tool stale notice includes `codex` in the aggregated list.
- Suppression via `AGENTCHROME_NO_SKILL_CHECK=1` and `skill.check_enabled = false` still applies.

### Documentation Amendment

Update the existing documentation locations that describe skill installer targets:

| File | Change |
|------|--------|
| `README.md` | Add Codex to supported tools and show `agentchrome skill install --tool codex`. |
| `docs/codex.md` | Prefer `agentchrome skill install --tool codex` as the native Codex setup path and document `$CODEX_HOME` fallback behavior. |
| `examples/AGENTS.md.example` | Mention that Codex users can install the AgentChrome skill with `agentchrome skill install --tool codex`. |

The shared `SKILL_TEMPLATE` remains compact and reusable. Codex does not receive a custom template in this issue.

### Test Amendment

Extend existing tests instead of creating a parallel test harness:

| File | Coverage |
|------|----------|
| `tests/features/skill-command-group.feature` | Add Codex scenarios matching AC19-AC22 and AC24. |
| `tests/features/skill-staleness.feature` | Add Codex-only and multi-tool aggregation scenarios for AC23. |
| `tests/bdd.rs` | Add Codex path helpers and step bindings analogous to the existing Gemini helpers. |
| `src/skill.rs` unit tests | Update registry count, `tool_for_name()` mapping, path resolution, list output, and Codex detection. |
| `src/skill_check.rs` unit tests or BDD | Verify stale notice formatting and registry iteration include Codex. |

### Risk Amendment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `$CODEX_HOME` fallback diverges between install/list and staleness check | Medium | Medium | Route every caller through `resolve_path()` and add tests for both set and unset `CODEX_HOME`. |
| Codex detection could preempt an existing tool signal in mixed-agent environments | Low | Medium | Preserve the existing detection tier order and place Codex after existing Tier 1 signals. |
| Empty `CODEX_HOME` creates a relative or invalid path | Low | Medium | Treat empty values as unset and fall back to `~/.codex`. |
| Registry-count tests become brittle when adding the eighth tool | Medium | Low | Update expected count from 7 to 8 and add a per-tool assertion for Codex. |

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: MCP-based skill delivery** | Serve skill content via an MCP server | Works with MCP-native tools | Requires running server, doesn't work for non-MCP tools | Rejected — adds runtime dependency |
| **B: Static file generation (selected)** | Embed skill template in binary, write to filesystem | Zero dependencies, works offline, covers all tools | Must update binary to change skill content | **Selected** — aligns with zero-config philosophy |
| **C: Download skill from GitHub** | Fetch latest skill from a URL at install time | Always up-to-date | Requires network access, breaks air-gapped setups | Rejected — violates "local only" principle |

---

## Security Considerations

- [x] **File paths**: Only writes to user-owned directories under `$HOME` (except Cursor which is project-level)
- [x] **No secrets**: No credentials or tokens stored
- [x] **No network**: Entirely local filesystem operations
- [x] **Input validation**: `--tool` is a clap `ValueEnum` — invalid values rejected at parse time
- [x] **Path traversal**: All paths are constructed from known templates with `$HOME` prefix, no user-controlled path segments

---

## Performance Considerations

- [x] **No CDP/Chrome**: All operations are pure filesystem I/O
- [x] **No async**: Synchronous execution like `examples.rs` and `capabilities.rs`
- [x] **Startup**: < 50ms — no network calls, no process spawning (except optional parent process check)

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Tool registry | Unit | All 7 tools have name, detection, and path |
| Detection logic | Unit | Env var detection, parent process detection, config dir detection |
| Install mode | Unit | Standalone write, append-section with markers, config patching |
| Uninstall mode | Unit | Standalone delete, section removal, config cleanup |
| Path resolution | Unit | `~` expansion, directory creation |
| Output format | Unit | JSON serialization matches schema |
| Idempotency | Unit | Install over existing file succeeds |
| CLI integration | BDD | End-to-end: install → list → uninstall → list |
| Error case | BDD | No tool detected → JSON error on stderr |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Parent process detection unreliable on some platforms | Medium | Low | `--tool` flag as explicit override; env vars are primary detection |
| Aider config YAML parsing edge cases | Low | Medium | Use simple line-based manipulation rather than full YAML parser |
| Tool updates change install paths | Low | Medium | Path constants are easy to update; `skill list` shows current paths |
| Append-section markers corrupted by user edits | Low | Low | Use HTML comment markers unlikely to be modified |

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #172 | 2026-03-12 | Initial feature spec |
| #214 | 2026-04-16 | Add Gemini CLI: `Gemini` variant in `ToolName`, `ToolInfo` entry with `Standalone` mode, Tier 1 + Tier 3 detection, README update |
| #263 | 2026-04-24 | Add Codex target with CODEX_HOME-aware path resolution, registry support, staleness coverage, docs, and tests |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All CLI changes documented with schemas
- [x] No database/storage changes needed
- [x] State management approach is clear (stateless — filesystem only)
- [x] No UI components needed (CLI only)
- [x] Security considerations addressed
- [x] Performance impact analyzed (< 50ms, no CDP)
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
