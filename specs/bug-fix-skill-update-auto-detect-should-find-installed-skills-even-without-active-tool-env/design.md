# Root Cause Analysis: `skill update` cannot refresh installed skills from a plain terminal

**Issue**: #254
**Date**: 2026-04-26
**Status**: Draft
**Author**: Codex (AI-assisted)

---

## Root Cause

The original defect comes from coupling omitted-`--tool` update behavior to active-tool inference. Active-tool inference (`detect_tool()` / `resolve_tool()`) is appropriate for commands that intentionally target the current agentic environment, but a stale-skill update is driven by files already installed on disk. A user can have `~/.claude/skills/agentchrome/SKILL.md` installed while running from a normal shell where `CLAUDE_CODE` and parent-process hints are absent. In that state, active-tool inference can fail before the command inspects the installed skill file that needs updating.

The current related implementation has moved bare update toward a stale-file scan through `update_stale_skills()` and `skill_check::stale_tools()`, but the scan returns only stale targets. An empty stale-target list conflates two successful no-op states: supported AgentChrome skills are installed and already current, or no AgentChrome skills are installed anywhere. `update_stale_skills()` currently maps that empty list to `no_stale_installed_skills_found()` as an `AppError`, which exits non-zero and emits a JSON error even though the issue requires a successful informational result for both no-op cases.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/skill.rs` | 402-428 | `resolve_tool()` and `no_supported_agentic_tool_detected()` define the active-tool error path that bare update must not use before checking installed skill files. |
| `src/skill.rs` | 433-440 | `no_stale_installed_skills_found()` currently models an empty stale scan as an error. |
| `src/skill.rs` | 541-570 | `update_skill()` is the correct explicit single-target updater and should remain unchanged for `--tool`. |
| `src/skill.rs` | 586-595 | `update_stale_skills()` only consumes stale targets and cannot distinguish all-current from no-installed. |
| `src/skill.rs` | 926-933 | `execute_skill()` routes explicit update to single-target behavior and bare update to the multi-target stale scan. |
| `src/skill_check.rs` | 124-147 | `stale_tools()` scans all supported tool paths but reports only stale installed skills. |

### Triggering Conditions

- The user omits `--tool` because the stale notice recommends `agentchrome skill update`.
- One or more supported AgentChrome skill files exist on disk independently of the current shell's active-tool signals.
- Either at least one installed skill is stale and must be found by disk scan, or every installed skill is already current and should produce a successful no-op.
- Existing tests cover stale multi-target updates, but do not require the no-op bare update outcomes to be exit 0 with informational JSON.

---

## Fix Strategy

### Approach

Keep explicit `--tool` update on the existing `tool_for_name()` -> `update_skill()` path. For omitted `--tool`, introduce an installed-skill inventory scan that walks the same `TOOLS` registry and path-resolution rules used by the staleness notice, classifies each supported tool as missing, installed-current, installed-stale, or unreadable/unversioned, and uses that inventory to decide the command result.

When stale entries exist, bare update should keep the existing batch-update behavior: update every stale installed target, report per-target results, and return non-zero only if one or more attempted target updates fail. When no stale entries exist but at least one installed skill is present, return exit 0 with structured JSON such as `{"results":[],"status":"ok","action":"noop","message":"all installed AgentChrome skills are up to date"}`. When no installed AgentChrome skill is present anywhere, return exit 0 with structured JSON such as `{"results":[],"status":"ok","action":"noop","message":"no AgentChrome skills are installed"}`.

This is the minimal correct fix because it reuses the existing registry, path resolution, version-marker parsing, and `update_skill()` implementation. It changes only the bare update target-selection and empty-result handling, leaving install, uninstall, explicit update, skill content generation, and stale-notice formatting intact.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/skill_check.rs` | Add a shared installed-skill inventory helper that resolves every `TOOLS` path, detects whether an AgentChrome skill is installed, reads its version marker when present, and classifies stale vs current against `binary_version()`. Keep `stale_tools()` as a thin stale-filter over the inventory. | Bare update needs more state than the current stale-only list, and staleness notices must stay aligned with update selection. |
| `src/skill.rs` | Replace `no_stale_installed_skills_found()` error handling for bare update with successful no-op JSON output that distinguishes all-current from no-installed. | Meets AC2 and AC3 while preserving machine-readable stdout. |
| `src/skill.rs` | Keep `Update(Some(tool))` on `tool_for_name()` + `update_skill()`, and keep `Update(None)` on registry scan + batch/no-op output. | Preserves AC4 and existing script compatibility. |
| `tests/features/254-fix-skill-update-auto-detect.feature` | Add four regression scenarios matching AC1-AC4. | Ensures the bug cannot regress independently of broader skill-command scenarios. |
| `tests/bdd.rs` | Reuse or extend existing skill/staleness temp-home helpers to plant stale, current, and missing install states and assert bare-update JSON/no-error outcomes. | Tests the real binary surface without touching the developer's real home directory. |

### Blast Radius

- **Direct impact**: `skill update` when `--tool` is omitted; staleness scan helper shape in `src/skill_check.rs`.
- **Indirect impact**: Stale-skill notice selection remains tied to the same registry scan; skill command BDD helpers gain new no-op assertions.
- **Risk level**: Medium, because bare update already has multi-target semantics from issue #268 and the fix changes the empty-result contract from error to success.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Stale notice selection and bare update selection diverge. | Medium | Share a structured inventory helper and keep `stale_tools()` as a stale-filter over it. |
| Scripts expecting `agentchrome skill update` to fail when no stale skill exists observe exit 0. | Medium | This is the intended behavior from #254; preserve explicit `--tool` missing-install errors for scripts that need strict single-target assertions. |
| Installed append-section skills are missed in shared instruction files. | Low | Reuse `read_version_marker()` so append-section marker scanning remains identical to issue #268 behavior. |
| Explicit `--tool` update accidentally starts returning batch/no-op JSON. | Low | AC4 and targeted tests assert the existing single-target object shape and one-target update scope. |
| Missing skill files are mistaken for current installs. | Low | Inventory must classify missing paths separately from paths with current version markers and AC3 covers the no-installed path. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Use `detect_tool()` first, then fall back to registry scan only on detection failure. | Preserve old active-tool behavior when a tool signal exists. | It can still miss lower-priority installed skills and conflicts with the stale notice's recommendation that bare update refresh all stale installs. |
| Keep empty stale-target collection as an error but change the message. | Improve wording while preserving non-zero exit. | The issue explicitly requires all-current and no-installed cases to exit 0. |
| Update every installed skill regardless of version. | Avoid inventory version classification. | It performs unnecessary writes and hides the distinction between current and stale installs; the command should be a no-op when nothing is stale. |

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #254 | 2026-04-26 | Initial defect design |

---

## Validation Checklist

- [x] Root cause is identified with specific code references
- [x] Fix is minimal - no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
