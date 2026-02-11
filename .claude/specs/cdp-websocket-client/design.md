# Design: CDP WebSocket Client

**Issue**: #4
**Date**: 2026-02-10
**Status**: Draft
**Author**: Claude (spec-driven)

---

## Overview

This design introduces the `src/cdp/` module — the foundational communication layer for chrome-cli. It provides an async WebSocket client that speaks the Chrome DevTools Protocol (CDP), handling command/response correlation, event subscriptions, session multiplexing, timeouts, and reconnection.

The client follows a split-architecture pattern: a public `CdpClient` handle that users interact with, and a background `Transport` task that owns the WebSocket connection and dispatches messages. Communication between them flows through tokio channels. This separation keeps the public API clean while enabling concurrent command/event handling on a single WebSocket connection.

The main.rs entry point will transition from synchronous to async (tokio runtime) as part of this work, since the CDP client is inherently async.

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────────┐
│                      CLI Layer (existing)                      │
│  main.rs → Cli::parse() → run() dispatches to commands        │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│                      CDP Layer (new)                           │
│                                                                │
│  ┌─────────────┐     ┌──────────────┐     ┌──────────────┐   │
│  │  CdpClient  │────▶│  Transport   │────▶│  WebSocket   │   │
│  │  (handle)   │     │  (bg task)   │     │  Connection  │   │
│  └──────┬──────┘     └──────┬───────┘     └──────────────┘   │
│         │                   │                                  │
│  ┌──────┴──────┐     ┌──────┴───────┐                         │
│  │ CdpSession  │     │   Message    │                         │
│  │  (per-tab)  │     │  Dispatch    │                         │
│  └─────────────┘     └─────────────┘                          │
│                                                                │
│  Types: CdpCommand, CdpResponse, CdpEvent, CdpError          │
└──────────────────────────────────────────────────────────────┘
                           │
                           ▼
                    Chrome Browser (CDP over WebSocket)
```

### Data Flow

#### Command (send and receive response)

```
1. Caller: client.send_command("Page.navigate", params).await
2. CdpClient: assigns unique ID (AtomicU64), creates oneshot channel
3. CdpClient: sends (id, method, params, session_id, oneshot_tx) to Transport via mpsc
4. Transport: serializes to JSON → {"id": N, "method": "...", "params": {...}}
5. Transport: writes JSON to WebSocket
6. Chrome: processes command, sends response
7. Transport: reads WebSocket message, parses JSON
8. Transport: matches response by id, sends result via oneshot_tx
9. CdpClient: receives result via oneshot_rx, returns to caller
```

#### Event (receive asynchronous notification)

```
1. Caller: let mut events = client.subscribe("Page.loadEventFired")
2. CdpClient: registers (method_filter, mpsc_tx) in Transport's subscriber list
3. Chrome: emits event (no id field, has method + params)
4. Transport: reads message, identifies as event (no id)
5. Transport: routes to matching subscribers by method name
6. Caller: events.recv().await returns the event
```

#### Session Multiplexing

```
1. Caller: let session = client.create_session(target_id).await
2. CdpClient: sends Target.attachToTarget command to Chrome
3. Chrome: returns sessionId
4. CdpClient: creates CdpSession with that sessionId
5. CdpSession.send_command(): includes sessionId in outgoing messages
6. Transport: routes incoming messages by sessionId to correct session
```

---

## Module Structure

Per `structure.md`, the CDP layer lives in `src/cdp/`:

```
src/cdp/
├── mod.rs              # Public API re-exports: CdpClient, CdpSession, CdpError, types
├── client.rs           # CdpClient (connection handle) and CdpSession (per-tab handle)
├── transport.rs        # Background WebSocket read/write loop, message dispatch
├── types.rs            # CDP message types: command, response, event, protocol error
└── error.rs            # CdpError enum (connection, protocol, timeout, parse errors)
```

### Public API Surface

```rust
// src/cdp/mod.rs — re-exports
pub use client::{CdpClient, CdpSession};
pub use error::CdpError;
pub use types::{CdpEvent, CdpResponse};
```

---

## API / Interface Design

### CdpClient

```rust
// src/cdp/client.rs

pub struct CdpClient { /* internal channels, config */ }

impl CdpClient {
    /// Connect to a Chrome CDP WebSocket endpoint.
    pub async fn connect(url: &str, config: CdpConfig) -> Result<Self, CdpError>;

    /// Send a CDP command (browser-level, no session).
    pub async fn send_command(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, CdpError>;

    /// Subscribe to CDP events matching a method name.
    /// Returns a receiver that yields CdpEvent values.
    pub fn subscribe(&self, method: &str) -> mpsc::Receiver<CdpEvent>;

    /// Unsubscribe by dropping the receiver (automatic cleanup).

    /// Create a CDP session attached to a specific target (tab).
    pub async fn create_session(&self, target_id: &str) -> Result<CdpSession, CdpError>;

    /// Gracefully close the WebSocket connection.
    pub async fn close(self) -> Result<(), CdpError>;

    /// Check if the client is currently connected.
    pub fn is_connected(&self) -> bool;
}
```

### CdpSession

```rust
pub struct CdpSession { /* session_id, shared transport channel */ }

impl CdpSession {
    /// Send a command within this session's context.
    pub async fn send_command(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, CdpError>;

    /// Subscribe to events within this session.
    pub fn subscribe(&self, method: &str) -> mpsc::Receiver<CdpEvent>;

    /// Get the session ID.
    pub fn session_id(&self) -> &str;
}
```

### CdpConfig

```rust
pub struct CdpConfig {
    /// Connection timeout (default: 10s)
    pub connect_timeout: Duration,
    /// Per-command timeout (default: 30s)
    pub command_timeout: Duration,
    /// Event channel capacity (default: 256)
    pub event_channel_capacity: usize,
    /// Reconnection settings
    pub reconnect: ReconnectConfig,
}

pub struct ReconnectConfig {
    /// Maximum number of reconnection attempts (default: 5)
    pub max_retries: u32,
    /// Initial backoff delay (default: 100ms)
    pub initial_backoff: Duration,
    /// Maximum backoff delay (default: 5s)
    pub max_backoff: Duration,
}
```

### Error Types

```rust
// src/cdp/error.rs

#[derive(Debug)]
pub enum CdpError {
    /// WebSocket connection could not be established
    Connection(String),

    /// Connection attempt exceeded timeout
    ConnectionTimeout,

    /// Command did not receive a response within the timeout
    CommandTimeout { method: String },

    /// Chrome returned a CDP protocol error
    Protocol { code: i64, message: String },

    /// WebSocket connection was closed unexpectedly
    ConnectionClosed,

    /// Failed to parse a message from Chrome
    InvalidResponse(String),

    /// Reconnection failed after all retries exhausted
    ReconnectFailed { attempts: u32, last_error: String },

    /// Internal channel error (transport task died)
    Internal(String),
}
```

Integration with existing `AppError`:

```rust
// src/error.rs — new From impl
impl From<CdpError> for AppError {
    fn from(e: CdpError) -> Self {
        match e {
            CdpError::Connection(_) | CdpError::ConnectionClosed
                | CdpError::ReconnectFailed { .. } => AppError {
                message: e.to_string(),
                code: ExitCode::ConnectionError,
            },
            CdpError::ConnectionTimeout | CdpError::CommandTimeout { .. } => AppError {
                message: e.to_string(),
                code: ExitCode::TimeoutError,
            },
            CdpError::Protocol { .. } => AppError {
                message: e.to_string(),
                code: ExitCode::ProtocolError,
            },
            CdpError::InvalidResponse(_) | CdpError::Internal(_) => AppError {
                message: e.to_string(),
                code: ExitCode::GeneralError,
            },
        }
    }
}
```

---

## CDP Message Types

```rust
// src/cdp/types.rs

use serde::{Deserialize, Serialize};

/// Outgoing command (client → Chrome)
#[derive(Debug, Serialize)]
pub struct CdpCommand {
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(rename = "sessionId", skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Incoming message (Chrome → client), before routing
#[derive(Debug, Deserialize)]
pub struct RawCdpMessage {
    pub id: Option<u64>,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<CdpProtocolError>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

/// CDP protocol error payload
#[derive(Debug, Clone, Deserialize)]
pub struct CdpProtocolError {
    pub code: i64,
    pub message: String,
}

/// Parsed incoming response (has id)
#[derive(Debug)]
pub struct CdpResponse {
    pub id: u64,
    pub result: Result<serde_json::Value, CdpProtocolError>,
    pub session_id: Option<String>,
}

/// Parsed incoming event (no id, has method)
#[derive(Debug, Clone)]
pub struct CdpEvent {
    pub method: String,
    pub params: serde_json::Value,
    pub session_id: Option<String>,
}
```

---

## Transport Design

### Internal Architecture

```rust
// src/cdp/transport.rs

/// Message sent from CdpClient to Transport
enum TransportCommand {
    /// Send a CDP command and deliver the response via oneshot
    SendCommand {
        command: CdpCommand,
        response_tx: oneshot::Sender<Result<serde_json::Value, CdpError>>,
    },
    /// Subscribe to events matching a method name
    Subscribe {
        method: String,
        session_id: Option<String>,
        event_tx: mpsc::Sender<CdpEvent>,
    },
    /// Unsubscribe (by subscriber ID)
    Unsubscribe { subscriber_id: u64 },
    /// Graceful shutdown
    Shutdown,
}
```

### Background Task

The transport runs as a single `tokio::spawn` task that:

1. **Selects** between:
   - Incoming WebSocket messages (read from the socket)
   - Outgoing commands (received from CdpClient via mpsc channel)
   - Shutdown signal

2. **On incoming WebSocket message**:
   - Parse JSON into `RawCdpMessage`
   - If `id` is present → it's a response → look up pending oneshot by `(id, session_id)`, send result
   - If `method` is present and no `id` → it's an event → route to matching subscribers by `(method, session_id)`
   - If JSON parse fails → log/report gracefully, continue

3. **On outgoing command**:
   - Store the oneshot sender in a `HashMap<u64, oneshot::Sender>` keyed by message ID
   - Serialize and write the command to the WebSocket
   - Start a per-command timeout (via `tokio::time::sleep` in a select)

4. **On WebSocket close/error**:
   - Drain all pending commands with `CdpError::ConnectionClosed`
   - Notify all event subscribers of disconnection
   - Attempt reconnection per `ReconnectConfig`

### Pending Request Tracking

```rust
/// Tracks in-flight commands awaiting responses
struct PendingRequests {
    /// Map from message ID to response sender
    requests: HashMap<u64, PendingRequest>,
}

struct PendingRequest {
    response_tx: oneshot::Sender<Result<serde_json::Value, CdpError>>,
    method: String,
    deadline: Instant,
}
```

Using `HashMap` (not `DashMap`) because only the transport task accesses it — no concurrent access needed. This is simpler and avoids the extra dependency.

### Message ID Generation

```rust
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_message_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}
```

Global atomic counter ensures uniqueness across all clients in the process. `Relaxed` ordering is sufficient — we only need uniqueness, not ordering guarantees.

---

## Reconnection Design

When the WebSocket connection drops:

1. Transport detects the close/error from the WebSocket read
2. All pending commands receive `CdpError::ConnectionClosed`
3. Transport enters reconnection loop:
   - Attempt reconnect with exponential backoff: 100ms → 200ms → 400ms → 800ms → 1600ms (capped at `max_backoff`)
   - Up to `max_retries` attempts (default: 5)
4. If reconnection succeeds:
   - Transport resumes normal operation with the new WebSocket
   - Event subscribers remain registered (they'll receive events from the new connection)
   - Sessions are invalidated (Chrome doesn't preserve sessions across reconnects)
   - `is_connected()` returns true again
5. If reconnection fails:
   - All event subscribers receive a disconnect notification
   - `is_connected()` returns false
   - Subsequent `send_command()` calls return `CdpError::ReconnectFailed`

---

## Changes to Existing Code

### main.rs — Add tokio runtime

```rust
// Before: fn main()
// After:  #[tokio::main] async fn main()

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(&cli).await {
        e.print_json_stderr();
        std::process::exit(e.code as i32);
    }
}

async fn run(cli: &Cli) -> Result<(), AppError> {
    // same match, but commands can now be async
    match &cli.command {
        // ... stubs remain synchronous (Err(AppError::not_implemented(...)))
    }
}
```

### Cargo.toml — Move tokio to production dependencies, add tokio-tungstenite

```toml
[dependencies]
clap = { version = "4", features = ["derive", "env"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time", "sync", "net"] }
tokio-tungstenite = "0.26"
futures-util = "0.3"
url = "2"
```

### src/error.rs — Add From<CdpError> conversion

Add the `From<CdpError> for AppError` implementation shown in the API section above.

### Module registration

```rust
// src/main.rs
mod cdp;   // new
mod cli;
mod error;
```

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: fastwebsockets** | Low-level, high-performance WebSocket library | Faster, smaller binary | Less mature, manual frame handling, no built-in TLS | Rejected — complexity not justified at this stage |
| **B: tokio-tungstenite** | Async WebSocket built on tungstenite | Mature, well-documented, easy to use, broad ecosystem | Slightly larger binary | **Selected** — reliability and ergonomics matter more than raw speed |
| **C: DashMap for pending requests** | Concurrent hashmap | Lock-free concurrent access | Extra dependency, unnecessary since only transport task accesses the map | Rejected — HashMap is sufficient |
| **D: Callback-based events** | Register closures for event handling | Flexible | Lifetime complexity in Rust, harder to compose | Rejected — channel-based is more idiomatic |
| **E: Split read/write tasks** | Separate tokio tasks for WebSocket read and write | True concurrency on read/write | More complex coordination, harder reconnection | Rejected — single task with `tokio::select!` is simpler and sufficient |

---

## Security Considerations

- [x] **Connection scope**: CDP connects only to localhost by default (enforced by `--host` default in CLI)
- [x] **No credentials**: CDP debug port has no authentication; security comes from local-only access
- [x] **Input validation**: WebSocket URL is parsed and validated before connection
- [x] **No TLS needed**: Localhost connections don't require encryption
- [x] **Malformed input**: Invalid JSON from Chrome is handled gracefully, never panics

---

## Performance Considerations

- [x] **Zero-copy where possible**: Use `serde_json::Value` to avoid double-parsing
- [x] **Bounded channels**: Event channels capped at 256 to prevent unbounded memory growth
- [x] **Single connection**: Session multiplexing avoids opening multiple WebSockets
- [x] **Efficient dispatch**: HashMap lookup by message ID is O(1) for response correlation
- [x] **Minimal allocations**: Reuse the transport's read buffer across messages

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| CDP types | Unit | Serialization/deserialization of all message types |
| CdpError | Unit | Display impls, From conversions |
| Message ID | Unit | Uniqueness, monotonic increment |
| Transport dispatch | Unit | Response routing, event routing, session routing |
| CdpClient | Integration | Full connect → command → response cycle with mock server |
| Concurrent commands | Integration | 100 concurrent commands with correct correlation |
| Event subscription | Integration | Subscribe, receive events, unsubscribe |
| Session multiplexing | Integration | Multiple sessions, correct routing |
| Timeouts | Integration | Connection timeout, command timeout |
| Reconnection | Integration | Disconnect → reconnect → resume |
| Error handling | Integration | Protocol errors, connection refused, invalid JSON |

### Mock WebSocket Server

Tests use `tokio-tungstenite`'s server capabilities to create an in-process mock CDP server:

```rust
async fn start_mock_server() -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        // Accept connections, echo CDP responses, emit events
    });
    (addr, handle)
}
```

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| tokio-tungstenite API changes | Low | Medium | Pin to specific minor version |
| Transport task panic | Low | High | Wrap in catch_unwind, report via channel |
| Channel backpressure on slow consumers | Medium | Medium | Bounded channels with configurable capacity; drop oldest event if full |
| Reconnection storm (rapid connect/disconnect) | Low | Medium | Exponential backoff with jitter |

---

## Open Questions

None — design decisions are resolved.

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with types
- [x] No database/storage changes needed
- [x] State management approach is clear (channels + HashMap)
- [x] No UI components (this is a library module)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
