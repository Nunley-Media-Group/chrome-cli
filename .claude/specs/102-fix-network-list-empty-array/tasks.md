# Tasks: network list always returns empty array

**Issue**: #102
**Date**: 2026-02-15
**Status**: Planning
**Author**: Claude (spec generation)

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix `collect_and_correlate()` to reload page and capture network events | [ ] |
| T002 | Add regression test for network list and filters | [ ] |
| T003 | Verify no regressions in network follow and existing tests | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/network.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `collect_and_correlate()` triggers `Page.reload` after enabling the Network domain and subscribing to events
- [ ] Subscribes to `Page.loadEventFired` (or equivalent) as a completion signal
- [ ] Waits for the page load event after reload, then applies a short idle window for trailing async requests
- [ ] Has a total timeout fallback (default ~5s, respects `--timeout` global flag) to prevent hanging on slow pages
- [ ] `execute_list()` returns non-empty results after a page has been loaded
- [ ] `execute_get()` can retrieve details for requests returned by `execute_list()`
- [ ] No changes to `execute_follow()` code path

**Notes**: Follow the fix strategy from design.md. The change is contained within `collect_and_correlate()`. Both `execute_list()` and `execute_get()` call this function, so both benefit automatically. Do not modify `execute_follow()` — it has its own independent event loop.

### T002: Add Regression Test

**File(s)**: `.claude/specs/102-fix-network-list-empty-array/feature.gherkin`, `tests/features/network.feature`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario reproduces the original bug condition (list after page load returns non-empty)
- [ ] Scenario for type filter (`--type document`) returning non-empty results
- [ ] Scenario for URL filter (`--url "google"`) returning non-empty results
- [ ] Scenario for `network get` with a valid ID from `network list`
- [ ] Scenario confirming `network follow` still works
- [ ] All scenarios tagged `@regression`
- [ ] Step definitions implemented (or existing steps reused)
- [ ] Tests pass with the fix applied

### T003: Verify No Regressions

**File(s)**: existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing BDD tests in `tests/features/network.feature` pass
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes with no warnings
- [ ] No side effects in `network follow` behavior (per blast radius from design.md)

---

## Validation Checklist

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
