# Tasks: Shell Completions Generation

**Issue**: #25
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude (writing-specs)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 1 | [ ] |
| Backend | 3 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **7** | |

---

## Phase 1: Setup

### T001: Add clap_complete dependency

**File(s)**: `Cargo.toml`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `clap_complete` crate added to `[dependencies]` with version compatible with clap 4
- [ ] `cargo check` passes

---

## Phase 2: Backend Implementation

### T002: Add Completions variant to Command enum

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `Command::Completions(CompletionsArgs)` variant added to `Command` enum
- [ ] `CompletionsArgs` struct defined with a positional `shell` field of type `clap_complete::Shell`
- [ ] `#[command]` attributes include `long_about` with per-shell installation instructions
- [ ] `cargo check` passes with no clippy warnings

**Notes**:
- `clap_complete::Shell` already implements `ValueEnum`, so it works directly as a clap argument
- The `long_about` should include installation instructions for bash, zsh, fish, powershell, and elvish

### T003: Implement completions handler function

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `execute_completions()` function defined that takes `&CompletionsArgs`
- [ ] Uses `clap_complete::generate()` with `Cli::command()`, the shell, binary name `"chrome-cli"`, and `std::io::stdout()`
- [ ] Returns `Result<(), AppError>` (though it should always succeed)
- [ ] Function is synchronous — no async/CDP/Chrome needed

### T004: Wire Command::Completions in run() dispatch

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `Command::Completions(args) => execute_completions(args)` arm added to the `match` in `run()`
- [ ] The completions arm does NOT load config or establish a Chrome connection
- [ ] Running `chrome-cli completions bash` produces a non-empty completion script on stdout
- [ ] Running `chrome-cli completions zsh` produces a non-empty completion script on stdout
- [ ] Exit code is 0 for all valid shells

---

## Phase 3: Integration

### T005: Update CLI skeleton help to include completions subcommand

**File(s)**: `tests/features/cli-skeleton.feature`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] The "Top-level help displays comprehensive tool description" scenario includes `And stdout should contain "completions"`
- [ ] Existing BDD tests still pass

---

## Phase 4: BDD Testing

### T006: Create BDD feature file for shell completions

**File(s)**: `tests/features/shell-completions.feature`
**Type**: Create
**Depends**: T004
**Acceptance**:
- [ ] Feature file contains scenarios for all acceptance criteria
- [ ] Scenario Outline covers all 5 shells (bash, zsh, fish, powershell, elvish)
- [ ] Scenarios for subcommand content verification
- [ ] Scenario for invalid shell error
- [ ] Scenario for help text with installation instructions
- [ ] Valid Gherkin syntax

### T007: Add step definitions and wire feature in bdd.rs

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T006
**Acceptance**:
- [ ] `CliWorld::run("tests/features/shell-completions.feature")` or equivalent added to `main()`
- [ ] Existing step definitions in `CliWorld` are reused (Given chrome-cli is built / When I run / Then exit code / Then stdout should contain)
- [ ] New step definition added if needed for "stdout should not be empty" or similar
- [ ] All completions BDD scenarios pass: `cargo test --test bdd`

---

## Dependency Graph

```
T001 ──▶ T002 ──▶ T003 ──▶ T004 ──┬──▶ T005
                                    │
                                    └──▶ T006 ──▶ T007
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
