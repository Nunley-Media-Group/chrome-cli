# Tasks: Session and Connection Management

**Issues**: #6, #185
**Date**: 2026-04-18
**Status**: Planning
**Author**: Claude (spec-driven)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #6 | 2026-02-11 | Initial task breakdown — T001–T013 across 5 phases |
| #185 | 2026-04-18 | Adds Phase 6 (T014–T037) for session reconnection, keep-alive, clap/capabilities/man/README coverage |

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Core Implementation | 4 | [ ] |
| CLI & Command Wiring | 4 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| Session Reconnection & Keep-Alive (#185) | 24 | [ ] |
| **Total** | **37** | |

---

## Task Format

Each task follows this structure:

```
### T[NNN]: [Task Title]

**File(s)**: `{layer}/path/to/file`
**Type**: Create | Modify | Delete
**Depends**: T[NNN], T[NNN] (or None)
**Acceptance**:
- [ ] [Verifiable criterion 1]
- [ ] [Verifiable criterion 2]

**Notes**: [Optional implementation hints]
```

Map `{layer}/` placeholders to actual project paths using `structure.md`.

---

## Phase 1: Setup

### T001: Create session types and error types

**File(s)**: `src/session.rs`, `src/lib.rs`
**Type**: Create (session.rs), Modify (lib.rs)
**Depends**: None
**Acceptance**:
- [ ] `src/session.rs` exists with `SessionData` struct (Serialize + Deserialize): `ws_url`, `port`, `pid`, `timestamp`
- [ ] `SessionError` enum defined: `NoHomeDir`, `Io(std::io::Error)`, `InvalidFormat(String)`
- [ ] `SessionError` implements `Display`, `Error`
- [ ] `From<SessionError>` for `AppError` with correct `ExitCode` mappings
- [ ] `session_file_path() -> Result<PathBuf, SessionError>` returns `~/.agentchrome/session.json` (cross-platform)
- [ ] `src/lib.rs` exports `pub mod session`
- [ ] `cargo check` passes

**Notes**: Use `std::env::var("HOME")` on Unix, `std::env::var("USERPROFILE")` on Windows for home directory resolution. Follow the same error pattern as `src/cdp/error.rs` and `src/chrome/error.rs`.

### T002: Add --status and --disconnect CLI flags

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `ConnectArgs` gains `status: bool` with `#[arg(long, conflicts_with_all = ["launch", "disconnect"])]`
- [ ] `ConnectArgs` gains `disconnect: bool` with `#[arg(long, conflicts_with_all = ["launch", "status"])]`
- [ ] `agentchrome connect --help` shows `--status` and `--disconnect` flags
- [ ] `agentchrome connect --status --launch` produces a clap conflict error
- [ ] `agentchrome connect --status --disconnect` produces a clap conflict error
- [ ] Existing tests still pass: `cargo test`
- [ ] `cargo clippy` passes

**Notes**: These are simple boolean flags. The `conflicts_with_all` ensures mutual exclusivity with `--launch` and each other.

---

## Phase 2: Core Implementation

### T003: Implement session file management

**File(s)**: `src/session.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `write_session(data: &SessionData) -> Result<(), SessionError>`:
  - Creates `~/.agentchrome/` directory if it doesn't exist (mode `0o700` on Unix)
  - Writes session JSON to a temp file then renames to `session.json` (atomic write)
  - Sets file permissions to `0o600` on Unix
- [ ] `read_session() -> Result<Option<SessionData>, SessionError>`:
  - Returns `Ok(None)` if file doesn't exist
  - Returns `Ok(Some(data))` if file exists and parses correctly
  - Returns `Err(InvalidFormat)` if file contains invalid JSON
- [ ] `delete_session() -> Result<(), SessionError>`:
  - Removes session file if it exists
  - Returns `Ok(())` even if file doesn't exist (idempotent)
- [ ] `now_iso8601() -> String` helper: formats current time as `"2026-02-11T12:00:00Z"` using `SystemTime`
- [ ] Unit test: write → read round-trip in temp directory
- [ ] Unit test: read from nonexistent file returns `None`
- [ ] Unit test: read invalid JSON returns `InvalidFormat`
- [ ] Unit test: delete nonexistent file returns `Ok(())`
- [ ] `cargo clippy` passes

**Notes**: For the atomic write, use `std::fs::write` to a temp file (e.g., `session.json.tmp`) in the same directory, then `std::fs::rename`. For `now_iso8601`, compute from `SystemTime::now().duration_since(UNIX_EPOCH)` with manual date/time arithmetic. The testable version should accept a custom base path for the session directory.

### T004: Implement health check function

**File(s)**: `src/connection.rs`, `src/lib.rs`
**Type**: Create (connection.rs), Modify (lib.rs)
**Depends**: T001
**Acceptance**:
- [ ] `src/connection.rs` exists
- [ ] `health_check(host: &str, port: u16) -> Result<(), AppError>`:
  - Calls `chrome::query_version(host, port)`
  - Returns `Ok(())` if Chrome responds
  - Returns `Err(AppError::stale_session())` if Chrome is unreachable
- [ ] `src/lib.rs` exports `pub mod connection`
- [ ] `cargo check` passes

**Notes**: Reuses existing `query_version` which has a 2-second TCP connect timeout. This is the health check — fast for local connections.

### T005: Implement connection resolution chain

**File(s)**: `src/connection.rs`
**Type**: Modify
**Depends**: T003, T004
**Acceptance**:
- [ ] `ResolvedConnection` struct defined: `ws_url: String`, `host: String`, `port: u16`
- [ ] `resolve_connection(global: &GlobalOpts) -> Result<ResolvedConnection, AppError>`:
  1. If `--ws-url` provided → extract port, return immediately
  2. If `--port` explicitly provided (user passed `--port`) → discover on that port via `query_version`
  3. Read session file → if found, health-check, return stored `ws_url`
  4. Auto-discover via `discover_chrome("127.0.0.1", 9222)`
  5. Return error with suggestion to run `agentchrome connect`
- [ ] Step 3: on health check failure, return stale session error (not fallthrough)
- [ ] `cargo clippy` passes

**Notes**: For step 2, detecting "user explicitly passed --port" vs "default 9222" is tricky with clap defaults. The simplest approach: if `global.ws_url` is Some, that takes priority. Otherwise if any session file exists, use it. The session file is the primary mechanism; `--port` and `--host` override it when explicitly provided. We can add a helper flag or check if the port differs from the default.

Actually, the cleaner approach: always check session file after explicit flags. If `--ws-url` is provided, use it. If `--port` is provided, discover on that port. Otherwise, try session file, then auto-discover. Since clap provides a default value for `--port`, we cannot distinguish "user typed `--port 9222`" from "default 9222" without extra work. So the chain is: `--ws-url` → session file → discover on `host:port` (which defaults to 9222) → error.

### T006: Implement tab targeting

**File(s)**: `src/connection.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `resolve_target(host: &str, port: u16, tab: Option<&str>) -> Result<TargetInfo, AppError>`:
  - Calls `chrome::query_targets(host, port)` to get target list
  - If `tab` is `None`: return first target with `target_type == "page"`, or `AppError::no_page_targets()` if none
  - If `tab` is `Some(value)`:
    a. Try parsing as `usize` → index into target list → return if in bounds
    b. Search for target with matching `id` field → return if found
    c. Return `AppError::target_not_found(value)`
- [ ] `TargetInfo` from `chrome::discovery` is reused (already public)
- [ ] Unit test: resolve by index 0 picks first target
- [ ] Unit test: resolve by target ID matches correct target
- [ ] Unit test: no tab option picks first page target, skipping non-page targets
- [ ] Unit test: invalid tab returns target_not_found error
- [ ] Unit test: empty target list returns no_page_targets error
- [ ] `cargo clippy` passes

**Notes**: The function fetches targets fresh each time via HTTP. This ensures the list is current and avoids caching stale data.

---

## Phase 3: CLI & Command Wiring

### T007: Update connect command to write session file

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] After successful connect (all strategies: `--ws-url`, `--launch`, auto-discover), `write_session` is called
- [ ] `SessionData` populated with ws_url, port, pid (if launched), and ISO 8601 timestamp
- [ ] Session file write failure is non-fatal: log warning to stderr, still output connection info
- [ ] Existing connect behavior is unchanged (same stdout output, same exit codes)
- [ ] `cargo clippy` passes

**Notes**: The session write is a "best-effort" side effect of the connect command. If the write fails (e.g., permission denied), the connect still succeeds because the user got their connection info. Print a warning to stderr like: `"warning: could not save session file: {error}"`.

### T008: Implement connect --status handler

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T002, T003, T004
**Acceptance**:
- [ ] `execute_connect` checks `args.status` early and dispatches to `execute_status`
- [ ] `execute_status`:
  - Reads session file via `read_session()`
  - If no session: returns `AppError::no_session()`
  - If session found: runs `health_check(host, port)` and captures result
  - Outputs `StatusInfo` as JSON: `{"ws_url", "port", "pid", "timestamp", "reachable": bool}`
- [ ] `StatusInfo` struct with serde `Serialize`
- [ ] With valid session and Chrome running: outputs reachable=true, exit 0
- [ ] With valid session and Chrome stopped: outputs reachable=false, exit 0
- [ ] With no session: error to stderr, exit 2
- [ ] `cargo clippy` passes

**Notes**: `--status` always returns exit 0 if a session file exists, even if Chrome is unreachable. The `reachable` field tells the user the state. Only the "no session" case returns a non-zero exit code.

### T009: Implement connect --disconnect handler with process kill

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T002, T003
**Acceptance**:
- [ ] `execute_connect` checks `args.disconnect` early and dispatches to `execute_disconnect`
- [ ] `execute_disconnect`:
  - Reads session file via `read_session()`
  - If session has a PID: attempts to kill the process
  - Deletes session file via `delete_session()`
  - Outputs `DisconnectInfo` as JSON: `{"disconnected": true, "killed_pid": N|null}`
- [ ] `DisconnectInfo` struct with serde `Serialize`
- [ ] Process kill implementation:
  - Unix: `std::process::Command::new("kill").arg(pid.to_string())` (SIGTERM)
  - Windows: `std::process::Command::new("taskkill").args(["/PID", &pid.to_string()])`
  - Kill failure silently ignored (process may already be dead)
- [ ] With session + PID: kills process, deletes file, exit 0
- [ ] With session + no PID: deletes file, exit 0
- [ ] With no session: deletes file (no-op), exit 0
- [ ] `cargo clippy` passes

**Notes**: Disconnect is always "successful" even if there's nothing to disconnect. This is consistent with idempotent CLI patterns (like `rm -f`).

### T010: Add session and target error constructors to AppError

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `AppError::stale_session()` → ExitCode::ConnectionError, message suggests `agentchrome connect`
- [ ] `AppError::no_session()` → ExitCode::ConnectionError, message suggests `agentchrome connect` or `--launch`
- [ ] `AppError::target_not_found(tab: &str)` → ExitCode::TargetError, message suggests `tabs list`
- [ ] `AppError::no_page_targets()` → ExitCode::TargetError, message suggests opening a tab
- [ ] Unit tests for each constructor: verify message content and exit code
- [ ] `cargo clippy` passes

**Notes**: Follow the existing pattern of `AppError::not_implemented()`. Error messages should be actionable per product.md brand voice guidelines.

---

## Phase 3: Frontend Implementation

### T007: [Client-side model]

**File(s)**: `{presentation-layer}/models/...`
**Type**: Create
**Depends**: T002
**Acceptance**:
- [ ] Model matches API response schema
- [ ] Serialization/deserialization works
- [ ] Immutable with update method (if applicable)
- [ ] Unit tests for serialization

### T008: [Client-side service / API client]

**File(s)**: `{presentation-layer}/services/...`
**Type**: Create
**Depends**: T007
**Acceptance**:
- [ ] All API calls implemented
- [ ] Error handling with typed exceptions
- [ ] Uses project's HTTP client pattern
- [ ] Unit tests pass

### T009: [State management]

**File(s)**: `{presentation-layer}/state/...` or `{presentation-layer}/providers/...`
**Type**: Create
**Depends**: T008
**Acceptance**:
- [ ] State class defined (immutable if applicable)
- [ ] Loading/error states handled
- [ ] State transitions match design spec
- [ ] Unit tests for state transitions

### T010: [UI components]

**File(s)**: `{presentation-layer}/components/...` or `{presentation-layer}/widgets/...`
**Type**: Create
**Depends**: T009
**Acceptance**:
- [ ] Components match design specs
- [ ] Uses project's design tokens (no hardcoded values)
- [ ] Loading/error/empty states
- [ ] Component tests pass

### T011: [Screen / Page]

**File(s)**: `{presentation-layer}/screens/...` or `{presentation-layer}/pages/...`
**Type**: Create
**Depends**: T010
**Acceptance**:
- [ ] Screen layout matches design
- [ ] State management integration working
- [ ] Navigation implemented

---

## Phase 4: Integration

### T011: Integration wiring and verification

**File(s)**: `src/main.rs`, `src/lib.rs`
**Type**: Modify
**Depends**: T005, T006, T007, T008, T009, T010
**Acceptance**:
- [ ] All new modules (`session`, `connection`) accessible from integration tests via `lib.rs`
- [ ] `cargo build` succeeds
- [ ] `cargo clippy -- -D warnings` passes (all clippy lints clean)
- [ ] `cargo fmt --check` passes
- [ ] Existing tests pass: `cargo test`
- [ ] Manual verification (if Chrome available):
  - `cargo run -- connect` writes session file to `~/.agentchrome/session.json`
  - `cargo run -- connect --status` reads session and shows reachable=true
  - `cargo run -- connect --disconnect` removes session file
  - Session file contains valid JSON with expected fields

---

## Phase 5: Testing

### T012: Unit tests for session and connection modules

**File(s)**: `src/session.rs` (inline tests), `src/connection.rs` (inline tests)
**Type**: Modify
**Depends**: T011
**Acceptance**:
- [ ] Session tests (use temp directory, not real `~/.agentchrome/`):
  - Write/read round-trip preserves all fields
  - Read nonexistent file returns `None`
  - Read invalid JSON returns `InvalidFormat` error
  - Delete nonexistent file returns `Ok(())`
  - `now_iso8601()` produces valid ISO 8601 format
  - `session_file_path()` returns a path ending in `.agentchrome/session.json`
- [ ] Connection resolution tests (mock-based or logic tests):
  - `resolve_target` with no tab picks first page target
  - `resolve_target` with numeric index picks correct target
  - `resolve_target` with target ID matches correct target
  - `resolve_target` with invalid tab returns error
  - `resolve_target` with empty list returns error
  - `resolve_target` skips non-page targets when no tab specified
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy` passes

**Notes**: Session tests should use a helper that creates a temp dir and overrides the session path. Connection resolution tests can construct `Vec<TargetInfo>` directly for the tab targeting logic, and may need to extract the target selection logic into a pure function that doesn't do HTTP.

### T013: Create BDD feature file for session and connection management

**File(s)**: `tests/features/session-connection-management.feature`
**Type**: Create
**Depends**: T011
**Acceptance**:
- [ ] Feature file contains scenarios for all 20 acceptance criteria from requirements
- [ ] Uses Given/When/Then format
- [ ] Includes Background section for common setup
- [ ] Valid Gherkin syntax
- [ ] Covers:
  - Session file write after connect (AC1, AC2)
  - Session file auto-read (AC3, AC4)
  - Status and disconnect (AC5-AC8)
  - Connection resolution chain (AC9-AC11)
  - Tab targeting (AC15-AC18)
  - CDP session lifecycle (AC12-AC14)
  - Error handling (AC19, AC20)
- [ ] Scenarios tagged appropriately (`@requires-chrome` for Chrome-dependent tests)

**Notes**: Some scenarios can be validated through unit-level step definitions (session file read/write). Others require Chrome and should be tagged for conditional execution.

---

## Dependency Graph

```
T001 ──┬──▶ T003 ──┬──▶ T005 ──┐
       │           │           │
       ├──▶ T004 ──┤──▶ T006 ──┤──▶ T011 ──┬──▶ T012
       │           │           │           │
       └──▶ T010   │    T007 ──┤           └──▶ T013
                   │           │
T002 ──────────────┴──▶ T008 ──┤
                       T009 ──┘
```

**Parallel tracks:**
- T001 and T002 can proceed in parallel (no dependency between them)
- T003 (session file) and T004 (health check) can proceed in parallel after T001
- T010 (error constructors) can proceed in parallel with T003/T004 after T001
- T007, T008, T009 can proceed in parallel once their dependencies are met

**Critical path:** T001 → T003 → T005 → T011 → T012

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included (T012, T013)
- [x] No circular dependencies
- [x] Tasks are in logical execution order

---

## Phase 6: Session Reconnection & Keep-Alive (Issue #185)

### T014: Extend SessionData with reconnect telemetry

**File(s)**: `src/session.rs`
**Type**: Modify
**Depends**: None (builds on T001/T003 which are complete)
**Acceptance**:
- [ ] `SessionData` gains `last_reconnect_at: Option<String>` with `#[serde(skip_serializing_if = "Option::is_none", default)]`
- [ ] `SessionData` gains `reconnect_count: u32` with `#[serde(default)]`
- [ ] A pre-#185 session file (without these fields) round-trips through `read_session` → `write_session` without data loss, producing `reconnect_count = 0` and `last_reconnect_at = None` on read
- [ ] Unit test: deserialize legacy JSON `{"ws_url":..., "port":..., "timestamp":...}` populates defaults correctly
- [ ] Unit test: round-trip with both new fields set preserves values
- [ ] `cargo clippy` passes

**Notes**: Serde defaults must be in place before any code reads these fields, so this must land before T015.

### T015: Add session::rewrite_preserving helper

**File(s)**: `src/session.rs`
**Type**: Modify
**Depends**: T014
**Acceptance**:
- [ ] New public function signature:
  `pub fn rewrite_preserving(existing: &SessionData, new_ws_url: String) -> Result<SessionData, SessionError>`
- [ ] Returns a `SessionData` where `ws_url` is updated, `pid` / `port` / `active_tab_id` are preserved from `existing`, `timestamp` and `last_reconnect_at` are set to `now_iso8601()`, `reconnect_count` is `existing.reconnect_count + 1`
- [ ] Writes atomically via temp file + rename (reuse T003 pattern)
- [ ] Returns the newly-persisted `SessionData` for callers that need to return it
- [ ] Unit test: preserves pid=12345 across rewrite
- [ ] Unit test: preserves active_tab_id across rewrite
- [ ] Unit test: increments reconnect_count by exactly 1 per call
- [ ] Unit test: sets last_reconnect_at to a fresh ISO 8601 timestamp
- [ ] `cargo clippy` passes

**Notes**: If the session file has been deleted between the read and this write, fall back to a full `write_session` rather than erroring — the resolution chain will handle the first-time case.

### T016: Add KeepAliveConfig and extend CdpConfig

**File(s)**: `src/cdp/transport.rs`, `src/cdp/client.rs`, `src/cdp/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `pub struct KeepAliveConfig { interval: Option<Duration>, pong_timeout: Duration }` added to `src/cdp/transport.rs`, publicly exported
- [ ] `Default` impl: `interval = Some(Duration::from_secs(30))`, `pong_timeout = Duration::from_secs(10)`
- [ ] `CdpConfig` gains `pub keepalive: KeepAliveConfig` field; `Default` impl uses `KeepAliveConfig::default()`
- [ ] `spawn_transport` signature extended with `keepalive: KeepAliveConfig` parameter (passed through `CdpClient::connect`)
- [ ] All call sites that construct `CdpConfig` compile (either via `Default` or by providing the new field)
- [ ] `cargo check` passes
- [ ] `cargo clippy` passes

### T017: Implement keep-alive loop inside TransportTask

**File(s)**: `src/cdp/transport.rs`
**Type**: Modify
**Depends**: T016
**Acceptance**:
- [ ] `TransportTask` stores `keepalive: KeepAliveConfig`, `last_outbound: Instant`, `last_ping_at: Option<Instant>`
- [ ] `tokio::select!` in `run()` gains a fourth branch that fires when `Instant::now() >= last_outbound + keepalive.interval` and `keepalive.interval.is_some()`:
  - Sends `Message::Ping(Vec::new().into())` via `ws_stream.send`
  - Updates `last_outbound = Instant::now()` and `last_ping_at = Some(Instant::now())`
- [ ] A fifth branch fires when `last_ping_at` is `Some(t)` and `Instant::now() >= t + keepalive.pong_timeout`: triggers `handle_disconnect()` then clears `last_ping_at`
- [ ] The `ws_msg` branch gains a match arm for `Message::Pong(_)` that clears `last_ping_at = None`
- [ ] `handle_send_command` updates `last_outbound = Instant::now()` after a successful send
- [ ] When `keepalive.interval == None`, the keep-alive and pong-deadline branches are `std::future::pending()` (never fire)
- [ ] Unit test (mock WebSocket): idle connection emits a Ping after the interval elapses
- [ ] Unit test: Pong clears `last_ping_at` without triggering disconnect
- [ ] Unit test: active JSON-RPC traffic (every `ws_stream.send`) resets the keep-alive timer — no Ping fires during steady traffic
- [ ] Unit test: no Ping emitted when `keepalive.interval == None`
- [ ] Unit test: no Pong within `pong_timeout` triggers `handle_disconnect` (observable via `connected` atomic flipping to false then true after reconnect)
- [ ] `cargo clippy` passes

**Notes**: Satisfies AC28, AC30, AC31, FR15. Use the existing mock-WS pattern from `managed_session_enables_domain_once` for unit tests.

### T018: Add structured-loss AppError constructors

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AppError::chrome_terminated()` constructor returns `ExitCode::ConnectionError` with `custom_json` containing `{"kind":"chrome_terminated","recoverable":false}` and a human message suggesting `agentchrome connect --launch`
- [ ] `AppError::transient_connection_loss(detail: impl Into<String>)` constructor returns `ExitCode::ConnectionError` with `custom_json` containing `{"kind":"transient","recoverable":true}` and a human message suggesting `agentchrome connect`
- [ ] Unit test: `chrome_terminated` renders stderr JSON containing `"kind":"chrome_terminated"` and `"recoverable":false`
- [ ] Unit test: `transient_connection_loss("probe timeout")` renders stderr JSON containing `"kind":"transient"` and `"recoverable":true` and the detail in the message
- [ ] `cargo clippy` passes

**Notes**: Satisfies AC26 / AC27 / FR16.

### T019: Add ReconnectPolicy type

**File(s)**: `src/connection.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `pub struct ReconnectPolicy { max_attempts: u32, initial_backoff: Duration, max_backoff: Duration, probe_timeout_ms: u64, verbose: bool }` defined and publicly exported
- [ ] `Default`: `max_attempts=3, initial_backoff=100ms, max_backoff=5s, probe_timeout_ms=500, verbose=false`
- [ ] `cargo check` passes
- [ ] `cargo clippy` passes

### T020: Implement rediscover_on_stored_port

**File(s)**: `src/connection.rs`
**Type**: Modify
**Depends**: T014, T015, T019
**Acceptance**:
- [ ] New internal async function:
  `async fn rediscover_on_stored_port(host: &str, port: u16, policy: &ReconnectPolicy) -> Result<String, AppError>`
- [ ] Calls `query_version(host, port)` wrapped in `tokio::time::timeout(Duration::from_millis(policy.probe_timeout_ms))`; returns the fresh `ws_debugger_url` on success
- [ ] Retries per `policy.max_attempts` with exponential backoff between `initial_backoff` and `max_backoff`
- [ ] Per-attempt latency never exceeds `probe_timeout_ms` (verified by unit test with a stalled mock server)
- [ ] Unit test: succeeds on attempt 1 if mock responds
- [ ] Unit test: retries `max_attempts` times then returns Err
- [ ] Unit test: each attempt respects `probe_timeout_ms` (bound the total-duration variance)
- [ ] `cargo clippy` passes

**Notes**: Satisfies AC24 / AC25.

### T021: Implement classify_loss with PID liveness check

**File(s)**: `src/connection.rs`, `src/chrome/platform.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `LossKind` enum: `ChromeTerminated`, `Transient`
- [ ] `fn classify_loss(stored_pid: Option<u32>, _final_error: &str) -> (LossKind, bool)` (second tuple element is `recoverable`)
- [ ] Platform probe `fn is_process_alive(pid: u32) -> ProbeResult` added to `chrome/platform.rs` with variants `Alive`, `Dead`, `Unknown`:
  - Unix: `unsafe { libc::kill(pid as i32, 0) }` — returns `Dead` on `ESRCH`, `Unknown` on `EPERM`, `Alive` otherwise
  - Windows: shell out to `tasklist /FI "PID eq <pid>" /NH` and check output parsing; `Unknown` if the call fails
- [ ] Return `(ChromeTerminated, false)` when probe is `Dead`
- [ ] Return `(Transient, true)` when probe is `Alive`, `Unknown`, or when `stored_pid is None`
- [ ] Unit test: `stored_pid = None` → `Transient, true`
- [ ] Unit test (manual mockable): `is_process_alive(u32::MAX)` → `Dead` on Unix
- [ ] `cargo clippy` passes

**Notes**: Satisfies the defensive posture documented in design — default to `Transient` when uncertain. Avoid adding a `nix` dependency if `libc` is already transitively present; otherwise use `libc` directly.

### T022: Implement resolve_connection_with_reconnect

**File(s)**: `src/connection.rs`
**Type**: Modify
**Depends**: T015, T019, T020, T021
**Acceptance**:
- [ ] New async function:
  `async fn resolve_connection_with_reconnect(host: &str, port: Option<u16>, ws_url: Option<&str>, policy: &ReconnectPolicy) -> Result<ResolvedConnection, AppError>`
- [ ] When `ws_url` provided: short-circuits and returns immediately (no reconnect path)
- [ ] When `port` explicitly provided: same behavior as existing `resolve_connection` (single probe, error if fails)
- [ ] When session file exists and `health_check` fails:
  1. Call `rediscover_on_stored_port(host, session.port, policy)`
  2. On success: call `session::rewrite_preserving` and return the new `ResolvedConnection`
  3. On failure: try `discover_chrome(host, DEFAULT_CDP_PORT)` with policy
  4. On both failing: `classify_loss(session.pid, ...)` and return `AppError::chrome_terminated()` or `AppError::transient_connection_loss(...)`
- [ ] Verbose path (`policy.verbose == true`): emit `tracing::info!` for each probe with attempt number, target, and duration; NEVER `println!` / stdout
- [ ] Unit test: session + reachable port → returns stored ws_url, no rewrite
- [ ] Unit test: session + stale ws_url + port reachable → rediscovers, rewrites session file with new ws_url, preserves pid
- [ ] Unit test: session + port unreachable + pid dead → returns `chrome_terminated` error
- [ ] Unit test: session + port unreachable + pid alive → returns `transient` error
- [ ] `cargo clippy` passes

**Notes**: Satisfies AC21, AC23, AC25, AC26, AC27.

### T023: Implement connect_for_command single entry point

**File(s)**: `src/connection.rs`, `src/lib.rs`
**Type**: Modify
**Depends**: T016, T019, T022
**Acceptance**:
- [ ] `pub struct CommandConnection { pub client: CdpClient, pub resolved: ResolvedConnection, pub reconnected: bool }`
- [ ] `pub async fn connect_for_command(global: &GlobalOpts, keepalive: KeepAliveConfig, reconnect: ReconnectPolicy) -> Result<CommandConnection, AppError>`
- [ ] Internally: call `resolve_connection_with_reconnect`, build `CdpConfig` with `keepalive`, call `CdpClient::connect`
- [ ] `reconnected: true` when `resolve_connection_with_reconnect` rewrote the session file
- [ ] Unit test: reachable session → returns CommandConnection with `reconnected=false`
- [ ] Unit test: stale session + reachable port → returns CommandConnection with `reconnected=true`, session file is rewritten
- [ ] `cargo clippy` passes

**Notes**: Satisfies FR18 by providing the one path.

### T024: Migrate src/output.rs call sites to connect_for_command

**File(s)**: `src/output.rs`
**Type**: Modify
**Depends**: T023
**Acceptance**:
- [ ] All direct uses of `CdpClient::connect` replaced by `connection::connect_for_command`
- [ ] `KeepAliveConfig` and `ReconnectPolicy` propagated from `GlobalOpts` via the main dispatcher
- [ ] No behavioral regressions on short commands (`tabs list`, `navigate`, etc. still pass existing BDD)
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy` passes

### T025: Migrate src/tabs.rs call sites to connect_for_command

**File(s)**: `src/tabs.rs`
**Type**: Modify
**Depends**: T023
**Acceptance**:
- [ ] All 4 direct uses of `CdpClient::connect` replaced by `connection::connect_for_command`
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy` passes

### T026: Migrate src/js.rs call sites to connect_for_command

**File(s)**: `src/js.rs`
**Type**: Modify
**Depends**: T023
**Acceptance**:
- [ ] Direct use of `CdpClient::connect` replaced by `connection::connect_for_command`
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy` passes

### T027: Migrate src/dialog.rs call sites to connect_for_command

**File(s)**: `src/dialog.rs`
**Type**: Modify
**Depends**: T023
**Acceptance**:
- [ ] Direct use of `CdpClient::connect` replaced by `connection::connect_for_command`
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy` passes

### T028: Migrate src/audit.rs and remaining call sites to connect_for_command

**File(s)**: `src/audit.rs`, any others surfaced by `grep "CdpClient::connect"`
**Type**: Modify
**Depends**: T023
**Acceptance**:
- [ ] All remaining `CdpClient::connect` sites — except `src/connection.rs::connect_for_command` itself and `src/main.rs` `connect` subcommand — are migrated or explicitly documented as exempt
- [ ] `grep -r "CdpClient::connect" src/` returns only `src/connection.rs` (the implementation), `src/main.rs` (the `connect` subcommand bootstrap), and test files
- [ ] A doc comment on `CdpClient::connect` states: "Prefer `connection::connect_for_command` unless you are implementing the `connect` subcommand itself."
- [ ] `cargo clippy` passes

**Notes**: Satisfies FR18 path audit.

### T029: Add --keepalive-interval and --no-keepalive CLI flags

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: T016
**Acceptance**:
- [ ] `GlobalOpts` gains `keepalive_interval: Option<u64>` with `#[arg(long = "keepalive-interval", value_name = "MS", value_parser = clap::value_parser!(u64), env = "AGENTCHROME_KEEPALIVE_INTERVAL", global = true, conflicts_with = "no_keepalive")]` and a doc comment
- [ ] `GlobalOpts` gains `no_keepalive: bool` with `#[arg(long = "no-keepalive", global = true)]` and a doc comment
- [ ] The `connect` subcommand's `after_long_help` EXAMPLES string is extended with at least one invocation using each flag, including at least one `--json` variant
- [ ] `agentchrome --help` prints both flags with their descriptions
- [ ] `agentchrome --help --keepalive-interval abc` produces a clap validation error (not a u64)
- [ ] `agentchrome --keepalive-interval 1000 --no-keepalive` produces a clap conflict error
- [ ] Unit test (`assert_cmd`): `--help` output contains `--keepalive-interval` and `--no-keepalive`
- [ ] `cargo clippy` passes

**Notes**: Satisfies AC34 / FR21 per `steering/tech.md` clap-help rules.

### T030: Add keepalive and reconnect config sections + precedence resolution

**File(s)**: `src/config.rs`, `src/main.rs`
**Type**: Modify
**Depends**: T029
**Acceptance**:
- [ ] `Config` struct (TOML) gains `keepalive: KeepaliveConfigFile` and `reconnect: ReconnectConfigFile` optional subsections
- [ ] `KeepaliveConfigFile { interval_ms: Option<u64> }`
- [ ] `ReconnectConfigFile { max_attempts: Option<u32>, initial_backoff_ms: Option<u64>, max_backoff_ms: Option<u64>, probe_timeout_ms: Option<u64> }`
- [ ] A helper in `main.rs` builds `KeepAliveConfig` and `ReconnectPolicy` with precedence: CLI flag > env var (already covered via clap `env`) > config.toml > compiled-in default
- [ ] `--no-keepalive` always wins (sets `interval = None`) regardless of other sources
- [ ] Setting `--keepalive-interval 0` also disables keep-alive (`interval = None`)
- [ ] Unit test: precedence ordering is respected
- [ ] Unit test: `--keepalive-interval 0` produces `KeepAliveConfig { interval: None, .. }`
- [ ] `cargo clippy` passes

**Notes**: Satisfies AC29 / AC30.

### T031: Extend connect --status output with keep-alive and reconnect telemetry

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T014, T030
**Acceptance**:
- [ ] `StatusInfo` struct gains `last_reconnect_at: Option<String>`, `reconnect_count: u32`, and `keepalive: KeepaliveStatus { interval_ms: Option<u64>, enabled: bool }`
- [ ] `keepalive.enabled == false` when interval is `None`; `enabled == true` and `interval_ms = Some(n)` when set
- [ ] When session file lacks these fields (legacy), fields default to `None` / `0` / default keep-alive
- [ ] Unit test: status JSON contains all six fields in the expected shape
- [ ] `cargo clippy` passes

**Notes**: Satisfies AC33 / FR19.

### T032: Update README.md with Session Resilience section

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T029
**Acceptance**:
- [ ] A new H2 section (or subsection of an existing session-management section) titled "Session resilience" or equivalent
- [ ] Explains:
  - Auto-reconnect behavior when Chrome is still running but the WebSocket has rotated
  - The keep-alive flag, env var (`AGENTCHROME_KEEPALIVE_INTERVAL`), config.toml key (`[keepalive].interval_ms`), and default 30 s interval
  - How to disable keep-alive (`--no-keepalive` or interval `0`)
  - How scripts distinguish `kind: chrome_terminated` from `kind: transient` and the `recoverable` boolean
- [ ] Includes at least one copy-pasteable example showing `--keepalive-interval 60000`
- [ ] BDD-testable grep: README contains strings `keepalive-interval`, `chrome_terminated`, `recoverable`
- [ ] No change to README tone or structure beyond the new section
- [ ] `markdownlint README.md` (if configured) passes

**Notes**: Satisfies AC36 / FR23.

### T033: Create BDD feature file for #185

**File(s)**: `tests/features/185-session-reconnect-keepalive.feature`
**Type**: Create
**Depends**: T023, T031, T032
**Acceptance**:
- [ ] Feature file covers AC21 through AC36 as Scenarios (scenario count ≥ 16)
- [ ] `@requires-chrome` tag on scenarios that need a live Chrome; scenarios that exercise only session-file logic run without Chrome
- [ ] Uses Scenario Outline for AC22 (uniform reconnect across commands)
- [ ] Uses Scenario Outline for AC29 (interval precedence: flag vs env vs config)
- [ ] Valid Gherkin syntax; `cargo test --test bdd -- --dry-run` (or equivalent parse check) succeeds
- [ ] File has a header comment indicating issue #185 and which ACs it covers

**Notes**: Satisfies the BDD requirement that every AC has a Gherkin scenario.

### T034: Add BDD step definitions for #185

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T033
**Acceptance**:
- [ ] Step definitions for: "a session file with a stale ws_url", "Chrome is running on the stored port", "Chrome has been terminated", "I pass --no-keepalive", "the session file contains `last_reconnect_at`"
- [ ] Non-Chrome scenarios use a temp HOME to avoid touching the developer's real `~/.agentchrome/`
- [ ] Chrome-requiring scenarios reuse the existing headless-Chrome world setup
- [ ] `cargo test --test bdd` passes (skipping `@requires-chrome` tags in CI as usual)
- [ ] `cargo clippy --tests` passes

### T035: Create test fixture for smoke test

**File(s)**: `tests/fixtures/session-reconnect-keepalive.html`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] Self-contained static HTML (no external resources)
- [ ] Includes a visible heading, a form, and a button (enough for a `page snapshot` + `form fill` + `interact click` round-trip)
- [ ] Header comment lists which ACs the fixture supports (primarily AC21–AC22, AC26)

### T036: Feature Exercise Gate — manual smoke test

**File(s)**: `verification/185-smoke.md` (inline in verification report during `/verify-code`)
**Type**: Manual
**Depends**: T034, T035
**Acceptance**:
- [ ] `cargo build` succeeds (debug)
- [ ] Headless Chrome launched, fixture navigated
- [ ] `tabs list` and `page snapshot` run cleanly with a fresh session
- [ ] Simulate stale ws_url (kill Chrome, relaunch on same port without running `connect`) → next `page snapshot` auto-reconnects, session file rewrites, stdout pure JSON
- [ ] Simulate Chrome termination (kill with SIGKILL) → next command emits `chrome_terminated` error JSON with `recoverable: false`
- [ ] Run `agentchrome console follow` against headless Chrome for > 60 s with `--keepalive-interval 30000` → connection stays alive
- [ ] Disconnect + kill orphan Chrome processes per `steering/tech.md` cleanup rules

### T037: Verify no regressions

**File(s)**: Full workspace
**Type**: Verification
**Depends**: T024–T036
**Acceptance**:
- [ ] `cargo build` succeeds
- [ ] `cargo test --lib` passes
- [ ] `cargo test --test bdd` passes (skipping `@requires-chrome` tags in CI)
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo xtask man connect` renders with the new flags
- [ ] `agentchrome capabilities` JSON includes `--keepalive-interval` and `--no-keepalive`
- [ ] No new `CdpClient::connect` sites have appeared outside the exempt locations

---

## Phase 6 Dependency Graph

```
T014 ───────────┬──▶ T015 ──┬──▶ T022 ──▶ T023 ──┬──▶ T024 ──┐
                │            │                   ├──▶ T025 ──┤
T016 ──▶ T017   │            │                   ├──▶ T026 ──┤
                │            │                   ├──▶ T027 ──┤
T018 ───────────┤            │                   └──▶ T028 ──┤
                │            │                               │
T019 ───────────┼──▶ T020 ──┘                                │
                │                                            │
T021 ───────────┘                                            │
                                                             │
T029 ──▶ T030 ──▶ T031 ──────────────────────────────────────┤
T029 ──▶ T032 ───────────────────────────────────────────────┤
                                                             │
                                       T033 ──▶ T034 ────────┤
                                       T035 ─────────────────┤
                                                             ▼
                                                           T036 ──▶ T037
```

**Parallel tracks:**
- T014, T016, T018, T019, T021, T029, T035 can start immediately (no #185 deps)
- T024–T028 are independent once T023 is done; they can land in parallel commits

**Critical path:** T014 → T015 → T022 → T023 → T028 → T037
