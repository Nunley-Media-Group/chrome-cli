# Tasks: perf vitals returns only URL with no performance metrics

**Issue**: #75
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix serialization and timing in perf vitals | [ ] |
| T002 | Add regression test | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/perf.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] Remove `#[serde(skip_serializing_if = "Option::is_none")]` from `lcp_ms`, `cls`, `ttfb_ms` fields on both `CoreWebVitals` and `PerfVitalsResult` structs
- [ ] Add `const POST_LOAD_SETTLE_MS: u64 = 3000;` constant
- [ ] Insert `tokio::time::sleep(Duration::from_millis(POST_LOAD_SETTLE_MS)).await;` after the `wait_for_event(load_rx, ...)` call and before `Tracing.end`
- [ ] Add fallback TTFB extraction in `extract_ttfb`: when no `ResourceSendRequest`/`ResourceReceiveResponse` pair is found, compute TTFB from `navigationStart` and `responseStart` events in `blink.user_timing`
- [ ] After `parse_trace_vitals`, if all three metrics are `None`, print a warning to stderr via `eprintln!` and return an `AppError` with a non-zero exit code
- [ ] Update `format_vitals_plain` to display "LCP: N/A", "CLS: N/A", "TTFB: N/A" for `None` values instead of omitting them
- [ ] Bug no longer reproduces: `perf vitals` on a real website returns all three metric fields in JSON output

**Notes**: Follow the fix strategy from design.md. Keep changes confined to `src/perf.rs`. The `skip_serializing_if` removal also affects `CoreWebVitals` used in `PerfStopResult` — this is intentional for consistency.

### T002: Add Regression Test

**File(s)**: `tests/features/75-perf-vitals-missing-metrics.feature`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario reproduces the original bug condition (perf vitals with metric fields present)
- [ ] Scenario tagged `@regression`
- [ ] Scenario verifies that all three metric fields (`lcp_ms`, `cls`, `ttfb_ms`) are present in JSON output
- [ ] Scenario verifies non-zero exit code when all metrics are null
- [ ] Test passes with the fix applied

### T003: Verify No Regressions

**File(s)**: existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo fmt --check` shows no formatting issues
- [ ] `perf start` / `perf stop` flow still works correctly (CoreWebVitals serialization change is backward-compatible)

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
