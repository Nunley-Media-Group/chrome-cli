# Root Cause Analysis: page snapshot returns empty accessibility tree on real-world websites

**Issue**: #73
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude

---

## Root Cause

The `page snapshot` command calls `Accessibility.getFullAXTree` via CDP (in `src/page.rs:250-253`) and passes the response to `build_tree()` in `src/snapshot.rs:120`. The `parse_ax_nodes()` function (line 48) extracts `childIds` from each node to build the parent-child relationships.

On complex, dynamically-rendered pages like google.com, Chrome's `Accessibility.getFullAXTree` returns nodes where the `childIds` arrays are empty or missing, even though the nodes themselves exist in the flat response array. The root node arrives with `childIds: []`, so `build_subtree()` has no children to recurse into, producing a tree containing only the root `RootWebArea` node.

The underlying CDP behavior is that `getFullAXTree` on large/dynamic pages may not populate `childIds` reliably. However, each node in the response still contains a `parentId` field pointing to its parent. The current implementation ignores `parentId` entirely, relying solely on `childIds` for tree construction. This is the root cause: a single-direction (top-down) tree-building strategy that fails when the top-down references (`childIds`) are absent, despite bottom-up references (`parentId`) being available.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/snapshot.rs` | 48-85 (`parse_ax_nodes`) | Parses CDP nodes; reads `childIds` but ignores `parentId` |
| `src/snapshot.rs` | 120-166 (`build_tree`) | Entry point for tree building; creates lookup from parsed nodes |
| `src/snapshot.rs` | 168-236 (`build_subtree`) | Recursive builder; iterates `child_ids` to find children — empty when `childIds` missing |
| `src/page.rs` | 250-253 | Calls `Accessibility.getFullAXTree` without parameters |

### Triggering Conditions

- The page is a real-world, dynamically-rendered site (google.com, etc.)
- Chrome's `Accessibility.getFullAXTree` returns nodes with empty/missing `childIds` arrays
- The `parentId` field is present on child nodes but not used by the parser
- Simple/static pages are unaffected because Chrome populates `childIds` correctly for them

---

## Fix Strategy

### Approach

Add `parentId` extraction to `parse_ax_nodes()` and implement a fallback parent-child resolution in `build_tree()`. When `childIds` are empty/missing across the response, reconstruct the tree by inverting `parentId` references into a `parent_id → Vec<child_id>` map. This is a minimal change that preserves the existing top-down tree-building logic while adding a bottom-up fallback.

The fix has two parts:

1. **Parse `parentId`**: In `parse_ax_nodes()`, extract the `parentId` field from each node (alongside the existing `childIds` extraction).

2. **Fallback child resolution in `build_tree()`**: After parsing, check if `childIds` are missing across the node set (e.g., root node has zero children despite many nodes existing). If so, build a `parent_id → Vec<child_id>` lookup from `parentId` fields and inject the computed `child_ids` into each `AxNode`. The existing `build_subtree()` then works unchanged.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/snapshot.rs` — `AxNode` struct (line 38-46) | Add `parent_id: Option<String>` field | Store the `parentId` from CDP response |
| `src/snapshot.rs` — `parse_ax_nodes()` (line 48-85) | Extract `n["parentId"]` into `parent_id` field | Capture bottom-up relationship data |
| `src/snapshot.rs` — `build_tree()` (line 120-166) | After parsing, detect empty `childIds` and rebuild them from `parentId` map | Fallback tree reconstruction when top-down refs are missing |
| `src/snapshot.rs` — tests (line 515+) | Add test for nodes with `parentId` but empty `childIds` | Prove the fallback works |

### Blast Radius

- **Direct impact**: `src/snapshot.rs` — `AxNode`, `parse_ax_nodes()`, `build_tree()`. These are internal functions; `build_tree()` is the only public entry point.
- **Indirect impact**: `src/page.rs` (`execute_snapshot`), `src/interact.rs`, `src/form.rs` — all consumers of `build_tree()`. Since the public API (`BuildResult`) doesn't change, callers are unaffected.
- **Risk level**: Low — the fallback only activates when `childIds` are missing, so existing behavior on well-formed responses is untouched.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Fallback incorrectly activates on pages with valid `childIds` | Low | Detection heuristic checks root node specifically — only triggers when root has zero `child_ids` but total node count > 1 |
| `parentId` field missing on some nodes | Low | Nodes without `parentId` remain unparented (same as current behavior); only nodes with valid `parentId` get linked |
| Tree structure differs between `childIds` and `parentId` resolution | Low | Both should produce identical trees since they encode the same relationship; unit test verifies equivalence |
| Performance impact from extra pass over nodes | Very Low | Single O(n) pass to build parent→children map; negligible compared to CDP round-trip |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Use `Accessibility.getPartialAXTree` with `fetchRelatives: true` | Fetch subtrees on-demand for specific nodes | Requires multiple CDP round-trips, significantly slower; doesn't solve the root issue of tree construction from flat data |
| Fall back to DOM-based tree traversal | Use `DOM.getDocument` + `querySelectorAll` when AX tree is empty | Much larger change; DOM tree != accessibility tree; would require mapping DOM nodes to AX roles manually |
| Pass parameters to `getFullAXTree` | Try `fetchRelatives` or `depth` params | `getFullAXTree` doesn't accept these params (they belong to `getPartialAXTree`); the data is already present, just not linked via `childIds` |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
