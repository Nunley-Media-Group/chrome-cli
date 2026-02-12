# Tasks: Tab Management Commands

**Issue**: #7
**Date**: 2026-02-11
**Status**: Planning
**Author**: Claude (spec-driven development)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 4 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **9** | |

---

## Phase 1: Setup

### T001: Define CLI subcommand types for `tabs`

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `Tabs` variant changes from unit variant to `Tabs(TabsArgs)`
- [ ] `TabsArgs` struct contains `#[command(subcommand)] command: TabsCommand`
- [ ] `TabsCommand` enum has `List(TabsListArgs)`, `Create(TabsCreateArgs)`, `Close(TabsCloseArgs)`, `Activate(TabsActivateArgs)` variants
- [ ] `TabsListArgs` has `--all` bool flag
- [ ] `TabsCreateArgs` has optional positional `url: Option<String>`, `--background` bool flag, `--timeout <MS>` option
- [ ] `TabsCloseArgs` has required positional `targets: Vec<String>` (multiple values)
- [ ] `TabsActivateArgs` has required positional `target: String`, `--quiet` bool flag
- [ ] `cargo check` passes with no errors
- [ ] All existing clap help text and long_about on `Tabs` command is preserved

**Notes**: The `TabsCommand` subcommands should have short help descriptions using `///` doc comments. The `Tabs` variant's `long_about` already exists and should be kept.

### T002: Add `last_tab()` error constructor

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AppError::last_tab()` constructor added, returns `AppError { message: "Cannot close the last tab. Chrome requires at least one open tab.", code: ExitCode::TargetError }`
- [ ] Unit test added for `last_tab()` verifying message content and exit code
- [ ] `cargo test --lib` passes

---

## Phase 2: Backend Implementation

### T003: Implement `execute_list` handler

**File(s)**: `src/tabs.rs` (create new file)
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] Module has `pub async fn execute_tabs(global: &GlobalOpts, args: &TabsArgs) -> Result<(), AppError>` that dispatches to subcommand handlers
- [ ] `execute_list()` resolves connection via `resolve_connection()`
- [ ] Calls `query_targets(host, port)` to get target list
- [ ] Filters to `target_type == "page"` only
- [ ] Default mode: excludes URLs starting with `chrome://` (except `chrome://newtab/`) and `chrome-extension://`
- [ ] `--all` flag includes all page-type targets without URL filtering
- [ ] First page target in list is marked `active: true`, rest `active: false`
- [ ] Output is a JSON array of `TabInfo { id, url, title, active }` structs
- [ ] `--plain` flag outputs a human-readable table with columns: #, ID (truncated), TITLE, URL, ACTIVE
- [ ] `cargo check` passes

**Notes**: Define `TabInfo` as a `#[derive(Serialize)]` struct. Use the global `OutputFormat` to determine JSON vs plain output. Truncate ID to first 8 characters in plain text mode for readability.

### T004: Implement `execute_create` handler

**File(s)**: `src/tabs.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_create()` resolves connection via `resolve_connection()`
- [ ] Connects to Chrome via `CdpClient::connect()` with browser WebSocket URL
- [ ] Sends `Target.createTarget` with `url` (default: `"about:blank"`) and `background` params
- [ ] Extracts `targetId` from CDP response
- [ ] Queries targets again to get the new tab's url and title
- [ ] Outputs JSON: `{ id, url, title }`
- [ ] `cargo check` passes

**Notes**: Use `CdpConfig::default()` for the client config. Apply global `--timeout` to override `command_timeout` if provided.

### T005: Implement `execute_close` handler

**File(s)**: `src/tabs.rs`
**Type**: Modify
**Depends**: T003, T002
**Acceptance**:
- [ ] `execute_close()` resolves connection via `resolve_connection()`
- [ ] Queries targets to get current page count
- [ ] For each target argument: resolves via `select_target()` (supports index or ID)
- [ ] Validates that closing the requested tabs would not leave zero page targets remaining
- [ ] If last-tab violation: returns `AppError::last_tab()`
- [ ] Connects to Chrome via `CdpClient::connect()`
- [ ] For each resolved target: sends `Target.closeTarget` with `targetId`
- [ ] After all closes: queries remaining target count
- [ ] Outputs JSON: `{ closed: [id1, id2, ...], remaining: N }`
- [ ] Single target close outputs `closed` as a single string (not array) for backwards compat with issue spec
- [ ] `cargo check` passes

**Notes**: Close tabs sequentially (not in parallel) to avoid race conditions. Resolve all targets before closing any to avoid index shifting issues.

### T006: Implement `execute_activate` handler

**File(s)**: `src/tabs.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_activate()` resolves connection via `resolve_connection()`
- [ ] Resolves target via `resolve_target()` (supports index or ID)
- [ ] Connects to Chrome via `CdpClient::connect()`
- [ ] Sends `Target.activateTarget` with `targetId`
- [ ] Outputs JSON: `{ activated: target_id, url, title }`
- [ ] `--quiet` flag suppresses stdout output
- [ ] `cargo check` passes

---

## Phase 3: Integration

### T007: Wire tabs command dispatch in main.rs

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T003, T004, T005, T006
**Acceptance**:
- [ ] `mod tabs;` declaration added to `main.rs`
- [ ] `run()` match arm updated: `Command::Tabs(args) => tabs::execute_tabs(&cli.global, args).await`
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo clippy` passes with project's lint settings (all=deny, pedantic=warn)
- [ ] Running `chrome-cli tabs --help` shows subcommands: list, create, close, activate
- [ ] Running `chrome-cli tabs list --help` shows --all flag

---

## Phase 4: Testing

### T008: Create BDD feature file for tab management

**File(s)**: `tests/features/tab-management.feature`
**Type**: Create
**Depends**: T007
**Acceptance**:
- [ ] Feature file covers all 16 acceptance criteria from requirements.md
- [ ] Uses Background for common Chrome-running precondition
- [ ] Scenarios are independent and self-contained
- [ ] Valid Gherkin syntax
- [ ] Scenario names match AC names from requirements

### T009: Add unit tests for tab command logic

**File(s)**: `src/tabs.rs`
**Type**: Modify
**Depends**: T003, T004, T005, T006
**Acceptance**:
- [ ] Test: `filter_targets` correctly filters `chrome://` and `chrome-extension://` URLs
- [ ] Test: `filter_targets` preserves `chrome://newtab/`
- [ ] Test: `filter_targets` with `--all` returns all page targets
- [ ] Test: `format_plain_table` produces expected table format
- [ ] Test: active tab detection (first page target is active)
- [ ] Test: last-tab protection (would_remove_all_pages logic)
- [ ] All tests pass with `cargo test`

---

## Dependency Graph

```
T001 ──┬──▶ T003 ──┬──▶ T004
       │           ├──▶ T005 (also depends on T002)
       │           ├──▶ T006
       │           │
T002 ──┘           └──┬──▶ T007 ──▶ T008
                      │
                      └──▶ T009
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
