# Design: Session and Connection Management

**Issue**: #6
**Date**: 2026-02-11
**Status**: Draft
**Author**: Claude (spec-driven)

---

## Overview

This feature adds persistent connection state across CLI invocations, CDP session management per tab, and a standardized connection resolution chain. It introduces a new `session` module for session file management and a `connection` module that provides the reusable "resolve → health-check → connect → target tab → create session" pipeline that all future commands will use.

The design builds on the existing architecture: `execute_connect` in `main.rs` currently discovers/launches Chrome and prints connection info. This feature extends that flow to also persist the connection info to a session file, and adds `--status`/`--disconnect` flags. For other commands, a new `resolve_connection` function encapsulates the resolution chain (explicit flags → session file → auto-discovery → error) and a `resolve_target` function handles tab targeting.

Key architectural decisions:
1. **New `session` module** at `src/session.rs` for session file I/O (read/write/delete/status)
2. **New `connection` module** at `src/connection.rs` for the reusable connection resolution chain and tab targeting
3. **Session file location** at `~/.chrome-cli/session.json` on Unix, `%USERPROFILE%\.chrome-cli\session.json` on Windows — using `std::env` for home directory (no new dependency)
4. **Lazy domain enabling** tracked by a `DomainSet` that wraps `CdpSession`
5. **Health check via existing `query_version`** — fast HTTP GET to `/json/version`

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                          CLI Layer                                 │
│  ┌──────────────┐    ┌─────────────────────────────────────────┐  │
│  │  Cli/Command  │───▶│  ConnectArgs (+status, +disconnect)     │  │
│  │               │    │  GlobalOpts (--tab already present)     │  │
│  └──────────────┘    └─────────────────────────────────────────┘  │
└───────────────────────────────┬────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────┐
│                       Command Layer                                │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  execute_connect()  — connect/launch/status/disconnect       │  │
│  │  resolve_connection() — flags→session→discover→error         │  │
│  │  resolve_target()   — tab ID / index / default → target_id  │  │
│  └──────────────────────────────────────────────────────────────┘  │
└──────┬──────────────────┬──────────────────┬──────────────────────┘
       │                  │                  │
       ▼                  ▼                  ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────────────────────┐
│ Session Layer │  │ Chrome Layer  │  │        CDP Layer              │
│ session.rs    │  │ discovery.rs  │  │  CdpClient::connect()        │
│ - read()      │  │ launcher.rs   │  │  CdpClient::create_session() │
│ - write()     │  │ platform.rs   │  │  CdpSession::send_command()  │
│ - delete()    │  └──────────────┘  └──────────────────────────────┘
│ - status()    │
└──────────────┘
```

### Data Flow: Connect Command

```
1. User runs: chrome-cli connect [--launch|--status|--disconnect|...]
2. CLI parses ConnectArgs (including new --status, --disconnect flags)
3. Command layer dispatches to appropriate handler:
   a. --status → read session file, health-check, output status
   b. --disconnect → read session file, kill PID if present, delete file
   c. Default → discover/launch Chrome, write session file, output info
4. Session file written to ~/.chrome-cli/session.json
```

### Data Flow: Future Commands (e.g., tabs list)

```
1. User runs: chrome-cli tabs list [--tab ID]
2. CLI parses command + GlobalOpts
3. resolve_connection(global_opts):
   a. If --ws-url → use directly
   b. If --port (non-default) → discover on that port
   c. Read session file → extract ws_url, health-check
   d. Auto-discover (port 9222)
   e. Error with suggestion
4. resolve_target(host, port, tab_option):
   a. If --tab <id> → try as target ID, then as numeric index
   b. If no --tab → first "page" type target
5. CdpClient::connect(ws_url)
6. CdpClient::create_session(target_id)
7. Enable required CDP domains (lazy)
8. Execute command logic
9. Session cleanup (drop CdpSession)
```

---

## File Structure

New and modified files:

```
src/
├── main.rs                    # Modify: update connect handler, add session write
├── session.rs                 # Create: session file management
├── connection.rs              # Create: connection resolution + tab targeting
├── cli/
│   └── mod.rs                 # Modify: add --status, --disconnect to ConnectArgs
├── chrome/
│   └── error.rs               # Modify: add SessionError variant mapping
└── error.rs                   # Modify: add session-related error constructors
```

---

## API / Interface Changes

### CLI Changes: ConnectArgs

Add `--status` and `--disconnect` flags to the `connect` subcommand. These are mutually exclusive with `--launch` and each other:

```rust
// src/cli/mod.rs — updated ConnectArgs
#[derive(Args)]
pub struct ConnectArgs {
    /// Launch a new Chrome instance
    #[arg(long)]
    pub launch: bool,

    /// Show current connection status
    #[arg(long, conflicts_with_all = ["launch", "disconnect"])]
    pub status: bool,

    /// Disconnect and remove session file
    #[arg(long, conflicts_with_all = ["launch", "status"])]
    pub disconnect: bool,

    // ... existing fields unchanged
    #[arg(long, requires = "launch")]
    pub headless: bool,
    #[arg(long, requires = "launch", default_value = "stable")]
    pub channel: ChromeChannel,
    #[arg(long, requires = "launch")]
    pub chrome_path: Option<PathBuf>,
    #[arg(long, requires = "launch")]
    pub chrome_arg: Vec<String>,
}
```

### Output Schemas

**connect --status (stdout):**
```json
{
  "ws_url": "ws://127.0.0.1:9222/devtools/browser/abc123",
  "port": 9222,
  "pid": 12345,
  "timestamp": "2026-02-11T12:00:00Z",
  "reachable": true
}
```

**connect --status with no session (stderr):**
```json
{
  "error": "No active session. Run 'chrome-cli connect' to establish a connection.",
  "code": 2
}
```

**connect --disconnect (stdout):**
```json
{
  "disconnected": true,
  "killed_pid": 12345
}
```

**connect (default, stdout) — unchanged format, now also writes session file:**
```json
{
  "ws_url": "ws://127.0.0.1:9222/devtools/browser/abc123",
  "port": 9222,
  "pid": 12345
}
```

---

## Module Design

### `session` — Session File Management

**Purpose**: Read, write, delete, and check the session file at a well-known location.

**Location**: `src/session.rs`

```rust
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Session file content persisted between CLI invocations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub ws_url: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub timestamp: String,
}

/// Returns the path to the session file: ~/.chrome-cli/session.json
pub fn session_file_path() -> Result<PathBuf, SessionError>;

/// Write session data to the session file. Creates ~/.chrome-cli/ if needed.
/// Sets file permissions to 0o600 on Unix.
pub fn write_session(data: &SessionData) -> Result<(), SessionError>;

/// Read session data from the session file.
/// Returns None if the file does not exist.
pub fn read_session() -> Result<Option<SessionData>, SessionError>;

/// Delete the session file. Returns Ok(()) even if the file doesn't exist.
pub fn delete_session() -> Result<(), SessionError>;

/// Session-specific errors.
#[derive(Debug)]
pub enum SessionError {
    /// Could not determine home directory.
    NoHomeDir,
    /// I/O error reading/writing session file.
    Io(std::io::Error),
    /// Session file contains invalid JSON.
    InvalidFormat(String),
}
```

**Session file path resolution:**
- Unix: `$HOME/.chrome-cli/session.json`
- Windows: `%USERPROFILE%\.chrome-cli\session.json`
- Uses `std::env::var("HOME")` on Unix, `std::env::var("USERPROFILE")` on Windows
- No new crate dependency needed

**File permissions (Unix):**
- Session file created with mode `0o600` (owner read/write only) via `std::fs::set_permissions`
- Directory created with mode `0o700`

### `connection` — Connection Resolution and Tab Targeting

**Purpose**: Provide the reusable resolution chain for all commands, plus tab targeting logic.

**Location**: `src/connection.rs`

```rust
use crate::chrome::{TargetInfo, discover_chrome, query_targets, query_version};
use crate::cli::GlobalOpts;
use crate::error::AppError;
use crate::session::{self, SessionData};

/// Resolved connection info ready for use by a command.
pub struct ResolvedConnection {
    pub ws_url: String,
    pub host: String,
    pub port: u16,
}

/// Resolve a Chrome connection using the priority chain:
/// 1. Explicit --ws-url flag
/// 2. Explicit --port flag (non-default)
/// 3. Session file
/// 4. Auto-discover (port 9222)
/// 5. Error with suggestion
pub async fn resolve_connection(global: &GlobalOpts) -> Result<ResolvedConnection, AppError>;

/// Health-check a connection by querying /json/version.
/// Returns Ok(()) if Chrome responds, Err with stale-session message if not.
pub async fn health_check(host: &str, port: u16) -> Result<(), AppError>;

/// Resolve the target tab from the --tab option.
/// - None → first "page" type target
/// - Some(id) → try as target ID match, then as numeric index
pub async fn resolve_target(
    host: &str,
    port: u16,
    tab: Option<&str>,
) -> Result<TargetInfo, AppError>;
```

**Resolution chain details:**

1. **`--ws-url` provided**: Extract port from URL, return directly. No health check needed (command will fail naturally if unreachable).
2. **`--port` provided and non-default (not 9222)**: Discover on that specific port via `query_version`. This indicates explicit user intent to target a specific port, skipping session file.
3. **Session file exists**: Read session data, run `health_check` against stored host:port. If reachable, use stored `ws_url`. If not reachable, return stale session error.
4. **Auto-discover**: Call `discover_chrome("127.0.0.1", 9222)` which tries DevToolsActivePort then port 9222.
5. **Error**: Return `AppError` with message: `"No Chrome instance found. Run 'chrome-cli connect' or 'chrome-cli connect --launch' to establish a connection."`

**Tab resolution details:**

The `resolve_target` function calls `query_targets(host, port)` to get the target list, then:

1. If `tab` is `None`: Find the first target with `target_type == "page"`. If none found, return error.
2. If `tab` is `Some(value)`:
   a. Try to parse as `usize` (numeric index). If valid and in range, use that target.
   b. Otherwise, search for a target with matching `id`. If not found, return error with suggestion to run `chrome-cli tabs list`.

---

## Connect Command Updated Flow

### Default connect (no flags or with --launch)

```rust
async fn execute_connect(global: &GlobalOpts, args: &ConnectArgs) -> Result<(), AppError> {
    // Handle --status
    if args.status {
        return execute_status(global).await;
    }
    // Handle --disconnect
    if args.disconnect {
        return execute_disconnect();
    }

    // Existing discovery/launch logic (unchanged)...
    // After successful connect, also write session file:
    let session_data = SessionData {
        ws_url: info.ws_url.clone(),
        port: info.port,
        pid: info.pid,
        timestamp: now_iso8601(),
    };
    session::write_session(&session_data)?;

    println!("{}", serde_json::to_string(&info).unwrap());
    Ok(())
}
```

### connect --status

```rust
async fn execute_status(global: &GlobalOpts) -> Result<(), AppError> {
    let session = session::read_session()?
        .ok_or_else(|| AppError {
            message: "No active session. Run 'chrome-cli connect' to establish a connection.".into(),
            code: ExitCode::ConnectionError,
        })?;

    let reachable = health_check(&global.host_or("127.0.0.1"), session.port).await.is_ok();

    let status = StatusInfo {
        ws_url: session.ws_url,
        port: session.port,
        pid: session.pid,
        timestamp: session.timestamp,
        reachable,
    };
    println!("{}", serde_json::to_string(&status).unwrap());
    Ok(())
}
```

### connect --disconnect

```rust
fn execute_disconnect() -> Result<(), AppError> {
    let session = session::read_session()?;
    let mut killed_pid = None;

    if let Some(session) = &session {
        if let Some(pid) = session.pid {
            // Send SIGTERM on Unix, TerminateProcess on Windows
            kill_process(pid);
            killed_pid = Some(pid);
        }
    }

    session::delete_session()?;

    let output = DisconnectInfo {
        disconnected: true,
        killed_pid,
    };
    println!("{}", serde_json::to_string(&output).unwrap());
    Ok(())
}
```

**Process termination:**
- Unix: `libc::kill(pid, libc::SIGTERM)` via `std::process::Command` or direct syscall
- Windows: `TerminateProcess` via `windows-sys` or `std::process::Command("taskkill")`
- Silently ignore errors (process may already be dead)

Since we just need a simple "kill by PID" operation, we'll use `std::process::Command` to invoke `kill` on Unix and `taskkill` on Windows. This avoids adding platform-specific crate dependencies.

---

## Timestamp Formatting

For the `timestamp` field in the session file, we need ISO 8601 formatting. To avoid adding a `chrono` or `time` dependency, we'll use `std::time::SystemTime` and format manually:

```rust
fn now_iso8601() -> String {
    // Use SystemTime::now() and format as simplified ISO 8601
    // e.g., "2026-02-11T12:00:00Z"
    // Implementation uses UNIX_EPOCH arithmetic
}
```

This keeps the dependency footprint at zero for this feature.

---

## Error Handling

### New Error Conversions

`SessionError` converts to `AppError`:

| SessionError | ExitCode | Message |
|-------------|----------|---------|
| `NoHomeDir` | GeneralError (1) | "Could not determine home directory" |
| `Io(e)` | GeneralError (1) | "Session file error: {e}" |
| `InvalidFormat(e)` | GeneralError (1) | "Invalid session file: {e}" |

### New AppError Constructors

```rust
impl AppError {
    pub fn stale_session() -> Self {
        Self {
            message: "Session is stale: Chrome is not reachable at the stored address. \
                      Run 'chrome-cli connect' to establish a new connection.".into(),
            code: ExitCode::ConnectionError,
        }
    }

    pub fn no_session() -> Self {
        Self {
            message: "No active session. Run 'chrome-cli connect' or \
                      'chrome-cli connect --launch' to establish a connection.".into(),
            code: ExitCode::ConnectionError,
        }
    }

    pub fn target_not_found(tab: &str) -> Self {
        Self {
            message: format!(
                "Tab '{tab}' not found. Run 'chrome-cli tabs list' to see available tabs."
            ),
            code: ExitCode::TargetError,
        }
    }

    pub fn no_page_targets() -> Self {
        Self {
            message: "No page targets found in Chrome. Open a tab first.".into(),
            code: ExitCode::TargetError,
        }
    }
}
```

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: XDG dirs crate** | Use `dirs` or `directories` crate for session path | Standards-compliant XDG paths on Linux | New dependency, over-engineering for a single file | Rejected — `~/.chrome-cli/` is fine, matches `kubectl`, `docker` |
| **B: /tmp session file** | Store at `/tmp/chrome-cli-session.json` | No dir creation needed, auto-cleaned on reboot | Not cross-platform, not user-scoped, stale across reboots | Rejected — home dir is more reliable |
| **C: chrono for timestamps** | Use `chrono` crate for ISO 8601 | Full datetime support, DST handling | Heavy dependency for one format string | Rejected — manual formatting is sufficient |
| **D: File locking** | flock/LockFile for concurrent access | Race-condition-free for concurrent invocations | Complexity, rare use case for CLI tool | Rejected — defer to future issue if needed |
| **E: Separate session module crate** | Extract session to workspace crate | Clean separation | Over-engineering for current scope | Rejected — YAGNI |

---

## Security Considerations

- [x] **File permissions**: Session file created with mode `0o600` (owner-only), directory with `0o700`
- [x] **No secrets**: Session file contains only connection metadata (URL, port, PID, timestamp)
- [x] **Localhost only**: Session file stores localhost WebSocket URLs; `warn_if_remote_host` already warns for non-localhost
- [x] **Process kill safety**: Only kills PID stored by chrome-cli (from `--launch`), not arbitrary processes. PID reuse risk is minimal for CLI tool usage patterns.

---

## Performance Considerations

- [x] **Session file read**: Single small JSON file read, < 1ms
- [x] **Health check**: HTTP GET to localhost `/json/version`, < 100ms for local connections. Uses existing `query_version` which already has a 2-second TCP connect timeout.
- [x] **No startup penalty**: Session file only read when needed (not for `connect --status/--disconnect`)
- [x] **No new dependencies**: Zero binary size impact from new crates

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| `session` | Unit | Write/read/delete round-trip, invalid JSON handling, missing dir, permissions |
| `connection` | Unit + Integration | Resolution chain priority, health check success/failure, stale detection |
| `connection::resolve_target` | Unit | Target by ID, by index, default page, invalid ID, empty target list |
| `connect --status` | Integration | With session, without session, stale session |
| `connect --disconnect` | Integration | With PID, without PID, no session |
| Feature | BDD (cucumber) | End-to-end scenarios from requirements |

Session module tests will use temp directories to avoid touching the real `~/.chrome-cli/` directory.

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| PID reuse after Chrome crash | Very Low | Low | PID is advisory; kill failure is silently ignored |
| Session file corruption (partial write) | Very Low | Low | Write to temp file then rename (atomic on most filesystems) |
| Home dir not writable | Low | Medium | Return clear error suggesting manual `--ws-url` flag |
| Health check false positive (port reused by another process) | Very Low | Low | Health check validates `/json/version` response format, not just TCP connectivity |

---

## Open Questions

- [x] Should `resolve_connection` health-check for `--ws-url` connections? — No, direct URL implies user knows what they're doing. Command will fail naturally if unreachable.
- [x] Should session file path be configurable via env var? — Deferred, nice-to-have for future.

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] N/A — No database/storage changes
- [x] State management approach is clear (session file as persistent state)
- [x] N/A — No UI components (CLI tool)
- [x] Security considerations addressed (file permissions, no secrets)
- [x] Performance impact analyzed (< 100ms health check, < 1ms file read)
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
