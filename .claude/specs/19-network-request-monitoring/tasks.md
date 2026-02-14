# Tasks: Network Request Monitoring

**Issue**: #19
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude (spec generation)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 4 | [ ] |
| Integration | 2 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **10** | |

---

## Phase 1: Setup

### T001: Define CLI argument types for network subcommands

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `NetworkArgs` struct with `command: NetworkCommand` subcommand enum
- [ ] `NetworkCommand` enum with `List(NetworkListArgs)`, `Get(NetworkGetArgs)`, `Follow(NetworkFollowArgs)` variants
- [ ] `NetworkListArgs` has: `--type`, `--url`, `--status`, `--method`, `--limit` (default 50), `--page` (default 0), `--include-preserved`
- [ ] `NetworkGetArgs` has: `<req_id>` positional (u64), `--save-request` (PathBuf), `--save-response` (PathBuf)
- [ ] `NetworkFollowArgs` has: `--type`, `--url`, `--method`, `--timeout` (u64), `--verbose`
- [ ] `Command::Network` variant changed from unit to `Network(NetworkArgs)`
- [ ] `cargo clippy` passes

**Notes**: Follow the exact pattern of `ConsoleArgs`/`ConsoleCommand`/`ConsoleReadArgs`/`ConsoleFollowArgs` at line 847-905 of `src/cli/mod.rs`.

### T002: Define output types for network requests

**File(s)**: `src/network.rs` (create)
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `NetworkRequestSummary` struct (id, method, url, status, type, size, duration_ms, timestamp) with `Serialize` derive
- [ ] `NetworkRequestDetail` struct with nested `RequestInfo`, `ResponseInfo`, `TimingInfo`, `RedirectEntry`
- [ ] `NetworkStreamEvent` struct for follow mode (method, url, status, type, size, duration_ms, timestamp, optional headers)
- [ ] `RawNetworkEvent` struct for internal event accumulation (params, navigation_id)
- [ ] `NetworkRequestBuilder` struct for correlating multiple CDP events
- [ ] All types compile and serialize correctly
- [ ] `print_output` helper function matching console.rs pattern

---

## Phase 2: Backend Implementation

### T003: Implement event collection and correlation logic

**File(s)**: `src/network.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `setup_session` helper (same pattern as console.rs)
- [ ] `collect_network_events()` function that subscribes to `Network.requestWillBeSent`, `Network.responseReceived`, `Network.loadingFinished`, `Network.loadingFailed`, `Page.frameNavigated`
- [ ] Event drain with 100ms idle timeout (matching console read pattern)
- [ ] `correlate_events()` builds `HashMap<String, NetworkRequestBuilder>` from raw events
- [ ] Navigation ID tracking increments on `Page.frameNavigated`
- [ ] Builders correctly handle events arriving in any order
- [ ] Unit tests for event correlation with mock event data

### T004: Implement `network list` subcommand

**File(s)**: `src/network.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_list()` function collects events, correlates, filters, paginates, outputs
- [ ] `filter_by_type()` — comma-separated resource type matching (document, xhr, fetch, script, etc.)
- [ ] `filter_by_url()` — substring match on URL
- [ ] `filter_by_status()` — exact match (e.g., `404`) or wildcard (e.g., `4xx` → 400-499)
- [ ] `filter_by_method()` — case-insensitive HTTP method match
- [ ] `paginate()` — offset/limit calculation from `--page` and `--limit`
- [ ] `--include-preserved` controls navigation filtering
- [ ] Empty result returns `[]` with exit code 0
- [ ] Assigns sequential numeric IDs starting from 0
- [ ] Unit tests for each filter function and pagination

### T005: Implement `network get` subcommand

**File(s)**: `src/network.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_get()` function finds request by numeric ID
- [ ] Fetches request body via `Network.getRequestPostData` (for POST/PUT methods)
- [ ] Fetches response body via `Network.getResponseBody`
- [ ] Inline body truncated to 10,000 chars with `truncated: true` flag
- [ ] Binary detection from `base64Encoded` flag in `getResponseBody` response
- [ ] Binary responses set `binary: true`, body set to null
- [ ] `--save-request` writes request body to file
- [ ] `--save-response` writes response body to file (full, not truncated)
- [ ] Timing extracted from `responseReceived.response.timing` CDP object
- [ ] Redirect chain accumulated from multiple `requestWillBeSent` events with same `requestId`
- [ ] Returns error with exit code 1 if request ID not found
- [ ] Unit tests for body truncation and binary detection

### T006: Implement `network follow` subcommand

**File(s)**: `src/network.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_follow()` function with `tokio::select!` streaming loop (matching console follow pattern)
- [ ] Subscribes to Network events and correlates in-flight requests
- [ ] Emits JSON line when `loadingFinished` or `loadingFailed` fires
- [ ] `--type`, `--url`, `--method` filters applied before output
- [ ] `--timeout` exits after specified milliseconds
- [ ] Ctrl+C (SIGINT) exits cleanly
- [ ] `--verbose` includes `request_headers` and `response_headers` in output
- [ ] Flushes stdout after each line
- [ ] Handles connection close with ConnectionError

---

## Phase 3: Integration

### T007: Wire network command into main dispatcher

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T001, T004, T005, T006
**Acceptance**:
- [ ] `pub mod network;` added to module declarations
- [ ] `Command::Network(args)` match arm calls `network::execute_network(&cli.global, args).await`
- [ ] Removes previous `Command::Network => Err(AppError::not_implemented("network"))` stub
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` passes with all=deny, pedantic=warn

### T008: Add auto-dismiss dialog support to network commands

**File(s)**: `src/network.rs`
**Type**: Modify
**Depends**: T007
**Acceptance**:
- [ ] `setup_session` checks `global.auto_dismiss_dialogs` and spawns auto-dismiss task
- [ ] Follows same pattern as navigate.rs (lines 113-115)

---

## Phase 4: Testing

### T009: Create BDD feature file for network commands

**File(s)**: `tests/features/network.feature`
**Type**: Create
**Depends**: T007
**Acceptance**:
- [ ] All 23 acceptance criteria from requirements.md are Gherkin scenarios
- [ ] Uses Given/When/Then format
- [ ] Includes happy path, error, and edge case scenarios
- [ ] Feature file is valid Gherkin syntax
- [ ] Scenario Outline used for parameterized filter tests

### T010: Add unit tests for network module

**File(s)**: `src/network.rs` (inline `#[cfg(test)]` module)
**Type**: Modify
**Depends**: T004, T005, T006
**Acceptance**:
- [ ] Tests for `filter_by_type()` with single and comma-separated types
- [ ] Tests for `filter_by_url()` substring matching
- [ ] Tests for `filter_by_status()` exact and wildcard (`4xx`) matching
- [ ] Tests for `filter_by_method()` case-insensitive matching
- [ ] Tests for `paginate()` boundary conditions
- [ ] Tests for body truncation logic (under/over 10,000 chars)
- [ ] Tests for binary detection
- [ ] Tests for status wildcard parsing
- [ ] Tests for `NetworkRequestSummary` and `NetworkRequestDetail` serialization
- [ ] All tests pass with `cargo test`

---

## Dependency Graph

```
T001 (CLI args) ──────────────────────────────┐
                                               ▼
T002 (output types) ──▶ T003 (event correlation) ──▶ T004 (list)  ──┐
                                                  ──▶ T005 (get)   ──┼──▶ T007 (main.rs wiring) ──▶ T008 (auto-dismiss)
                                                  ──▶ T006 (follow)──┘          │
                                                                                ▼
                                                                         T009 (BDD feature)
                                                                         T010 (unit tests)
```

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure
- [x] Test tasks are included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
