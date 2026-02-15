# Tasks: Fix page snapshot dropping children under ignored AX nodes

**Issue**: #83
**Date**: 2026-02-15
**Status**: Planning
**Author**: Claude (writing-specs)

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix `build_subtree` to promote ignored nodes' children | [ ] |
| T002 | Add regression tests for ignored node promotion | [ ] |
| T003 | Verify no regressions in existing tests | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/snapshot.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `build_subtree` return type changed from `Option<SnapshotNode>` to `Vec<SnapshotNode>`
- [ ] When a node is ignored, `build_subtree` recurses into its `child_ids` and returns promoted children as a flat `Vec`
- [ ] When a node is not ignored, `build_subtree` returns `vec![snapshot_node]`
- [ ] When a node is not found in lookup, `build_subtree` returns an empty `Vec`
- [ ] Children collection loop uses `.flat_map()` instead of `.filter_map()` to flatten promoted children
- [ ] `build_tree` call site updated to handle `Vec` return (take first element or construct fallback root)
- [ ] Ignored leaf nodes (no children) produce an empty `Vec` — same filtering effect as before
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo fmt --check` passes

**Notes**: Follow the fix strategy from design.md. The key insight is that `Vec<SnapshotNode>` naturally handles all cases: empty vec for not-found/ignored-leaf, single-element vec for normal nodes, multi-element vec for promoted children. Keep `node_count` increment only on non-ignored nodes (unchanged from current behavior).

### T002: Add Regression Test

**File(s)**: `src/snapshot.rs` (inline unit tests), `tests/features/83-fix-snapshot-ignored-nodes.feature`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] New unit test: ignored node with children promotes those children to parent
- [ ] New unit test: deeply nested ignored chain (3+ levels) promotes descendants to nearest non-ignored ancestor
- [ ] New unit test: ignored node's interactive children get UIDs assigned correctly
- [ ] Existing test `build_tree_filters_ignored_nodes` updated to verify promotion behavior (ignored sibling-level node with no children still produces no output; ignored ancestor-level node's children are promoted)
- [ ] Gherkin feature file created with `@regression` tag
- [ ] Step definitions implemented (or existing BDD step patterns reused)
- [ ] All new tests pass with the fix applied

### T003: Verify No Regressions

**File(s)**: Existing test files
**Type**: Verify (no file changes expected)
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo test --lib` passes (all unit tests)
- [ ] `cargo test --test bdd` passes (all BDD tests)
- [ ] `cargo clippy` passes
- [ ] No side effects in related code paths (snapshot formatting, UID mapping, truncation)

---

## Validation Checklist

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
