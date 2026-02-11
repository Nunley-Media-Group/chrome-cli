# Tasks: CLI Skeleton with Clap Derive Macros

**Issue**: #3
**Date**: 2026-02-10
**Status**: Planning
**Author**: Claude (spec-driven)

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

### T001: Add clap, serde, and serde_json dependencies to Cargo.toml

**File(s)**: `Cargo.toml`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `clap` added to `[dependencies]` with `derive` and `env` features
- [ ] `serde` added to `[dependencies]` with `derive` feature
- [ ] `serde_json` added to `[dependencies]`
- [ ] `cargo check` passes with no errors
- [ ] Existing `[dev-dependencies]` remain unchanged

**Notes**: Use `clap = { version = "4", features = ["derive", "env"] }`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`.

### T002: Create error types and exit code enum

**File(s)**: `src/error.rs`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] `ExitCode` enum with variants: `Success = 0`, `GeneralError = 1`, `ConnectionError = 2`, `TargetError = 3`, `TimeoutError = 4`, `ProtocolError = 5`
- [ ] `ExitCode` derives `Debug`, `Clone`, `Copy` and has a `repr(u8)` attribute
- [ ] `AppError` struct with `message: String` and `code: ExitCode` fields
- [ ] `AppError::not_implemented(command: &str) -> Self` constructor
- [ ] `AppError::print_json_stderr(&self)` method that writes `{"error": "<message>", "code": <N>}` to stderr
- [ ] Serialization uses `serde_json` (not manual string formatting)
- [ ] `cargo clippy` passes with no warnings

---

## Phase 2: Backend Implementation

### T003: Create CLI module with clap derive structs

**File(s)**: `src/cli/mod.rs`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] `Cli` struct with `#[derive(Parser)]` and comprehensive `long_about` description
- [ ] `command` field with `#[command(subcommand)]` attribute
- [ ] `global` field with `#[command(flatten)]` attribute for `GlobalOpts`
- [ ] `term_width = 100` set in `#[command(...)]` attributes
- [ ] `GlobalOpts` struct with `#[derive(Args)]`:
  - `--port` (u16, default 9222, global)
  - `--host` (String, default "127.0.0.1", global)
  - `--ws-url` (Option<String>, global)
  - `--timeout` (Option<u64>, global)
  - `--tab` (Option<String>, global)
  - Flattened `OutputFormat`
- [ ] `OutputFormat` struct with `#[derive(Args)]` and `#[group(multiple = false)]`:
  - `--json` (bool, global)
  - `--pretty` (bool, global)
  - `--plain` (bool, global)
- [ ] `Command` enum with `#[derive(Subcommand)]` and 12 variants: Connect, Tabs, Navigate, Page, Dom, Js, Console, Network, Interact, Form, Emulate, Perf
- [ ] Each Command variant has a doc comment (short help) and `long_about` (detailed AI-friendly description)
- [ ] All help text is descriptive enough for an AI agent to understand each command's purpose
- [ ] `cargo clippy` passes with no warnings

### T004: Rewrite main.rs with CLI dispatch and error handling

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T002, T003
**Acceptance**:
- [ ] `mod cli;` and `mod error;` declarations
- [ ] `main()` calls `Cli::parse()`, dispatches to `run()`, handles errors
- [ ] `run()` matches on all 13 `Command` variants
- [ ] Each match arm returns `Err(AppError::not_implemented("<command-name>"))`
- [ ] On error: prints JSON to stderr via `AppError::print_json_stderr()` and exits with correct code
- [ ] `chrome-cli --help` produces comprehensive output
- [ ] `chrome-cli --version` prints name and version
- [ ] `chrome-cli <any-subcommand>` prints error JSON to stderr and exits with code 1
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt --check` passes

---

## Phase 3: Integration

### T005: Verify all CLI behaviors end-to-end

**File(s)**: None (manual verification)
**Type**: Verify
**Depends**: T004
**Acceptance**:
- [ ] `chrome-cli --help` lists all 13 subcommands with descriptions
- [ ] `chrome-cli --version` displays version
- [ ] `chrome-cli connect` outputs `{"error":"...","code":1}` to stderr with exit code 1
- [ ] `chrome-cli --port 9333 --host 192.168.1.100 tabs` parses without error (still exits 1 for stub)
- [ ] `chrome-cli --json --plain tabs` is rejected by clap with a conflict error
- [ ] `chrome-cli --ws-url ws://localhost:9222/devtools/browser/abc tabs` parses without error
- [ ] `chrome-cli --timeout 5000 tabs` parses without error
- [ ] `chrome-cli --tab abc123 js` parses without error
- [ ] `cargo build` produces binary < 10MB
- [ ] `cargo test --lib` passes (if any unit tests exist)

---

## Phase 4: Testing

### T006: Create BDD feature file for CLI skeleton

**File(s)**: `tests/features/cli-skeleton.feature`
**Type**: Create
**Depends**: T004
**Acceptance**:
- [ ] Feature file covers all 13 acceptance criteria from requirements.md
- [ ] Uses Given/When/Then format
- [ ] Includes scenarios for: help output, version flag, output format conflicts, default values, subcommand stubs, exit codes, stderr JSON errors, all 13 subcommands listed
- [ ] Valid Gherkin syntax
- [ ] Scenario Outline used for parameterized subcommand stub tests

### T007: Implement BDD step definitions for CLI skeleton

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T006
**Acceptance**:
- [ ] New `CliWorld` struct (or extend existing `WorkflowWorld`) with fields for command output, exit code, stderr
- [ ] Step definitions for running `chrome-cli` via `std::process::Command`
- [ ] Steps for asserting stdout content, stderr content, and exit codes
- [ ] Steps for asserting JSON structure in stderr
- [ ] All scenarios in `cli-skeleton.feature` pass
- [ ] `cargo test --test bdd` passes
- [ ] Existing `release-pipeline.feature` tests still pass

---

## Dependency Graph

```
T001 ──┬──▶ T002 ──┐
       │           ├──▶ T004 ──▶ T005
       └──▶ T003 ──┘       │
                            ├──▶ T006 ──▶ T007
                            │
```

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included (BDD feature + step definitions)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
