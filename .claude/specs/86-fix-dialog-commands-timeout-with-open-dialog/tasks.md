# Tasks: Dialog commands timeout when a dialog is actually open

**Issue**: #86
**Date**: 2026-02-15
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix dialog commands to skip blocking session setup | [ ] |
| T002 | Add regression test scenarios | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Fix Dialog Commands to Skip Blocking Session Setup

**File(s)**: `src/dialog.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] Add `setup_dialog_session()` that creates a CDP session without calling `apply_emulate_state()`
- [ ] Update `execute_handle()` to use `setup_dialog_session()` and remove the `ensure_domain("Page")` call
- [ ] Update `execute_info()` to use `setup_dialog_session()` and remove `ensure_domain("Page")` and `ensure_domain("Runtime")` calls
- [ ] Replace the `Runtime.evaluate` probe in `execute_info()` with a non-blocking dialog detection approach (e.g., attempt `Runtime.evaluate` without prior `Runtime.enable`, or use the event channel directly)
- [ ] `dialog handle accept` succeeds when an alert dialog is open (no timeout)
- [ ] `dialog info` returns `{"open": true, ...}` when an alert dialog is open (no timeout)
- [ ] No unrelated changes included in the diff

**Notes**: Follow the fix strategy from design.md. The key insight is that `Page.handleJavaScriptDialog` works without `Page.enable`. For dialog info, the probe detection may need to attempt `Runtime.evaluate` directly (without `Runtime.enable`) and interpret a timeout or error as evidence of an open dialog. Event subscriptions for `Page.javascriptDialogOpening` should still work without `Page.enable` because CDP delivers dialog events at the session level once attached.

### T002: Add Regression Test Scenarios

**File(s)**: `tests/features/dialog-timeout-fix.feature`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenarios cover AC1–AC6 from requirements.md
- [ ] All scenarios tagged `@regression`
- [ ] Step definitions implemented (or reuse existing dialog steps from `tests/bdd.rs`)
- [ ] Tests pass with the fix applied

### T003: Verify No Regressions

**File(s)**: Existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing tests pass (`cargo test`)
- [ ] Existing dialog feature tests still pass
- [ ] No side effects in related code paths (connection.rs, emulate.rs unchanged)

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
