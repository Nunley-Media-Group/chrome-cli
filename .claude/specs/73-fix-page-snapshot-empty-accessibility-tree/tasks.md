# Tasks: page snapshot returns empty accessibility tree on real-world websites

**Issue**: #73
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix tree building to use `parentId` fallback when `childIds` are empty | [ ] |
| T002 | Add regression tests for `parentId`-based tree reconstruction | [ ] |
| T003 | Verify no regressions in existing snapshot tests | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/snapshot.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AxNode` struct includes `parent_id: Option<String>` field
- [ ] `parse_ax_nodes()` extracts `parentId` from CDP response JSON
- [ ] `build_tree()` detects when root node has empty `child_ids` despite total nodes > 1
- [ ] When detected, `build_tree()` builds a `parent_id → Vec<child_id>` map and injects computed `child_ids` into each `AxNode`
- [ ] After fallback injection, existing `build_subtree()` recursion produces a populated tree
- [ ] No changes to the `BuildResult` public API or `SnapshotNode` structure
- [ ] No unrelated changes included in the diff

**Notes**: Follow the fix strategy from design.md. The fallback should be a single O(n) pass after `parse_ax_nodes()` returns. Mutate `child_ids` in-place on the parsed `AxNode` vec before building the lookup map. Keep `build_subtree()` unchanged.

### T002: Add Regression Test

**File(s)**: `src/snapshot.rs` (inline test module), `tests/features/73-snapshot-empty-tree.feature`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Unit test provides CDP-like JSON nodes with `parentId` but empty `childIds` arrays
- [ ] Unit test verifies `build_tree()` produces a populated tree with correct hierarchy
- [ ] Unit test verifies UIDs are assigned to interactive elements in the fallback path
- [ ] Unit test verifies that when both `childIds` and `parentId` are present, `childIds` takes precedence (no double-linking)
- [ ] Gherkin scenario tagged `@regression` reproduces the original bug condition
- [ ] Step definitions implemented for the Gherkin scenario
- [ ] Test passes with the fix applied
- [ ] Test fails if the fallback logic is removed (confirms it catches the bug)

### T003: Verify No Regressions

**File(s)**: existing test files in `src/snapshot.rs`
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing `build_tree_*` unit tests pass unchanged
- [ ] All existing `format_text_*` tests pass unchanged
- [ ] All existing `search_tree_*` tests pass unchanged
- [ ] `truncation_large_tree` test passes unchanged
- [ ] `cargo test` passes with no failures
- [ ] `cargo clippy` passes with no new warnings

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
