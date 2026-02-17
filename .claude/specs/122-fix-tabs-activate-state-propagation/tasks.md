# Tasks: tabs activate not reflected in subsequent tabs list

**Issue**: #122
**Date**: 2026-02-16
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix the defect | [ ] |
| T002 | Add regression test | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/tabs.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `execute_activate()` polls `query_targets()` after `Target.activateTarget` until the activated tab is the first page-type target in `/json/list`
- [ ] Polling uses 50 iterations with 10ms sleep (500ms max budget), matching `execute_create()`
- [ ] Loop breaks early when the condition is met
- [ ] Function proceeds gracefully if retries are exhausted (no error, no panic)
- [ ] No unrelated changes included in the diff

**Notes**: Insert the polling loop between the `send_command("Target.activateTarget", ...)` call (line ~297) and the `if quiet` check (line ~299). Use the same pattern as `execute_create()` lines 203–210: `query_targets()`, find first page-type target, compare its `id` to `target.id`.

### T002: Add Regression Test

**File(s)**: `tests/features/122-fix-tabs-activate-state-propagation.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenarios cover AC1 (activate reflected in list) and AC2 (correct return info)
- [ ] Scenarios tagged `@regression`
- [ ] Step definitions implemented in `tests/bdd.rs`
- [ ] Tests pass with the fix applied

### T003: Verify No Regressions

**File(s)**: Existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing tests pass (`cargo test`)
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo fmt --check` passes
- [ ] No side effects in `execute_create` or `execute_close` code paths

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
