# Root Cause Analysis: perf stop cannot find trace started by perf start — cross-invocation CDP state loss

**Issue**: #76
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude

---

## Root Cause

The `perf start` / `perf stop` workflow is fundamentally broken due to the CLI's per-invocation connection model. Each CLI invocation creates a new WebSocket connection to Chrome and a new CDP session via `Target.attachToTarget`. Chrome's `Tracing` domain binds trace state to the CDP session that initiated `Tracing.start`. When `perf start` completes and its process exits, the WebSocket connection closes, Chrome tears down the session, and the in-progress trace is terminated.

When `perf stop` runs as a separate invocation, it creates an entirely new CDP session. This new session has no knowledge of the trace started by the previous session. Sending `Tracing.end` on this new session causes Chrome to respond with "Tracing is not started", which is mapped to `AppError::no_active_trace()`.

This is not a race condition or timing issue — it is a fundamental architectural incompatibility between the stateless per-invocation CLI model and Chrome's session-scoped tracing state. No amount of retry logic or timing adjustment can fix it.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/perf.rs` | 137–147 | `setup_session()` — creates a fresh CDP client and session per invocation |
| `src/perf.rs` | 167–223 | `execute_start()` — starts trace, returns immediately, drops session |
| `src/perf.rs` | 229–237 | `execute_stop()` — creates new session, sends `Tracing.end` to wrong session |
| `src/perf.rs` | 240–264 | `stop_and_collect()` — error path maps "Tracing is not started" to `no_active_trace()` |
| `src/error.rs` | 173–178 | `no_active_trace()` — the error users see |

### Triggering Conditions

- User runs `perf start` without `--auto-stop` flag (the non-auto-stop path at lines 217–222)
- `perf start` process exits, dropping the `CdpClient` and `ManagedSession`
- User runs `perf stop` as a separate CLI invocation
- New `setup_session()` call creates a different session ID than the one that started the trace

---

## Fix Strategy

### Approach

Replace the broken `perf start` / `perf stop` two-command workflow with a single long-running `perf record` command. This follows the same pattern as `network follow` (`src/network.rs:923–1101`), which successfully maintains a CDP session across an event loop by staying alive as a single process.

The `perf record` command will:
1. Create a CDP session (same as current `execute_start`)
2. Start tracing with `Tracing.start`
3. Optionally reload the page (if `--reload` is specified)
4. Enter a `tokio::select!` loop waiting for either:
   - `Ctrl+C` (SIGINT) signal
   - `--duration` timeout (if specified)
5. On either event, call `stop_and_collect()` to end the trace and write the file
6. Return the trace summary (same output as current `perf stop`)

This eliminates the cross-invocation problem entirely because the session that starts the trace is the same session that stops it.

The existing `perf start` and `perf stop` subcommands will be removed from the CLI since they cannot work correctly with per-invocation sessions and their continued presence would confuse users.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/perf.rs` | Add `execute_record()` function implementing long-running trace with signal/timeout handling | Core fix — single-session tracing |
| `src/perf.rs` | Remove `execute_start()` and `execute_stop()` functions | Dead code — these cannot work correctly |
| `src/perf.rs` | Update `execute_perf()` dispatcher to route `PerfCommand::Record` | Wire up new command |
| `src/cli/mod.rs` | Replace `PerfCommand::Start` and `PerfCommand::Stop` with `PerfCommand::Record(PerfRecordArgs)` | CLI interface change |
| `src/cli/mod.rs` | Add `PerfRecordArgs` struct with `--file`, `--duration`, `--reload`, `--auto-dismiss-dialogs` flags | New command arguments |
| `src/cli/mod.rs` | Remove `PerfStartArgs` and `PerfStopArgs` structs | No longer needed |

### Blast Radius

- **Direct impact**: `src/perf.rs` (execute_start, execute_stop replaced by execute_record), `src/cli/mod.rs` (PerfCommand enum, arg structs)
- **Indirect impact**: Any scripts or documentation referencing `perf start` / `perf stop` will break. The `perf vitals` and `perf analyze` commands are unaffected (they already work as single-invocation commands).
- **Risk level**: Medium — this is a breaking CLI interface change, but the existing `perf start`/`perf stop` workflow never worked, so no functioning workflows are broken.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `perf vitals` breaks due to shared code changes | Low | `perf vitals` already uses its own tracing flow (start → reload → wait → stop) within a single invocation; it does not share code paths being removed |
| `perf analyze` breaks | Very Low | `perf analyze` operates on trace files only, no CDP session involvement |
| Signal handling interferes with other async tasks | Low | Use `tokio::signal::ctrl_c()` in `tokio::select!`, same proven pattern as `network follow` |
| Trace data incomplete on Ctrl+C | Low | `stop_and_collect()` already handles streaming and draining; Ctrl+C triggers graceful stop, not abrupt exit |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| **Persist session ID in state file** | Save the CDP session ID from `perf start` to `~/.chrome-cli/trace-state.json`, reattach in `perf stop` | Chrome terminates the trace when the WebSocket connection closes, regardless of session ID persistence. The connection itself cannot be persisted across process boundaries. |
| **Background daemon** | Spawn a background process that keeps the WebSocket alive between `perf start` and `perf stop` | Significantly more complex (process management, IPC, cleanup on crash). Overkill when a single long-running command solves the problem simply. |
| **`--auto-stop` as default** | Make `perf start --auto-stop` the recommended workflow | Only works for page-load scenarios. Users need to trace arbitrary interactions (clicks, form submissions) which require manual stop. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`) — mirrors `network follow` pattern
