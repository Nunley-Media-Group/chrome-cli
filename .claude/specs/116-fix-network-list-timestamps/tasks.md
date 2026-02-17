# Tasks: Fix network list timestamps showing 1970-01-01

**Issue**: #116
**Date**: 2026-02-16
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix the defect — use `wallTime` for network timestamps | [ ] |
| T002 | Add regression test | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/network.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `requestWillBeSent` handler reads `wallTime` (epoch seconds) and stores it as the display timestamp instead of the monotonic `timestamp`
- [ ] Monotonic `timestamp` is still stored separately for duration calculations (difference between `loadingFinished` timestamp and request timestamp)
- [ ] For `loadingFinished` timestamps (used in streaming/follow mode), compute and apply the monotonic-to-epoch offset derived from `wallTime - timestamp` of the originating request
- [ ] `timestamp_to_iso()` doc comment is corrected to say "epoch seconds" (not "CDP Network timestamps")
- [ ] `network list`, `network get`, and `network follow` all produce wall-clock timestamps
- [ ] Bug no longer reproduces using the steps from requirements.md

**Notes**: The `NetworkRequestBuilder` struct and the streaming `InFlightRequest` struct both store a `timestamp` field. Add a `wall_time` field (or rename `timestamp` to reflect its new semantic) to hold the epoch seconds from `wallTime`. Keep the monotonic `timestamp` for duration math. Follow the fix strategy from design.md — prefer `wallTime` with fallback to monotonic + offset for events that lack `wallTime`.

### T002: Add Regression Test

**File(s)**: `tests/features/116-fix-network-list-timestamps.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario reproduces the original bug condition (network list timestamps)
- [ ] Scenario tagged `@regression`
- [ ] Step definitions implemented in `tests/bdd.rs` (or reuses existing network steps)
- [ ] Test passes with the fix applied
- [ ] Test verifies timestamps are from the current year (not 1970)
- [ ] Test verifies timestamps match ISO 8601 format

### T003: Verify No Regressions

**File(s)**: existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing tests pass (`cargo test`)
- [ ] Existing network feature tests still pass (`cargo test --test bdd`)
- [ ] Console timestamps are unaffected (separate code path in `console.rs`)
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo fmt --check` passes

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
