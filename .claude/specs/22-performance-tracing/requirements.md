# Requirements: Performance Tracing

**Issue**: #22
**Date**: 2026-02-12
**Status**: Draft
**Author**: Claude (writing-specs)

---

## User Story

**As a** developer or automation engineer
**I want** to capture and analyze Chrome performance traces from the CLI
**So that** I can diagnose page performance issues, measure Core Web Vitals, and integrate performance checks into CI/CD pipelines

---

## Background

Chrome DevTools Protocol provides the `Tracing` domain for recording detailed performance traces. These traces capture page loading, rendering, script execution, and network activity. The MCP server already exposes `performance_start_trace`, `performance_stop_trace`, and `performance_analyze_insight` tools — chrome-cli needs equivalent CLI commands.

Performance tracing is essential for developers who want to measure page load speed, identify render-blocking resources, and track Core Web Vitals (LCP, CLS, TTFB) in automated workflows. Trace files produced should be compatible with `chrome://tracing` and Chrome DevTools for deeper manual analysis.

---

## Acceptance Criteria

### AC1: Start a performance trace with default options

**Given** Chrome is running with CDP enabled and a page is loaded
**When** I run `chrome-cli perf start`
**Then** a performance trace begins recording
**And** the output contains `{"tracing": true, "file": "<path>"}`
**And** the exit code is 0

### AC2: Start a trace with reload

**Given** Chrome is running with CDP enabled and a page is loaded
**When** I run `chrome-cli perf start --reload`
**Then** the page reloads before the trace begins
**And** a performance trace begins recording
**And** the output confirms tracing is active

### AC3: Start a trace with auto-stop

**Given** Chrome is running with CDP enabled and a page is loaded
**When** I run `chrome-cli perf start --auto-stop`
**Then** a performance trace begins recording
**And** the trace automatically stops after the page load completes
**And** trace data is saved to a file
**And** the output contains a trace summary

### AC4: Start a trace with a custom output file path

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli perf start --file /tmp/my-trace.json`
**Then** the trace file is written to `/tmp/my-trace.json`
**And** the output confirms the file path

### AC5: Start a trace targeting a specific tab

**Given** Chrome is running with multiple tabs open
**When** I run `chrome-cli perf start --tab <ID>`
**Then** the trace is recorded for the specified tab
**And** the exit code is 0

### AC6: Stop an active trace

**Given** a performance trace is currently recording
**When** I run `chrome-cli perf stop`
**Then** the trace stops recording
**And** trace data is collected and saved to a file
**And** the output contains a trace summary with:
  - File path of the saved trace
  - Core Web Vitals (LCP, CLS, TTFB) when available
  - Total trace duration
**And** the exit code is 0

### AC7: Stop a trace with custom output file

**Given** a performance trace is currently recording
**When** I run `chrome-cli perf stop --file /tmp/output-trace.json`
**Then** the trace data is saved to `/tmp/output-trace.json`
**And** the output confirms the file path

### AC8: Analyze a specific performance insight

**Given** a trace file exists at a known path
**When** I run `chrome-cli perf analyze LCPBreakdown --trace-file /tmp/trace.json`
**Then** the output contains a detailed breakdown of the LCP insight
**And** the exit code is 0

### AC9: Analyze with an invalid insight name

**Given** a trace file exists
**When** I run `chrome-cli perf analyze InvalidInsight --trace-file /tmp/trace.json`
**Then** an error message indicates the insight name is not recognized
**And** the exit code is non-zero

### AC10: Quick Core Web Vitals measurement

**Given** Chrome is running with CDP enabled and a page is loaded
**When** I run `chrome-cli perf vitals`
**Then** a trace is started, the page is reloaded, the trace is stopped
**And** the output contains Core Web Vitals metrics as JSON:
  - `lcp_ms` (Largest Contentful Paint)
  - `cls` (Cumulative Layout Shift)
  - `ttfb_ms` (Time to First Byte)
**And** the exit code is 0

### AC11: Stop when no trace is active

**Given** no performance trace is currently recording
**When** I run `chrome-cli perf stop`
**Then** an error message indicates no trace is active
**And** the exit code is non-zero

### AC12: Trace file is loadable in Chrome DevTools

**Given** a trace has been recorded and saved
**When** the trace file is opened in `chrome://tracing`
**Then** it loads successfully and displays trace events

### AC13: JSON output format

**Given** Chrome is running with CDP enabled
**When** I run any `chrome-cli perf` subcommand with `--json`
**Then** the output is valid JSON matching the documented schema
**And** errors are reported as `{"error": "...", "code": N}`

### AC14: Plain text output format

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli perf vitals --plain`
**Then** the output is human-readable plain text with labeled metrics

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `perf start` begins CDP Tracing with appropriate categories | Must | Core tracing functionality |
| FR2 | `perf stop` ends tracing and collects trace data | Must | Must stream data to file |
| FR3 | `perf stop` extracts Core Web Vitals from trace data | Must | LCP, CLS, TTFB at minimum |
| FR4 | `perf analyze <INSIGHT>` provides detailed insight breakdown | Should | DocumentLatency, LCPBreakdown, RenderBlocking, LongTasks |
| FR5 | `perf vitals` runs start+reload+stop+report as a single command | Must | Convenience shorthand |
| FR6 | `--reload` flag on `perf start` reloads page before tracing | Should | Important for clean measurements |
| FR7 | `--auto-stop` flag stops trace after page load completes | Should | Listens for Page.loadEventFired |
| FR8 | `--file` flag specifies output trace file path | Must | Default: auto-generated temp file |
| FR9 | `--tab` flag (global) targets a specific tab | Must | Uses existing target resolution |
| FR10 | Trace files saved in Chrome trace event format | Must | Compatible with chrome://tracing |
| FR11 | Stream trace data to file (not hold in memory) | Should | Traces can be 10s of MB |
| FR12 | Trace duration reported in output | Should | Useful for scripting |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | `perf start` command should return within 1s; trace data streaming should not block the CLI |
| **Memory** | Trace data must be streamed to disk, not accumulated in memory (traces can be >10MB) |
| **Reliability** | If Chrome disconnects mid-trace, report partial data and save what was collected |
| **Platforms** | macOS, Linux, Windows (same as core CLI) |
| **Compatibility** | Trace files must load in Chrome DevTools and chrome://tracing |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--reload` | bool flag | N/A | No |
| `--auto-stop` | bool flag | N/A | No |
| `--file` | PathBuf | Must be a writable path | No (auto-generated default) |
| `--trace-file` | PathBuf | Must exist and be readable | Yes (for `analyze`) |
| `<INSIGHT>` | String | Must be a recognized insight name | Yes (for `analyze`) |

### Output Data — `perf start`

| Field | Type | Description |
|-------|------|-------------|
| `tracing` | bool | Whether tracing is active |
| `file` | String | Path where trace will be saved |

### Output Data — `perf stop`

| Field | Type | Description |
|-------|------|-------------|
| `file` | String | Path to saved trace file |
| `duration_ms` | u64 | Total trace duration in milliseconds |
| `vitals` | Object | Core Web Vitals (lcp_ms, cls, ttfb_ms) |
| `size_bytes` | u64 | Trace file size |

### Output Data — `perf vitals`

| Field | Type | Description |
|-------|------|-------------|
| `lcp_ms` | f64 | Largest Contentful Paint in ms |
| `cls` | f64 | Cumulative Layout Shift score |
| `ttfb_ms` | f64 | Time to First Byte in ms |
| `url` | String | URL that was measured |

### Output Data — `perf analyze`

| Field | Type | Description |
|-------|------|-------------|
| `insight` | String | Name of the insight analyzed |
| `details` | Object | Insight-specific breakdown data |

---

## Dependencies

### Internal Dependencies
- [x] Issue #4 — CDP client (implemented)
- [x] Issue #6 — Session management (implemented)

### External Dependencies
- Chrome/Chromium with Tracing domain support (all modern versions)

### Blocked By
- None (all dependencies are resolved)

---

## Out of Scope

- Full Chrome trace format parsing (use simplified extraction for v1)
- `chrome-devtools-frontend` package integration (Rust-native approach instead)
- Interactive flame chart or visualization
- Network waterfall analysis (covered by a separate `network` command)
- INP (Interaction to Next Paint) — requires user interaction to measure
- FID (First Input Delay) — requires user interaction to measure
- Continuous performance monitoring / watch mode

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Vitals accuracy | Within 10% of DevTools values | Compare `perf vitals` output to DevTools Performance panel |
| Trace compatibility | 100% loadable | Open saved traces in chrome://tracing |
| Command latency | `perf start` < 1s, `perf vitals` < 15s | Measure wall-clock time on typical pages |

---

## Open Questions

- [x] Which trace categories to include by default? → Use standard performance categories: `devtools.timeline`, `v8.execute`, `blink.user_timing`, `loading`, `disabled-by-default-devtools.timeline`
- [x] How to handle extremely large trace files? → Stream `Tracing.dataCollected` events directly to file
- [x] Rust-native trace parsing vs shelling out? → Rust-native; parse Chrome Trace Event Format JSON for CWV extraction

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented
