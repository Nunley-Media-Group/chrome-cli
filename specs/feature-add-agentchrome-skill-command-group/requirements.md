# Requirements: Add agentchrome skill Command Group

**Issues**: #172, #214, #263, #268
**Date**: 2026-04-25
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

### AC13: Gemini skill installs successfully

**Given** agentchrome is built with Gemini support
**When** the user runs `agentchrome skill install --tool gemini`
**Then** a standalone skill file is created at `~/.gemini/instructions/agentchrome.md` containing the standard skill template with the current version

**Example**:
- When: `agentchrome skill install --tool gemini`
- Then: stdout contains `{"tool":"gemini","path":"~/.gemini/instructions/agentchrome.md","action":"installed","version":"..."}`

### AC14: Gemini appears in skill list

**Given** agentchrome supports Gemini
**When** the user runs `agentchrome skill list`
**Then** the JSON output includes a `gemini` entry with the correct path (`~/.gemini/instructions/agentchrome.md`), detection description, and installed status

### AC15: Gemini auto-detection works

**Given** a `GEMINI_*` environment variable (e.g. `GEMINI_API_KEY`) is set, or `~/.gemini/` directory exists
**When** the user runs `agentchrome skill install` without `--tool`
**Then** Gemini is detected and the skill is installed to `~/.gemini/instructions/agentchrome.md`

### AC16: Gemini skill uninstalls cleanly

**Given** the Gemini skill is installed at `~/.gemini/instructions/agentchrome.md`
**When** the user runs `agentchrome skill uninstall --tool gemini`
**Then** the file is removed and empty parent directories are cleaned up

### AC17: Gemini skill updates in place

**Given** the Gemini skill is already installed
**When** the user runs `agentchrome skill update --tool gemini`
**Then** the skill file is overwritten with the latest version content

### AC18: README lists Gemini as a supported tool

**Given** the project README.md documents the skill installer
**When** a user reads the supported tools section
**Then** Gemini CLI is listed alongside the other 6 tools with its install path and detection method

### AC12: All subcommand JSON output compliance

**Given** any `agentchrome skill` subcommand is invoked
**When** the command completes (success or error)
**Then** success output is JSON on stdout and error output is JSON on stderr, consistent with the global output contract

### AC19: Codex skill installs explicitly

**Given** no particular agentic environment is active
**When** the user runs `agentchrome skill install --tool codex`
**Then** the command exits 0
**And** stdout contains JSON with `"tool": "codex"` and `"action": "installed"`
**And** AgentChrome writes `SKILL.md` to `$CODEX_HOME/skills/agentchrome/SKILL.md` when `CODEX_HOME` is set
**And** AgentChrome writes `SKILL.md` to `~/.codex/skills/agentchrome/SKILL.md` when `CODEX_HOME` is not set

### AC20: Codex appears in skill list

**Given** the skill command is available
**When** the user runs `agentchrome skill list`
**Then** stdout contains JSON with a `tools` array
**And** the array includes an entry with `"name": "codex"`
**And** the Codex entry includes `detection`, `path`, and `installed` fields
**And** `installed` reflects whether the Codex skill file exists at the resolved install path

### AC21: Codex auto-detection works

**Given** Codex-specific signals are present, such as `CODEX_HOME` or an existing `~/.codex/` directory
**When** the user runs `agentchrome skill install` without `--tool`
**Then** Codex is selected when no higher-priority explicit tool signal applies
**And** the skill is installed to the resolved Codex skill path
**And** stdout reports `"tool": "codex"`

### AC22: Codex lifecycle commands work

**Given** a Codex skill was previously installed
**When** the user runs `agentchrome skill update --tool codex`
**Then** the skill file is rewritten with the current AgentChrome skill template
**And** stdout reports `"action": "updated"`

**Given** a Codex skill was previously installed
**When** the user runs `agentchrome skill uninstall --tool codex`
**Then** the Codex skill file is removed
**And** stdout reports `"action": "uninstalled"`

### AC23: Staleness check includes Codex

**Given** an installed Codex skill has an older embedded version than the AgentChrome binary
**When** any `agentchrome` command is invoked
**Then** stderr contains exactly one staleness notice line using the existing format
**And** the notice names `codex` when only the Codex skill is stale
**And** the notice includes `codex` in the aggregated stale-tool list when multiple skills are stale
**And** existing suppression via `AGENTCHROME_NO_SKILL_CHECK=1` and config still applies

### AC24: Documentation and tests cover Codex

**Given** Codex support is implemented
**When** the README, Codex guide, examples, and BDD tests are reviewed
**Then** Codex is documented as a supported skill installer target
**And** the docs show `agentchrome skill install --tool codex`
**And** focused BDD or unit tests cover install, list, detection, update, uninstall, and staleness behavior for Codex

### AC25: Bare update refreshes every stale installed skill

**Given** AgentChrome skills are installed for multiple supported tools
**And** more than one installed skill has an embedded version older than the running AgentChrome binary
**When** the user runs `agentchrome skill update` without `--tool`
**Then** AgentChrome scans all supported skill locations for outdated AgentChrome skills
**And** append-section installs are discovered by scanning the AgentChrome section markers even when shared-file content places that section after the first 20 lines
**And** every stale installed skill whose path can be resolved and written is updated to the current version
**And** stdout returns structured JSON naming every target touched, with each target's tool name, path, action, and version
**And** a subsequent AgentChrome invocation no longer emits stale notices for those updated targets

### AC26: Bare update does not stop at the first detected tool

**Given** one supported agent has a higher-priority detection signal
**And** another supported agent also has an installed stale AgentChrome skill
**When** the user runs `agentchrome skill update` without `--tool`
**Then** the lower-priority stale install is updated too
**And** the result is not limited to the tool that `detect_tool()` would have returned first

### AC27: Bare install installs into all detected agents

**Given** signals for multiple supported agentic tools are present
**When** the user runs `agentchrome skill install` without `--tool`
**Then** AgentChrome installs the current skill into every detected supported agent target
**And** stdout returns structured JSON naming every installed target and path
**And** no detected target is skipped merely because another target appeared earlier in detection priority

### AC28: Explicit targeting remains single-target

**Given** the user passes `--tool <name>` to `skill install`, `skill update`, or `skill uninstall`
**When** the command runs
**Then** AgentChrome operates only on the specified target
**And** the existing explicit-target JSON/error contract remains compatible

### AC29: Multi-target failures are reported per target

**Given** a bare multi-target install or update finds more than one relevant target
**And** one target cannot be written or cannot be resolved
**When** the command runs
**Then** AgentChrome reports success or failure for each target in structured output
**And** successful targets remain updated or installed
**And** the process exits non-zero if any target failed

### AC30: Staleness notice guidance is actionable

**Given** AgentChrome emits a stale notice that names multiple tools and recommends `agentchrome skill update`
**When** the user runs that exact command
**Then** the command updates all stale tools named in the notice where the target paths are writable
**And** the user does not need to rerun `agentchrome skill update --tool <name>` for each stale tool

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
| FR16 | Add `Gemini` variant to `ToolName` enum in `src/cli/mod.rs` | Must | Issue #214 |
| FR17 | Add `ToolInfo` entry to `TOOLS` registry in `src/skill.rs` with `Standalone` install mode and path `~/.gemini/instructions/agentchrome.md` | Must | Issue #214 |
| FR18 | Add `tool_for_name` mapping for `ToolName::Gemini` → `"gemini"` in `src/skill.rs` | Must | Issue #214 |
| FR19 | Add Tier 1 detection for `GEMINI_*` env var prefix in `detect_tool()` | Must | Issue #214 |
| FR20 | Add Tier 3 detection for `~/.gemini/` directory existence in `detect_tool()` | Must | Issue #214 |
| FR21 | Update unit tests: registry count (6→7), `tool_for_name` mapping, list output count, and add Gemini-specific assertions | Must | Issue #214 |
| FR22 | Update README.md to list Gemini CLI as a supported tool in the skill installer section, including the `--tool gemini` example and `~/.gemini/instructions/agentchrome.md` install path | Must | Issue #214 |
| FR23 | Add `Codex` to the `ToolName` enum and map it to the CLI value `codex`. | Must | Issue #263 |
| FR24 | Add a Codex `ToolInfo` entry to the `TOOLS` registry. | Must | Issue #263 |
| FR25 | Resolve the Codex install path as `$CODEX_HOME/skills/agentchrome/SKILL.md` when `CODEX_HOME` is set, otherwise `~/.codex/skills/agentchrome/SKILL.md`. | Must | Issue #263 |
| FR26 | Support Codex in `install`, `update`, `uninstall`, and `list` with the existing JSON stdout/error contract. | Must | Issue #263 |
| FR27 | Add Codex detection using Codex-specific environment or config-directory signals without breaking existing detection priority. | Should | Issue #263 |
| FR28 | Include Codex in the staleness-check registry behavior. | Must | Issue #263 |
| FR29 | Update README, Codex guide, and examples to list Codex as a supported tool and show the install/update workflow. | Must | Issue #263 |
| FR30 | Add focused BDD and/or unit coverage for Codex install, listing, auto-detection, lifecycle commands, and stale-skill notice behavior. | Must | Issue #263 |
| FR31 | Bare `skill install` discovers all currently detected supported agent targets instead of selecting only the first detected target. | Must | Issue #268; applies only when `--tool` is omitted |
| FR32 | Bare `skill update` scans all supported AgentChrome skill locations for outdated installed skills instead of selecting only the first detected target. | Must | Issue #268; align with existing staleness scan behavior |
| FR32a | Stale-skill scanning discovers version markers inside append-section installs wherever the AgentChrome section appears in a shared instructions file. | Must | Issue #268 review amendment; prevents stale Windsurf/Copilot sections from being skipped after long preambles |
| FR33 | Explicit `--tool` commands keep existing single-target behavior for install, update, and uninstall. | Must | Issue #268; preserve script compatibility |
| FR34 | Multi-target success output is structured JSON with per-target tool, path, action, and version details. | Must | Issue #268; preserve machine readability for AI agents |
| FR35 | Multi-target partial failure output identifies which targets succeeded and failed. | Must | Issue #268; preserve typed exit-code behavior and actionable errors |
| FR36 | Tests cover Codex plus at least one other installed or detected tool in the same command run. | Must | Issue #268; prevents regression to first-detected behavior |

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
| `--tool` | String (enum) | Must be one of: `claude-code`, `windsurf`, `aider`, `continue`, `copilot-jb`, `cursor`, `gemini` | No (auto-detect if omitted) |
| Subcommand | Enum | Must be one of: `install`, `uninstall`, `update`, `list` | Yes |

### Input Data Amendment (#263)

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--tool` | String (enum) | Also accepts `codex` as a supported explicit target | No (auto-detect if omitted) |
| `CODEX_HOME` | Path-like environment variable | When set, used as Codex's home directory for skill installation | No |

### Output Data (install/uninstall/update)

| Field | Type | Description |
|-------|------|-------------|
| `tool` | String | Name of the agentic tool |
| `path` | String | File path where skill was installed/removed |
| `action` | String | One of: `installed`, `uninstalled`, `updated` |
| `version` | String | agentchrome version (for install/update) |

### Output Data Amendment (#268): Bare multi-target install/update

| Field | Type | Description |
|-------|------|-------------|
| `results` | Array | Per-target outcomes for a bare multi-target `install` or `update` invocation |
| `results[].tool` | String | Supported tool identifier |
| `results[].path` | String | File path where skill was installed or updated |
| `results[].action` | String | `installed` or `updated` for successful targets |
| `results[].version` | String | AgentChrome version written for successful install/update targets |
| `results[].status` | String | `ok` or `error` |
| `results[].error` | String or null | Failure reason for targets that could not be resolved or written |

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
| Gemini CLI | `GEMINI_*` env var or `~/.gemini/` directory exists | `~/.gemini/instructions/agentchrome.md` |
| Codex | `CODEX_HOME` env var or `~/.codex/` directory exists | `$CODEX_HOME/skills/agentchrome/SKILL.md`, or `~/.codex/skills/agentchrome/SKILL.md` when `CODEX_HOME` is unset |

### Multi-Target Selection Amendment (#268)

| Command shape | Target selection |
|---------------|------------------|
| `skill install --tool <name>` | The named single target only |
| `skill update --tool <name>` | The named single target only |
| `skill uninstall --tool <name>` | The named single target only |
| `skill install` | Every supported target with a positive detection signal |
| `skill update` | Every supported target with an installed AgentChrome skill older than the running binary |

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
- MCP server registration in Gemini's `~/.gemini/settings.json` (separate feature)
- Gemini-specific skill template content (uses the shared template; customization is a follow-up)
- GEMINI.md project-level file generation
- Publishing AgentChrome to OpenAI's external skills repository
- Changing Codex itself or its skill loader behavior
- Rewriting the existing AgentChrome skill template beyond what Codex support requires
- Changing the JSON output contract for existing `agentchrome skill` commands

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Supported tools | 7 (Claude Code, Windsurf, Aider, Continue.dev, Copilot JB, Cursor, Gemini CLI) | Count of working tool integrations |
| Install round-trip | install → list shows installed → uninstall → list shows not installed | End-to-end verification |
| Startup overhead | < 50ms for all skill subcommands | Benchmark timing |

### Success Metrics Amendment (#263)

| Metric | Target | Measurement |
|--------|--------|-------------|
| Supported tools | 8 (existing seven tools plus Codex) | Count of working tool integrations |
| Codex lifecycle round-trip | install → list shows installed → update rewrites → uninstall removes → list shows not installed | Focused Codex BDD or unit coverage |
| Staleness coverage | Codex stale installs are included in single-tool and aggregated stale-tool notices | Staleness tests with Codex-only and multi-tool stale fixtures |

### Success Metrics Amendment (#268)

| Metric | Target | Measurement |
|--------|--------|-------------|
| Bare update coverage | Multi-tool stale notice cleared by one `agentchrome skill update` invocation | BDD scenario with Codex plus at least one other stale installed skill |
| Bare install coverage | Multiple detected tools installed by one `agentchrome skill install` invocation | BDD scenario with multiple temp-home detection signals |
| Explicit-target compatibility | Explicit install/update/uninstall still return the existing single-target JSON shape | Unit or BDD assertion for `--tool codex` plus another explicit target |

---

## Open Questions

- [x] None — all requirements are derivable from the issue

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #172 | 2026-03-12 | Initial feature spec |
| #214 | 2026-04-16 | Add Gemini CLI as 7th supported tool (AC13–AC18, FR16–FR22) |
| #263 | 2026-04-24 | Add Codex as a supported skill installer target |
| #268 | 2026-04-25 | Make bare skill install/update cover all detected or stale agent targets |

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
