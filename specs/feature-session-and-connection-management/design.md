# Design: Session and Connection Management

**Issues**: #6, #185
**Date**: 2026-04-18
**Status**: Draft
**Author**: Claude (spec-driven)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #6 | 2026-02-11 | Initial design — session file, resolution chain, tab targeting |
| #185 | 2026-04-18 | Adds auto-reconnect pipeline, WebSocket keep-alive, structured loss-kind errors, and reconnect telemetry in session file |

---

## Overview

This feature adds persistent connection state across CLI invocations, CDP session management per tab, and a standardized connection resolution chain. It introduces a new `session` module for session file management and a `connection` module that provides the reusable "resolve → health-check → connect → target tab → create session" pipeline that all future commands will use.

The design builds on the existing architecture: `execute_connect` in `main.rs` currently discovers/launches Chrome and prints connection info. This feature extends that flow to also persist the connection info to a session file, and adds `--status`/`--disconnect` flags. For other commands, a new `resolve_connection` function encapsulates the resolution chain (explicit flags → session file → auto-discovery → error) and a `resolve_target` function handles tab targeting.

Key architectural decisions:
1. **New `session` module** at `src/session.rs` for session file I/O (read/write/delete/status)
2. **New `connection` module** at `src/connection.rs` for the reusable connection resolution chain and tab targeting
3. **Session file location** at `~/.agentchrome/session.json` on Unix, `%USERPROFILE%\.agentchrome\session.json` on Windows — using `std::env` for home directory (no new dependency)
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
1. User runs: agentchrome connect [--launch|--status|--disconnect|...]
2. CLI parses ConnectArgs (including new --status, --disconnect flags)
3. Command layer dispatches to appropriate handler:
   a. --status → read session file, health-check, output status
   b. --disconnect → read session file, kill PID if present, delete file
   c. Default → discover/launch Chrome, write session file, output info
4. Session file written to ~/.agentchrome/session.json
```

### Data Flow: Future Commands (e.g., tabs list)

```
1. User runs: agentchrome tabs list [--tab ID]
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
  "error": "No active session. Run 'agentchrome connect' to establish a connection.",
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

/// Returns the path to the session file: ~/.agentchrome/session.json
pub fn session_file_path() -> Result<PathBuf, SessionError>;

/// Write session data to the session file. Creates ~/.agentchrome/ if needed.
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
- Unix: `$HOME/.agentchrome/session.json`
- Windows: `%USERPROFILE%\.agentchrome\session.json`
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
5. **Error**: Return `AppError` with message: `"No Chrome instance found. Run 'agentchrome connect' or 'agentchrome connect --launch' to establish a connection."`

**Tab resolution details:**

The `resolve_target` function calls `query_targets(host, port)` to get the target list, then:

1. If `tab` is `None`: Find the first target with `target_type == "page"`. If none found, return error.
2. If `tab` is `Some(value)`:
   a. Try to parse as `usize` (numeric index). If valid and in range, use that target.
   b. Otherwise, search for a target with matching `id`. If not found, return error with suggestion to run `agentchrome tabs list`.

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
            message: "No active session. Run 'agentchrome connect' to establish a connection.".into(),
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
                      Run 'agentchrome connect' to establish a new connection.".into(),
            code: ExitCode::ConnectionError,
        }
    }

    pub fn no_session() -> Self {
        Self {
            message: "No active session. Run 'agentchrome connect' or \
                      'agentchrome connect --launch' to establish a connection.".into(),
            code: ExitCode::ConnectionError,
        }
    }

    pub fn target_not_found(tab: &str) -> Self {
        Self {
            message: format!(
                "Tab '{tab}' not found. Run 'agentchrome tabs list' to see available tabs."
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

## Database / Storage Changes

### Schema Changes

| Table / Collection | Column / Field | Type | Nullable | Default | Change |
|--------------------|----------------|------|----------|---------|--------|
| [name] | [name] | [type] | Yes/No | [value] | Add/Modify/Remove |

### Migration Plan

```
-- Describe the migration approach
-- Reference tech.md for migration conventions
```

### Data Migration

[If existing data needs transformation, describe the approach]

---

## State Management

Reference `structure.md` and `tech.md` for the project's state management patterns.

### New State Shape

```
// Pseudocode — use project's actual language/framework
FeatureState {
  isLoading: boolean
  items: List<Item>
  error: string | null
  selected: Item | null
}
```

### State Transitions

```
Initial → Loading → Success (with data)
                  → Error (with message)

User action → Optimistic update → Confirm / Rollback
```

---

## UI Components

### New Components

| Component | Location | Purpose |
|-----------|----------|---------|
| [name] | [path per structure.md] | [description] |

### Component Hierarchy

```
FeatureScreen
├── Header
├── Content
│   ├── LoadingState
│   ├── ErrorState
│   ├── EmptyState
│   └── DataView
│       ├── ListItem × N
│       └── DetailView
└── Actions
```

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: XDG dirs crate** | Use `dirs` or `directories` crate for session path | Standards-compliant XDG paths on Linux | New dependency, over-engineering for a single file | Rejected — `~/.agentchrome/` is fine, matches `kubectl`, `docker` |
| **B: /tmp session file** | Store at `/tmp/agentchrome-session.json` | No dir creation needed, auto-cleaned on reboot | Not cross-platform, not user-scoped, stale across reboots | Rejected — home dir is more reliable |
| **C: chrono for timestamps** | Use `chrono` crate for ISO 8601 | Full datetime support, DST handling | Heavy dependency for one format string | Rejected — manual formatting is sufficient |
| **D: File locking** | flock/LockFile for concurrent access | Race-condition-free for concurrent invocations | Complexity, rare use case for CLI tool | Rejected — defer to future issue if needed |
| **E: Separate session module crate** | Extract session to workspace crate | Clean separation | Over-engineering for current scope | Rejected — YAGNI |

---

## Security Considerations

- [x] **File permissions**: Session file created with mode `0o600` (owner-only), directory with `0o700`
- [x] **No secrets**: Session file contains only connection metadata (URL, port, PID, timestamp)
- [x] **Localhost only**: Session file stores localhost WebSocket URLs; `warn_if_remote_host` already warns for non-localhost
- [x] **Process kill safety**: Only kills PID stored by agentchrome (from `--launch`), not arbitrary processes. PID reuse risk is minimal for CLI tool usage patterns.

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

Session module tests will use temp directories to avoid touching the real `~/.agentchrome/` directory.

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

---

# Enhancement Design: Session Reconnection & Keep-Alive (Issue #185)

## Overview

The base feature (#6) implements a one-shot resolution chain: commands probe the session file once, fail with `stale_session` if the stored `ws_url` is dead, and ask the user to reconnect. #185 replaces the hard failure with an in-band recovery path (transparent rediscover + retry) and adds a client-side WebSocket keep-alive so long-running commands don't die to idle timeout.

Two cross-cutting concerns drive the design:

1. **Single-path reconnect** — every Chrome-needing command MUST route through one reconnect-aware function. Today several modules (`js.rs`, `tabs.rs`, `dialog.rs`, `output.rs`, `audit.rs`) call `CdpClient::connect` after `resolve_connection` returns. Any module that bypasses the new pipeline defeats FR18. We introduce `connection::connect_for_command` as the single entry point.
2. **Silent on stdout, observable on stderr** — reconnect is for the JSON-contract CLI; stdout must stay pure. All diagnostics go through the existing `tracing` pathway (stderr-only) gated on `--verbose`.

The existing `TransportTask` in `cdp/transport.rs` already has a WebSocket-level reconnect loop that fires on `Message::Close` / socket error within a single `CdpClient` lifetime. #185 adds two orthogonal layers on top:

- **Layer A (CLI-invocation reconnect)**: runs before `CdpClient::connect`. If the session file's `ws_url` is stale, rediscover Chrome on the stored port, rewrite the session file (preserving `pid`/`port`/`active_tab_id`), and return a fresh `ResolvedConnection`.
- **Layer B (WebSocket keep-alive)**: runs inside `TransportTask`. Sends a `Message::Ping` at the configured interval when no outbound CDP traffic has flowed, and tracks the most recent Pong to detect dead peers.

These two layers are independent and configurable independently.

---

## Architecture Additions

### Component Diagram (Layer A)

```
┌───────────────────────────────────────────────────────────────────┐
│  Command entry (main.rs dispatch)                                  │
│       │                                                            │
│       ▼                                                            │
│  connection::connect_for_command(global, cmd_needs_target)  ◄──── single entry point (FR18)
│       │                                                            │
│       ├─► resolve_connection_with_reconnect(global)                │
│       │      │                                                     │
│       │      ├─ session file present? ──► health_check             │
│       │      │      ├─ ok  ──► use stored ws_url                   │
│       │      │      └─ fail ──► rediscover_on_stored_port          │
│       │      │                    ├─ ok  ──► rewrite session file  │
│       │      │                    │           (preserve pid,       │
│       │      │                    │            bump reconnect_count)│
│       │      │                    └─ fail ──► auto_discover        │
│       │      │                                 ├─ ok  ──► rewrite  │
│       │      │                                 └─ fail ──►         │
│       │      │                                    classify_loss()  │
│       │      │                                    ├─ terminated    │
│       │      │                                    │   (AppError    │
│       │      │                                    │    with        │
│       │      │                                    │    kind=chrome_│
│       │      │                                    │    terminated, │
│       │      │                                    │    recoverable=│
│       │      │                                    │    false)      │
│       │      │                                    └─ transient     │
│       │      │                                        (recoverable=│
│       │      │                                         true)       │
│       │      └─ each probe wrapped in tokio::time::timeout(        │
│       │         probe_timeout_ms)                                  │
│       │                                                            │
│       └─► CdpClient::connect(ws_url, cdp_config_with_keepalive)    │
└───────────────────────────────────────────────────────────────────┘
```

### Component Diagram (Layer B — Keep-Alive inside TransportTask)

```
┌───────────────────────────────────────────────────────────────────┐
│  TransportTask (src/cdp/transport.rs) — existing run loop          │
│                                                                    │
│  tokio::select! {                                                  │
│    ws_msg = self.ws_stream.next()           (branch 1, existing)   │
│    cmd    = self.command_rx.recv()          (branch 2, existing)   │
│    _      = timeout_sleep                   (branch 3, existing)   │
│    _      = keepalive_tick   ◄── NEW        (branch 4, #185)       │
│  }                                                                 │
│                                                                    │
│  On keepalive_tick:                                                │
│    if now - last_outbound >= keepalive_interval:                   │
│      ws_stream.send(Message::Ping(vec![]))                         │
│      record last_ping_at                                           │
│    if last_ping_at && now - last_ping_at > pong_timeout:           │
│      trigger handle_disconnect() (existing reconnect path)         │
│                                                                    │
│  On Message::Pong (new match arm in branch 1):                     │
│    last_pong_at = now; last_ping_at = None                         │
└───────────────────────────────────────────────────────────────────┘
```

---

## Data Flow

### Flow A-1: Stale session, Chrome still running on stored port

```
1. User: agentchrome page snapshot
2. main.rs → connection::connect_for_command(global)
3. read_session() → Some(SessionData{ws_url=OLD, port=9222, pid=12345, ...})
4. health_check(127.0.0.1, 9222) → Err (ws_url rotated, but port up)
5. rediscover_on_stored_port(9222) → Ok(new_ws_url)
6. session::write_session(SessionData {
     ws_url: new_ws_url,
     port: 9222,                 ← preserved
     pid: Some(12345),           ← preserved (FR17)
     active_tab_id: ... ,        ← preserved
     timestamp: now(),
     last_reconnect_at: Some(now()),
     reconnect_count: prev + 1,
   })
7. CdpClient::connect(new_ws_url, cdp_config) → Ok
8. Command proceeds normally. stdout = snapshot JSON. stderr = silent.
```

### Flow A-2: Chrome fully gone

```
1. User: agentchrome page snapshot
2. read_session() → Some(SessionData{port=9222, pid=12345})
3. health_check(9222) → Err
4. rediscover_on_stored_port(9222) → Err (no Chrome on port)
5. auto_discover(DEFAULT_CDP_PORT) → Err
6. classify_loss():
   - if pid is Some: check process liveness via platform call
     - alive → kind="transient", recoverable=true
     - dead  → kind="chrome_terminated", recoverable=false
   - if pid is None: we can't tell definitively → kind="transient" (default to recoverable)
7. AppError with structured JSON body:
   { "error": "Chrome process has terminated. Run 'agentchrome connect --launch' to start a new session.",
     "code": 2,
     "kind": "chrome_terminated",
     "recoverable": false }
8. Session file is NOT deleted (user can inspect it).
9. Exit code 2.
```

### Flow B: Keep-alive during long-running command

```
1. User: agentchrome console follow (runs for minutes)
2. CdpClient::connect(...) spawns TransportTask with keepalive_interval=30s, pong_timeout=10s
3. Every 30s (since last outbound), TransportTask sends Message::Ping
4. Chrome responds with Message::Pong (auto-handled by tokio-tungstenite at protocol level;
   we match the Pong arm to record last_pong_at)
5. If a Pong doesn't arrive within 10s of a Ping, trigger handle_disconnect()
   which runs the existing WebSocket-level reconnect with backoff.
6. If WS-level reconnect succeeds, command continues transparently.
   If it fails, existing ReconnectFailed error path fires.
```

---

## File Structure (Delta)

```
src/
├── connection.rs              # Modify: add connect_for_command, resolve_connection_with_reconnect,
│                              #         rediscover_on_stored_port, classify_loss
├── session.rs                 # Modify: extend SessionData with last_reconnect_at, reconnect_count;
│                              #         add write_preserving helper
├── cdp/
│   ├── client.rs              # Modify: add KeepAliveConfig to CdpConfig
│   └── transport.rs           # Modify: add branch 4 keepalive_tick, Pong match arm,
│                              #         last_outbound/last_ping_at tracking
├── cli/
│   └── mod.rs                 # Modify: add --keepalive-interval, --no-keepalive global flags;
│                              #         add clap metadata per steering/tech.md
├── error.rs                   # Modify: extend AppError with optional `kind` and `recoverable`
│                              #         fields (via custom_json for structured JSON); add
│                              #         chrome_terminated() and transient_connection_loss()
│                              #         constructors
├── config.rs                  # Modify: add keepalive.interval_ms and reconnect.probe_timeout_ms
│                              #         to TOML config
├── js.rs, tabs.rs, dialog.rs, # Modify: replace direct CdpClient::connect calls with
│ audit.rs, output.rs          #         connection::connect_for_command
└── ...

README.md                      # Modify: add "Session resilience" section
tests/
├── features/
│   └── 185-session-reconnect-keepalive.feature  # New BDD scenarios (FR21: mirror ACs)
└── bdd.rs                     # Modify: add steps for stale-session + keep-alive worlds
```

---

## API / Interface Changes

### `connection::connect_for_command` (new)

```rust
pub struct CommandConnection {
    pub client: CdpClient,
    pub resolved: ResolvedConnection,
    pub reconnected: bool,           // true if Layer A rewrote the session file
}

/// Single entry point for commands that need a Chrome connection.
/// Handles Layer A reconnect, builds a CdpConfig with keep-alive settings,
/// and returns a live CdpClient.
///
/// # Errors
///
/// Returns AppError with `kind: "chrome_terminated"` + `recoverable: false`
/// when Chrome is definitively gone, or `kind: "transient"` + `recoverable: true`
/// for other transient failures.
pub async fn connect_for_command(
    global: &GlobalOpts,
    keepalive: &KeepAliveConfig,
    reconnect: &ReconnectPolicy,
) -> Result<CommandConnection, AppError>;
```

### `connection::resolve_connection_with_reconnect` (new, internal)

```rust
async fn resolve_connection_with_reconnect(
    host: &str,
    port: Option<u16>,
    ws_url: Option<&str>,
    policy: &ReconnectPolicy,
) -> Result<ResolvedConnection, AppError>;
```

Implements Flow A-1 / A-2.

### `connection::ReconnectPolicy` (new)

```rust
pub struct ReconnectPolicy {
    pub max_attempts: u32,           // default 3
    pub initial_backoff: Duration,   // default 100ms
    pub max_backoff: Duration,       // default 5s
    pub probe_timeout_ms: u64,       // default 500ms
    pub verbose: bool,               // emit reconnect diagnostics to stderr
}
```

`max_attempts`, `initial_backoff`, `max_backoff` mirror `cdp::transport::ReconnectConfig` so the two layers can share a TOML section.

### `connection::classify_loss` (new, internal)

```rust
/// Classify a final reconnect failure as `chrome_terminated` or `transient`.
/// Uses the stored pid (if present) to check process liveness.
fn classify_loss(
    stored_pid: Option<u32>,
    final_error: &str,
) -> (LossKind, bool);      // (kind, recoverable)

enum LossKind {
    ChromeTerminated,
    Transient,
}
```

Process-liveness check uses platform-specific light probes:

- Unix: `nix::sys::signal::kill(Pid::from_raw(pid), None)` → `Err(ESRCH)` means dead. If `nix` isn't already a dep, fall back to `libc::kill(pid, 0)` in an `unsafe` block, or parse `/proc/{pid}/status`.
- Windows: `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)` + `GetExitCodeProcess` via `windows-sys`, or a `tasklist /FI "PID eq N"` subprocess shelling. Prefer the API route only if `windows-sys` is already a dep; otherwise shell out to `tasklist`.

If we can't determine liveness (unknown pid, permission denied on the check), default to `Transient` / `recoverable=true` — fail conservatively so we never tell the user "unrecoverable" when we aren't sure.

### `cdp::client::CdpConfig` (modified)

```rust
pub struct CdpConfig {
    pub connect_timeout: Duration,
    pub command_timeout: Duration,
    pub channel_capacity: usize,
    pub reconnect: ReconnectConfig,
    pub keepalive: KeepAliveConfig,   // NEW
}
```

### `cdp::transport::KeepAliveConfig` (new)

```rust
#[derive(Debug, Clone, Copy)]
pub struct KeepAliveConfig {
    /// Interval between keep-alive pings. `None` disables keep-alive.
    pub interval: Option<Duration>,
    /// Time to wait for a Pong before declaring the connection dead.
    pub pong_timeout: Duration,
}

impl Default for KeepAliveConfig {
    fn default() -> Self {
        Self {
            interval: Some(Duration::from_secs(30)),
            pong_timeout: Duration::from_secs(10),
        }
    }
}
```

### `session::SessionData` (modified)

```rust
pub struct SessionData {
    pub ws_url: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub active_tab_id: Option<String>,
    pub timestamp: String,

    /// ISO-8601 timestamp of the most recent auto-reconnect, or `None` if never (NEW, #185).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_reconnect_at: Option<String>,

    /// Cumulative auto-reconnects for this session file (NEW, #185).
    #[serde(default)]
    pub reconnect_count: u32,
}
```

Serde `default`/`skip_serializing_if` ensure backwards compatibility with pre-#185 session files on disk.

### New `session::rewrite_preserving` helper (new)

```rust
/// Rewrite the session file, preserving `pid`, `port`, `active_tab_id` from
/// an existing record. Bumps `reconnect_count`, updates `last_reconnect_at`
/// and `timestamp`. Writes atomically via temp-file + rename.
pub fn rewrite_preserving(
    existing: &SessionData,
    new_ws_url: String,
) -> Result<SessionData, SessionError>;
```

This helper enforces AC23 / FR17 at the API layer so callers can't accidentally drop `pid`.

### `AppError` structured JSON (modified)

The existing `AppError` has a `custom_json: Option<Value>` field. For unrecoverable errors from #185 we populate it so the rendered stderr JSON contains all required fields:

```json
{
  "error": "Chrome process has terminated. Run 'agentchrome connect --launch' to start a new session.",
  "code": 2,
  "kind": "chrome_terminated",
  "recoverable": false
}
```

New constructors:

```rust
impl AppError {
    pub fn chrome_terminated() -> Self {
        Self {
            message: "Chrome process has terminated. \
                      Run 'agentchrome connect --launch' to start a new session.".into(),
            code: ExitCode::ConnectionError,
            custom_json: Some(json!({
                "kind": "chrome_terminated",
                "recoverable": false,
            })),
        }
    }

    pub fn transient_connection_loss(detail: impl Into<String>) -> Self {
        let detail = detail.into();
        Self {
            message: format!("Chrome connection failed: {detail}. \
                              Run 'agentchrome connect' to rediscover."),
            code: ExitCode::ConnectionError,
            custom_json: Some(json!({
                "kind": "transient",
                "recoverable": true,
            })),
        }
    }
}
```

### CLI flags (new globals)

```rust
// src/cli/mod.rs — extend GlobalOpts
pub struct GlobalOpts {
    // ... existing fields ...

    /// Keep-alive ping interval in ms. Default: 30000. Set to 0 or pass
    /// `--no-keepalive` to disable.
    #[arg(
        long = "keepalive-interval",
        value_name = "MS",
        value_parser = clap::value_parser!(u64),
        env = "AGENTCHROME_KEEPALIVE_INTERVAL",
        global = true,
        conflicts_with = "no_keepalive",
        help = "WebSocket keep-alive interval in milliseconds (default 30000). 0 disables."
    )]
    pub keepalive_interval: Option<u64>,

    /// Disable the WebSocket keep-alive ping.
    #[arg(long = "no-keepalive", global = true, help = "Disable WebSocket keep-alive pings.")]
    pub no_keepalive: bool,
}
```

Per `steering/tech.md` the `connect` subcommand's `after_long_help` string will grow an `EXAMPLES:` entry showing both flags, including at least one `--json` example. The capabilities manifest and man pages pick these up automatically from clap metadata.

### Config TOML additions

```toml
[keepalive]
interval_ms = 30000     # 0 or unset to disable

[reconnect]
max_attempts = 3
initial_backoff_ms = 100
max_backoff_ms = 5000
probe_timeout_ms = 500
```

Precedence: CLI flag > env var > `config.toml` > compiled-in default.

---

## Layer B: Keep-Alive Implementation Notes

### Transport task additions

Add fields to `TransportTask`:

```rust
struct TransportTask {
    // ... existing fields ...

    keepalive: KeepAliveConfig,
    last_outbound: Instant,              // bumped on every ws_stream.send
    last_ping_at: Option<Instant>,       // Some(_) while awaiting a Pong
}
```

Modify the `tokio::select!` in `run()`:

```rust
let keepalive_tick = async {
    match self.keepalive.interval {
        None => std::future::pending::<()>().await,
        Some(interval) => {
            let next_tick = self.last_outbound + interval;
            tokio::time::sleep_until(next_tick.into()).await;
        }
    }
};

let pong_deadline = async {
    match self.last_ping_at {
        None => std::future::pending::<()>().await,
        Some(sent_at) => {
            tokio::time::sleep_until((sent_at + self.keepalive.pong_timeout).into()).await;
        }
    }
};

tokio::select! {
    // ... existing 3 branches ...

    _ = keepalive_tick => {
        let _ = self.ws_stream.send(Message::Ping(Vec::new().into())).await;
        self.last_outbound = Instant::now();
        self.last_ping_at = Some(Instant::now());
    }

    _ = pong_deadline => {
        // Pong didn't arrive — treat as dead connection.
        self.handle_disconnect().await;
        self.last_ping_at = None;
    }
}
```

Add to the existing `ws_msg` branch:

```rust
Some(Ok(Message::Pong(_))) => {
    self.last_ping_at = None;
}
```

Update `handle_send_command` to set `self.last_outbound = Instant::now()` after `ws_stream.send` succeeds.

### Why this does not collide with JSON-RPC (AC31 / FR15)

`tokio-tungstenite` multiplexes `Text`, `Binary`, `Ping`, `Pong`, `Close` on the WebSocket stream as distinct `Message` variants. Our JSON-RPC path uses `Message::Text` exclusively; ping/pong use `Message::Ping`/`Message::Pong`. The `select!` is over the command channel and the WebSocket source, not over the request path — keep-alive sends and JSON-RPC sends both ultimately call `ws_stream.send`, and the underlying Tungstenite sink serializes writes. No JSON-RPC request is stalled by a ping write beyond a single WebSocket frame.

### Chrome side

Chrome's CDP WebSocket server handles standard WebSocket control frames per RFC 6455 — it replies to `Ping` with `Pong` at the protocol layer without surfacing the frame to the CDP session. We do NOT need a CDP-level keep-alive RPC (no `Browser.version` poll, for instance). This keeps overhead minimal: a Ping frame is 2–6 bytes over the wire.

---

## Error Taxonomy

| Scenario | `kind` | `recoverable` | Exit | Remediation in message |
|----------|--------|---------------|------|-----------------------|
| Session fresh, ws_url reachable | n/a (success) | n/a | 0 | n/a |
| Stale ws_url, rediscover succeeds on stored port | n/a (success, Layer A silent) | n/a | 0 | n/a |
| Stale ws_url, auto-discover succeeds | n/a (success) | n/a | 0 | n/a |
| Stored pid alive, port unreachable, all probes fail | `transient` | `true` | 2 | `agentchrome connect` |
| No stored pid, all probes fail | `transient` | `true` | 2 | `agentchrome connect` |
| Stored pid dead (process gone) | `chrome_terminated` | `false` | 2 | `agentchrome connect --launch` |
| Keep-alive pong timeout → WS reconnect exhausted | bubbles as `transient` | `true` | 2 | `agentchrome connect` |

---

## Observability

`connect --status` JSON grows two optional fields (already specified in requirements AC33 / FR19):

```json
{
  "ws_url": "...",
  "port": 9222,
  "pid": 12345,
  "timestamp": "...",
  "reachable": true,
  "last_reconnect_at": "2026-04-18T12:34:56Z",
  "reconnect_count": 2,
  "keepalive": {
    "interval_ms": 30000,
    "enabled": true
  }
}
```

Reconnect diagnostics (attempt number, per-probe latency, last error) go through `tracing::info!` / `tracing::debug!` to stderr. Activated by `--verbose` or `RUST_LOG`. Never touches stdout.

---

## Path Audit (FR18)

Existing `CdpClient::connect` call sites to migrate onto `connection::connect_for_command`:

| Module | Line location | Change |
|--------|---------------|--------|
| `src/output.rs` | 2 sites | Replace `resolve_connection` + `CdpClient::connect` pair with `connect_for_command` |
| `src/tabs.rs` | 4 sites | Same |
| `src/js.rs` | 1 site | Same |
| `src/dialog.rs` | 1 site | Same |
| `src/main.rs` | `connect` command itself | NOT migrated (this creates the session file; it must use the raw path) |

After migration, `CdpClient::connect` is callable only from `connection::connect_for_command` + the `connect` subcommand. Add a module-level doc comment on `CdpClient::connect` noting this invariant.

> **Retrospective-backed note:** Per the "path audit" learning, we don't just fix the primary call site; we enumerate every sibling path and either migrate or document why it's exempt. The `connect` subcommand is exempt because it's what creates the session file in the first place — there's nothing to reconnect *to* yet.

---

## Alternatives Considered (additions)

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **F: Full daemon for persistent WS** | Run a background agentchrome daemon holding the WS open across CLI invocations | Zero cross-invocation reconnect cost; keep-alive spans invocations | Violates "single binary, no daemon" product principle; lifecycle management complexity | Rejected — explicitly out of scope per issue |
| **G: Reuse CdpConfig.reconnect for Layer A** | Drive Layer A from the transport's existing ReconnectConfig | One knob for users | Layer A reconnect triggers *before* CdpClient exists; the transport's loop triggers *after*. Collapsing them couples unrelated lifecycles | Rejected — two policies, shared TOML section for ergonomics |
| **H: CDP-level keep-alive (Browser.version poll)** | Poll `Browser.getVersion` every N seconds instead of WebSocket Ping | Works even if some proxy strips Ping frames | Ping frame is bytes-per-minute; RPC call is kilobytes and pollutes the command ID space; slower | Rejected — Ping is the idiomatic WS keep-alive |
| **I: Auto-launch on chrome_terminated** | If Chrome is gone, spawn a new one and retry | Even more transparent UX | Explicitly out of scope per issue; hides the user's Chrome lifecycle; surprising for CI | Rejected — out of scope |
| **J: Per-probe ability to abort via CancellationToken** | Use `CancellationToken` rather than `tokio::time::timeout` for probes | Caller can cancel mid-probe | `tokio::time::timeout` already does what we need; adding CT would be over-engineering | Rejected — timeout suffices |

---

## Security Considerations (additions)

- **Process liveness check**: On Unix, `kill(pid, 0)` can leak information about process existence across UIDs but only for processes owned by the same user. Since the session file was written by the current user and stores that user's own Chrome PID, this is benign.
- **Session file rewrite**: `rewrite_preserving` uses atomic write (temp-file + rename) so a crashed process can never leave a partial session file. File mode remains `0o600` on the rewritten file.
- **Keep-alive frames carry no payload**: Ping frames are sent with an empty body. No identifying or secret data leaves the client during idle.

---

## Performance Considerations (additions)

- **Reconnect overhead (happy path)**: When `ws_url` is fresh, the only additional cost vs. pre-#185 is a single field read on `SessionData` and a no-op keepalive timer (pending future). Negligible.
- **Per-probe latency**: Bounded by `probe_timeout_ms` (default 500 ms). Worst-case Layer A reconnect is `max_attempts × (probe_timeout + max_backoff)` ≈ 3 × (0.5 + 5) = 16.5 s. This upper-bound should be documented in README.
- **Keep-alive traffic**: At 30 s interval, 2 Ping frames + 2 Pong frames per minute. Each frame is ≤ 6 bytes framing + 0-byte payload. Well under the <1 KB/min success metric.
- **Short-command regression guard**: `tabs list` completes in well under 30 s, so the keep-alive timer never fires for short commands. No regression on P95 latency.

---

## Testing Strategy (additions)

| Layer | Type | Coverage |
|-------|------|----------|
| `connection::resolve_connection_with_reconnect` | Unit | stale + port-reachable → rewrite; stale + port-unreachable, pid alive → transient; stale + pid dead → chrome_terminated; preserves `pid` |
| `connection::classify_loss` | Unit | pid present + alive, pid present + dead, pid absent, platform fallback |
| `session::rewrite_preserving` | Unit | preserves pid/port/active_tab_id, bumps reconnect_count, atomic write |
| `cdp::transport` keep-alive | Unit (mock WS) | Ping fires after idle interval; Pong clears ping timer; Pong timeout triggers reconnect; no Ping during active JSON-RPC traffic |
| `cdp::transport` keep-alive disabled | Unit | No Ping ever sent when `interval: None` or `0` |
| Layer A integration | BDD | Every AC21–AC33 scenario in `tests/features/185-session-reconnect-keepalive.feature` |
| Clap help / capabilities / man | Unit + BDD | AC34/AC35 verified via `assert_cmd` invocations |
| README check | BDD | AC36 — a simple grep over README.md content for the required headings and example |
| Feature Exercise Gate | Manual + scripted | `tests/fixtures/session-reconnect-keepalive.html` (static page); test procedure in `tasks.md` |

> **Retrospective-backed note (environment variants):** Per the learning about headed vs headless divergence, the BDD scenarios for #185 run in both headless and headed configurations where feasible. The smoke test targets the headless configuration that matches the issue's original reproduction scenario (Salesforce email-to-case workflow).

---

## Risks & Mitigations (additions)

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| PID reuse: pid check returns alive for a different process on the same PID | Low | Medium | `rediscover_on_stored_port` runs *first*; classify_loss runs only when no Chrome responds on the stored port. Even if PID is reused by another process, we still report chrome_terminated correctly when CDP is not reachable. |
| Keep-alive Ping frames interpreted as noise by proxies/middleboxes | Low | Medium | CDP connections are strictly localhost per `warn_if_remote_host`; no middlebox should be in the path. Pong timeout triggers reconnect as a safety net. |
| Race between Layer A reconnect and a concurrent `connect` command | Low | Low | Session file writes are atomic (temp + rename). Last writer wins; behavior converges to a usable session. |
| `--keepalive-interval 0` confused with "default" | Low | Low | `0` explicitly disables (same as `--no-keepalive`). Documented in help text; value_parser + clap message highlight it. |
| `tracing` chatter leaking to stdout if a future refactor changes subscriber setup | Low | Medium | Golden test in BDD asserts that stdout after a reconnect is exactly the expected JSON payload, no extra lines. |

---

## Open Questions (additions)

- [ ] Should the keep-alive interval default to Chrome's own idle timeout once documented? For now, 30 s is a safe under-approximation.
- [ ] Should `rewrite_preserving` on a file-not-found degrade to a fresh write (first-time reconnect scenario) or fail? Proposed: fall back to a fresh `write_session` — the resolution chain will handle pid/active_tab preservation on the next read. Will confirm during implementation.

---

## Validation Checklist (additions)

- [x] Single reconnect-aware entry point (`connect_for_command`) enforces FR18 path audit
- [x] Session file schema extended backwards-compatibly (serde defaults)
- [x] Layer A (invocation-level) and Layer B (transport-level) reconnect are decoupled
- [x] Keep-alive uses WebSocket control frames; does not contend with JSON-RPC channel
- [x] Structured error JSON includes `kind` + `recoverable` fields for script consumers
- [x] Clap metadata complete per steering/tech.md (help, long_about, value_parser, conflicts_with)
- [x] Capabilities manifest + man pages inherit new flags via clap
- [x] README section specified (AC36)
- [x] Test strategy covers unit, transport integration, BDD, and smoke; environment variants covered
