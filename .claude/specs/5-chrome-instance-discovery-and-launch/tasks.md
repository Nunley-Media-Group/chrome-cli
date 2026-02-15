# Tasks: Chrome Instance Discovery and Launch

**Issue**: #5
**Date**: 2026-02-10
**Status**: Planning
**Author**: Claude (spec-driven)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Core Implementation | 4 | [ ] |
| CLI & Command Wiring | 2 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **11** | |

---

## Phase 1: Setup

### T001: Create chrome module with error types

**File(s)**: `src/chrome/mod.rs`, `src/chrome/error.rs`, `src/lib.rs`
**Type**: Create (mod.rs, error.rs), Modify (lib.rs)
**Depends**: None
**Acceptance**:
- [ ] `src/chrome/mod.rs` exists and re-exports public types
- [ ] `ChromeError` enum defined with variants: `NotFound`, `LaunchFailed`, `StartupTimeout`, `HttpError`, `ParseError`, `NoActivePort`, `NotRunning`, `Io`
- [ ] `ChromeError` implements `Display`, `Error`, and `From<std::io::Error>`
- [ ] Conversion from `ChromeError` to `AppError` with correct `ExitCode` mappings
- [ ] `src/lib.rs` exports `pub mod chrome`
- [ ] `cargo check` passes

**Notes**: Follow the existing pattern in `src/cdp/error.rs` and `src/error.rs`. The `ChromeError → AppError` conversion uses `From` impl.

### T002: Create platform-specific Chrome executable discovery

**File(s)**: `src/chrome/platform.rs`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] `Channel` enum defined (Stable, Canary, Beta, Dev)
- [ ] `find_chrome_executable(channel: Channel) -> Result<PathBuf, ChromeError>` implemented
- [ ] macOS: checks `/Applications/Google Chrome.app/...` and channel variants
- [ ] Linux: searches PATH for `google-chrome`, `google-chrome-stable`, `chromium-browser`, `chromium`
- [ ] Windows: checks `Program Files` and `Program Files (x86)` standard paths
- [ ] `default_user_data_dir() -> Option<PathBuf>` returns platform-specific Chrome user data dir
- [ ] Platform-specific code isolated via `#[cfg(target_os = "...")]`
- [ ] Returns `ChromeError::NotFound` with actionable message when Chrome not found
- [ ] Unit tests for path candidate generation (at least on current platform)
- [ ] `cargo check` passes on current platform
- [ ] `cargo clippy` passes

**Notes**: Use `std::env::var("PATH")` and `which`-style lookup for Linux. On macOS, check existence with `std::path::Path::exists()`. `CHROME_PATH` env var override should be checked first.

---

## Phase 2: Core Implementation

### T003: Implement minimal async HTTP GET client

**File(s)**: `src/chrome/discovery.rs` (private function within discovery module)
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] `async fn http_get(host: &str, port: u16, path: &str) -> Result<String, ChromeError>` implemented
- [ ] Uses `tokio::net::TcpStream` for raw TCP connection
- [ ] Sends well-formed HTTP/1.1 GET request with `Host` header and `Connection: close`
- [ ] Reads full response body (handles chunked transfer or Content-Length)
- [ ] Parses HTTP status line and returns error for non-200 responses
- [ ] Respects a connection timeout (2 seconds)
- [ ] Unit test with mock verification of request format
- [ ] `cargo check` passes

**Notes**: Keep it minimal — this only needs to work for Chrome's `/json/*` endpoints on localhost. Chrome returns simple JSON responses with `Content-Length` headers. No need to handle redirects, chunked encoding, or TLS.

### T004: Implement Chrome instance discovery

**File(s)**: `src/chrome/discovery.rs`
**Type**: Create
**Depends**: T001, T002, T003
**Acceptance**:
- [ ] `BrowserVersion` struct with serde deserialization for `/json/version` response
- [ ] `TargetInfo` struct with serde deserialization for `/json/list` response
- [ ] `query_version(host, port) -> Result<BrowserVersion, ChromeError>` — queries `/json/version`
- [ ] `query_targets(host, port) -> Result<Vec<TargetInfo>, ChromeError>` — queries `/json/list`
- [ ] `read_devtools_active_port() -> Result<(u16, String), ChromeError>` — reads DevToolsActivePort file from platform default dir
- [ ] `discover_chrome(host, port) -> Result<(String, u16), ChromeError>` — tries DevToolsActivePort, then explicit port, returns (ws_url, port)
- [ ] Handles DevToolsActivePort file format: first line = port, second line = WebSocket path
- [ ] Returns `ChromeError::NotRunning` when no instance found
- [ ] Unit tests for JSON parsing (BrowserVersion, TargetInfo)
- [ ] Unit test for DevToolsActivePort file parsing
- [ ] `cargo clippy` passes

**Notes**: The DevToolsActivePort file format is two lines:
```
9222
/devtools/browser/abc123-...
```

### T005: Implement Chrome process launcher

**File(s)**: `src/chrome/launcher.rs`
**Type**: Create
**Depends**: T001, T002, T003
**Acceptance**:
- [ ] `LaunchConfig` struct defined (executable, port, headless, extra_args, user_data_dir)
- [ ] `ChromeProcess` struct holds `std::process::Child`, port, optional `TempDir`
- [ ] `ChromeProcess::pid()` and `ChromeProcess::port()` accessor methods
- [ ] `ChromeProcess::kill()` terminates the child process
- [ ] `Drop` impl on `ChromeProcess` calls `kill()` and cleans up temp directory
- [ ] `TempDir` struct wraps a `PathBuf` and removes the directory on `Drop`
- [ ] `launch_chrome(config, timeout) -> Result<ChromeProcess, ChromeError>` implemented:
  - Spawns Chrome via `std::process::Command` with `--remote-debugging-port=<port>`
  - Adds `--headless=new` if headless requested
  - Adds `--user-data-dir=<temp>` for temp directory
  - Adds `--no-first-run --no-default-browser-check` to suppress prompts
  - Passes through extra_args
  - Polls `/json/version` every 100ms until ready or timeout
- [ ] `find_available_port() -> Result<u16, ChromeError>` binds to port 0 then releases
- [ ] Returns `ChromeError::LaunchFailed` if Chrome exits immediately
- [ ] Returns `ChromeError::StartupTimeout` if polling exceeds timeout
- [ ] `cargo clippy` passes

**Notes**: Use `std::process::Command::new(executable).args([...]).spawn()`. The temp directory should use `std::env::temp_dir()` with a unique subdirectory name (e.g., `chrome-cli-{random}`). Use `tokio::time::sleep` between poll attempts.

### T006: Wire up chrome module exports

**File(s)**: `src/chrome/mod.rs`
**Type**: Modify
**Depends**: T002, T004, T005
**Acceptance**:
- [ ] `mod discovery`, `mod launcher`, `mod platform`, `mod error` declared
- [ ] Public re-exports: `ChromeError`, `ChromeProcess`, `LaunchConfig`, `BrowserVersion`, `TargetInfo`, `Channel`
- [ ] Public re-exports of key functions: `discover_chrome`, `query_version`, `query_targets`, `launch_chrome`, `find_chrome_executable`, `find_available_port`, `read_devtools_active_port`
- [ ] `cargo check` passes

---

## Phase 3: CLI & Command Wiring

### T007: Add ConnectArgs to CLI definition

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `ConnectArgs` struct defined with clap `Args` derive:
  - `--launch` (bool flag)
  - `--headless` (bool flag, requires "launch")
  - `--channel` (ChromeChannel enum with ValueEnum derive, requires "launch", default "stable")
  - `--chrome-path` (Option<PathBuf>, requires "launch")
  - `--chrome-arg` (Vec<String>, requires "launch")
- [ ] `ChromeChannel` enum defined (Stable, Canary, Beta, Dev) with ValueEnum derive
- [ ] `Command::Connect` variant changed from unit to `Connect(ConnectArgs)`
- [ ] `chrome-cli connect --help` shows all new flags
- [ ] Existing BDD tests still pass (`cargo test --test bdd`)
- [ ] `cargo clippy` passes

**Notes**: The `requires = "launch"` ensures headless/channel/chrome-path/chrome-arg are only valid when `--launch` is present. Global options `--port`, `--host`, `--ws-url`, `--timeout` are unchanged.

### T008: Implement connect command handler

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T004, T005, T006, T007
**Acceptance**:
- [ ] `Command::Connect(args)` match arm calls connect execution logic
- [ ] Connection strategy implemented:
  1. If `--ws-url` provided → validate and output connection info
  2. If `--launch` provided → find executable, launch, poll, output connection info
  3. Default → try `discover_chrome(host, port)`, fallback to auto-launch
- [ ] `ConnectionInfo` struct with serde `Serialize` (ws_url, port, pid)
- [ ] Outputs `ConnectionInfo` as JSON to stdout
- [ ] Error cases produce `AppError` with correct exit codes:
  - Chrome not found → exit 1 with actionable error
  - Launch failed → exit 2
  - Timeout → exit 4
  - Connection failed → exit 2
- [ ] `ChromeChannel` converts to `chrome::Channel` for platform module
- [ ] Global `--timeout` used for Chrome startup wait (default 30s)
- [ ] `cargo clippy` passes
- [ ] Manual smoke test: `cargo run -- connect --help` works

**Notes**: Keep the handler logic in `main.rs` for now (consistent with existing pattern). Extract to `src/cli/commands/connect.rs` later if it grows too large.

---

## Phase 4: Integration

### T009: Integration wiring and end-to-end verification

**File(s)**: `src/main.rs`, `src/lib.rs`
**Type**: Modify
**Depends**: T008
**Acceptance**:
- [ ] `chrome` module accessible from integration tests via `lib.rs`
- [ ] `cargo build` succeeds
- [ ] `cargo clippy -- -D warnings` passes (all clippy lints clean)
- [ ] `cargo fmt --check` passes
- [ ] Existing tests pass: `cargo test`
- [ ] Manual end-to-end test (if Chrome available):
  - `cargo run -- connect` discovers or launches Chrome
  - Output is valid JSON with `ws_url`, `port`, and `pid` fields
  - Process exits cleanly

---

## Phase 5: Testing

### T010: Create BDD feature file for Chrome discovery and launch

**File(s)**: `tests/features/chrome-discovery-launch.feature`
**Type**: Create
**Depends**: T009
**Acceptance**:
- [ ] Feature file contains scenarios for all 15 acceptance criteria from requirements
- [ ] Uses Given/When/Then format
- [ ] Includes Background section for common setup
- [ ] Valid Gherkin syntax
- [ ] Covers happy paths, error paths, and edge cases

**Notes**: Some scenarios (launch, headless, channel) require Chrome to be installed. Tag these with `@requires-chrome` for conditional execution.

### T011: Implement unit tests and integration test scaffolding

**File(s)**: `src/chrome/discovery.rs` (inline tests), `src/chrome/launcher.rs` (inline tests), `src/chrome/platform.rs` (inline tests), `tests/bdd.rs` (step definitions)
**Type**: Create / Modify
**Depends**: T010
**Acceptance**:
- [ ] Unit tests in `chrome::discovery`:
  - Parse `/json/version` response JSON
  - Parse `/json/list` response JSON
  - Parse DevToolsActivePort file content
  - HTTP error handling
- [ ] Unit tests in `chrome::platform`:
  - Chrome candidate paths for current platform
  - Default user data dir for current platform
- [ ] Unit tests in `chrome::launcher`:
  - `find_available_port()` returns a valid port
  - `TempDir` cleanup on drop
- [ ] BDD step definitions added to `tests/bdd.rs` for connect-related scenarios
  - Steps for "chrome-cli connect" command invocation
  - Steps for JSON output validation
  - Steps for error message validation
- [ ] `cargo test --lib` passes (unit tests)
- [ ] `cargo test --test bdd` passes (BDD tests, Chrome-dependent ones may be skipped)

**Notes**: Chrome-dependent integration tests should check for `CHROME_AVAILABLE` env var or `@requires-chrome` tag. Unit tests that parse JSON or file content should work everywhere.

---

## Dependency Graph

```
T001 ──┬──▶ T002 ──┬──▶ T004 ──┐
       │           │           │
       ├──▶ T003 ──┤──▶ T005 ──┤──▶ T006 ──▶ T008 ──▶ T009 ──▶ T010 ──▶ T011
       │           │           │         ▲
       │           │           │         │
T007 ──┼───────────┼───────────┼─────────┘
       │           │           │
       └───────────┴───────────┘
```

**Parallel tracks:**
- T001 → T003 (HTTP client) can proceed in parallel with T001 → T002 (platform)
- T007 (CLI args) has no code dependency on the chrome module and can be done in parallel with T001-T006

**Critical path:** T001 → T003 → T005 → T006 → T008 → T009 → T010 → T011

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included (T010, T011)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
