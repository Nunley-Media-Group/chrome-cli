# Tasks: Session and Connection Management

**Issue**: #6
**Date**: 2026-02-11
**Status**: Planning
**Author**: Claude (spec-driven)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Core Implementation | 4 | [ ] |
| CLI & Command Wiring | 4 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **13** | |

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
- [ ] `session_file_path() -> Result<PathBuf, SessionError>` returns `~/.chrome-cli/session.json` (cross-platform)
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
- [ ] `chrome-cli connect --help` shows `--status` and `--disconnect` flags
- [ ] `chrome-cli connect --status --launch` produces a clap conflict error
- [ ] `chrome-cli connect --status --disconnect` produces a clap conflict error
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
  - Creates `~/.chrome-cli/` directory if it doesn't exist (mode `0o700` on Unix)
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
  5. Return error with suggestion to run `chrome-cli connect`
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
- [ ] `AppError::stale_session()` → ExitCode::ConnectionError, message suggests `chrome-cli connect`
- [ ] `AppError::no_session()` → ExitCode::ConnectionError, message suggests `chrome-cli connect` or `--launch`
- [ ] `AppError::target_not_found(tab: &str)` → ExitCode::TargetError, message suggests `tabs list`
- [ ] `AppError::no_page_targets()` → ExitCode::TargetError, message suggests opening a tab
- [ ] Unit tests for each constructor: verify message content and exit code
- [ ] `cargo clippy` passes

**Notes**: Follow the existing pattern of `AppError::not_implemented()`. Error messages should be actionable per product.md brand voice guidelines.

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
  - `cargo run -- connect` writes session file to `~/.chrome-cli/session.json`
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
- [ ] Session tests (use temp directory, not real `~/.chrome-cli/`):
  - Write/read round-trip preserves all fields
  - Read nonexistent file returns `None`
  - Read invalid JSON returns `InvalidFormat` error
  - Delete nonexistent file returns `Ok(())`
  - `now_iso8601()` produces valid ISO 8601 format
  - `session_file_path()` returns a path ending in `.chrome-cli/session.json`
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
