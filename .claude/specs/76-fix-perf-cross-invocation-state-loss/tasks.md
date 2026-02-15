# Tasks: perf stop cannot find trace started by perf start — cross-invocation CDP state loss

**Issue**: #76
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Replace `perf start`/`perf stop` with `perf record` | [ ] |
| T002 | Add regression test | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Replace `perf start`/`perf stop` with `perf record`

**File(s)**: `src/cli/mod.rs`, `src/perf.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `PerfCommand::Start` and `PerfCommand::Stop` variants removed from CLI enum
- [ ] `PerfStartArgs` and `PerfStopArgs` structs removed
- [ ] `PerfCommand::Record(PerfRecordArgs)` variant added with `--file`, `--duration`, `--reload` flags
- [ ] `execute_record()` function implemented in `src/perf.rs`:
  - Creates a single CDP session
  - Sends `Tracing.start` with standard categories
  - Optionally reloads page if `--reload` specified
  - Enters `tokio::select!` loop awaiting `Ctrl+C` or `--duration` timeout
  - On signal/timeout, calls `stop_and_collect()` to end trace and write file
  - Returns `PerfStopResult` with file path, duration, size, and vitals
- [ ] `execute_start()` and `execute_stop()` functions removed
- [ ] `execute_perf()` dispatcher updated to route `PerfCommand::Record`
- [ ] `perf record` without `--duration` waits indefinitely until Ctrl+C
- [ ] `perf record --duration 5000` auto-stops after 5 seconds
- [ ] `perf record --reload` reloads the page before entering the wait loop
- [ ] Existing `--auto-stop` flag on old `perf start` is superseded by `perf record` (removed)
- [ ] `perf vitals` and `perf analyze` continue to work unchanged

**Notes**: Follow the `network follow` pattern (`src/network.rs:923–1101`) for the `tokio::select!` event loop with `tokio::signal::ctrl_c()`. Reuse `stop_and_collect()` and `stream_trace_to_file()` unchanged.

### T002: Add Regression Test

**File(s)**: `tests/features/perf-record.feature`, `src/perf.rs` (unit tests)
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario reproduces the original bug condition (two-invocation workflow is gone, single-invocation works)
- [ ] Scenario tagged `@regression`
- [ ] Unit tests for `PerfRecordArgs` serialization and output types
- [ ] Test passes with the fix applied

### T003: Verify No Regressions

**File(s)**: existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo test` passes (all existing unit tests)
- [ ] `cargo clippy` passes with no new warnings
- [ ] `perf vitals` continues to work (single-invocation command, unaffected)
- [ ] `perf analyze` continues to work (file-based, no CDP session)
- [ ] No side effects in `src/connection.rs`, `src/session.rs`, or other shared code

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
