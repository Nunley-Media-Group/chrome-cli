# Tasks: Man Page Generation

**Issue**: #27
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude (writing-specs)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 3 | [ ] |
| Backend | 2 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **8** | |

---

## Phase 1: Setup

### T001: Add clap_mangen dependency to Cargo.toml

**File(s)**: `Cargo.toml`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `clap_mangen` added to `[dependencies]` (needed for runtime `chrome-cli man`)
- [ ] Version is compatible with clap 4
- [ ] `cargo check` passes

**Notes**: `clap_mangen` must be a regular dependency (not dev-dependency) since the `chrome-cli man` subcommand uses it at runtime.

### T002: Create xtask workspace member

**File(s)**: `Cargo.toml`, `xtask/Cargo.toml`, `xtask/src/main.rs`, `.cargo/config.toml`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] `xtask/` directory created with its own `Cargo.toml`
- [ ] `xtask` added to workspace members in root `Cargo.toml`
- [ ] `xtask/Cargo.toml` depends on `clap_mangen` and `clap` (for `Cli::command()` access)
- [ ] `.cargo/config.toml` defines alias: `xtask = "run --package xtask --"`
- [ ] `cargo xtask --help` runs successfully
- [ ] `xtask/src/main.rs` has placeholder structure with `man` subcommand

### T003: Add Man subcommand to CLI definition

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `ManArgs` struct defined with optional `command: Option<String>` field
- [ ] `Command::Man(ManArgs)` variant added to the `Command` enum
- [ ] `about`, `long_about`, and `after_long_help` attributes with usage examples
- [ ] `cargo check` passes
- [ ] `chrome-cli man --help` shows usage information

---

## Phase 2: Backend Implementation

### T004: Implement `execute_man()` handler in main.rs

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T001, T003
**Acceptance**:
- [ ] `Command::Man` match arm calls `execute_man(args)`
- [ ] `execute_man()` calls `Cli::command()` to get the clap Command builder
- [ ] Without argument: renders the top-level man page to stdout
- [ ] With argument: finds the named subcommand and renders its man page
- [ ] Invalid subcommand name returns appropriate error
- [ ] Uses `clap_mangen::Man::new(cmd).render(&mut stdout)`
- [ ] Exit code 0 on success, non-zero on error

**Notes**: Follow the same pattern as `execute_completions()` — synchronous, no async, no Chrome connection.

### T005: Implement xtask man subcommand

**File(s)**: `xtask/src/main.rs`
**Type**: Create
**Depends**: T002
**Acceptance**:
- [ ] `cargo xtask man` generates man pages for all commands
- [ ] Recursively walks the command tree (top-level + all nested subcommands)
- [ ] Writes `.1` files to `man/` directory (creates it if needed)
- [ ] File naming: `chrome-cli.1`, `chrome-cli-connect.1`, `chrome-cli-tabs-list.1`, etc.
- [ ] Prints a summary of generated files to stdout
- [ ] Exit code 0 on success

**Notes**: The xtask needs to import `chrome-cli`'s `Cli` struct. Since `chrome-cli` is a binary crate, the xtask should use `Cli::command()` from the library export in `lib.rs`. May need to re-export `Cli::command()` from `lib.rs`.

---

## Phase 3: Integration

### T006: Wire up lib.rs export and gitignore

**File(s)**: `src/lib.rs`, `.gitignore`
**Type**: Modify
**Depends**: T004, T005
**Acceptance**:
- [ ] `src/lib.rs` re-exports `cli::Cli` (or a function returning `Command`) so xtask can access it
- [ ] `.gitignore` includes `man/` directory (generated files not tracked)
- [ ] `cargo xtask man` successfully generates man pages to `man/`
- [ ] `chrome-cli man` successfully renders man pages to stdout

---

## Phase 4: BDD Testing

### T007: Create BDD feature file for man page generation

**File(s)**: `tests/features/man-page-generation.feature`
**Type**: Create
**Depends**: T004
**Acceptance**:
- [ ] All acceptance criteria from requirements.md have corresponding scenarios
- [ ] Uses Given/When/Then format
- [ ] Includes happy path (top-level and subcommand man pages)
- [ ] Includes error case (invalid subcommand)
- [ ] Includes help text scenario
- [ ] Feature file is valid Gherkin syntax

### T008: Add BDD step definitions for man page tests

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T007
**Acceptance**:
- [ ] Any new step definitions needed for man page scenarios are added
- [ ] Existing steps (e.g., "I run", "stdout should contain", "exit code should be") are reused where possible
- [ ] All scenarios in `man-page-generation.feature` pass
- [ ] `cargo test --test bdd` passes with no regressions

---

## Dependency Graph

```
T001 (clap_mangen dep) ──┬──▶ T004 (execute_man handler) ──┐
                         │                                   │
T003 (ManArgs CLI def) ──┘                                   ├──▶ T006 (lib.rs + gitignore)
                                                             │
T002 (xtask workspace) ──────▶ T005 (xtask man cmd) ────────┘
                                                             │
                                                             ▼
                                                    T007 (feature file)
                                                             │
                                                             ▼
                                                    T008 (step definitions)
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
