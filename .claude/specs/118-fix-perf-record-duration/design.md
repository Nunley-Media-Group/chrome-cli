# Root Cause Analysis: perf record --duration reports incorrect duration_ms

**Issue**: #118
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude

---

## Root Cause

The `duration_ms` reported by `perf record` only measures the trace stop/collection overhead instead of the actual recording duration. This happens because the timer is started inside `stop_and_collect()` (line 230 of `src/perf.rs`) — **after** the recording duration has already elapsed.

In `execute_record()`, the recording flow is:
1. Start tracing via `Tracing.start`
2. Optionally reload the page and wait for load
3. Sleep for the `--duration` timeout (or wait for Ctrl+C)
4. Call `stop_and_collect()`, which starts its own `Instant::now()` timer
5. Send `Tracing.end`, stream data, measure elapsed time

The timer at step 4 only captures the time for steps 4–5 (typically 21–133ms), completely missing the actual recording period from steps 1–3.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/perf.rs` | 163–223 | `execute_record()` — orchestrates the recording but does not capture start time |
| `src/perf.rs` | 226–279 | `stop_and_collect()` — starts its own timer at line 230, only measuring collection overhead |

### Triggering Conditions

- Any invocation of `perf record` with `--duration` triggers this — the reported `duration_ms` is always wrong
- Without `--duration` (Ctrl+C mode), the same problem occurs: `duration_ms` only reflects collection time, not the time the user waited before pressing Ctrl+C
- The bug was not caught because the timer placement was never validated against the `--duration` value

---

## Fix Strategy

### Approach

Move the `Instant::now()` call from inside `stop_and_collect()` to `execute_record()`, capturing the start time **before** `Tracing.start` is sent. Then pass the elapsed duration into `stop_and_collect()` (or compute it in `execute_record()` after `stop_and_collect()` returns) so that `duration_ms` reflects the full recording period.

The simplest approach is to:
1. Record `start_time = Instant::now()` in `execute_record()` before calling `Tracing.start`
2. Change `stop_and_collect()` to accept a `start_time: Instant` parameter instead of creating its own
3. Use `start_time.elapsed()` inside `stop_and_collect()` (after collection completes) to compute the total duration

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/perf.rs` (`execute_record`) | Add `let start_time = Instant::now();` before `Tracing.start`, pass it to `stop_and_collect()` | Captures the true start of the recording period |
| `src/perf.rs` (`stop_and_collect`) | Add `start_time: Instant` parameter, remove the local `Instant::now()` | Uses the caller-provided start time instead of measuring only collection overhead |

### Blast Radius

- **Direct impact**: `execute_record()` and `stop_and_collect()` in `src/perf.rs`
- **Indirect impact**: None — `stop_and_collect()` is only called from `execute_record()`. The `perf vitals` command has its own separate trace collection path and is not affected.
- **Risk level**: Low

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `stop_and_collect()` caller contract changes | Low | Only one call site (`execute_record()`), updated in the same change |
| `perf vitals` duration reporting breaks | None | `perf vitals` does not use `stop_and_collect()` and does not report `duration_ms` |
| Existing `perf record` output schema changes | None | The `PerfRecordResult` struct and its fields remain identical; only the value of `duration_ms` changes |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
