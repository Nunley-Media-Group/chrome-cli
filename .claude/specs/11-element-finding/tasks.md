# Tasks: Element Finding

**Issue**: #11
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

### T001: Add `PageFindArgs` and `Find` variant to CLI definitions

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `PageFindArgs` struct defined with: `query: Option<String>`, `--selector`, `--role`, `--exact`, `--limit` (default 10)
- [ ] `PageCommand::Find(PageFindArgs)` variant added to `PageCommand` enum
- [ ] `--selector` documented as alternative to text query
- [ ] `chrome-cli page find --help` displays correct usage
- [ ] Compiles without errors or clippy warnings

**Notes**: Follow the same patterns as `PageTextArgs` and `PageSnapshotArgs`. The `query` argument is positional and optional (required only when `--selector` is not provided). Validation of "at least one of query/selector" happens at runtime in the executor, not in clap.

### T002: Define output types for find results

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `FindMatch` struct defined with fields: `uid: Option<String>`, `role: String`, `name: String`, `bounding_box: Option<BoundingBox>`
- [ ] `BoundingBox` struct defined with fields: `x: f64`, `y: f64`, `width: f64`, `height: f64`
- [ ] Both structs derive `Debug`, `Serialize`
- [ ] `bounding_box` serializes as `boundingBox` (camelCase) via `#[serde(rename)]`
- [ ] `bounding_box` is skipped when `None` via `#[serde(skip_serializing_if)]`
- [ ] Unit tests for serialization (with and without bounding box, with and without uid)

---

## Phase 2: Backend Implementation

### T003: Implement accessibility tree search logic

**File(s)**: `src/snapshot.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] New public function `search_tree(root, query, role_filter, exact, limit) -> Vec<SearchHit>` added
- [ ] `SearchHit` struct with: `uid: Option<String>`, `role: String`, `name: String`, `backend_dom_node_id: Option<i64>`
- [ ] Substring match: case-insensitive contains on node name (when `exact` is false)
- [ ] Exact match: case-sensitive equality on node name (when `exact` is true)
- [ ] Role filter: only includes nodes matching the specified role string
- [ ] Results in depth-first (document) order
- [ ] Stops collecting after `limit` matches
- [ ] Unit tests for: substring match, exact match, role filter, combined role+text, limit, no matches, empty tree

**Notes**: Walk the `SnapshotNode` tree depth-first. Also pass the `uid_map` (from `BuildResult`) to resolve `backend_dom_node_id` for each matched node. Alternatively, the search can return uid strings and the caller resolves backend IDs from the map.

### T004: Implement CSS selector search via CDP DOM methods

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Helper function `find_by_selector(managed, selector, limit)` that:
  1. Enables DOM domain
  2. Calls `DOM.getDocument` to get root `nodeId`
  3. Calls `DOM.querySelectorAll` with the CSS selector
  4. For each returned `nodeId` (up to `limit`):
     - Calls `DOM.describeNode` to get `backendDOMNodeId`
     - Calls `Accessibility.getPartialAXTree` with `backendDOMNodeId` to get role/name
     - Calls `DOM.getBoxModel` for bounding box (returns `None` on error)
  5. Returns `Vec<FindMatch>`
- [ ] Invalid CSS selector results in a CDP protocol error which propagates as `AppError`
- [ ] Invisible elements (getBoxModel fails) get `None` bounding box

### T005: Implement bounding box resolution for accessibility search path

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] Helper function `resolve_bounding_box(managed, backend_dom_node_id) -> Option<BoundingBox>` that:
  1. Enables DOM domain
  2. Calls `DOM.describeNode` with `backendNodeId` param to get `nodeId`
  3. Calls `DOM.getBoxModel` with that `nodeId`
  4. Extracts content quad from box model → computes x, y, width, height
  5. Returns `None` if any step fails (element invisible, removed, etc.)
- [ ] Does not error on failure — returns `None` gracefully

---

## Phase 3: Integration

### T006: Wire up `execute_find()` dispatcher and output

**File(s)**: `src/page.rs`, `src/main.rs` (if needed)
**Type**: Modify
**Depends**: T003, T004, T005
**Acceptance**:
- [ ] `execute_find(global, args)` function implemented:
  1. Validates that at least one of `query` or `selector` is provided (returns `AppError` if neither)
  2. Sets up CDP session via existing `setup_session()`
  3. If `--selector` provided: calls CSS selector path (T004) — also triggers full snapshot for UID assignment
  4. If text query provided: captures snapshot via `Accessibility.getFullAXTree`, builds tree, searches (T003), resolves bounding boxes (T005)
  5. Persists snapshot state (uid_map) to `~/.chrome-cli/snapshot.json`
  6. Outputs results as JSON array (default) or plain text (`--plain`)
- [ ] `PageCommand::Find` arm added to `execute_page()` dispatcher
- [ ] JSON output: `Vec<FindMatch>` serialized directly (compact or pretty based on flags)
- [ ] Plain text output: `[uid] role "name" (x,y widthxheight)` format, one per line
- [ ] Empty results → empty JSON array `[]`, exit code 0
- [ ] `--limit` respected in both search paths

---

## Phase 4: Testing

### T007: Create BDD feature file

**File(s)**: `tests/features/page_find.feature`
**Type**: Create
**Depends**: T006
**Acceptance**:
- [ ] All 12 acceptance criteria from requirements.md are Gherkin scenarios
- [ ] Uses Given/When/Then format
- [ ] Background includes "Chrome is connected with a page loaded"
- [ ] Includes error scenarios (no matches, invalid selector)
- [ ] Feature file is valid Gherkin syntax

### T008: Add unit tests for search and output logic

**File(s)**: `src/snapshot.rs`, `src/page.rs`
**Type**: Modify
**Depends**: T003, T006
**Acceptance**:
- [ ] `snapshot.rs` tests: `search_tree` with various inputs (substring, exact, role, combined, limit, empty)
- [ ] `page.rs` tests: `FindMatch` and `BoundingBox` serialization, plain text formatting
- [ ] All tests pass with `cargo test`

---

## Dependency Graph

```
T001 (CLI args) ──┐
                   ├──▶ T003 (AX search) ──┐
T002 (types)   ──┤                          ├──▶ T006 (wire up) ──▶ T007 (BDD)
                   ├──▶ T004 (CSS search) ──┤                        T008 (unit tests)
                   └──▶ T005 (bounding box)─┘
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
