# Root Cause Analysis: Page snapshot drops all children under ignored AX nodes

**Issue**: #83
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude (writing-specs)

---

## Root Cause

Chrome's CDP `Accessibility.getFullAXTree` response includes intermediate **ignored** nodes — generic structural containers with `role: "none"` and `ignored: true` — that wrap visible content. On most real-world pages, the root `RootWebArea` node's direct children are one or more of these ignored wrapper nodes. The actual visible content (headings, paragraphs, links) lives as descendants of these ignored nodes.

The `build_subtree` function in `src/snapshot.rs` (lines 213–215) handles ignored nodes by immediately returning `None`:

```rust
if ax.ignored {
    return None;
}
```

Since `build_subtree` is called recursively via `filter_map`, returning `None` causes the ignored node **and its entire subtree** to be discarded. When the root's only child is an ignored node, the root ends up with zero children — the entire page content is lost.

This is the standard tree-walking pitfall for accessibility trees. The correct approach is to treat ignored nodes as **transparent** — skip rendering them as nodes, but recurse into their children and promote those children to the parent level.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/snapshot.rs` | 213–215 | The `if ax.ignored { return None; }` guard that discards ignored nodes and their subtrees |
| `src/snapshot.rs` | 241–256 | The children collection loop that calls `build_subtree` via `filter_map` — needs to handle the new "promoted children" return path |

### Triggering Conditions

- CDP returns an accessibility tree containing ignored intermediate nodes (this is the common case on virtually all real-world pages)
- The ignored node is an ancestor of visible content (not just a leaf ignored node)
- Specifically triggered when the root's direct children are all ignored, producing a completely empty snapshot

---

## Fix Strategy

### Approach

Change `build_subtree` so that when it encounters an ignored node, instead of returning `None`, it recurses into the ignored node's children and collects any non-`None` results. These promoted children are returned to the caller for flattening into the parent's children list.

The cleanest approach is to change the return type from `Option<SnapshotNode>` to `Vec<SnapshotNode>`:
- For a **normal** (non-ignored) node: return a `Vec` containing the single `SnapshotNode` (with its recursively-built children).
- For an **ignored** node: return a `Vec` of promoted children (0 or more) gathered by recursing into the ignored node's `child_ids`.

The caller (`build_tree` and the recursive children loop) collects results with `flat_map` instead of `filter_map`.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/snapshot.rs` — `build_subtree` | Change return type from `Option<SnapshotNode>` to `Vec<SnapshotNode>`. When `ax.ignored`, recurse into children and return promoted children as a flat `Vec`. When not ignored, return `vec![node]`. | Directly addresses the root cause: ignored nodes become transparent instead of opaque blockers. |
| `src/snapshot.rs` — `build_subtree` children loop | Change `.filter_map(\|cid\| build_subtree(...))` to `.flat_map(\|cid\| build_subtree(...))` so promoted children are flattened into the parent's children list. | Required to consume the new `Vec` return type. |
| `src/snapshot.rs` — `build_tree` | Update the call site that invokes `build_subtree` for the root node. Change from `.unwrap_or_else(...)` to handling the `Vec` return (take first element or construct fallback). | Required to consume the new `Vec` return type at the top-level call site. |
| `src/snapshot.rs` — unit tests | Update existing test `build_tree_filters_ignored_nodes` to verify that an ignored node's children are promoted rather than dropped. Add a new test for deeply nested ignored chains. | Validates the fix and prevents regression. |

### Blast Radius

- **Direct impact**: `build_subtree` function and its caller `build_tree` in `src/snapshot.rs`
- **Indirect impact**: The `SnapshotResult` returned by `build_tree` — downstream consumers (`page snapshot` command, `format_tree`, `to_json`) are unaffected because they consume the `SnapshotNode` tree, which has the same shape; the only difference is that previously-missing children now appear
- **Risk level**: Low — the change is contained within `build_subtree` and `build_tree`. The `SnapshotNode` struct and all output formatting are unchanged.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Sibling-level ignored nodes (no children) no longer filtered | Low | These nodes return an empty `Vec` from the ignored branch, so they produce zero entries — same effect as the current `None` return |
| UID numbering changes because more nodes are now visible | Medium | UIDs are assigned in depth-first order; adding previously-hidden nodes will shift UIDs. This is expected and correct — the current UIDs are wrong because they're based on an incomplete tree. Existing test `build_tree_deterministic_uid_order` will need tree data updated. |
| Node count / truncation behavior changes | Low | `node_count` is only incremented for non-ignored nodes, which is unchanged. Promoted children are counted when they're processed as normal nodes in recursive calls. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Keep `Option<SnapshotNode>` return type and add a separate promotion pass | Walk the tree after building to fix up parent-child relationships | More complex, two-pass approach, harder to reason about correctness |
| Return `Option<Vec<SnapshotNode>>` (None for lookup failure, Some(empty) for ignored leaf, Some(children) for ignored parent) | Preserves the "node not found" semantics of `?` operator | `Vec` already handles this: empty vec for not-found or ignored-leaf is equivalent, simpler API |

---

## Validation Checklist

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
