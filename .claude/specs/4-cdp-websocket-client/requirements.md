# Requirements: CDP WebSocket Client

**Issue**: #4
**Date**: 2026-02-10
**Status**: Draft
**Author**: Claude (spec-driven)

---

## User Story

**As a** developer/automation engineer using chrome-cli
**I want** a reliable CDP client that communicates with Chrome over WebSocket
**So that** all CLI commands can send DevTools Protocol messages and receive responses/events from the browser

---

## Background

Chrome exposes a debugging protocol (CDP) over WebSocket. This is the foundational communication layer for the entire chrome-cli project — every command (navigate, screenshot, DOM inspection, JS execution, etc.) depends on this client to talk to Chrome. The MCP server ecosystem uses Puppeteer as its CDP client; chrome-cli needs a lightweight, purpose-built Rust implementation that handles CDP's JSON-RPC-like message format, multiplexed sessions, and event subscriptions.

The client must be async (tokio-based) since CDP communication is inherently concurrent: commands are sent and correlated by ID, events arrive at any time, and multiple sessions (one per tab) share a single WebSocket connection.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Connect to Chrome CDP endpoint

**Given** Chrome is running with CDP enabled on a known WebSocket URL
**When** the CDP client attempts to connect to the WebSocket endpoint
**Then** a WebSocket connection is established successfully
**And** the client is ready to send commands

**Example**:
- Given: Chrome running at `ws://127.0.0.1:9222/devtools/browser/abc-123`
- When: `CdpClient::connect("ws://127.0.0.1:9222/devtools/browser/abc-123")` is called
- Then: Connection succeeds, client handle is returned

### AC2: Send CDP command and receive response

**Given** a connected CDP client
**When** a CDP command is sent with a method name and parameters
**Then** the command is assigned a unique message ID
**And** the JSON response is returned correlated to the correct command
**And** the response result is deserialized

**Example**:
- Given: Connected client
- When: Send `Page.navigate` with `{"url": "https://example.com"}`
- Then: Response contains `{"frameId": "...", "loaderId": "..."}`

### AC3: Message ID generation and response correlation

**Given** a connected CDP client with multiple commands in flight
**When** two commands are sent concurrently
**Then** each command receives its own unique message ID
**And** each response is delivered to the correct caller
**And** responses are not mixed up between callers

### AC4: Receive CDP events

**Given** a connected CDP client subscribed to an event
**When** Chrome emits a CDP event (e.g., `Page.loadEventFired`)
**Then** the event is delivered to registered listeners
**And** the event contains the method name and parameters

### AC5: Event subscription and unsubscription

**Given** a connected CDP client
**When** a listener is registered for a specific CDP event method
**Then** only matching events are delivered to that listener
**And** when the listener is dropped or unsubscribed, events stop being delivered

### AC6: Session multiplexing over single WebSocket

**Given** a connected CDP client with a browser-level connection
**When** multiple CDP sessions are created (one per tab/target)
**Then** all sessions share the single WebSocket connection
**And** commands routed to different sessions include the correct `sessionId`
**And** responses and events are delivered to the correct session

### AC7: Flatten session protocol support

**Given** a CDP client connected to Chrome
**When** a command is sent targeting a specific session
**Then** the outgoing message includes the `sessionId` field
**And** incoming messages are routed by their `sessionId` field

### AC8: Connection timeout

**Given** a CDP client attempting to connect to an unreachable endpoint
**When** the connection attempt exceeds the configured timeout
**Then** a timeout error is returned
**And** the error includes a descriptive message

**Example**:
- Given: Unreachable endpoint `ws://127.0.0.1:9999/devtools/browser/nonexistent`
- When: Connect with 5-second timeout
- Then: `CdpError::ConnectionTimeout` after ~5 seconds

### AC9: Command timeout

**Given** a connected CDP client
**When** a command is sent and Chrome does not respond within the configured timeout
**Then** a timeout error is returned for that specific command
**And** other in-flight commands are not affected

### AC10: WebSocket close handling

**Given** a connected CDP client
**When** the WebSocket connection is closed (by Chrome or network failure)
**Then** all pending commands receive a connection-closed error
**And** event listeners are notified of the disconnection
**And** the client reports its disconnected state

### AC11: CDP protocol error handling

**Given** a connected CDP client
**When** Chrome returns an error response (e.g., `{"id": 1, "error": {"code": -32000, "message": "..."}}`)
**Then** the error is parsed and returned as a typed `CdpError::Protocol` error
**And** the error includes the CDP error code and message

### AC12: Connection failure error handling

**Given** a CDP client
**When** the WebSocket connection cannot be established (refused, DNS failure, etc.)
**Then** a typed `CdpError::Connection` error is returned
**And** the error includes the underlying cause

### AC13: Reconnection after disconnection

**Given** a CDP client that was previously connected
**When** the WebSocket connection is lost unexpectedly
**Then** the client attempts to reconnect to the same endpoint
**And** if reconnection succeeds, the client is usable again
**And** if reconnection fails after configured retries, a permanent error is reported

**Example**:
- Given: Connected client, Chrome restarts
- When: WebSocket drops
- Then: Client retries connection (e.g., 3 attempts with backoff)
- And: If Chrome is back, connection is restored

### AC14: Invalid response handling

**Given** a connected CDP client
**When** Chrome sends a malformed JSON message
**Then** the message is handled gracefully without crashing
**And** a typed error is reported for the affected command (if correlatable)

### Generated Gherkin Preview

```gherkin
Feature: CDP WebSocket Client
  As a developer using chrome-cli
  I want a reliable CDP client that communicates with Chrome over WebSocket
  So that CLI commands can send DevTools Protocol messages to the browser

  Background:
    Given a mock CDP WebSocket server is running

  Scenario: Connect to Chrome CDP endpoint
    When the CDP client connects to the mock server
    Then the connection is established successfully

  Scenario: Send command and receive response
    Given a connected CDP client
    When I send a "Page.navigate" command with params '{"url": "https://example.com"}'
    Then I receive a response with a matching message ID
    And the response contains a result object

  Scenario: Concurrent command correlation
    Given a connected CDP client
    When I send 10 commands concurrently
    Then each command receives its own response
    And no responses are mismatched

  Scenario: Receive CDP events
    Given a connected CDP client subscribed to "Page.loadEventFired"
    When the server emits a "Page.loadEventFired" event
    Then the event is delivered to the listener

  Scenario: Session multiplexing
    Given a connected CDP client with two sessions
    When I send a command on each session
    Then each command includes the correct sessionId
    And each response is routed to the correct session

  Scenario: Connection timeout
    Given an unreachable CDP endpoint
    When the client attempts to connect with a 1-second timeout
    Then a ConnectionTimeout error is returned

  Scenario: Command timeout
    Given a connected CDP client
    When I send a command and the server does not respond
    Then a Timeout error is returned after the configured duration

  Scenario: WebSocket close handling
    Given a connected CDP client with a pending command
    When the server closes the WebSocket connection
    Then the pending command receives a connection error

  Scenario: Protocol error handling
    Given a connected CDP client
    When I send a command that triggers a CDP error
    Then a Protocol error is returned with the CDP error code and message

  Scenario: Connection refused
    Given a CDP endpoint that refuses connections
    When the client attempts to connect
    Then a Connection error is returned

  Scenario: Reconnection after disconnection
    Given a connected CDP client
    When the server drops the connection and restarts
    Then the client reconnects automatically
    And the client is usable again after reconnection

  Scenario: Reconnection failure
    Given a connected CDP client
    When the server drops the connection permanently
    Then the client reports a permanent connection error after retries

  Scenario: Invalid JSON handling
    Given a connected CDP client
    When the server sends malformed JSON
    Then the client does not crash
    And the error is handled gracefully
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | WebSocket connection to Chrome CDP endpoint | Must | Foundation for all communication |
| FR2 | Send CDP commands (method + params), receive JSON responses | Must | Core request/response cycle |
| FR3 | Unique message ID generation and response correlation | Must | Enables concurrent commands |
| FR4 | Event subscription system for CDP events | Must | Required for waiting/monitoring commands |
| FR5 | Session multiplexing over single WebSocket | Must | One connection, multiple tabs |
| FR6 | Flatten session protocol (sessionId in messages) | Must | Standard CDP session routing |
| FR7 | Async/await API using tokio | Must | Non-blocking I/O for CLI responsiveness |
| FR8 | Connection timeout configuration | Must | Fail fast on unreachable Chrome |
| FR9 | Command-level timeout | Must | Prevent hanging on unresponsive commands |
| FR10 | Graceful WebSocket close handling | Must | Clean shutdown, error propagation |
| FR11 | Typed error types for all failure modes | Must | Connection, protocol, timeout, parse errors |
| FR12 | Background message dispatch loop | Should | Efficient multiplexing of reads/writes |
| FR13 | Reconnection with retry and backoff | Must | Auto-reconnect on transient failures (per issue AC) |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Command round-trip overhead < 1ms beyond network latency; support 100+ concurrent in-flight commands |
| **Security** | Connect only to localhost by default (per tech.md); no TLS required for local CDP |
| **Reliability** | Graceful degradation on connection loss; no panics on malformed input |
| **Platforms** | macOS, Linux, Windows (tokio + tungstenite are cross-platform) |
| **Memory** | < 50MB idle with connection open (per tech.md performance targets) |

---

## Data Requirements

### Input Data (Outgoing CDP Messages)

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| id | u64 | Auto-generated, monotonically increasing | Yes |
| method | String | Non-empty, dot-separated domain.method | Yes |
| params | serde_json::Value | Valid JSON object (or omitted) | No |
| sessionId | String | Valid CDP session ID | No (browser-level commands) |

### Output Data (Incoming CDP Messages)

| Field | Type | Description |
|-------|------|-------------|
| id | u64 | Correlates to sent command |
| result | serde_json::Value | Command success payload |
| error | CdpProtocolError | Command error (code + message) |
| method | String | Event name (events only, no id) |
| params | serde_json::Value | Event payload |
| sessionId | String | Session routing (if session-scoped) |

---

## Dependencies

### Internal Dependencies
- [x] Issue #1 — Repo setup (complete)
- [x] Issue #3 — CLI skeleton (complete, provides error types and CLI structure)

### External Dependencies
- `tokio` — Async runtime
- `tokio-tungstenite` — WebSocket client (or `fastwebsockets`)
- `serde_json` — JSON serialization (already in Cargo.toml)
- `futures-util` — Stream utilities for WebSocket message processing

---

## Out of Scope

- **CDP domain-specific command wrappers** (e.g., `Page.navigate` helper) — that's for downstream issues
- **Chrome process launch/discovery** — covered by issue #5
- **HTTP endpoint discovery** (fetching `/json/version` to get WebSocket URL) — covered by issue #5
- **Automatic Chrome lifecycle management** — covered by issue #5/#6
- **TLS/WSS support** — CDP is localhost-only, not needed now
- **Full CDP type generation from protocol spec** — manual types are sufficient for now

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| All acceptance criteria pass | 14/14 BDD scenarios green | `cargo test --test bdd` |
| No panics on malformed input | 0 panics | Fuzz-style test with garbage JSON |
| Concurrent command correctness | 100% correlation accuracy | Test with 100 concurrent commands |
| Connection timeout accuracy | Within 500ms of configured timeout | Timing test |

---

## Open Questions

- [x] `tokio-tungstenite` vs `fastwebsockets` — tokio-tungstenite is more mature and widely used; prefer it unless benchmarks show a problem
- [x] Channel capacity for event subscriptions — start with bounded channel (capacity 256), caller can configure

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements (kept at behavior level)
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified (AC8-AC13)
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented and resolved
