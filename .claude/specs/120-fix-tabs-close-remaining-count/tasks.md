# Tasks: tabs close reports incorrect remaining count (off-by-one race condition)

**Issue**: #120
**Date**: 2026-02-16
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix the defect — add polling loop to `execute_close()` | [ ] |
| T002 | Add regression test — Gherkin scenario + step definitions | [ ] |
| T003 | Verify no regressions — run existing test suite | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/tabs.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] In `execute_close()`, the immediate `query_targets()` call after closing tabs is replaced with a polling loop
- [ ] The polling loop waits for the remaining page-type target count to equal `page_count - closing_page_count`
- [ ] The loop polls up to 10 times with 10ms `tokio::time::sleep` delays between iterations (matching `execute_create()` pattern)
- [ ] The loop breaks early once the expected count is observed
- [ ] If the loop exhausts retries, the last polled count is used (graceful degradation)
- [ ] Bug no longer reproduces using the steps from requirements.md
- [ ] No unrelated changes included in the diff

**Notes**: Follow the exact polling pattern from `execute_create()` lines 203-210. The expected count is `page_count - closing_page_count`, both of which are already computed before the close loop. Ensure `Duration` is imported from `std::time` (already imported in the file for `execute_create`).

### T002: Add Regression Test

**File(s)**: `tests/features/120-fix-tabs-close-remaining-count.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin feature file created at `tests/features/120-fix-tabs-close-remaining-count.feature`
- [ ] Scenarios cover AC1 (single close correct count) and AC2 (sequential closes correct counts)
- [ ] Scenario covers AC3 (existing close behavior preserved)
- [ ] All scenarios tagged `@regression`
- [ ] Step definitions implemented in `tests/bdd.rs` (or reuse existing tab management steps)
- [ ] Tests pass with the fix applied
- [ ] Tests would fail if the fix were reverted (confirms they catch the bug)

### T003: Verify No Regressions

**File(s)**: Existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo test --test bdd` passes (all BDD scenarios including new ones)
- [ ] `cargo test --lib` passes (unit tests)
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo fmt --check` passes
- [ ] Existing tab management tests still pass (`tab-management.feature`)

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
