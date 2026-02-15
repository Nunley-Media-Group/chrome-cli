# Design: Performance Tracing

**Issue**: #22
**Date**: 2026-02-12
**Status**: Draft
**Author**: Claude (writing-specs)

---

## Overview

This feature adds a `perf` subcommand group to chrome-cli that wraps the CDP `Tracing` domain. It supports starting/stopping performance traces, extracting Core Web Vitals from trace data, and analyzing specific performance insights. The implementation follows the same patterns as existing commands (`navigate.rs`, `page.rs`): a single `src/perf.rs` module with output types, a dispatcher, session setup, and subcommand handlers.

Trace data is streamed to disk via `Tracing.dataCollected` CDP events to avoid holding large traces (10+ MB) in memory. A lightweight Rust-native trace parser extracts Core Web Vitals (LCP, CLS, TTFB) from the Chrome Trace Event Format JSON. Detailed insight analysis (LCPBreakdown, RenderBlocking, LongTasks, etc.) parses trace events by category to produce structured breakdowns.

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────┐
│                      CLI Layer                            │
│  cli/mod.rs: PerfArgs, PerfCommand enum                   │
│  main.rs: Command::Perf(args) → perf::execute_perf()      │
└────────────────────────┬─────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────┐
│                   Command Layer                           │
│  src/perf.rs                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────┐  │
│  │execute_start│  │execute_stop │  │execute_analyze   │  │
│  └──────┬──────┘  └──────┬──────┘  └───────┬──────────┘  │
│         │                │                  │             │
│  ┌──────┴──────┐  ┌──────┴──────┐  ┌───────┴──────────┐  │
│  │execute_     │  │             │  │trace_parser      │  │
│  │  vitals     │  │             │  │  module           │  │
│  └─────────────┘  └─────────────┘  └──────────────────┘  │
└────────────────────────┬─────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────┐
│                     CDP Layer                             │
│  Tracing.start / Tracing.end                              │
│  Tracing.dataCollected / Tracing.tracingComplete events   │
│  Page.reload (for --reload / vitals)                      │
└──────────────────────────────────────────────────────────┘
```

### Data Flow

#### `perf start`
```
1. Parse CLI args (PerfStartArgs)
2. Resolve connection + target (setup_session)
3. Enable Page domain (if --reload)
4. Subscribe to Tracing.tracingComplete
5. Determine trace file path (--file or auto-generated tempfile)
6. Send Tracing.start with configured categories
7. If --reload: send Page.reload, wait for Page.loadEventFired
8. If --auto-stop: wait for Page.loadEventFired, then send Tracing.end,
   subscribe to Tracing.dataCollected, stream to file, return summary
9. Otherwise: print {"tracing": true, "file": "<path>"} and exit
```

#### `perf stop`
```
1. Parse CLI args (PerfStopArgs)
2. Resolve connection + target (setup_session)
3. Subscribe to Tracing.dataCollected and Tracing.tracingComplete
4. Send Tracing.end
5. Stream Tracing.dataCollected chunks to file
6. Wait for Tracing.tracingComplete
7. Parse trace file for Core Web Vitals
8. Print summary (file path, duration, vitals, size)
```

#### `perf vitals`
```
1. Resolve connection + target (setup_session)
2. Enable Page + Tracing domains
3. Generate temp file path
4. Subscribe to Tracing.dataCollected, Tracing.tracingComplete, Page.loadEventFired
5. Send Tracing.start
6. Send Page.reload
7. Wait for Page.loadEventFired
8. Send Tracing.end
9. Stream trace data to file
10. Wait for Tracing.tracingComplete
11. Parse trace for CWV
12. Print vitals JSON
```

#### `perf analyze`
```
1. Parse CLI args (insight name, --trace-file)
2. Read trace file from disk
3. Parse trace events, filter by insight category
4. Compute insight-specific analysis
5. Print analysis JSON
```

---

## API / Interface Changes

### New CLI Commands

| Command | Purpose |
|---------|---------|
| `chrome-cli perf start` | Begin trace recording |
| `chrome-cli perf stop` | Stop trace and collect data |
| `chrome-cli perf analyze <INSIGHT>` | Analyze a performance insight |
| `chrome-cli perf vitals` | Quick Core Web Vitals measurement |

### CLI Argument Definitions

```rust
// cli/mod.rs additions

#[derive(Args)]
pub struct PerfArgs {
    #[command(subcommand)]
    pub command: PerfCommand,
}

#[derive(Subcommand)]
pub enum PerfCommand {
    /// Start a performance trace recording
    Start(PerfStartArgs),
    /// Stop the active trace and collect data
    Stop(PerfStopArgs),
    /// Analyze a specific performance insight from a trace
    Analyze(PerfAnalyzeArgs),
    /// Quick Core Web Vitals measurement
    Vitals(PerfVitalsArgs),
}

#[derive(Args)]
pub struct PerfStartArgs {
    /// Reload the page before tracing
    #[arg(long)]
    pub reload: bool,
    /// Automatically stop after page load completes
    #[arg(long)]
    pub auto_stop: bool,
    /// Path to save the trace file (default: auto-generated)
    #[arg(long)]
    pub file: Option<PathBuf>,
}

#[derive(Args)]
pub struct PerfStopArgs {
    /// Override output file path for the trace
    #[arg(long)]
    pub file: Option<PathBuf>,
}

#[derive(Args)]
pub struct PerfAnalyzeArgs {
    /// Insight name to analyze (e.g., LCPBreakdown, RenderBlocking)
    pub insight: String,
    /// Path to a previously saved trace file
    #[arg(long)]
    pub trace_file: PathBuf,
}

#[derive(Args)]
pub struct PerfVitalsArgs {
    /// Path to save the trace file (default: auto-generated temp)
    #[arg(long)]
    pub file: Option<PathBuf>,
}
```

### Output Schemas

#### `perf start` (non-auto-stop)

```json
{"tracing": true, "file": "/tmp/chrome-trace-1707753600.json"}
```

#### `perf start --auto-stop` / `perf stop`

```json
{
  "file": "/tmp/chrome-trace-1707753600.json",
  "duration_ms": 3456,
  "size_bytes": 1234567,
  "vitals": {
    "lcp_ms": 1200.5,
    "cls": 0.05,
    "ttfb_ms": 180.3
  }
}
```

#### `perf vitals`

```json
{
  "url": "https://example.com",
  "lcp_ms": 1200.5,
  "cls": 0.05,
  "ttfb_ms": 180.3
}
```

#### `perf analyze`

```json
{
  "insight": "LCPBreakdown",
  "details": {
    "ttfb_ms": 180.3,
    "load_delay_ms": 320.0,
    "load_duration_ms": 450.2,
    "render_delay_ms": 250.0,
    "total_ms": 1200.5
  }
}
```

### Error Cases

| Condition | Error Message | Exit Code |
|-----------|---------------|-----------|
| `perf stop` with no active trace | "No active trace. Run 'chrome-cli perf start' first." | 1 (GeneralError) |
| `perf analyze` with invalid insight | "Unknown insight: '{name}'. Available: DocumentLatency, LCPBreakdown, RenderBlocking, LongTasks" | 1 (GeneralError) |
| `perf analyze` with missing trace file | "Trace file not found: {path}" | 1 (GeneralError) |
| `perf analyze` with invalid trace data | "Failed to parse trace file: {error}" | 1 (GeneralError) |
| Trace timeout | "Trace timed out after {N}ms" | 4 (TimeoutError) |

---

## State Management

### Trace State

There is no persistent trace state between `perf start` and `perf stop`. The CDP `Tracing` domain manages the trace state on the browser side. The user must specify `--file` on `perf stop` to override the output path, or rely on the file path printed by `perf start`.

For `perf start` without `--auto-stop`, the command simply initiates tracing and exits. The trace continues running in the browser until `perf stop` is called or the browser session ends.

### Trace File Path Resolution

```
Priority:
1. --file <PATH> if provided
2. Auto-generated: {temp_dir}/chrome-trace-{timestamp}.json
```

---

## CDP Protocol Details

### Tracing Domain Methods

| Method | Params | Purpose |
|--------|--------|---------|
| `Tracing.start` | `categories`, `options`, `transferMode` | Start trace |
| `Tracing.end` | (none) | Stop trace |

### Tracing Domain Events

| Event | Data | Purpose |
|-------|------|---------|
| `Tracing.dataCollected` | `value: [TraceEvent, ...]` | Chunks of trace data |
| `Tracing.tracingComplete` | `dataLossOccurred`, `stream` | Signal trace is done |

### Trace Categories

Default categories for performance tracing:

```
devtools.timeline,v8.execute,blink.user_timing,loading,
disabled-by-default-devtools.timeline,disabled-by-default-lighthouse
```

### Transfer Mode

Use `transferMode: "ReturnAsStream"` is complex (requires IO.read). For v1, use the default `ReportEvents` mode where trace data arrives via `Tracing.dataCollected` events. This is simpler and sufficient for our needs — we stream each chunk to the file as it arrives.

---

## Trace Parsing (Core Web Vitals Extraction)

### Chrome Trace Event Format

Trace files are JSON arrays of trace event objects:

```json
{"traceEvents": [
  {"pid": 1, "tid": 1, "ts": 123456, "ph": "X", "cat": "loading", "name": "...", "args": {...}},
  ...
]}
```

### CWV Extraction Strategy

| Metric | Trace Event Source | Extraction Method |
|--------|-------------------|-------------------|
| **TTFB** | `ResourceSendRequest` + `ResourceReceiveResponse` for main document | Diff between response timestamp and request timestamp |
| **LCP** | `largestContentfulPaint::Candidate` in `loading` category | Read `args.data.size` and `ts` from the last LCP candidate event |
| **CLS** | `LayoutShift` events in `loading` category | Sum of `args.data.score` for events where `args.data.had_recent_input` is false |

### Insight Analysis

| Insight | Description | Trace Events Used |
|---------|-------------|-------------------|
| `DocumentLatency` | Network latency breakdown for the main document | `ResourceSendRequest`, `ResourceReceiveResponse`, `ResourceFinish` |
| `LCPBreakdown` | TTFB → Load Delay → Load Duration → Render Delay | LCP candidate + network events for LCP resource |
| `RenderBlocking` | Render-blocking resources | `ResourceSendRequest` with `renderBlocking: "blocking"` |
| `LongTasks` | Tasks > 50ms on the main thread | `RunTask` events in `devtools.timeline` with dur > 50000 (µs) |

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Shell out to Chrome DevTools Frontend** | Use the `chrome-devtools-frontend` npm package for trace analysis | Rich analysis, same as DevTools | Requires Node.js dependency, defeats "single binary" goal | Rejected — breaks zero-dependency principle |
| **B: Rust-native trace parsing** | Parse Chrome Trace Event Format in Rust, extract CWV metrics | No external dependencies, fast, predictable | More code to write, may miss edge cases | **Selected** |
| **C: Performance.getMetrics only** | Use `Performance.getMetrics` CDP method instead of tracing | Very simple, no file I/O | Limited metrics, no CWV, no trace file | Rejected — too limited for the use case |

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: ReportEvents transfer mode** | Trace data arrives via `Tracing.dataCollected` events | Simple to implement, works with existing event subscription | Data arrives in multiple chunks | **Selected** |
| **B: ReturnAsStream transfer mode** | Trace data available via `IO.read` stream | Single stream, can seek | Requires IO domain, more complex | Rejected — unnecessary complexity for v1 |

---

## Security Considerations

- [x] **File paths**: Validate that `--file` path is writable before starting trace to fail fast
- [x] **Trace data**: Trace files may contain sensitive data (URLs, cookies in network events). Users should be aware.
- [x] **No telemetry**: Trace data is only saved locally, never transmitted
- [x] **Localhost only**: CDP connections remain localhost-only by default (existing constraint)

---

## Performance Considerations

- [x] **Streaming to disk**: Trace data chunks written to file as `Tracing.dataCollected` events arrive — never accumulates full trace in memory
- [x] **File buffering**: Use `BufWriter` for efficient disk writes
- [x] **Trace parsing**: Parse trace file sequentially (streaming JSON parser not needed for v1 — `serde_json::from_reader` with `BufReader` is sufficient for traces up to ~100MB)
- [x] **Auto-stop timeout**: Default 30s timeout to prevent indefinite traces

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Output types | Unit | Serialization of all output structs |
| Trace parsing | Unit | CWV extraction from sample trace data |
| Insight analysis | Unit | Each insight type with known trace events |
| Error constructors | Unit | New error types serialize correctly |
| CLI args | Unit | Argument parsing and validation |
| Full commands | BDD | End-to-end CLI invocations with mock CDP |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Trace event format changes across Chrome versions | Low | Medium | Pin to well-documented event names from Chrome Trace Event Format spec |
| Very large traces cause OOM during parsing | Medium | High | Use streaming write; for parse, `serde_json::from_reader` with BufReader keeps memory bounded |
| CWV extraction accuracy differs from DevTools | Medium | Low | Document as "best-effort v1 extraction"; validate against DevTools for common cases |
| `Tracing.end` fails if no trace is active | Medium | Low | Handle CDP error and return user-friendly message |

---

## Open Questions

- [x] Should `perf start` block until tracing is confirmed active? → Yes, wait for `Tracing.start` response before returning
- [x] How to handle `perf stop` when user didn't use `perf start`? → CDP will return a protocol error; catch and return "No active trace" message
- [x] Should `perf vitals` clean up the temp trace file? → No, keep it for potential follow-up `perf analyze`

---

## Validation Checklist

- [x] Architecture follows existing project patterns (single module per command group)
- [x] All CLI argument structures documented
- [x] All output schemas documented with JSON examples
- [x] No database/storage changes needed
- [x] State management approach is clear (stateless — CDP manages trace state)
- [x] Security considerations addressed
- [x] Performance impact analyzed (streaming I/O)
- [x] Testing strategy defined
- [x] Alternatives considered and documented
- [x] Risks identified with mitigations
