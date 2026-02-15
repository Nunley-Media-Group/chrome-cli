# Tasks: Fix tabs create --background active tab focus

**Issue**: #82
**Date**: 2026-02-15
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix the defect in `execute_create` | [ ] |
| T002 | Add regression test | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/tabs.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] When `--background` is true, the active tab's `targetId` is recorded before calling `Target.createTarget`
- [ ] After creation with `--background`, `Target.activateTarget` is sent with the original tab's `targetId`
- [ ] When `--background` is false, no additional CDP calls are made (existing behavior unchanged)
- [ ] The function still returns `CreateResult` with `id`, `url`, and `title` fields
- [ ] Code passes `cargo clippy` with the project's strict lint config (`all = "deny"`, `pedantic = "warn"`)
- [ ] Code is formatted with `cargo fmt`

**Notes**: Follow the fix strategy from design.md. The pattern for `query_targets` is already used at line 179. The pattern for `Target.activateTarget` is already used in `execute_activate` at lines 250–253. Gate all new logic on `if background { ... }`.

### T002: Add Regression Test

**File(s)**: `tests/features/82-fix-tabs-create-background.feature`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario reproduces the original bug condition (background create keeps original tab active)
- [ ] Scenario for non-background create still activates (regression guard)
- [ ] Scenario for output format preservation
- [ ] All scenarios tagged `@regression`
- [ ] Feature file is valid Gherkin syntax

### T003: Verify No Regressions

**File(s)**: (existing test files)
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` passes with zero warnings/errors
- [ ] `cargo test` passes (all existing tests)
- [ ] No side effects in related code paths (per blast radius from design.md)

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
