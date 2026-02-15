# Defect Report: perf stop cannot find trace started by perf start — cross-invocation CDP state loss

**Issue**: #76
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude
**Severity**: Critical
**Related Spec**: N/A

---

## Reproduction

### Steps to Reproduce

1. Launch Chrome with remote debugging enabled (`--remote-debugging-port=9222`)
2. Run `chrome-cli perf start --pretty`
3. Observe success output: `{"tracing":true,"file":"/var/folders/.../chrome-trace-NNNN.json"}`
4. Immediately run `chrome-cli perf stop --pretty`

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (any), Linux (any) |
| **Version / Commit** | Current `main` branch |
| **Browser / Runtime** | Chrome/Chromium with `--remote-debugging-port` |
| **Configuration** | Default options; no `--auto-stop` or `--reload` flags |

### Frequency

Always — 100% reproducible. The bug is architectural, not a race condition.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `perf stop` stops the active trace started by `perf start`, collects trace data, writes it to the file, and returns vitals summary |
| **Actual** | `perf stop` fails with `{"error":"No active trace. Run 'chrome-cli perf start' first.","code":1}` |

### Error Output

```json
{"error":"No active trace. Run 'chrome-cli perf start' first.","code":1}
```

This error is generated when Chrome responds to `Tracing.end` with "Tracing is not started" because the new CDP session created by `perf stop` never initiated a trace.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Long-running perf record command captures a complete trace

**Given** Chrome is running with CDP enabled and a page is loaded
**When** the user runs `chrome-cli perf record` (or `perf start --record`) which starts tracing, waits for a signal/timeout, then stops and collects in the same session
**Then** the trace file is written with valid Chrome Trace Event Format data
**And** the exit code is 0
**And** Core Web Vitals are extracted from the trace

### AC2: Ctrl+C gracefully stops recording

**Given** a `perf record` command is actively recording a trace
**When** the user sends SIGINT (Ctrl+C)
**Then** the trace is stopped gracefully via `Tracing.end`
**And** collected trace data is written to the output file
**And** the exit code is 0

### AC3: Timeout stops recording automatically

**Given** a `perf record` command is actively recording with a `--duration` timeout
**When** the timeout duration elapses
**Then** the trace is stopped automatically
**And** collected trace data is written to the output file
**And** the exit code is 0

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Implement a `perf record` subcommand that starts tracing, holds the session open, and stops on signal or timeout — avoiding the cross-invocation session problem entirely | Must |
| FR2 | `perf record` must accept `--file` for custom output path, `--duration` for auto-stop timeout, and `--reload` to reload the page before recording | Must |
| FR3 | `perf record` must handle Ctrl+C (SIGINT) gracefully, stopping the trace and writing collected data before exiting | Must |
| FR4 | Remove or deprecate the broken `perf start` / `perf stop` two-command workflow since it cannot work with per-invocation CDP sessions | Should |

---

## Out of Scope

- Background daemon process for persistent CDP sessions
- Persisting WebSocket session IDs across CLI invocations
- Changes to `perf analyze` or `perf vitals` (these already work correctly as single-invocation commands)
- Multi-tab tracing support

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC2, AC3 cover edge cases)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
