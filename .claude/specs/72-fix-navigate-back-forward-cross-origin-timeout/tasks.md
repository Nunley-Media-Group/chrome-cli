# Tasks: navigate back/forward timeout on cross-origin history navigation

**Issue**: #72
**Date**: 2026-02-14
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

**File(s)**: `src/navigate.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `execute_back()` subscribes to `Page.frameNavigated` instead of `Page.loadEventFired`
- [ ] `execute_forward()` subscribes to `Page.frameNavigated` instead of `Page.loadEventFired`
- [ ] Strategy label strings updated from `"load"` to `"navigation"` for accurate error messages
- [ ] Cross-origin back/forward navigation no longer times out
- [ ] No unrelated changes included in the diff

**Notes**: Follow the fix strategy from design.md. The change is two subscription lines (lines ~231 and ~289) and two strategy label strings (lines ~242 and ~298).

### T002: Add Regression Test

**File(s)**: `tests/features/url-navigation.feature` (add scenarios), step definitions as needed
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario for cross-origin `navigate back` added
- [ ] Gherkin scenario for cross-origin `navigate forward` added
- [ ] Scenarios tagged `@regression`
- [ ] Test passes with the fix applied
- [ ] Test fails if the fix is reverted (confirms it catches the bug)

### T003: Verify No Regressions

**File(s)**: existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing tests pass (`cargo test`)
- [ ] Existing same-origin back/forward scenarios (AC9, AC10, AC11, AC12) still pass
- [ ] `cargo clippy` passes with no new warnings
- [ ] No side effects in related code paths (per blast radius from design.md)

---

## Validation Checklist

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
