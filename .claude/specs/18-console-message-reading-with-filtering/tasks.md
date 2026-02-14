# Tasks: Console Message Reading with Filtering

**Issue**: #18
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude (writing-specs)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 3 | [ ] |
| Integration | 1 | [ ] |
| Testing | 3 | [ ] |
| **Total** | **9** | |

---

## Phase 1: Setup

### T001: Add ConsoleArgs, ConsoleCommand, ReadArgs, and FollowArgs to CLI definitions

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `ConsoleArgs` struct with `command: ConsoleCommand` subcommand field
- [ ] `ConsoleCommand` enum with variants `Read(ReadArgs)` and `Follow(FollowArgs)`
- [ ] `ReadArgs` struct with fields:
  - `msg_id: Option<u64>` (positional, optional)
  - `--type <TYPES>` (comma-separated string, conflicts with `--errors-only`)
  - `--errors-only` (bool flag, conflicts with `--type`)
  - `--limit <N>` (default: 50)
  - `--page <N>` (default: 0)
  - `--include-preserved` (bool flag)
- [ ] `FollowArgs` struct with fields:
  - `--type <TYPES>` (comma-separated string, conflicts with `--errors-only`)
  - `--errors-only` (bool flag, conflicts with `--type`)
  - `--timeout <MS>` (optional positive integer)
- [ ] `Command::Console` variant changed from unit to `Console(ConsoleArgs)`
- [ ] `--tab` handled via existing `GlobalOpts.tab`
- [ ] Clap conflict groups: `--type` conflicts with `--errors-only`
- [ ] `cargo clippy` passes with no new warnings

**Notes**: Follow the existing pattern of `DialogArgs`/`DialogCommand` (enum with subcommand variants). The `--tab` flag is already in `GlobalOpts` so no need to duplicate it.

### T002: Define ConsoleMessage, ConsoleMessageDetail, StackFrame, and StreamMessage output types

**File(s)**: `src/console.rs`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `ConsoleMessage` struct with fields: `id`, `type` (renamed `msg_type` with `#[serde(rename = "type")]`), `text`, `timestamp`, `url`, `line`, `column`
- [ ] `ConsoleMessageDetail` struct extending `ConsoleMessage` with `args: Vec<Value>`, `stack_trace: Vec<StackFrame>` (with `#[serde(rename = "stackTrace")]`)
- [ ] `StackFrame` struct with fields: `file`, `line`, `column`, `function_name` (with `#[serde(rename = "functionName")]`)
- [ ] `StreamMessage` struct for follow output: `type`, `text`, `timestamp`
- [ ] All structs derive `Serialize`
- [ ] `print_read_plain()` function for plain text output of message list
- [ ] `print_detail_plain()` function for plain text output of single message detail
- [ ] Module declared in `src/main.rs`

**Notes**: Follow the serialization patterns from `src/interact.rs` (e.g., `ScrollResult`). Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields.

---

## Phase 2: Backend Implementation

### T003: Implement console message collection and formatting helpers

**File(s)**: `src/console.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `format_console_args(args: &[Value]) -> String` formats CDP `RemoteObject` array into text (value → description → JSON fallback, joined by spaces)
- [ ] `extract_stack_trace(stack_trace: &Value, max_frames: usize) -> Vec<StackFrame>` parses CDP stack trace with 50-frame limit
- [ ] `parse_console_event(event_params: &Value, id: usize) -> Option<ConsoleMessage>` converts a `Runtime.consoleAPICalled` event into a `ConsoleMessage`
- [ ] `parse_console_event_detail(event_params: &Value, id: usize) -> Option<ConsoleMessageDetail>` converts event into detailed view with args and stack trace
- [ ] `filter_by_type(messages: &[ConsoleMessage], types: &[String]) -> Vec<ConsoleMessage>` filters messages by type list
- [ ] `resolve_type_filter(type_arg: Option<&str>, errors_only: bool) -> Option<Vec<String>>` resolves `--type` or `--errors-only` into a type list
- [ ] `paginate(messages: Vec<ConsoleMessage>, limit: usize, page: usize) -> Vec<ConsoleMessage>` applies limit and page offset
- [ ] `map_cdp_type(cdp_type: &str) -> &str` maps CDP type names (e.g., "warning" → "warn")
- [ ] All helpers use `AppError` for error mapping

**Notes**: Reuse the arg formatting pattern from `src/js.rs:extract_console_entries()`. For stack traces, parse `stackTrace.callFrames[]` from the CDP event.

### T004: Implement execute_read (list and detail modes)

**File(s)**: `src/console.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_read(global, args) -> Result<(), AppError>` function implemented
- [ ] Session setup via existing `setup_session()` pattern (resolve connection, target, create session)
- [ ] Enables "Runtime" domain via `managed_session.ensure_domain("Runtime")`
- [ ] Enables "Page" domain if `--include-preserved` is set
- [ ] Subscribes to `Runtime.consoleAPICalled` events
- [ ] Drains event channel with short timeout (100ms) to collect pending messages
- [ ] **List mode** (no MSG_ID): collects messages, applies type filter, paginates, outputs JSON array
- [ ] **Detail mode** (MSG_ID provided): finds message by ID, outputs detailed JSON with args and stack trace
- [ ] Returns `AppError` with "Message ID N not found" if MSG_ID is out of range
- [ ] Navigation-aware collection: tags messages with navigation counter from `Page.frameNavigated`
- [ ] Without `--include-preserved`: only messages from current navigation
- [ ] With `--include-preserved`: messages from last 3 navigations
- [ ] Supports `--plain` output format via `print_read_plain()` / `print_detail_plain()`
- [ ] Uses existing `OutputFormat` from global args

**Notes**: Follow the pattern of `execute_dialog()` in `src/dialog.rs`. The session setup pattern is: `resolve_connection` → `resolve_target` → `CdpClient::connect` → `create_session` → `ManagedSession::new`.

### T005: Implement execute_follow (streaming mode)

**File(s)**: `src/console.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_follow(global, args) -> Result<(), AppError>` function implemented
- [ ] Session setup via existing pattern
- [ ] Enables "Runtime" domain
- [ ] Subscribes to `Runtime.consoleAPICalled` events
- [ ] Event loop: awaits events using `tokio::select!` with optional timeout
- [ ] Each event is formatted as a `StreamMessage` and printed as one JSON line to stdout
- [ ] Type filter applied: messages not matching filter are silently dropped
- [ ] Tracks whether any error-level messages (`error`, `assert`) were seen
- [ ] On timeout: exits with code 0 (no errors seen) or code 1 (errors seen)
- [ ] On SIGINT (Ctrl+C): exits gracefully with appropriate exit code
- [ ] On connection closed: exits with `ExitCode::ConnectionError`
- [ ] Uses `tokio::signal::ctrl_c()` for signal handling
- [ ] Flushes stdout after each message for real-time output

**Notes**: Use `tokio::select!` to race between event reception, timeout, and signal. Follow the streaming pattern — no buffering, immediate output per message.

---

## Phase 3: Integration

### T006: Wire Console command into dispatcher

**File(s)**: `src/main.rs`, `src/console.rs`
**Type**: Modify
**Depends**: T001, T004, T005
**Acceptance**:
- [ ] `Command::Console(args) => console::execute_console(&cli.global, args).await` in main dispatcher
- [ ] `execute_console()` dispatches to `execute_read()` or `execute_follow()` based on subcommand
- [ ] Remove `AppError::not_implemented("console")` from main.rs
- [ ] `mod console;` added to main.rs
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` passes
- [ ] `chrome-cli console --help` lists `read` and `follow` subcommands
- [ ] `chrome-cli console read --help` shows all read flags
- [ ] `chrome-cli console follow --help` shows all follow flags
- [ ] `chrome-cli console read --type error --errors-only` produces a clap conflict error

---

## Phase 4: BDD Testing

### T007: Create BDD feature file for console message reading

**File(s)**: `tests/features/console.feature`
**Type**: Create
**Depends**: T006
**Acceptance**:
- [ ] Feature file contains scenarios for all 18 acceptance criteria from requirements.md
- [ ] CLI argument validation scenarios (no Chrome required)
- [ ] Chrome-required scenarios use established Background/Given patterns
- [ ] Valid Gherkin syntax
- [ ] Covers: list messages, type filter, errors-only, limit, pagination, include-preserved, tab targeting, detail view, stack traces, follow stream, follow with filter, follow timeout, follow exit code, empty results, invalid message ID

### T008: Add console step definitions and wire into BDD runner

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T007
**Acceptance**:
- [ ] Console feature file registered in BDD test runner
- [ ] Existing step definitions (e.g., `I run {string}`, `the exit code should be {int}`) cover console scenarios
- [ ] Any new console-specific steps defined if needed (e.g., JSON array length assertions, message field assertions)
- [ ] `cargo test --test bdd` includes console scenarios

### T009: Add unit tests for output types, filtering, and pagination

**File(s)**: `src/console.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] Unit test: `ConsoleMessage` serialization produces correct JSON with `type` field name
- [ ] Unit test: `ConsoleMessageDetail` serialization includes `args` and `stackTrace` fields
- [ ] Unit test: `StackFrame` serialization uses `functionName` field name
- [ ] Unit test: `format_console_args` handles string, number, object, and undefined types
- [ ] Unit test: `filter_by_type` filters correctly with single and multiple types
- [ ] Unit test: `resolve_type_filter` returns `["error", "assert"]` for `--errors-only`
- [ ] Unit test: `paginate` returns correct slice for page 0 and page 1
- [ ] Unit test: `paginate` handles page beyond available data (returns empty)
- [ ] Unit test: `extract_stack_trace` limits to 50 frames
- [ ] Unit test: `map_cdp_type` maps "warning" to "warn"
- [ ] All unit tests pass: `cargo test --lib`

---

## Dependency Graph

```
T001 (CLI args) ──┬──▶ T004 (execute_read) ──────────────┐
                  │                                        │
T002 (types) ─────┼──▶ T003 (helpers) ──▶ T004            ├──▶ T006 (wire dispatcher)
                  │                    ──▶ T005 (follow) ──┘         │
                  │                         │                        │
                  └──▶ T009 (unit tests) ◀──┘                        ▼
                                                              T007 (feature file)
                                                                     │
                                                                     ▼
                                                              T008 (BDD wiring)
```

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
