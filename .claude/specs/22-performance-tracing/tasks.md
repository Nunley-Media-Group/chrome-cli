# Tasks: Performance Tracing

**Issue**: #22
**Date**: 2026-02-12
**Status**: Planning
**Author**: Claude (writing-specs)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 5 | [ ] |
| Integration | 2 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **11** | |

---

## Phase 1: Setup

### T001: Define CLI argument types for `perf` subcommand

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `PerfArgs` struct with `PerfCommand` subcommand enum added
- [ ] `PerfCommand` has variants: `Start(PerfStartArgs)`, `Stop(PerfStopArgs)`, `Analyze(PerfAnalyzeArgs)`, `Vitals(PerfVitalsArgs)`
- [ ] `PerfStartArgs` has `--reload` (bool), `--auto-stop` (bool), `--file` (Option<PathBuf>)
- [ ] `PerfStopArgs` has `--file` (Option<PathBuf>)
- [ ] `PerfAnalyzeArgs` has positional `insight` (String) and `--trace-file` (PathBuf, required)
- [ ] `PerfVitalsArgs` has `--file` (Option<PathBuf>)
- [ ] `Command::Perf` variant changed from unit to `Perf(PerfArgs)`
- [ ] `cargo clippy` passes

**Notes**: Follow the same pattern as `TabsArgs`/`TabsCommand` and `PageArgs`/`PageCommand`.

### T002: Add error constructors for perf commands

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AppError::no_active_trace()` — "No active trace. Run 'chrome-cli perf start' first."
- [ ] `AppError::unknown_insight(name: &str)` — lists available insight names
- [ ] `AppError::trace_file_not_found(path: &str)` — "Trace file not found: {path}"
- [ ] `AppError::trace_parse_failed(error: &str)` — "Failed to parse trace file: {error}"
- [ ] `AppError::trace_timeout(timeout_ms: u64)` — "Trace timed out after {N}ms"
- [ ] Unit tests for each new constructor
- [ ] `cargo clippy` passes

---

## Phase 2: Backend Implementation

### T003: Create perf module with output types and session setup

**File(s)**: `src/perf.rs`
**Type**: Create
**Depends**: T001, T002
**Acceptance**:
- [ ] Module file created with standard structure (output types, print_output, cdp_config, setup_session)
- [ ] Output structs defined: `PerfStartResult`, `PerfStopResult`, `PerfVitalsResult`, `PerfAnalyzeResult`
- [ ] All output structs derive `Serialize`
- [ ] `print_output` supports `--json`, `--pretty`, `--plain` formats
- [ ] Plain text formatting for each output type
- [ ] `execute_perf` dispatcher function delegates to subcommand handlers
- [ ] Unit tests for output type serialization
- [ ] `cargo clippy` passes

**Notes**: Follow `navigate.rs` patterns exactly. `setup_session` reuses `resolve_connection`, `resolve_target`, `CdpClient::connect`, `ManagedSession::new`.

### T004: Implement `perf start` command

**File(s)**: `src/perf.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_start` function implemented
- [ ] Sends `Tracing.start` with default performance trace categories
- [ ] `--file` sets custom output path; default auto-generates temp path with timestamp
- [ ] `--reload` sends `Page.reload` and waits for `Page.loadEventFired` before starting trace
- [ ] `--auto-stop` waits for `Page.loadEventFired`, sends `Tracing.end`, streams data to file, returns summary
- [ ] Without `--auto-stop`: returns `{"tracing": true, "file": "..."}` immediately after trace starts
- [ ] Default trace timeout of 30s for `--auto-stop`
- [ ] `cargo clippy` passes

### T005: Implement `perf stop` command with streaming trace collection

**File(s)**: `src/perf.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_stop` function implemented
- [ ] Subscribes to `Tracing.dataCollected` and `Tracing.tracingComplete` events
- [ ] Sends `Tracing.end`
- [ ] Streams `Tracing.dataCollected` chunks to file using `BufWriter`
- [ ] Writes valid Chrome Trace Event Format JSON: `{"traceEvents": [...]}`
- [ ] Waits for `Tracing.tracingComplete` event
- [ ] Reports file path, duration, file size, and Core Web Vitals
- [ ] Handles CDP error when no trace is active (returns `AppError::no_active_trace()`)
- [ ] `--file` overrides output path
- [ ] `cargo clippy` passes

**Notes**: `Tracing.dataCollected` sends an array of trace events in `params.value`. Each chunk should be appended to the trace file. The file must start with `{"traceEvents":[` and end with `]}` to be valid Chrome trace format.

### T006: Implement trace parser for Core Web Vitals extraction

**File(s)**: `src/perf.rs`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] `parse_trace_vitals(path: &Path) -> Result<CoreWebVitals, AppError>` function
- [ ] Extracts TTFB from `ResourceSendRequest` / `ResourceReceiveResponse` pair for main document
- [ ] Extracts LCP from last `largestContentfulPaint::Candidate` event
- [ ] Extracts CLS by summing `LayoutShift` scores (excluding those with recent input)
- [ ] Returns `CoreWebVitals { lcp_ms: Option<f64>, cls: Option<f64>, ttfb_ms: Option<f64> }` with `None` for metrics that can't be determined
- [ ] Reads trace file with `BufReader` for memory efficiency
- [ ] Unit tests with sample trace event data
- [ ] `cargo clippy` passes

**Notes**: Use `serde_json::from_reader` to parse the trace file. Filter events by `cat` and `name` fields. Timestamps in traces are in microseconds — convert to milliseconds for output.

### T007: Implement `perf analyze` and `perf vitals` commands

**File(s)**: `src/perf.rs`
**Type**: Modify
**Depends**: T006
**Acceptance**:
- [ ] `execute_analyze` function reads trace file from `--trace-file`, validates insight name, runs analysis
- [ ] Supported insights: `DocumentLatency`, `LCPBreakdown`, `RenderBlocking`, `LongTasks`
- [ ] Invalid insight name returns `AppError::unknown_insight()` with list of valid names
- [ ] Missing trace file returns `AppError::trace_file_not_found()`
- [ ] `execute_vitals` function orchestrates: start trace → reload → wait for load → stop → parse → report
- [ ] `perf vitals` output includes `url`, `lcp_ms`, `cls`, `ttfb_ms`
- [ ] `cargo clippy` passes

---

## Phase 3: Integration

### T008: Wire perf command into CLI dispatcher

**File(s)**: `src/main.rs`, `src/cli/mod.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `mod perf;` added to `main.rs`
- [ ] `Command::Perf(args)` match arm calls `perf::execute_perf(&cli.global, args).await`
- [ ] `Command::Perf` variant updated from unit to `Perf(PerfArgs)`
- [ ] CLI help text renders correctly for `chrome-cli perf --help` and all subcommands
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` passes

### T009: Add plain text output formatting for perf commands

**File(s)**: `src/perf.rs`
**Type**: Modify
**Depends**: T004, T005, T007
**Acceptance**:
- [ ] `perf start --plain` outputs: `Tracing started. File: /path/to/trace.json`
- [ ] `perf stop --plain` outputs: human-readable summary with labeled metrics
- [ ] `perf vitals --plain` outputs: labeled vitals (e.g., `LCP: 1200.5ms  CLS: 0.05  TTFB: 180.3ms`)
- [ ] `perf analyze --plain` outputs: labeled insight breakdown
- [ ] Default output (no format flag) is compact JSON (consistent with other commands)
- [ ] `cargo clippy` passes

---

## Phase 4: Testing

### T010: Create BDD feature file for perf commands

**File(s)**: `tests/features/perf.feature`
**Type**: Create
**Depends**: T008
**Acceptance**:
- [ ] All 14 acceptance criteria from requirements.md have corresponding scenarios
- [ ] Scenarios use Given/When/Then format
- [ ] Feature file is valid Gherkin syntax
- [ ] Includes Background section for shared CDP setup
- [ ] Error scenarios included (no active trace, invalid insight, missing file)

### T011: Add unit tests for trace parsing and output types

**File(s)**: `src/perf.rs`
**Type**: Modify
**Depends**: T006, T007
**Acceptance**:
- [ ] Unit tests for all output type serialization (PerfStartResult, PerfStopResult, PerfVitalsResult, PerfAnalyzeResult)
- [ ] Unit tests for `parse_trace_vitals` with known trace data
- [ ] Unit tests for TTFB extraction from trace events
- [ ] Unit tests for LCP extraction from trace events
- [ ] Unit tests for CLS extraction from trace events
- [ ] Unit tests for insight analysis functions
- [ ] Unit tests for trace file path generation
- [ ] All tests pass with `cargo test`

---

## Dependency Graph

```
T001 ──┐
       ├──▶ T003 ──▶ T004 ──┐
T002 ──┘      │              ├──▶ T009
              │      T005 ──┤
              │        │    │
              │        ▼    │
              │      T006 ──┤
              │        │    │
              │        ▼    │
              │      T007 ──┘
              │
              └──▶ T008
                      │
                      ▼
                   T010

T006, T007 ──▶ T011
```

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure
- [x] Test tasks are included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
