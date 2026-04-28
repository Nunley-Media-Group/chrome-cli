# Defect Report: Fix stale-skill notice during explicit skill update

**Issue**: #281
**Date**: 2026-04-28
**Status**: Draft
**Author**: Codex (AI-assisted)
**Severity**: Medium
**Related Spec**: `specs/feature-harden-progressive-disclosure-enrich-skill-md-extend-temp-file-gating-notify-on-stale-skill/`

---

## Reproduction

### Steps to Reproduce

1. Install an AgentChrome skill for `copilot-jb` at its supported canonical path with an embedded version older than the running binary.
2. Run `agentchrome skill update --tool copilot-jb`.
3. Observe stderr and stdout from the same invocation.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS, Linux, and Windows; the affected path is pure CLI/filesystem logic. |
| **Version / Commit** | Reported with installed skill v1.50.0 and binary v1.51.5; current source VERSION is 1.57.0. |
| **Browser / Runtime** | N/A - no Chrome/CDP runtime is required. |
| **Configuration** | A stale AgentChrome skill exists at a supported canonical install path and `skill update --tool <tool>` is invoked for that same tool. |

### Frequency

Always when the selected explicit update target is installed, stale, and eligible for the global pre-dispatch stale-skill notice before `skill::execute_skill()` updates it.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | An explicit `agentchrome skill update --tool <tool>` invocation updates the selected stale skill, exits 0, preserves the single-target JSON object on stdout, and does not first warn that the same selected skill still needs to be refreshed. |
| **Actual** | `run()` emits the global stale-skill notice before command dispatch, so stderr can name the selected tool as stale immediately before stdout reports that same tool was updated successfully. |

### Error Output

```text
note: installed agentchrome skill for copilot-jb is v1.50.0 but binary is v1.51.5 - run 'agentchrome skill update' to refresh
{"tool":"copilot-jb","path":"/Users/probello/.config/github-copilot/intellij/global-copilot-instructions.md","action":"updated","version":"1.51.5"}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Explicit Update Suppresses Self-Stale Notice

**Given** an installed `copilot-jb` AgentChrome skill is stale
**When** the user runs `agentchrome skill update --tool copilot-jb`
**Then** the command updates the `copilot-jb` skill to the running binary version
**And** stderr does not contain a stale-skill notice naming `copilot-jb`
**And** stdout keeps the existing single-target JSON object shape.

### AC2: Explicit Update Preserves Successful Command Contract

**Given** a stale AgentChrome skill exists for a supported tool
**When** the user runs `agentchrome skill update --tool <tool>` for that installed tool
**Then** the command exits 0
**And** stdout reports `action` as `updated`
**And** stdout includes the selected `tool`, `path`, and `version` fields.

### AC3: Unrelated Stale-Notice Behavior Is Preserved

**Given** stale-skill notice checks are enabled
**When** the user runs a non-update AgentChrome command while the active tool's installed skill is stale
**Then** the existing active-tool-scoped stale notice behavior is preserved.

### AC4: Bare Update Flow Is Preserved

**Given** one or more installed AgentChrome skills are stale
**When** the user runs `agentchrome skill update` without `--tool`
**Then** the existing multi-target update flow continues to update stale installed skills
**And** a subsequent invocation emits no stale notice for updated targets.

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | The global stale-skill notice MUST NOT be emitted before an explicit `skill update --tool <tool>` invocation for the selected stale tool. | Must |
| FR2 | The explicit `--tool` update path MUST preserve the current single-target JSON object shape and success exit code. | Must |
| FR3 | Non-update commands MUST preserve existing stale-notice suppression gates and active-tool-scoped notice behavior. | Must |
| FR4 | Bare `skill update` without `--tool` MUST preserve the multi-target stale installed-skill update behavior from issue #254. | Must |
| FR5 | Regression tests MUST use isolated temp home/cwd fixtures and MUST NOT inspect or mutate the developer's real skill files. | Must |

---

## Out of Scope

- Changing AgentChrome skill template content.
- Changing `agentchrome skill install` or `agentchrome skill uninstall` target-selection semantics.
- Changing the bare `agentchrome skill update` multi-target contract.
- Auto-updating stale skills from ordinary non-update commands.
- Reworking the `TOOLS` registry or adding new supported tools.
- Changing stale-skill notice wording for commands that still emit the notice.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #281 | 2026-04-28 | Initial defect report |

---

## Validation Checklist

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included
- [x] Fix scope is minimal - no feature work mixed in
- [x] Out of scope is defined
