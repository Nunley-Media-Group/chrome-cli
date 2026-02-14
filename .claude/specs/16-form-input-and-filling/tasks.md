# Tasks: Form Input and Filling

**Issue**: #16
**Date**: 2026-02-13
**Status**: Planning
**Author**: Claude (nmg-sdlc)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 1 | [ ] |
| Backend | 4 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **8** | |

---

## Phase 1: Setup

### T001: Add FormArgs CLI types to cli/mod.rs

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `FormArgs` struct with `#[command(subcommand)]` field
- [ ] `FormCommand` enum with `Fill`, `FillMany`, `Clear` variants
- [ ] `FormFillArgs` struct: `target: String`, `value: String`, `include_snapshot: bool`
- [ ] `FormFillManyArgs` struct: `json: Option<String>`, `file: Option<PathBuf>`, `include_snapshot: bool`; `json` and `file` have mutual requirement (one must be provided)
- [ ] `FormClearArgs` struct: `target: String`, `include_snapshot: bool`
- [ ] `Command::Form` variant updated from unit to `Form(FormArgs)`
- [ ] `cargo check` passes

**Notes**: Follow the same patterns as `InteractArgs`/`InteractCommand` and `DialogArgs`/`DialogCommand`. The `--tab` flag is already global so no per-command addition needed.

---

## Phase 2: Backend Implementation

### T002: Create form.rs module with session setup and output helpers

**File(s)**: `src/form.rs`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] Module declares output types: `FillResult { filled, value, snapshot? }`, `FillManyResult { results, snapshot? }`, `ClearResult { cleared, snapshot? }`
- [ ] `print_output` helper for JSON/pretty output (same pattern as interact.rs)
- [ ] Plain text print helpers for each result type
- [ ] `setup_session` and `cdp_config` helpers (same pattern as interact.rs)
- [ ] `take_snapshot` helper (same pattern as interact.rs)
- [ ] Target resolution helpers (`is_uid`, `is_css_selector`, `resolve_target_to_backend_node_id`) — duplicated from interact.rs
- [ ] `execute_form` public dispatcher function
- [ ] `cargo check` passes

### T003: Implement execute_fill command

**File(s)**: `src/form.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Resolves target (UID or CSS selector) to backend node ID
- [ ] Uses `DOM.resolveNode` to get Runtime object ID from backend node ID
- [ ] Calls `Runtime.callFunctionOn` with JS function that:
  - Detects element type (input/select/textarea/checkbox/radio)
  - Sets value appropriately for each type
  - Dispatches `input` and `change` events with `bubbles: true`
- [ ] Returns `FillResult` as JSON output
- [ ] Supports `--include-snapshot` flag
- [ ] Handles error cases: UID not found, element not found, element not fillable
- [ ] `cargo check` passes

### T004: Implement execute_fill_many command

**File(s)**: `src/form.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] Accepts inline JSON string argument: `[{"uid":"s1","value":"John"}, ...]`
- [ ] Accepts `--file <PATH>` to read JSON from file
- [ ] Validates JSON structure (array of objects with `uid` and `value` fields)
- [ ] Iterates over entries, calling the same fill logic as execute_fill
- [ ] Returns array of `FillResult` items (or wrapped with snapshot)
- [ ] Supports `--include-snapshot` (snapshot taken once after all fills)
- [ ] `cargo check` passes

### T005: Implement execute_clear command

**File(s)**: `src/form.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Resolves target to backend node ID
- [ ] Uses `Runtime.callFunctionOn` to set value to empty string (or unchecked for checkboxes)
- [ ] Dispatches `input` and `change` events
- [ ] Returns `ClearResult` as JSON output
- [ ] Supports `--include-snapshot` flag
- [ ] `cargo check` passes

---

## Phase 3: Integration

### T006: Wire form module into main.rs dispatch

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `mod form;` declaration added
- [ ] `Command::Form(args) => form::execute_form(&cli.global, args).await` replaces the `not_implemented` stub
- [ ] `cargo check` passes
- [ ] `cargo clippy` passes with no new warnings

---

## Phase 4: Testing

### T007: Create BDD feature file for form commands

**File(s)**: `tests/features/form.feature`
**Type**: Create
**Depends**: T003, T004, T005
**Acceptance**:
- [ ] All 15 acceptance criteria from requirements.md have corresponding scenarios
- [ ] Uses Given/When/Then format
- [ ] Includes error handling scenarios (invalid UID, missing args)
- [ ] Includes data-driven scenarios where appropriate
- [ ] Valid Gherkin syntax

### T008: Add unit tests to form.rs

**File(s)**: `src/form.rs`
**Type**: Modify
**Depends**: T003, T004, T005
**Acceptance**:
- [ ] `FillResult` serialization tests (with and without snapshot)
- [ ] `FillManyResult` serialization tests
- [ ] `ClearResult` serialization tests
- [ ] Target format validation tests (UID, CSS selector, invalid)
- [ ] JSON input parsing tests for fill-many
- [ ] `cargo test` passes

---

## Dependency Graph

```
T001 ──▶ T002 ──┬──▶ T003 ──┬──▶ T004
                │           │
                │           └──▶ T007, T008
                │
                ├──▶ T005 ──┘
                │
                └──▶ T006
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
