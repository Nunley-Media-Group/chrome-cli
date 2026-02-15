# Design: Chrome Instance Discovery and Launch

**Issue**: #5
**Date**: 2026-02-10
**Status**: Draft
**Author**: Claude (spec-driven)

---

## Overview

This feature adds the ability to discover running Chrome instances and launch new ones, implementing the `connect` subcommand. It introduces a new `chrome/` module layer for Chrome process management (discovery, launch, platform-specific paths) and wires it into the existing CLI through a `connect` command handler.

The design follows the existing layer architecture: CLI → Command → Chrome Layer → CDP Layer. The `connect` command orchestrates discovery/launch through the chrome layer, then returns connection info. A minimal async HTTP client is included for querying Chrome's JSON debug endpoints, avoiding heavy HTTP dependencies to keep the binary small.

Key architectural decisions:
1. **New `chrome/` module** for all Chrome process management, separate from CDP protocol concerns
2. **Platform-specific code isolated** in `chrome/platform.rs` using compile-time `cfg` attributes
3. **Minimal HTTP client** using raw `tokio::net::TcpStream` for `/json/version` and `/json/list` (localhost-only, no TLS needed)
4. **No new external dependencies** — uses existing `tokio`, `serde`, `serde_json`

---

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI Layer                                │
│  ┌──────────────┐    ┌──────────────────────────────────────┐   │
│  │  Cli/Command  │───▶│  ConnectArgs (clap derive)           │   │
│  └──────────────┘    └──────────────────────────────────────┘   │
└────────────────────────────────┬────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Command Layer                              │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  connect::execute(global_opts, connect_args)              │   │
│  │  - Orchestrates discovery → launch → connect flow         │   │
│  │  - Returns ConnectionInfo to stdout                       │   │
│  └──────────────────────────────────────────────────────────┘   │
└────────────┬───────────────────────────────┬────────────────────┘
             │                               │
             ▼                               ▼
┌────────────────────────────┐  ┌────────────────────────────────┐
│      Chrome Layer          │  │         CDP Layer               │
│  ┌──────────────────────┐  │  │  ┌──────────────────────────┐  │
│  │  discovery.rs         │  │  │  │  CdpClient::connect()    │  │
│  │  - query_version()    │  │  │  │  (existing, unchanged)   │  │
│  │  - query_targets()    │  │  │  └──────────────────────────┘  │
│  │  - read_devtools_port │  │  └────────────────────────────────┘
│  │  - find_chrome()      │  │
│  ├──────────────────────┤  │
│  │  launcher.rs          │  │
│  │  - launch_chrome()    │  │
│  │  - wait_for_ready()   │  │
│  │  - ChromeProcess      │  │
│  ├──────────────────────┤  │
│  │  platform.rs          │  │
│  │  - chrome_paths()     │  │
│  │  - default_data_dir() │  │
│  │  - find_executable()  │  │
│  └──────────────────────┘  │
└────────────────────────────┘
```

### Data Flow

```
1. User runs: chrome-cli connect [--port|--ws-url|--launch|...]
2. CLI layer parses ConnectArgs from clap
3. Command layer (connect::execute) determines strategy:
   a. If --ws-url provided → skip discovery, connect directly
   b. If --launch provided → launch Chrome, then connect
   c. If --port provided → discover on that port
   d. Default → try discovery (DevToolsActivePort, then port 9222), then launch
4. Chrome layer performs discovery/launch:
   - discovery.rs: HTTP GET /json/version → parse ws_url
   - launcher.rs: spawn Chrome process → poll until ready
   - platform.rs: find Chrome executable path per OS
5. Command layer connects CdpClient to discovered/launched ws_url
6. Output ConnectionInfo as JSON to stdout
7. Exit 0
```

---

## File Structure

New and modified files:

```
src/
├── main.rs                      # Modify: dispatch to connect::execute
├── lib.rs                       # Modify: export chrome module
├── cli/
│   └── mod.rs                   # Modify: add ConnectArgs to Connect variant
├── chrome/
│   ├── mod.rs                   # Create: module exports
│   ├── discovery.rs             # Create: Chrome discovery via HTTP and DevToolsActivePort
│   ├── launcher.rs              # Create: Chrome process launch and lifecycle
│   └── platform.rs              # Create: platform-specific paths and executables
└── error.rs                     # Modify: add chrome-specific error variants or mapping
```

---

## API / Interface Changes

### CLI Changes: Connect Subcommand

The `Connect` variant in `Command` enum gains associated `ConnectArgs`:

```rust
// src/cli/mod.rs — updated Connect variant
#[derive(Subcommand)]
pub enum Command {
    Connect(ConnectArgs),
    // ... other variants unchanged
}

#[derive(Args)]
pub struct ConnectArgs {
    /// Always launch a new Chrome instance
    #[arg(long)]
    pub launch: bool,

    /// Launch Chrome in headless mode
    #[arg(long, requires = "launch")]
    pub headless: bool,

    /// Chrome channel to launch (stable, canary, beta, dev)
    #[arg(long, requires = "launch", default_value = "stable")]
    pub channel: ChromeChannel,

    /// Path to Chrome executable (overrides auto-detection)
    #[arg(long, requires = "launch")]
    pub chrome_path: Option<PathBuf>,

    /// Additional Chrome flags (repeatable)
    #[arg(long = "chrome-arg", requires = "launch")]
    pub chrome_args: Vec<String>,
}

#[derive(Clone, ValueEnum)]
pub enum ChromeChannel {
    Stable,
    Canary,
    Beta,
    Dev,
}
```

Note: `--port`, `--host`, `--ws-url`, and `--timeout` are already global options in `GlobalOpts`.

### Output Schema

**Success (stdout):**
```json
{
  "ws_url": "ws://127.0.0.1:9222/devtools/browser/abc123",
  "port": 9222,
  "pid": 12345
}
```

- `ws_url`: WebSocket debugger URL (always present)
- `port`: CDP debugging port (always present)
- `pid`: Chrome process ID (null when connecting to existing instance, integer when launched)

**Error (stderr):**
```json
{
  "error": "Chrome not found. Install Chrome or use --chrome-path to specify the executable location.",
  "code": 2
}
```

---

## Module Design

### `chrome::discovery` — Chrome Instance Discovery

**Purpose**: Find running Chrome instances via HTTP endpoints and DevToolsActivePort files.

**Key Types and Functions:**

```rust
/// Browser version info from /json/version
#[derive(Debug, Deserialize)]
pub struct BrowserVersion {
    #[serde(rename = "Browser")]
    pub browser: String,
    #[serde(rename = "Protocol-Version")]
    pub protocol_version: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub ws_debugger_url: String,
}

/// A discovered Chrome target from /json/list
#[derive(Debug, Deserialize)]
pub struct TargetInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub target_type: String,
    pub title: String,
    pub url: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub ws_debugger_url: Option<String>,
}

/// Query /json/version on a given host:port
pub async fn query_version(host: &str, port: u16) -> Result<BrowserVersion, ChromeError>;

/// Query /json/list on a given host:port
pub async fn query_targets(host: &str, port: u16) -> Result<Vec<TargetInfo>, ChromeError>;

/// Read the DevToolsActivePort file from the platform default Chrome user data dir.
/// Returns (port, ws_path) if found.
pub fn read_devtools_active_port() -> Result<(u16, String), ChromeError>;

/// Attempt to discover a running Chrome instance.
/// Tries DevToolsActivePort first, then common ports.
pub async fn discover_chrome(host: &str) -> Result<(String, u16), ChromeError>;
```

**HTTP Client**: A minimal async HTTP/1.1 GET function using `tokio::net::TcpStream`. Since all discovery queries are to localhost, no TLS is needed. This avoids adding `reqwest` or `hyper` as dependencies.

```rust
/// Perform a minimal HTTP GET to localhost and return the response body.
async fn http_get(host: &str, port: u16, path: &str) -> Result<String, ChromeError>;
```

### `chrome::launcher` — Chrome Process Launch

**Purpose**: Launch Chrome with remote debugging enabled and manage the process lifecycle.

**Key Types and Functions:**

```rust
/// A launched Chrome process with cleanup on drop.
pub struct ChromeProcess {
    child: std::process::Child,
    port: u16,
    temp_dir: Option<TempDir>,
}

impl ChromeProcess {
    /// Get the process ID.
    pub fn pid(&self) -> u32;

    /// Get the debugging port.
    pub fn port(&self) -> u16;

    /// Kill the Chrome process and clean up temp directory.
    pub fn kill(&mut self);
}

impl Drop for ChromeProcess {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Temporary directory wrapper that cleans up on drop.
struct TempDir {
    path: PathBuf,
}

/// Launch configuration.
pub struct LaunchConfig {
    pub executable: PathBuf,
    pub port: u16,
    pub headless: bool,
    pub extra_args: Vec<String>,
    pub user_data_dir: Option<PathBuf>,
}

/// Launch Chrome and wait for it to be ready.
pub async fn launch_chrome(config: LaunchConfig, timeout: Duration) -> Result<ChromeProcess, ChromeError>;

/// Find an available TCP port for Chrome debugging.
pub fn find_available_port() -> Result<u16, ChromeError>;
```

**Port selection**: Bind to port 0 and read the assigned port from the OS, then release it before passing to Chrome. This avoids port conflicts.

**Readiness polling**: After launching Chrome, poll `http://127.0.0.1:{port}/json/version` every 100ms until it responds or the timeout is exceeded.

### `chrome::platform` — Platform-Specific Paths

**Purpose**: Locate Chrome executables and data directories per platform.

**Key Functions:**

```rust
/// Chrome channel for selecting which Chrome variant to use.
/// (Re-exported from cli module or defined here with conversion)
pub enum Channel {
    Stable,
    Canary,
    Beta,
    Dev,
}

/// Find the Chrome executable for the given channel on the current platform.
pub fn find_chrome_executable(channel: Channel) -> Result<PathBuf, ChromeError>;

/// Get the default Chrome user data directory for the current platform.
pub fn default_user_data_dir() -> Option<PathBuf>;

/// Get all candidate Chrome executable paths for the current platform.
fn chrome_candidates(channel: Channel) -> Vec<PathBuf>;
```

**Platform implementation** uses `#[cfg(target_os = "...")]` attributes:

- **macOS**: Checks `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome` and variants for each channel. User data dir: `~/Library/Application Support/Google/Chrome/`.
- **Linux**: Searches PATH for `google-chrome`, `google-chrome-stable`, `chromium-browser`, `chromium`. User data dir: `~/.config/google-chrome/`.
- **Windows**: Checks `Program Files` and `Program Files (x86)` standard install paths. User data dir: `%LOCALAPPDATA%\Google\Chrome\User Data\`.

### `chrome::error` — Chrome-Specific Errors

```rust
#[derive(Debug)]
pub enum ChromeError {
    /// Chrome executable not found on this system.
    NotFound(String),
    /// Failed to launch Chrome process.
    LaunchFailed(String),
    /// Chrome did not become ready within the timeout.
    StartupTimeout { port: u16 },
    /// HTTP request to Chrome debug endpoint failed.
    HttpError(String),
    /// Failed to parse Chrome response.
    ParseError(String),
    /// DevToolsActivePort file not found or unreadable.
    NoActivePort,
    /// No running Chrome instance found.
    NotRunning,
    /// I/O error.
    Io(std::io::Error),
}
```

`ChromeError` converts to `AppError` with appropriate `ExitCode` mappings:
- `NotFound` → `GeneralError` (1)
- `LaunchFailed` → `ConnectionError` (2)
- `StartupTimeout` → `TimeoutError` (4)
- `HttpError`, `NotRunning` → `ConnectionError` (2)
- `ParseError` → `GeneralError` (1)

---

## Command Implementation

### `connect::execute`

The connect command handler lives conceptually in `main.rs`'s `run` function (or extracted to a separate module if desired). The flow:

```rust
async fn execute_connect(global: &GlobalOpts, args: &ConnectArgs) -> Result<(), AppError> {
    let timeout = Duration::from_millis(global.timeout.unwrap_or(30_000));

    // Strategy 1: Direct WebSocket URL
    if let Some(ws_url) = &global.ws_url {
        return output_connection(ws_url, extract_port(ws_url), None);
    }

    // Strategy 2: Explicit launch
    if args.launch {
        let executable = match &args.chrome_path {
            Some(path) => path.clone(),
            None => find_chrome_executable(args.channel)?,
        };
        let port = find_available_port()?;
        let process = launch_chrome(LaunchConfig {
            executable,
            port,
            headless: args.headless,
            extra_args: args.chrome_args.clone(),
            user_data_dir: None, // use temp dir
        }, timeout).await?;
        let version = query_version("127.0.0.1", process.port()).await?;
        return output_connection(&version.ws_debugger_url, process.port(), Some(process.pid()));
    }

    // Strategy 3: Discover on explicit port
    // (port comes from global.port which defaults to 9222)
    // Try discovery first
    match discover_chrome(&global.host).await {
        Ok((ws_url, port)) => output_connection(&ws_url, port, None),
        Err(_) => {
            // Auto-launch fallback
            let executable = find_chrome_executable(Channel::Stable)?;
            let port = find_available_port()?;
            let process = launch_chrome(/* ... */).await?;
            let version = query_version("127.0.0.1", process.port()).await?;
            output_connection(&version.ws_debugger_url, process.port(), Some(process.pid()))
        }
    }
}
```

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: reqwest for HTTP** | Use `reqwest` crate for HTTP GET | Battle-tested, async, full HTTP support | Heavy dependency (~2-3MB binary impact), pulls in hyper + http + tower | Rejected — overkill for localhost-only GET requests |
| **B: ureq for HTTP** | Use `ureq` blocking HTTP client | Lightweight, simple API | Blocking (needs spawn_blocking), still adds a dependency | Rejected — unnecessary dependency |
| **C: Raw TCP HTTP** | Minimal HTTP/1.1 GET using tokio TcpStream | Zero new dependencies, tiny code (~30 lines), localhost-only is simple | Not a full HTTP client, wouldn't work for complex scenarios | **Selected** — sufficient for our use case, keeps binary small |
| **D: Separate browser crate** | Extract chrome module to a separate workspace crate | Clean separation, reusable | Over-engineering for current scope, workspace adds complexity | Rejected — YAGNI, can extract later |

---

## Security Considerations

- [x] **Localhost only**: All HTTP discovery and WebSocket connections default to `127.0.0.1` (per tech.md)
- [x] **No secrets**: No credentials stored; Chrome debug port is ephemeral
- [x] **Process isolation**: Launched Chrome uses a temporary user data directory, not the user's profile
- [x] **Temp directory cleanup**: `Drop` trait ensures temp dirs are removed even on panic
- [x] **Input validation**: Port numbers validated by clap (u16 type), WebSocket URLs validated before connection

---

## Performance Considerations

- [x] **Fast discovery**: HTTP GET to localhost is sub-millisecond; parsing JSON response is trivial
- [x] **Polling interval**: 100ms polling for Chrome readiness balances speed vs CPU usage
- [x] **No new dependencies**: Binary size impact is minimal (only new source code)
- [x] **Port selection**: OS-assigned port via bind-to-0 is fast and race-free

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| `chrome::platform` | Unit | Executable path resolution per platform (compile-time cfg) |
| `chrome::discovery` | Unit + Integration | HTTP response parsing; mock HTTP server for endpoint queries |
| `chrome::launcher` | Integration | Launch/kill/cleanup (gated by `CHROME_AVAILABLE` env var) |
| `connect` command | BDD (cucumber) | Full end-to-end scenarios from requirements |
| HTTP client | Unit | Response parsing, error handling |

Integration tests requiring a real Chrome browser will be gated by an environment variable or feature flag to allow CI to skip them when Chrome is not installed.

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Port race condition (port freed then Chrome starts) | Low | Low | Very short window; retry with different port if launch fails |
| Chrome startup time varies by system | Medium | Medium | Configurable timeout via `--timeout`, reasonable default (30s) |
| Platform-specific path changes in Chrome updates | Low | Medium | Use `which` fallback on Linux, document `--chrome-path` override |
| Raw HTTP client doesn't handle edge cases | Low | Low | Only used for localhost, Chrome's debug server is simple and consistent |

---

## Open Questions

- [x] Where should `ChromeProcess` ownership live for long-running sessions? — For `connect`, it prints info and exits. Process cleanup happens when `ChromeProcess` is dropped. Future commands that need a persistent connection will need a different ownership model (future issue).

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] N/A — No database/storage changes
- [x] N/A — No state management (CLI tool, no persistent state)
- [x] N/A — No UI components (CLI tool)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
