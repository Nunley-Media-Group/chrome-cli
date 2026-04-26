# Defect Report: `skill update` cannot refresh installed skills from a plain terminal

**Issue**: #254
**Date**: 2026-04-26
**Status**: Draft
**Author**: Codex (AI-assisted)
**Severity**: Medium
**Related Spec**: `specs/feature-add-agentchrome-skill-command-group/`

---

## Reproduction

### Steps to Reproduce

1. Install an AgentChrome skill at a canonical supported-tool path, such as `~/.claude/skills/agentchrome/SKILL.md`, with an embedded version older than the running binary.
2. Run the command from a plain terminal with no active agentic-tool environment variable or parent-process signal.
3. Invoke `agentchrome skill update` without `--tool`.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS, Linux, and Windows; the affected path is pure CLI/filesystem logic. |
| **Version / Commit** | Reported against AgentChrome 1.42.0 -> 1.43.0; current branch: `254-fix-skill-update-auto-detect-should-find-installed-skills-even-without-active-tool-env`. |
| **Browser / Runtime** | N/A - no Chrome/CDP runtime is required. |
| **Configuration** | One or more AgentChrome skill files already exist on disk; no `--tool` flag is supplied. |

### Frequency

Always when the bare update path cannot infer a single active tool even though supported AgentChrome skill files exist on disk, or when the stale-skill scan returns no stale targets and the command treats that as an error.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `agentchrome skill update` without `--tool` scans supported skill locations, updates every stale installed AgentChrome skill, and exits 0. If there is nothing to change, it exits 0 with a clear informational JSON message. |
| **Actual** | The command can fail before checking installed skill files by relying on active-tool detection, or can return a non-zero "no stale installed AgentChrome skills found" error when no stale target is found. Both outcomes make the recommended bare update command unsuitable as the one-step stale-skill remedy. |

### Error Output

```json
{"error":"no supported agentic tool detected","supported_tools":[...]}
{"error":"no stale installed AgentChrome skills found"}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Bare Update Finds Installed Stale Skills Without Active Tool Signals

**Given** one or more stale AgentChrome skill files exist at supported canonical install paths
**And** no active agentic-tool environment variable or parent-process signal is present
**When** the user runs `agentchrome skill update` without `--tool`
**Then** every stale installed AgentChrome skill is updated to the running binary version
**And** the command exits 0 with structured JSON naming each updated target.

### AC2: Bare Update Is Informational When Installed Skills Are Already Current

**Given** one or more AgentChrome skill files exist at supported canonical install paths
**And** every installed skill already carries the running binary version
**When** the user runs `agentchrome skill update` without `--tool`
**Then** the command exits 0
**And** stdout contains structured JSON indicating all installed AgentChrome skills are already up to date
**And** stderr does not contain a JSON error object.

### AC3: Bare Update Is Informational When No Skills Are Installed

**Given** no AgentChrome skill files exist at any supported canonical install path
**And** no active agentic-tool environment variable or parent-process signal is present
**When** the user runs `agentchrome skill update` without `--tool`
**Then** the command exits 0
**And** stdout contains structured JSON indicating no AgentChrome skills are installed
**And** stderr does not contain a JSON error object.

### AC4: Explicit `--tool` Update Remains Single-Target

**Given** stale AgentChrome skill files exist for `claude-code` and `codex`
**When** the user runs `agentchrome skill update --tool claude-code`
**Then** only the Claude Code skill is updated
**And** the Codex skill remains stale
**And** stdout keeps the existing single-target JSON object shape.

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | The omitted-`--tool` update path MUST scan all supported AgentChrome skill install locations for installed files before considering active-tool detection failures. | Must |
| FR2 | The omitted-`--tool` update path MUST update every stale installed AgentChrome skill found in one invocation. | Must |
| FR3 | The omitted-`--tool` update path MUST distinguish `stale installed`, `installed but already current`, and `not installed` states. | Must |
| FR4 | Empty or all-current bare-update outcomes MUST be successful no-op responses, not `AppError`s. | Must |
| FR5 | Explicit `agentchrome skill update --tool <name>` MUST preserve the existing single-target behavior, error behavior, and JSON object shape. | Must |

---

## Out of Scope

- Changing `agentchrome skill install` or `agentchrome skill uninstall` target-selection semantics.
- Installing missing skills from the bare update path.
- Changing skill template content.
- Changing the stale-notice text other than ensuring its recommended command is actionable.
- Reworking the `TOOLS` registry or adding new supported tools.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #254 | 2026-04-26 | Initial defect report |

---

## Validation Checklist

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included
- [x] Fix scope is minimal - no feature work mixed in
- [x] Out of scope is defined
