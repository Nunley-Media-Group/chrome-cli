# Tasks: Detect Codex runtime env vars for active skill-notice scoping

**Issue**: #278
**Date**: 2026-04-27
**Status**: Complete
**Author**: Codex (AI-assisted)

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix Codex runtime active detection | [x] |
| T002 | Add focused unit coverage | [x] |
| T003 | Add BDD regression coverage | [x] |
| T004 | Verify focused skill behavior | [x] |

---

### T001: Fix Codex Runtime Active Detection

**File(s)**: `src/skill.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [x] `detect_active_tool_with()` returns `codex` when `CODEX_CI`, `CODEX_MANAGED_BY_NPM`, or `CODEX_THREAD_ID` is present with a non-empty value and `CODEX_HOME` is unset.
- [x] Existing non-empty `CODEX_HOME` active detection still returns `codex`.
- [x] Passive `~/.codex` directory detection remains available for install targeting but does not by itself make `detect_active_tool_with(&[], None)` return Codex.
- [x] Existing runtime priority is preserved: earlier explicit tool signals such as `CLAUDE_CODE` still win over Codex runtime env vars.
- [x] The Codex detection description shown by `agentchrome skill list` remains accurate after the new runtime signals are supported.

**Notes**: Prefer a small allowlist helper over broad `CODEX_*` prefix detection so passive or unrelated variables do not silently change notice scoping.

### T002: Add Focused Unit Coverage

**File(s)**: `src/skill.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [x] Unit tests cover active Codex detection from `CODEX_CI`.
- [x] Unit tests cover active Codex detection from `CODEX_MANAGED_BY_NPM`.
- [x] Unit tests cover active Codex detection from `CODEX_THREAD_ID`.
- [x] Unit tests preserve existing `CODEX_HOME`, empty-`CODEX_HOME`, and passive-config-directory behavior.
- [x] Unit tests prove `CLAUDE_CODE` remains higher priority than Codex runtime env vars.

**Notes**: Keep tests close to the existing `detect_active_tool_*` tests in `src/skill.rs`.

### T003: Add BDD Regression Coverage

**File(s)**: `tests/features/278-detect-codex-runtime-env-vars-for-active-skill-notice-scoping.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [x] The Gherkin file contains one `@regression` scenario for each acceptance criterion in `requirements.md`.
- [x] Scenarios run with isolated temp-home fixtures and `env_clear()` so host `CODEX_HOME`, `CLAUDE_CODE`, or other runtime variables cannot leak into the result.
- [x] The AC1 scenario proves Codex runtime env vars classify the active tool as `codex` without `CODEX_HOME`.
- [x] The AC2 scenario plants a current Codex AgentChrome skill and a stale inactive tool skill, then proves a successful command emits no inactive stale notice.
- [x] The AC3 scenario plants stale Codex and inactive tool skills, then proves exactly one notice names `codex` and omits the inactive stale tool.
- [x] Step definitions reuse existing `SkillWorld` helpers where practical instead of introducing a separate binary harness.

**Notes**: Use a no-Chrome AgentChrome command such as `agentchrome skill list` or another successful command already supported by the skill BDD harness.

### T004: Verify Focused Skill Behavior

**File(s)**: existing - no direct changes expected
**Type**: Verify
**Depends**: T002, T003
**Acceptance**:
- [x] `cargo fmt --check` passes.
- [x] `cargo test --bin agentchrome skill` passes.
- [x] `cargo test --bin agentchrome skill_check` passes.
- [x] The nearest supported focused BDD invocation passes: `cargo test --test bdd -- --input tests/features/278-detect-codex-runtime-env-vars-for-active-skill-notice-scoping.feature --fail-fast`.
- [x] Full `cargo test --test bdd` was not required because the focused feature invocation passed.
- [x] Manual or automated temp-home smoke confirms `CODEX_CI=1` with no `CODEX_HOME` scopes stale notices to Codex.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #278 | 2026-04-27 | Initial defect tasks |

---

## Validation Checklist

- [x] Tasks are focused on the fix - no feature work
- [x] Regression test is included (T003)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
