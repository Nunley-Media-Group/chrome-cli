# Tasks: Browser Dialog Handling

**Issue**: #20
**Date**: 2026-02-13
**Status**: Planning
**Author**: Claude (writing-specs)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 3 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **8** | |

---

## Phase 1: Setup

### T001: Define CLI argument types for dialog commands

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `DialogArgs` struct with `#[command(subcommand)]` field
- [ ] `DialogCommand` enum with `Handle(DialogHandleArgs)` and `Info` variants
- [ ] `DialogHandleArgs` struct with `action: DialogAction` positional arg and `--text` optional arg
- [ ] `DialogAction` enum (`Accept`, `Dismiss`) deriving `ValueEnum`
- [ ] `Command::Dialog(DialogArgs)` variant added to the `Command` enum with appropriate `long_about`
- [ ] `--auto-dismiss-dialogs` flag added to `GlobalOpts`
- [ ] `cargo check` passes with no errors

**Notes**: Follow the exact pattern used by `TabsArgs`/`TabsCommand` and `NavigateArgs`/`NavigateCommand`. The `action` field is a positional argument (like tab targets), not a subcommand. `--text` is only meaningful for prompt dialogs with the accept action.

### T002: Add error constructors for dialog errors

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AppError::no_dialog_open()` constructor returning `ExitCode::GeneralError` with message "No dialog is currently open. A dialog must be open before it can be handled."
- [ ] `AppError::dialog_handle_failed(reason: &str)` constructor returning `ExitCode::ProtocolError`
- [ ] `cargo check` passes with no errors

---

## Phase 2: Backend Implementation

### T003: Implement dialog command module — output types and helpers

**File(s)**: `src/dialog.rs` (create)
**Type**: Create
**Depends**: T001, T002
**Acceptance**:
- [ ] `HandleResult` struct (Serialize): `action`, `dialog_type`, `message`, optional `text`
- [ ] `InfoResult` struct (Serialize): `open`, optional `type`, `message`, `default_value`
- [ ] `print_output()` helper (JSON/pretty format — same pattern as navigate.rs)
- [ ] `cdp_config()` helper (same pattern as navigate.rs)
- [ ] `setup_session()` helper (same pattern as navigate.rs)
- [ ] Plain text formatting functions for both result types
- [ ] `cargo check` passes with no errors

### T004: Implement `execute_handle` — accept/dismiss dialogs

**File(s)**: `src/dialog.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_handle()` function that:
  - Sets up session via `setup_session()`
  - Enables `Page` domain via `managed.ensure_domain("Page")`
  - Subscribes to `Page.javascriptDialogOpening` to capture dialog metadata
  - Calls `Page.handleJavaScriptDialog` with `accept` (bool) and optional `promptText`
  - On CDP error → returns `AppError::no_dialog_open()` or `AppError::dialog_handle_failed()`
  - On success → checks event channel for dialog metadata, builds `HandleResult`
  - Outputs result via `print_output()` or plain text formatter
- [ ] Handles all 4 dialog types (alert, confirm, prompt, beforeunload)
- [ ] `--text` value is passed as `promptText` to CDP only when provided
- [ ] `cargo check` passes with no errors

### T005: Implement `execute_info` — query dialog state

**File(s)**: `src/dialog.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_info()` function that:
  - Sets up session via `setup_session()`
  - Enables `Page` and `Runtime` domains
  - Subscribes to `Page.javascriptDialogOpening`
  - Uses `Runtime.evaluate("0")` with a short timeout as a probe:
    - If evaluate succeeds → no dialog open → output `{"open": false}`
    - If evaluate times out or returns error → dialog is open → use event data for response
  - Builds `InfoResult` and outputs via `print_output()` or plain text formatter
- [ ] Returns `default_value` only for prompt dialogs
- [ ] `cargo check` passes with no errors

---

## Phase 3: Integration

### T006: Register dialog command in main dispatcher and add auto-dismiss support

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T004, T005
**Acceptance**:
- [ ] `mod dialog;` added to module declarations
- [ ] `Command::Dialog(args) => dialog::execute_dialog(&cli.global, args).await` in `run()` match arm
- [ ] `execute_dialog()` dispatcher function in `dialog.rs` that matches `DialogCommand::Handle` and `DialogCommand::Info`
- [ ] `--auto-dismiss-dialogs` integration: when the flag is set, the command sets up a background tokio task that subscribes to `Page.javascriptDialogOpening` and auto-dismisses with `Page.handleJavaScriptDialog(accept: false)`
- [ ] Auto-dismiss task is spawned before primary command execution and dropped after
- [ ] `cargo check` passes with no errors
- [ ] `cargo clippy` passes with no warnings

**Notes**: Auto-dismiss is scoped to commands that create a CDP session. For the `dialog` command group itself, auto-dismiss is not applicable (you wouldn't auto-dismiss while trying to manually handle a dialog). The auto-dismiss logic can be a helper function in `dialog.rs` or `connection.rs`.

---

## Phase 4: BDD Testing

### T007: Create BDD feature file

**File(s)**: `tests/features/dialog.feature` (create)
**Type**: Create
**Depends**: T006
**Acceptance**:
- [ ] All 11 acceptance criteria from requirements.md are scenarios
- [ ] Uses Given/When/Then format
- [ ] Includes happy paths (accept alert, dismiss confirm, prompt with text, beforeunload)
- [ ] Includes error case (no dialog open)
- [ ] Includes `dialog info` scenarios (open and closed)
- [ ] Includes auto-dismiss scenario
- [ ] Includes plain text output scenarios
- [ ] Feature file is valid Gherkin syntax

### T008: Implement step definitions and unit tests

**File(s)**: `tests/bdd.rs` (modify), `src/dialog.rs` (add unit tests)
**Type**: Modify
**Depends**: T007
**Acceptance**:
- [ ] Step definitions for all dialog scenarios in BDD test harness
- [ ] Unit tests for `HandleResult` and `InfoResult` serialization
- [ ] Unit tests for plain text formatting
- [ ] `cargo test --lib` passes
- [ ] `cargo test` passes (all tests including BDD)

---

## Dependency Graph

```
T001 (CLI args) ──┐
                   ├──▶ T003 (output types) ──▶ T004 (handle) ──┐
T002 (errors) ────┘                          ──▶ T005 (info) ───┤
                                                                  ├──▶ T006 (integration)
                                                                  │         │
                                                                  │         ▼
                                                                  └──▶ T007 (feature file)
                                                                             │
                                                                             ▼
                                                                        T008 (step defs + unit tests)
```

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
