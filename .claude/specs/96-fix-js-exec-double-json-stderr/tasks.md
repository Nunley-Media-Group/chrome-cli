# Tasks: JS execution errors emit two JSON objects on stderr

**Issue**: #96
**Date**: 2026-02-15
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix the defect — eliminate double JSON output | [ ] |
| T002 | Add regression test | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/error.rs`, `src/js.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AppError` gains an optional `custom_json` field (defaulting to `None`)
- [ ] `AppError::print_json_stderr()` uses `custom_json` when `Some`, falling back to `ErrorOutput` serialization when `None`
- [ ] A constructor or builder method is added to `AppError` to set `custom_json` (e.g., `AppError::js_execution_failed_with_json()` or a `.with_custom_json()` builder)
- [ ] `src/js.rs` lines 319-327: remove `eprintln!("{err_json}")`, serialize `JsExecError` to a string, and pass it via the new `AppError` constructor so the global handler emits it
- [ ] Running `chrome-cli js exec "throw new Error('test')"` produces exactly one JSON object on stderr
- [ ] The single JSON object contains `error`, `stack`, and `code` fields (same schema as the current first JSON)
- [ ] No changes to `src/main.rs`

**Notes**: Follow the fix strategy from design.md (Option A). Keep the `JsExecError` struct and its serialization — just move the output responsibility from `js.rs` to the global error path in `error.rs`.

### T002: Add Regression Test

**File(s)**: `tests/features/js-exec-double-json-stderr.feature`, `tests/steps/` (if step definitions needed)
**Type**: Create or Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario reproduces the original bug condition (throw + check single JSON on stderr)
- [ ] Scenario tagged `@regression`
- [ ] Step definitions exist for counting JSON objects on stderr
- [ ] Test passes with the fix applied
- [ ] Test would fail if the fix were reverted (exactly-one-JSON assertion catches the double output)

**Notes**: May also update existing scenarios in `tests/features/js-execution.feature` to add stricter "exactly one JSON" assertions, or add the regression scenarios to that existing file.

### T003: Verify No Regressions

**File(s)**: Existing test suite
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing tests pass (`cargo test`)
- [ ] `cargo clippy` passes with no new warnings
- [ ] Successful JS execution still produces correct stdout JSON and empty stderr
- [ ] Other command errors (non-JS) still produce exactly one JSON on stderr

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
