# Tasks: CDP WebSocket Client

**Issue**: #4
**Date**: 2026-02-10
**Status**: Planning
**Author**: Claude (spec-driven)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 3 | [ ] |
| Core Implementation | 2 | [ ] |
| Integration | 2 | [ ] |
| Testing | 4 | [ ] |
| **Total** | **11** | |

---

## Phase 1: Setup

### T001: Add production dependencies to Cargo.toml

**File(s)**: `Cargo.toml`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `tokio` moved from dev-dependencies to dependencies with features: `macros`, `rt-multi-thread`, `time`, `sync`, `net`
- [ ] `tokio-tungstenite = "0.26"` added to dependencies
- [ ] `futures-util = "0.3"` added to dependencies
- [ ] `url = "2"` added to dependencies
- [ ] `cargo check` passes

**Notes**: tokio is already in dev-dependencies; move it to production and expand features. Remove the duplicate from dev-dependencies.

### T002: Create CDP error types

**File(s)**: `src/cdp/error.rs`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `CdpError` enum with 8 variants: `Connection`, `ConnectionTimeout`, `CommandTimeout`, `Protocol`, `ConnectionClosed`, `InvalidResponse`, `ReconnectFailed`, `Internal`
- [ ] `Display` impl with descriptive messages for each variant
- [ ] `std::error::Error` impl
- [ ] No `unwrap()` or `panic!()` in non-test code
- [ ] `cargo clippy` passes

### T003: Create CDP message types

**File(s)**: `src/cdp/types.rs`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `CdpCommand` struct with `id`, `method`, `params`, `session_id` — derives `Serialize`
- [ ] `RawCdpMessage` struct for incoming JSON — derives `Deserialize`
- [ ] `CdpProtocolError` struct with `code` and `message` — derives `Deserialize`, `Clone`
- [ ] `CdpResponse` struct with `id`, `result` (Result type), `session_id`
- [ ] `CdpEvent` struct with `method`, `params`, `session_id` — derives `Clone`
- [ ] `RawCdpMessage` has a method to classify as response or event
- [ ] `serde` field renames: `sessionId` ↔ `session_id`
- [ ] `cargo clippy` passes

---

## Phase 2: Core Implementation

### T004: Create transport layer

**File(s)**: `src/cdp/transport.rs`
**Type**: Create
**Depends**: T001, T002, T003
**Acceptance**:
- [ ] `TransportCommand` enum for CdpClient→Transport communication
- [ ] `Transport::connect(url, config)` establishes WebSocket connection with connection timeout
- [ ] `Transport::run()` spawns a background tokio task
- [ ] Background task uses `tokio::select!` over WebSocket reads and command channel
- [ ] Incoming responses dispatched by `(id, session_id)` to correct `oneshot::Sender`
- [ ] Incoming events dispatched by `(method, session_id)` to matching `mpsc::Sender` subscribers
- [ ] Pending requests tracked in `HashMap<u64, PendingRequest>` with per-command deadlines
- [ ] Timed-out commands receive `CdpError::CommandTimeout`
- [ ] On WebSocket close: all pending commands receive `CdpError::ConnectionClosed`
- [ ] Reconnection with exponential backoff (configurable retries, initial backoff, max backoff)
- [ ] After successful reconnect: transport resumes, event subscribers persist
- [ ] After failed reconnect: `CdpError::ReconnectFailed` reported
- [ ] Malformed JSON handled gracefully (logged/skipped, no panic)
- [ ] Graceful shutdown on `TransportCommand::Shutdown`
- [ ] No `unwrap()` or `panic!()` in non-test code
- [ ] `cargo clippy` passes

**Notes**: This is the most complex task. The transport owns the WebSocket and is the only component that reads/writes it. All other components communicate via channels.

### T005: Create CdpClient, CdpSession, and module entry point

**File(s)**: `src/cdp/client.rs`, `src/cdp/mod.rs`
**Type**: Create
**Depends**: T002, T003, T004
**Acceptance**:
- [ ] `CdpConfig` struct with `connect_timeout`, `command_timeout`, `event_channel_capacity`, `reconnect` — has `Default` impl with sensible values
- [ ] `ReconnectConfig` struct with `max_retries`, `initial_backoff`, `max_backoff` — has `Default` impl
- [ ] `CdpClient::connect(url, config)` returns `Result<Self, CdpError>`
- [ ] `CdpClient::send_command(method, params)` assigns unique ID, sends via transport, awaits oneshot response with timeout
- [ ] `CdpClient::subscribe(method)` registers event listener, returns `mpsc::Receiver<CdpEvent>`
- [ ] `CdpClient::create_session(target_id)` sends `Target.attachToTarget`, returns `CdpSession`
- [ ] `CdpClient::close()` sends shutdown to transport, awaits WebSocket close
- [ ] `CdpClient::is_connected()` returns current connection state
- [ ] `CdpSession::send_command()` includes `session_id` in outgoing messages
- [ ] `CdpSession::subscribe()` filters events by `session_id`
- [ ] `CdpSession::session_id()` returns the session ID
- [ ] Message ID generation via `AtomicU64` (global, monotonically increasing)
- [ ] `src/cdp/mod.rs` re-exports: `CdpClient`, `CdpSession`, `CdpConfig`, `ReconnectConfig`, `CdpError`, `CdpEvent`, `CdpResponse`
- [ ] No `unwrap()` or `panic!()` in non-test code
- [ ] `cargo clippy` passes

---

## Phase 3: Integration

### T006: Add From<CdpError> for AppError conversion

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `From<CdpError> for AppError` impl maps each variant to the correct `ExitCode`
- [ ] `Connection`, `ConnectionClosed`, `ReconnectFailed` → `ExitCode::ConnectionError`
- [ ] `ConnectionTimeout`, `CommandTimeout` → `ExitCode::TimeoutError`
- [ ] `Protocol` → `ExitCode::ProtocolError`
- [ ] `InvalidResponse`, `Internal` → `ExitCode::GeneralError`
- [ ] Existing tests still pass
- [ ] `cargo clippy` passes

### T007: Convert main.rs to async and register CDP module

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T005, T006
**Acceptance**:
- [ ] `mod cdp;` declaration added
- [ ] `main()` uses `#[tokio::main]` attribute
- [ ] `run()` is `async fn`
- [ ] All existing subcommand stubs still work (return `AppError::not_implemented`)
- [ ] Existing BDD tests for CLI skeleton still pass
- [ ] `cargo clippy` passes
- [ ] `cargo test` passes (all existing tests)

---

## Phase 4: Testing

### T008: Unit tests for CDP types and errors

**File(s)**: `src/cdp/types.rs`, `src/cdp/error.rs`
**Type**: Modify (add `#[cfg(test)] mod tests`)
**Depends**: T002, T003
**Acceptance**:
- [ ] `CdpCommand` serializes to correct JSON (with and without `sessionId`, with and without `params`)
- [ ] `RawCdpMessage` deserializes from response JSON (with `id` + `result`)
- [ ] `RawCdpMessage` deserializes from error response JSON (with `id` + `error`)
- [ ] `RawCdpMessage` deserializes from event JSON (with `method` + `params`, no `id`)
- [ ] `RawCdpMessage` deserializes from session-scoped messages (with `sessionId`)
- [ ] `RawCdpMessage` classification method correctly identifies responses vs events
- [ ] `CdpError::Display` produces descriptive messages for each variant
- [ ] Message ID generation produces unique, monotonically increasing IDs
- [ ] All tests pass with `cargo test --lib`

### T009: Integration tests with mock WebSocket server

**File(s)**: `tests/cdp_integration.rs`
**Type**: Create
**Depends**: T005, T007
**Acceptance**:
- [ ] Mock CDP WebSocket server helper that accepts connections and responds to commands
- [ ] Test: connect to mock server successfully
- [ ] Test: send command, receive correct response
- [ ] Test: send 100 concurrent commands, all responses correctly correlated
- [ ] Test: subscribe to event, receive event when mock server emits it
- [ ] Test: unsubscribe (drop receiver), no more events delivered
- [ ] Test: session multiplexing — two sessions, commands routed by sessionId
- [ ] Test: connection timeout — connect to unreachable address, receive `CdpError::ConnectionTimeout`
- [ ] Test: command timeout — send command, mock server doesn't respond, receive `CdpError::CommandTimeout`
- [ ] Test: WebSocket close — mock server closes connection, pending commands get `CdpError::ConnectionClosed`
- [ ] Test: protocol error — mock server returns CDP error, receive `CdpError::Protocol`
- [ ] Test: invalid JSON — mock server sends garbage, client doesn't panic
- [ ] Test: reconnection success — mock server drops, restarts, client reconnects
- [ ] Test: reconnection failure — mock server drops permanently, client reports `CdpError::ReconnectFailed`
- [ ] All tests pass with `cargo test --test cdp_integration`

### T010: Create BDD feature file

**File(s)**: `tests/features/cdp-websocket-client.feature`
**Type**: Create
**Depends**: None (derived from requirements)
**Acceptance**:
- [ ] All 14 acceptance criteria from requirements.md are Gherkin scenarios
- [ ] Valid Gherkin syntax
- [ ] Uses Background for shared mock server setup
- [ ] Scenarios are independent and self-contained
- [ ] File location matches `tech.md` convention (`tests/features/`)

### T011: Implement BDD step definitions

**File(s)**: `tests/bdd.rs`
**Type**: Modify (add new World and step definitions)
**Depends**: T009, T010
**Acceptance**:
- [ ] New `CdpWorld` cucumber world with mock server and CdpClient state
- [ ] Step definitions for all CDP feature file scenarios
- [ ] Steps use the mock WebSocket server from T009 (shared test infrastructure)
- [ ] All BDD scenarios pass with `cargo test --test bdd`
- [ ] Existing CLI skeleton BDD scenarios still pass

---

## Dependency Graph

```
T001 (deps) ────────────────────────────┐
                                        │
T002 (errors) ──┬───────────────────────┤
                │                       │
T003 (types) ───┤                       │
                │                       │
                ├──▶ T004 (transport) ──┤
                │                       │
                ├──▶ T008 (unit tests)  │
                │                       │
                └──▶ T004 ──┐           │
                            │           │
T004 (transport) ───────────┴──▶ T005 (client + mod.rs)
                                        │
T002 ──▶ T006 (AppError conversion) ────┤
                                        │
T005 + T006 ──▶ T007 (async main) ──────┤
                                        │
T005 + T007 ──▶ T009 (integration tests)│
                                        │
T010 (feature file) ──┐                 │
                      └──▶ T011 (BDD step definitions)
T009 ─────────────────┘
```

**Critical path**: T001 → T004 → T005 → T007 → T009 → T011

**Parallelizable**:
- T002 and T003 can be done in parallel (no dependencies between them)
- T008 can start as soon as T002 and T003 are done
- T006 can start as soon as T002 is done
- T010 can be done at any time (derived from requirements, not code)

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included for each layer
- [x] No circular dependencies
- [x] Tasks are in logical execution order
