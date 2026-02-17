# Tasks: Fix perf vitals returning null for CLS and TTFB metrics

**Issue**: #119
**Date**: 2026-02-16
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix `extract_cls` to return 0.0 when no shifts found | [ ] |
| T002 | Fix `extract_ttfb` with additional fallback | [ ] |
| T003 | Add regression tests | [ ] |
| T004 | Verify no regressions | [ ] |

---

### T001: Fix `extract_cls` to return 0.0 when no layout shifts found

**File(s)**: `src/perf.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `extract_cls()` returns `Some(0.0)` when no `LayoutShift` events exist in the trace
- [ ] `extract_cls()` still returns the correct cumulative score when `LayoutShift` events are present
- [ ] The `found` flag is removed; function always returns `Some(total_cls)`
- [ ] Existing unit test `extract_cls_from_layout_shifts` still passes
- [ ] Existing unit test `extract_cls_excludes_recent_input` still passes

**Notes**: Remove the `found` boolean and the conditional return. Change `if found { Some(total_cls) } else { None }` to simply `Some(total_cls)`.

### T002: Fix `extract_ttfb` with additional fallback mechanism

**File(s)**: `src/perf.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `extract_ttfb()` returns a valid measurement for pages where `ResourceSendRequest`/`ResourceReceiveResponse` document events are missing
- [ ] The primary extraction path (document resource events) remains unchanged
- [ ] The secondary fallback (`blink.user_timing` navigationStart/responseStart) remains unchanged
- [ ] A third-level fallback is added for when both primary and secondary fail
- [ ] The fallback produces a reasonable positive value (not 0.0 or negative)

**Notes**: In `extract_ttfb_fallback()`, add logic to find `navigationStart` paired with the first `ResourceReceiveResponse` for any resource as a conservative TTFB estimate. Alternatively, look for additional timing events in the trace that can approximate TTFB.

### T003: Add regression tests

**File(s)**: `tests/features/119-fix-perf-vitals-null-cls-ttfb.feature`, `tests/bdd.rs`, `src/perf.rs` (unit tests)
**Type**: Create / Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] Gherkin feature file created with `@regression` tag
- [ ] Scenario for AC1: CLS returns 0.0 (not null) on a no-shift page
- [ ] Scenario for AC2: TTFB returns a positive number (not null) for a loaded page
- [ ] Scenario for AC3: LCP continues to work correctly
- [ ] Step definitions implemented in `tests/bdd.rs` (or reusing existing steps)
- [ ] Unit test added for `extract_cls` returning `Some(0.0)` with empty event list
- [ ] Unit test added for new TTFB fallback path
- [ ] All new tests pass

**Notes**: Reuse existing BDD step definitions for JSON field validation where possible. The feature file naming convention is `{issue#}-{description}.feature` per `structure.md`.

### T004: Verify no regressions

**File(s)**: existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002, T003
**Acceptance**:
- [ ] `cargo test --lib` passes (all unit tests including updated perf tests)
- [ ] `cargo test --test bdd` passes (all BDD tests including new regression feature)
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo fmt --check` passes
- [ ] Existing perf-related features still pass (`perf.feature`, `75-perf-vitals-missing-metrics.feature`, `perf-record.feature`)

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T003)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
