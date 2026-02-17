# Tasks: Fix tabs create --background not keeping original tab active

**Issue**: #121
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
- [ ] The polling loop iteration count in `execute_create` is increased from `10` to `50` (line 203)
- [ ] No other lines in the function are changed
- [ ] `cargo build` succeeds without warnings
- [ ] `cargo clippy` passes

**Notes**: The fix is a single-constant change: `0..10` becomes `0..50` on line 203 of `src/tabs.rs`. This extends the maximum polling budget from 100ms to 500ms while keeping the 10ms poll interval unchanged.

### T002: Add Regression Test

**File(s)**: `tests/features/121-fix-tabs-create-background.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin feature file covers AC1 (background tab does not become active), AC2 (background tab is created successfully), and AC3 (non-background create still activates)
- [ ] All scenarios tagged `@regression`
- [ ] Step definitions implemented in `tests/bdd.rs` (reuse existing steps where possible)
- [ ] `cargo test --test bdd` passes with the fix applied

### T003: Verify No Regressions

**File(s)**: Existing test files (no changes)
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo test` passes (all unit, integration, and BDD tests)
- [ ] `cargo clippy` passes
- [ ] `cargo fmt --check` passes
- [ ] Manual verification: `chrome-cli tabs create --background` keeps the original tab active in headless mode

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
