# Requirements: Add agentchrome skill Command Group

**Issues**: #172
**Date**: 2026-03-12
**Status**: Draft
**Author**: Claude (AI-assisted)

---

## User Story

**As an** AI agent or developer using an agentic coding tool (Claude Code, Cursor, Windsurf, Aider, Continue.dev, GitHub Copilot)
**I want** to run a single command that installs a concise agentchrome skill/instruction into my agentic tool at the user level
**So that** the AI agent automatically knows when to use agentchrome and how to discover its capabilities through the CLI's own help system

---

## Background

agentchrome is an AI-native CLI tool with rich built-in help (`--help`, `capabilities`, `examples`, `man`), but agentic coding tools have no built-in awareness that agentchrome exists or when to reach for it. Each major agentic platform supports user-level persistent instructions or skills. A `skill install` command would detect the running agentic environment and install a minimal pointer skill — telling the agent only: (1) what agentchrome is for, (2) when to use it, and (3) how to invoke its help to discover functionality. The skill is a signpost, not a manual — it must NOT duplicate content already available through CLI help.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Auto-detected installation

**Given** an agentic coding tool (e.g., Claude Code) is the parent environment
**When** the user runs `agentchrome skill install`
**Then** the command detects the active tool, installs the concise agentchrome skill to its user-level location, and prints a JSON result on stdout confirming the tool name and installed file path

**Example**:
- Given: `CLAUDE_CODE` env var is set
- When: `agentchrome skill install`
- Then: stdout contains `{"tool":"claude-code","path":"~/.claude/skills/agentchrome/SKILL.md","action":"installed"}`

### AC2: Explicit tool targeting

**Given** a user wants to install for a specific tool regardless of environment
**When** the user runs `agentchrome skill install --tool cursor`
**Then** the skill is installed to the correct location for Cursor and a JSON confirmation is returned on stdout

**Example**:
- Given: No particular agentic environment is active
- When: `agentchrome skill install --tool cursor`
- Then: stdout contains `{"tool":"cursor","path":".cursor/rules/agentchrome.mdc","action":"installed"}`

### AC3: List supported tools

**Given** a user wants to know which tools are supported
**When** the user runs `agentchrome skill list`
**Then** a JSON array of supported tool objects is returned on stdout, each containing the tool name, detection method, and install path

**Example**:
- When: `agentchrome skill list`
- Then: stdout contains a JSON array with entries like `{"name":"claude-code","detection":"CLAUDE_CODE env var or 'claude' parent process","path":"~/.claude/skills/agentchrome/SKILL.md"}`

### AC4: Uninstall

**Given** an agentchrome skill was previously installed for a detected tool
**When** the user runs `agentchrome skill uninstall`
**Then** the installed file is removed and a JSON confirmation is returned on stdout

**Example**:
- Given: Skill was installed at `~/.claude/skills/agentchrome/SKILL.md`
- When: `agentchrome skill uninstall`
- Then: stdout contains `{"tool":"claude-code","path":"~/.claude/skills/agentchrome/SKILL.md","action":"uninstalled"}`

### AC5: Update installed skill

**Given** an agentchrome skill was previously installed and the agentchrome version has changed
**When** the user runs `agentchrome skill update`
**Then** the installed skill file is replaced with the current version's content and a JSON confirmation is returned on stdout

**Example**:
- Given: Skill was installed with version 1.7.0, now running 1.8.0
- When: `agentchrome skill update`
- Then: stdout contains `{"tool":"claude-code","path":"~/.claude/skills/agentchrome/SKILL.md","action":"updated","version":"1.8.0"}`

### AC6: Unknown environment

**Given** no supported agentic tool can be detected
**When** the user runs `agentchrome skill install` without `--tool`
**Then** the command exits with a non-zero exit code and a JSON error on stderr listing supported tools and their manual install paths

**Example**:
- Given: No agentic env vars set, no matching parent process, no tool config dirs found
- When: `agentchrome skill install`
- Then: exit code is 1, stderr contains `{"error":"no supported agentic tool detected","supported_tools":[...]}`

### AC7: Cross-validate install via list

**Given** a skill was installed via `agentchrome skill install --tool claude-code`
**When** the user runs `agentchrome skill list`
**Then** the Claude Code entry in the list shows `"installed": true` indicating the skill file exists at the expected path

### AC8: Uninstall with explicit --tool flag

**Given** a skill was previously installed for Aider
**When** the user runs `agentchrome skill uninstall --tool aider`
**Then** the installed Aider skill file is removed and a JSON confirmation is returned on stdout

### AC9: Install idempotency

**Given** a skill is already installed for a tool
**When** the user runs `agentchrome skill install` for the same tool again
**Then** the command overwrites the existing file with the current version's content and returns a JSON confirmation with `"action":"installed"` (no error)

### AC10: Detection priority order

**Given** multiple agentic tool signals are present (e.g., `CLAUDE_CODE` env var and `~/.continue/` directory both exist)
**When** the user runs `agentchrome skill install`
**Then** the tool detected via the highest-priority signal (env vars first, then parent process, then config dirs) is selected

### AC11: README features skill install in setup

**Given** a user or AI agent reads the project README
**When** they look at the setup/quickstart section
**Then** `agentchrome skill install` is prominently featured as the recommended first step for AI agent integration, ahead of manual configuration, and `agentchrome skill update` is documented as the way to refresh the skill after upgrading agentchrome

### AC12: All subcommand JSON output compliance

**Given** any `agentchrome skill` subcommand is invoked
**When** the command completes (success or error)
**Then** success output is JSON on stdout and error output is JSON on stderr, consistent with the global output contract

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Detect running agentic tool via env vars and parent process inspection | Must | Detection priority: env vars > parent process > config dirs |
| FR2 | Install concise skill to correct user-level path for detected tool | Must | |
| FR3 | Skill content is a minimal signpost: when to use agentchrome (browser automation via CDP) and how to discover functionality (`agentchrome --help`, `agentchrome capabilities`, `agentchrome examples`). Must NOT duplicate CLI help content. | Must | |
| FR4 | All output is JSON on stdout; errors JSON on stderr | Must | Consistent with global output contract |
| FR5 | Support: Claude Code, Windsurf, Aider, Continue.dev | Must | |
| FR6 | Support: GitHub Copilot (JetBrains global file) | Should | |
| FR7 | Support: Cursor (project-level `.cursor/rules/`) | Could | Cursor only has project-level path |
| FR8 | `--tool <name>` flag for explicit override of auto-detection | Must | |
| FR9 | `agentchrome skill list` returns JSON array of supported tools with detection method, path, and installed status | Should | |
| FR10 | `agentchrome skill uninstall [--tool <name>]` removes previously installed skill | Should | |
| FR11 | `agentchrome skill update [--tool <name>]` replaces installed skill with current version content | Should | |
| FR12 | For tools that use append-based install (Windsurf, GitHub Copilot), install appends a clearly delimited section; uninstall removes only the agentchrome section | Must | Use markers like `<!-- agentchrome:start -->` / `<!-- agentchrome:end -->` |
| FR13 | For Aider, install creates `~/.aider/agentchrome.md` and adds a `read` entry to `~/.aider.conf.yml` | Must | Uninstall reverses both |
| FR14 | `skill list` shows `installed` field per tool indicating whether the skill file currently exists at the expected path | Should | |
| FR15 | Update README.md to feature `agentchrome skill install` prominently in the setup/quickstart section, positioning it as the recommended first step for AI agent users. Include documentation for `agentchrome skill update` as the recommended post-upgrade step to refresh the installed skill. | Must | Make skill install the focus of the setup flow; document update workflow |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Sub-50ms execution for all skill subcommands (no Chrome/CDP needed) |
| **Security** | Only writes to user-controlled paths; no network access; no secrets stored |
| **Reliability** | Graceful handling of missing directories (create them); atomic file writes where possible |
| **Platforms** | macOS, Linux, Windows (per tech.md) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--tool` | String (enum) | Must be one of: `claude-code`, `windsurf`, `aider`, `continue`, `copilot-jb`, `cursor` | No (auto-detect if omitted) |
| Subcommand | Enum | Must be one of: `install`, `uninstall`, `update`, `list` | Yes |

### Output Data (install/uninstall/update)

| Field | Type | Description |
|-------|------|-------------|
| `tool` | String | Name of the agentic tool |
| `path` | String | File path where skill was installed/removed |
| `action` | String | One of: `installed`, `uninstalled`, `updated` |
| `version` | String | agentchrome version (for install/update) |

### Output Data (list)

| Field | Type | Description |
|-------|------|-------------|
| `tools` | Array | Array of tool objects |
| `tools[].name` | String | Tool identifier |
| `tools[].detection` | String | How the tool is detected |
| `tools[].path` | String | Install path for the skill |
| `tools[].installed` | Boolean | Whether a skill file currently exists at the path |

---

## Tool Detection & Install Paths

| Tool | Detection Signal | User-level Install Path |
|------|-----------------|------------------------|
| Claude Code | `CLAUDE_CODE` env or `claude` in parent process | `~/.claude/skills/agentchrome/SKILL.md` |
| Windsurf | `WINDSURF_*` env or `~/.codeium/` exists | `~/.codeium/windsurf/memories/global_rules.md` (append section) |
| Aider | `AIDER_*` env or `aider` in parent process | `~/.aider/agentchrome.md` + `read` entry in `~/.aider.conf.yml` |
| Continue.dev | `~/.continue/` exists | `~/.continue/rules/agentchrome.md` |
| GitHub Copilot (JB) | `~/.config/github-copilot/` exists | `~/.config/github-copilot/intellij/global-copilot-instructions.md` (append section) |
| Cursor | `CURSOR_*` env or `~/.cursor/` exists | `.cursor/rules/agentchrome.mdc` (project-level only) |

---

## Dependencies

### Internal Dependencies
- [x] CLI framework (clap) — already in place
- [x] JSON output infrastructure — already in place
- [x] Error types (`AppError`, exit codes) — already in place

### External Dependencies
- None (no Chrome/CDP needed)

---

## Out of Scope

- GUI or interactive installer
- Project-level skill installation (user-level only, except Cursor which has no user-level path)
- Firefox/Safari browser tool support
- Automatic updates (agent must run `skill update` manually)
- Skill content customization by the user
- MCP server integration or other non-file-based skill delivery

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Supported tools | 6 (Claude Code, Windsurf, Aider, Continue.dev, Copilot JB, Cursor) | Count of working tool integrations |
| Install round-trip | install → list shows installed → uninstall → list shows not installed | End-to-end verification |
| Startup overhead | < 50ms for all skill subcommands | Benchmark timing |

---

## Open Questions

- [x] None — all requirements are derivable from the issue

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #172 | 2026-03-12 | Initial feature spec |

---

## Validation Checklist

Before moving to PLAN phase:

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states specified (AC6, AC9, AC10)
- [x] Dependencies identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
