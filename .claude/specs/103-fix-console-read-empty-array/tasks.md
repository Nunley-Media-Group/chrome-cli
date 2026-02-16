# Tasks: console read always returns empty array

**Issue**: #103
**Date**: 2026-02-16
**Status**: Planning
**Author**: Claude (spec generation)

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix `execute_read()` to reload page and capture console events | [ ] |
| T002 | Add regression test for console read and filters | [ ] |
| T003 | Verify no regressions in console follow and existing tests | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/console.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `execute_read()` enables the `Page` domain and subscribes to `Page.loadEventFired`
- [ ] After subscribing, triggers `Page.reload` to replay page scripts and regenerate console events
- [ ] Waits for `Page.loadEventFired` to signal reload completion, then applies a short idle window (~200ms) for trailing console messages from deferred scripts
- [ ] Has a total timeout fallback (default ~5s, respects `--timeout` global flag) to prevent hanging on slow pages
- [ ] `execute_read()` returns non-empty results for pages that generate console output during load
- [ ] Type filters (`--type`, `--errors-only`) work on the captured messages
- [ ] Detail mode (`console read <MSG_ID>`) works with IDs from the captured list
- [ ] `--include-preserved` navigation tracking still works correctly (reload increments nav counter)
- [ ] No changes to `execute_follow()` code path

**Notes**: Follow the fix strategy from design.md. Model the implementation after the `network list` fix in `src/network.rs` (issue #102). The change is contained within `execute_read()`. The `execute_follow()` function has its own independent event loop and must not be modified.

### T002: Add Regression Test

**File(s)**: `tests/features/103-fix-console-read-empty-array.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario reproduces the original bug condition (read after page generates console messages returns non-empty)
- [ ] Scenario for `--errors-only` filter returning only error-level messages
- [ ] Scenario confirming messages persist across CLI invocations
- [ ] Scenario confirming `console follow` still works
- [ ] All scenarios tagged `@regression`
- [ ] Feature file registered in `tests/bdd.rs` with `filter_run_and_exit` (matching the #102 pattern: all scenarios require Chrome, so filter returns `false`)
- [ ] Tests pass with the fix applied

### T003: Verify No Regressions

**File(s)**: existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing BDD tests in `tests/features/console.feature` pass
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes with no warnings
- [ ] No side effects in `console follow` behavior (per blast radius from design.md)

---

## Validation Checklist

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
