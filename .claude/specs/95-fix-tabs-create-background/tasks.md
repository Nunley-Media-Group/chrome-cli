# Tasks: tabs create --background does not preserve active tab

**Issue**: #95
**Date**: 2026-02-15
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
- [ ] After `Target.activateTarget` (line 197), a verification loop polls `query_targets` to confirm the original tab is the first page target
- [ ] The loop retries up to 10 times with a 10ms delay between attempts
- [ ] The loop exits early when the original tab is confirmed as the first page target
- [ ] If retries are exhausted, the command proceeds without error (best-effort)
- [ ] All new logic is gated on the `if let Some(ref active_id) = original_active_id` block
- [ ] Bug no longer reproduces: `tabs list` after `tabs create --background` shows the original tab as active
- [ ] No unrelated changes included in the diff

**Notes**: Follow the fix strategy from design.md. The verification loop should use the existing `query_targets` function (from `chrome_cli::chrome`) and `tokio::time::sleep` for the delay. Keep the loop inside the existing `if let Some(ref active_id)` block.

### T002: Add Regression Test

**File(s)**: `tests/features/95-fix-tabs-create-background.feature`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario reproduces the original bug condition (background tab creation followed by tab list verification)
- [ ] All scenarios tagged `@regression`
- [ ] Scenarios cover AC1 (background tab stays inactive), AC2 (normal create still activates), and AC3 (background tab appears in list)
- [ ] Feature file is valid Gherkin syntax

### T003: Verify No Regressions

**File(s)**: existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing tests pass (`cargo test`)
- [ ] Clippy passes (`cargo clippy`)
- [ ] No side effects in related code paths (per blast radius from design.md)
- [ ] `tabs create` without `--background` still activates the new tab
- [ ] `tabs list`, `tabs close`, and `tabs activate` commands still work correctly

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
