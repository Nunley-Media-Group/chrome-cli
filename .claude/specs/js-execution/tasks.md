# Tasks: JavaScript Execution in Page Context

**Issue**: #13
**Date**: 2026-02-12
**Status**: Planning
**Author**: Claude (writing-specs)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 2 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **7** | |

---

## Phase 1: Setup

### T001: Add error helper constructors for JS execution

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AppError::js_execution_failed(description)` returns message `"JavaScript execution failed: {description}"` with `ExitCode::GeneralError`
- [ ] `AppError::script_file_not_found(path)` returns message `"Script file not found: {path}"` with `ExitCode::GeneralError`
- [ ] `AppError::script_file_read_failed(path, error)` returns message `"Failed to read script file: {path}: {error}"` with `ExitCode::GeneralError`
- [ ] `AppError::no_js_code()` returns message `"No JavaScript code provided. Specify code as argument, --file, or pipe via stdin."` with `ExitCode::GeneralError`
- [ ] Unit tests for all four constructors verify message content and exit code

**Notes**: Follow the existing pattern of `evaluation_failed()`, `snapshot_failed()`, etc. The existing `evaluation_failed()` is specific to `page text` ("Text extraction failed"); the new `js_execution_failed()` uses generic wording.

### T002: Add CLI argument types for `js exec`

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `JsArgs` struct with `#[command(subcommand)]` field of type `JsCommand`
- [ ] `JsCommand` enum with `Exec(JsExecArgs)` variant
- [ ] `JsExecArgs` struct with fields:
  - `code`: `Option<String>` — positional argument for inline JavaScript
  - `--file`: `Option<PathBuf>` — path to JavaScript file
  - `--uid`: `Option<String>` — element UID from snapshot
  - `--no-await`: `bool` flag — disable promise awaiting
  - `--timeout`: `Option<u64>` — execution timeout override in ms
  - `--max-size`: `Option<usize>` — maximum result size in bytes
- [ ] `code` and `--file` are mutually exclusive (via clap conflict)
- [ ] `Command::Js` variant changed from unit to `Js(JsArgs)`
- [ ] `cargo build` compiles without errors
- [ ] `chrome-cli js exec --help` shows all options and global flags

**Notes**: Follow the `PageArgs`/`PageCommand` and `PerfArgs`/`PerfCommand` patterns. The `--tab`, `--json`, `--pretty`, `--plain` flags are global and need no changes.

---

## Phase 2: Backend Implementation

### T003: Implement JavaScript execution command

**File(s)**: `src/js.rs` (new file)
**Type**: Create
**Depends**: T001, T002
**Acceptance**:
- [ ] `JsExecResult` struct with `result` (`serde_json::Value`), `type` (`String`), optional `console` (Vec of console entries), optional `truncated` (`bool`) — all with appropriate `skip_serializing_if`
- [ ] `JsExecError` struct with `error` (`String`), optional `stack` (`String`), `code` (`u8`) for structured error output on stderr
- [ ] `execute_js()` dispatches `JsCommand::Exec` to `execute_exec()`
- [ ] `execute_exec()` follows the session setup pattern from `page.rs`:
  - Resolves connection and target via `resolve_connection` / `resolve_target`
  - Creates `CdpClient`, `CdpSession`, `ManagedSession`
  - Enables `Runtime` domain via `ensure_domain`
- [ ] Code resolution:
  - `--file <PATH>`: reads file to string; returns `script_file_not_found` or `script_file_read_failed` on error
  - `code == "-"`: reads stdin to string via `std::io::read_to_string(std::io::stdin())`
  - `code == Some(expr)`: uses expression directly
  - Neither code nor --file: returns `no_js_code` error
- [ ] Without `--uid`: uses `Runtime.evaluate` with `returnByValue: true` and `awaitPromise: true` (unless `--no-await`)
- [ ] With `--uid`:
  - Reads snapshot state via `crate::snapshot::read_snapshot_state()`
  - Resolves UID to `backendNodeId` from `uid_map`
  - Enables `DOM` domain
  - Calls `DOM.resolveNode({ backendNodeId })` to get `objectId`
  - Calls `Runtime.callFunctionOn({ functionDeclaration, objectId, arguments: [{objectId}], returnByValue: true, awaitPromise: true })`
- [ ] Exception handling:
  - Checks `exceptionDetails` in CDP response
  - Extracts `exception.description` and `exception.stackTrace` (or `text` fallback)
  - Prints structured `JsExecError` JSON to stderr
  - Returns `js_execution_failed` error
- [ ] Result type extraction: maps CDP `result.type` and `result.subtype` to output `type` field
- [ ] Console capture:
  - Subscribes to `Runtime.consoleAPICalled` before execution
  - Collects console messages during execution
  - Includes in output as `console` array (omitted when empty)
- [ ] `--max-size` truncation:
  - Serializes result to JSON string
  - If byte length exceeds limit, truncates and sets `truncated: true`
- [ ] `print_output()` helper handles `--json` and `--pretty` (same pattern as `page.rs`)
- [ ] `--plain` mode prints only the raw result value to stdout (string values unquoted, others JSON-encoded)
- [ ] `cdp_config()` helper for timeout (same pattern as `page.rs`)
- [ ] Unit tests for `JsExecResult` serialization (JSON fields present, skip_serializing_if works)
- [ ] Unit tests for code resolution logic (file, stdin marker, inline, missing)

**Notes**: The `DOM.resolveNode` → `Runtime.callFunctionOn` flow is new to the codebase. The existing `resolve_uid_clip` in `page.rs` uses `DOM.describeNode` to get a `nodeId`, but for `callFunctionOn` we need an `objectId` from `DOM.resolveNode` instead.

### T004: Wire `js` command into main dispatcher

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `mod js;` declaration added
- [ ] `Command::Js(args)` match arm calls `js::execute_js(&cli.global, args).await`
- [ ] Previous `Err(AppError::not_implemented("js"))` is removed
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy` passes (all=deny, pedantic=warn)

---

## Phase 3: Integration

### T005: Verify end-to-end with cargo clippy and existing tests

**File(s)**: (all modified files)
**Type**: Verify
**Depends**: T004
**Acceptance**:
- [ ] `cargo clippy --all-targets -- -D warnings` passes with zero warnings
- [ ] `cargo fmt --check` passes
- [ ] `cargo test --lib` passes (all unit tests including new ones)
- [ ] `cargo build` succeeds
- [ ] `chrome-cli js exec --help` displays expected usage info
- [ ] `chrome-cli js --help` shows the `exec` subcommand

---

## Phase 4: Testing

### T006: Create BDD feature file for JavaScript execution

**File(s)**: `tests/features/js-execution.feature`
**Type**: Create
**Depends**: T004
**Acceptance**:
- [ ] All 16 acceptance criteria from `requirements.md` are Gherkin scenarios
- [ ] Uses `Background:` for shared Chrome setup
- [ ] Valid Gherkin syntax
- [ ] Scenarios are independent and declarative
- [ ] Includes data-driven scenario outline for JS return types

### T007: Implement BDD step definitions for JavaScript execution

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T006
**Acceptance**:
- [ ] Step definitions exist for all scenarios in `js-execution.feature`
- [ ] Steps follow existing cucumber-rs patterns from the project
- [ ] New steps are reusable where possible (e.g., "I run {string}" already exists)
- [ ] `cargo test --test bdd` compiles (tests may skip if no Chrome available)

---

## Dependency Graph

```
T001 ──┐
       ├──▶ T003 ──▶ T004 ──▶ T005
T002 ──┘                │
                        ├──▶ T006 ──▶ T007
                        │
                        └──▶ (done)
```

T001 and T002 can be done in parallel (no interdependency).
T006 and T007 can proceed once T004 is complete.
T005 is a verification gate before merging.

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] BDD test tasks included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
