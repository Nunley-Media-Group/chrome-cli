# Tasks: Accessibility Tree Snapshot

**Issue**: #10
**Date**: 2026-02-12
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

### T001: Add error helper constructors for snapshot

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AppError::snapshot_failed(description)` returns message `"Accessibility tree capture failed: {description}"` with `ExitCode::GeneralError`
- [ ] `AppError::file_write_failed(path, error)` returns message `"Failed to write snapshot to file: {path}: {error}"` with `ExitCode::GeneralError`
- [ ] Unit tests for both constructors verify message content and exit code

**Notes**: Follow the existing pattern of `navigation_failed()`, `element_not_found()`, etc.

### T002: Add CLI argument types for `page snapshot`

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `PageSnapshotArgs` struct with `--verbose` (`bool`) and `--file` (`Option<PathBuf>`) arguments
- [ ] `Snapshot(PageSnapshotArgs)` variant added to `PageCommand` enum
- [ ] `cargo build` compiles without errors
- [ ] `chrome-cli page snapshot --help` shows the verbose and file options plus global flags

**Notes**: Follow the pattern of `PageTextArgs`. The `--json`, `--pretty`, `--plain`, `--tab` flags are already global.

---

## Phase 2: Backend Implementation

### T003: Implement snapshot module — types, tree building, UID assignment, formatting

**File(s)**: `src/snapshot.rs` (new file)
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] `AxNode` struct parses CDP `Accessibility.getFullAXTree` response nodes
- [ ] `SnapshotNode` struct with `role`, `name`, `uid` (Option), `properties` (Option), `children` — derives `Serialize`
- [ ] `build_tree(nodes: &[serde_json::Value]) -> (SnapshotNode, HashMap<String, i64>)`:
  - Reconstructs tree from flat CDP node list using `childIds`
  - Filters out ignored nodes (`ignored: true`)
  - Assigns sequential UIDs (`s1`, `s2`, ...) to interactive roles in depth-first order
  - Returns root `SnapshotNode` and uid-to-`backendDOMNodeId` mapping
  - Interactive roles: link, button, textbox, checkbox, radio, combobox, menuitem, tab, switch, slider, spinbutton, searchbox, option, treeitem
- [ ] `format_text(root: &SnapshotNode, verbose: bool) -> String`:
  - Produces hierarchical text: `- role "name" [uid]` with 2-space indentation
  - uid bracket only for nodes with a uid
  - Verbose mode appends `key=value` pairs after the brackets
- [ ] Truncation: if node count exceeds 10,000, stops building and appends truncation message
- [ ] `SnapshotState` struct with `url`, `timestamp`, `uid_map` — derives `Serialize`, `Deserialize`
- [ ] `write_snapshot_state(state: &SnapshotState) -> Result<(), AppError>`:
  - Writes to `~/.chrome-cli/snapshot.json`
  - Uses atomic write pattern (write tmp, rename)
  - Sets `0o600` permissions on Unix
- [ ] `read_snapshot_state() -> Result<Option<SnapshotState>, AppError>`:
  - Reads from `~/.chrome-cli/snapshot.json`
  - Returns `None` if file doesn't exist
- [ ] Unit tests:
  - `SnapshotNode` serialization (JSON fields, skip_serializing_if)
  - `build_tree` with sample CDP response: correct hierarchy, UIDs only on interactive elements
  - `build_tree` with ignored nodes: filtered out
  - `format_text`: indentation, uid brackets, verbose properties
  - `format_text`: empty tree (just document root)
  - UID assignment: deterministic order matches tree traversal
  - Snapshot state write/read round-trip
  - Truncation: large node list truncated at limit

**Notes**: This is the core module. Parse CDP `Value` nodes defensively — use `as_str().unwrap_or_default()` patterns for missing fields, consistent with existing code.

### T004: Implement execute_snapshot command handler

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T002, T003
**Acceptance**:
- [ ] `execute_snapshot()` follows the session setup pattern from `execute_text()`:
  - Resolves connection and target via existing `setup_session()`
  - Enables `Accessibility` domain via `ensure_domain`
- [ ] Sends `Accessibility.getFullAXTree` via `managed.send_command()`
- [ ] Calls `snapshot::build_tree()` to construct the tree and uid mapping
- [ ] Fetches page URL via existing `get_page_info()`
- [ ] Persists `SnapshotState` with uid map, URL, and timestamp
- [ ] Output routing:
  - `--json` or `--pretty` → serialize `SnapshotNode` tree as JSON
  - Default or `--plain` → `snapshot::format_text()` to produce text tree
  - `--file <PATH>` → write formatted output to file, no stdout
  - No `--file` → write to stdout
- [ ] Dispatched from `execute_page()` via `PageCommand::Snapshot(args) => execute_snapshot(global, args).await`
- [ ] Handles CDP errors with `AppError::snapshot_failed()`
- [ ] Handles file write errors with `AppError::file_write_failed()`

**Notes**: Import and use `snapshot` module functions. Follow the output pattern from `execute_text()` but with the text-default behavior (snapshot defaults to text, not JSON).

### T005: Wire snapshot module into binary

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `mod snapshot;` declaration added to `main.rs`
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy --all-targets -- -D warnings` passes

**Notes**: Simple one-line addition. The dispatch is already handled in `page.rs`.

---

## Phase 3: Integration

### T006: Verify end-to-end with cargo clippy and existing tests

**File(s)**: (all modified files)
**Type**: Verify
**Depends**: T005
**Acceptance**:
- [ ] `cargo clippy --all-targets -- -D warnings` passes with zero warnings
- [ ] `cargo fmt --check` passes
- [ ] `cargo test --lib` passes (all unit tests including new ones)
- [ ] `cargo build` succeeds
- [ ] `chrome-cli page snapshot --help` displays expected usage info
- [ ] `chrome-cli page snapshot --verbose --help` shows verbose flag
- [ ] `chrome-cli page snapshot --file /tmp/test.txt --help` shows file option

---

## Phase 4: Testing

### T007: Create BDD feature file for accessibility tree snapshot

**File(s)**: `tests/features/accessibility-tree-snapshot.feature`
**Type**: Create
**Depends**: T005
**Acceptance**:
- [ ] All 12 acceptance criteria from `requirements.md` are Gherkin scenarios
- [ ] Uses `Background:` for shared Chrome setup
- [ ] Valid Gherkin syntax
- [ ] Scenarios are independent and declarative

### T008: Implement BDD step definitions for accessibility tree snapshot

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T007
**Acceptance**:
- [ ] Step definitions exist for all scenarios in `accessibility-tree-snapshot.feature`
- [ ] Steps follow existing cucumber-rs patterns from the project
- [ ] `cargo test --test bdd` compiles (tests may skip if no Chrome available)

---

## Dependency Graph

```
T001 ──┐
       ├──▶ T003 ──┐
T002 ──┘            ├──▶ T004 ──▶ T005 ──▶ T006
                    │                │
                    │                ├──▶ T007 ──▶ T008
                    │                │
                    │                └──▶ (done)
                    │
                    └──▶ (unit tests in T003 run independently)
```

T001 and T002 can be done in parallel (no interdependency).
T003 depends on T001 (error types).
T004 depends on T002 (CLI args) and T003 (snapshot module).
T005 depends on T004 (wiring).
T006 is a verification gate.
T007 and T008 can proceed once T005 is complete.

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
