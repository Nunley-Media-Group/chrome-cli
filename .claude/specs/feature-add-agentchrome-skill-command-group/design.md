# Design: Add agentchrome skill Command Group

**Issues**: #172
**Date**: 2026-03-12
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
  "supported_tools": ["claude-code", "windsurf", "aider", "continue", "copilot-jb", "cursor"]
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

2. **Parent process name**:
   - Process tree contains `claude` → Claude Code
   - Process tree contains `aider` → Aider

3. **Config directory existence** (lowest priority):
   - `~/.codeium/` exists → Windsurf
   - `~/.continue/` exists → Continue.dev
   - `~/.config/github-copilot/` exists → GitHub Copilot JB
   - `~/.cursor/` exists → Cursor

Parent process inspection uses `std::env::var("_")` on Unix (which contains the parent process path in many shells) or falls back to platform-specific APIs if needed. The detection is best-effort — the `--tool` flag exists for cases where auto-detection is insufficient.

### Install Strategies

**Standalone file** (Claude Code, Continue.dev, Cursor):
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
| Tool registry | Unit | All 6 tools have name, detection, and path |
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
