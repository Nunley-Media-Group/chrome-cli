# Design: Compact Snapshot Mode for AI Agent Token Efficiency

**Issues**: #162
**Date**: 2026-03-16
**Status**: Draft
**Author**: Claude

---

## Overview

This feature adds a `--compact` flag to `page snapshot` and all `--include-snapshot` commands that filters the accessibility tree to only interactive and semantically meaningful elements. The implementation uses a post-processing tree-pruning approach: the full tree is built and UIDs are assigned first (preserving stable UID mapping), then a separate `compact_tree()` function prunes non-essential nodes.

The design follows the existing pattern where `snapshot.rs` owns all tree manipulation logic, command modules in `page/snapshot.rs`, `interact.rs`, and `form.rs` call into it, and `cli/mod.rs` defines the argument structs. The compact filter adds a single new public function to `snapshot.rs` and a boolean field to several CLI argument structs.

---

## Architecture

### Component Diagram

```
CLI Layer (cli/mod.rs)
    │
    │  PageSnapshotArgs { compact: bool, verbose, file }
    │  ClickArgs, TypeArgs, ... { include_snapshot, compact: bool }
    │
    ▼
Command Modules (page/snapshot.rs, interact.rs, form.rs)
    │
    │  1. Call build_tree() → BuildResult { root, uid_map }
    │  2. Write snapshot state (uid_map — always from full tree)
    │  3. If compact: call compact_tree(&root) → pruned SnapshotNode
    │  4. Format/serialize the (possibly compacted) root
    │
    ▼
snapshot.rs
    │
    │  build_tree()       — existing, unchanged
    │  compact_tree()     — NEW: prune non-essential nodes
    │  format_text()      — existing, unchanged
    │
    ▼
CDP Client → Chrome
```

### Data Flow

```
1. User runs `agentchrome page snapshot --compact`
2. CLI layer parses --compact flag into PageSnapshotArgs.compact = true
3. page/snapshot.rs calls build_tree(nodes, verbose) — full tree, UIDs assigned
4. Snapshot state (uid_map) is written — always from full tree
5. compact_tree(&build.root) prunes the tree:
   a. Walk tree depth-first
   b. Keep nodes that are interactive (have UID) OR are kept roles
   c. Recursively check if subtree has any interactive descendants
   d. Collapse nodes with no kept role and no interactive descendants
   e. Inline StaticText content into parent name when appropriate
6. format_text() or serde_json::to_value() operates on the pruned tree
7. Output emitted via existing output pipeline
```

---

## API / Interface Changes

### CLI Changes

#### `page snapshot` — new flag

| Flag | Type | Default | Purpose |
|------|------|---------|---------|
| `--compact` | bool | false | Filter tree to interactive and landmark elements only |

#### `--include-snapshot` commands — new flag

All commands that currently accept `--include-snapshot` gain a `--compact` flag:

| Command | Args Struct |
|---------|-------------|
| `interact click` | `ClickArgs` |
| `interact click-at` | `ClickAtArgs` |
| `interact hover` | `HoverArgs` |
| `interact drag` | `DragArgs` |
| `interact type` | `TypeArgs` |
| `interact key` | `KeyArgs` |
| `interact scroll` | `ScrollArgs` |
| `form fill` | `FormFillArgs` |
| `form fill-many` | `FormFillManyArgs` |
| `form clear` | `FormClearArgs` |
| `form upload` | `FormUploadArgs` |
| `form submit` | `FormSubmitArgs` |

The `--compact` flag is independent of `--include-snapshot` for these commands — it only takes effect when `--include-snapshot` is also set. No conflict annotation needed; when `--compact` is true but `--include-snapshot` is false, compact is silently ignored.

### Output Schema

No changes to the JSON schema. The `SnapshotNode` structure is identical — compact mode simply produces a tree with fewer nodes. The `role`, `name`, `uid`, `properties`, and `children` fields remain the same.

---

## Core Algorithm: `compact_tree()`

### New Public Function in `snapshot.rs`

```rust
pub fn compact_tree(root: &SnapshotNode) -> SnapshotNode
```

### Kept Roles

A new constant defines which non-interactive roles are preserved in compact mode:

```rust
const COMPACT_KEPT_ROLES: &[&str] = &[
    // Landmarks
    "banner",
    "complementary",
    "contentinfo",
    "form",
    "main",
    "navigation",
    "region",
    "search",
    // Structural
    "heading",
    "list",
    "listitem",
    "table",
    "row",
    "cell",
    "columnheader",
    "rowheader",
    // Document root
    "RootWebArea",
    "document",
];
```

Interactive roles (from existing `INTERACTIVE_ROLES`) are always kept when they have a UID.

### Excluded Roles

Nodes with these roles are always pruned (their children are promoted to the parent):

```rust
const COMPACT_EXCLUDED_ROLES: &[&str] = &[
    "InlineTextBox",
    "LineBreak",
];
```

### Pruning Logic

The algorithm walks the tree depth-first and applies three rules:

1. **Always exclude**: Nodes with roles in `COMPACT_EXCLUDED_ROLES` are removed. Their children are not promoted (InlineTextBox/LineBreak are leaf noise).

2. **Always keep**: Nodes that are interactive (have a UID) or have a role in `COMPACT_KEPT_ROLES` are kept.

3. **Conditional keep**: All other nodes (e.g., `generic`, `paragraph`, `StaticText`, `group`) are kept only if they have at least one interactive descendant (transitively). If kept, they serve as structural pass-through. If not kept, they are removed and their children are promoted to the nearest kept ancestor.

4. **Text inlining**: When a `StaticText` node is the only child of a kept node and the parent's name is empty, the `StaticText` name is inlined into the parent's name. This preserves readable labels without the extra tree level.

### Implementation Approach

```rust
fn compact_node(node: &SnapshotNode) -> Option<SnapshotNode> {
    // Rule 1: Always exclude
    if COMPACT_EXCLUDED_ROLES.contains(&node.role.as_str()) {
        return None;
    }

    // Recursively compact children
    let compacted_children: Vec<SnapshotNode> = node.children
        .iter()
        .filter_map(|child| compact_node(child))
        .collect();

    let is_interactive = node.uid.is_some();
    let is_kept_role = COMPACT_KEPT_ROLES.contains(&node.role.as_str());

    // Rule 2: Always keep interactive or kept-role nodes
    if is_interactive || is_kept_role {
        let mut result = node.clone_without_children();
        // Rule 4: Text inlining
        result.children = inline_text_children(result.name.is_empty(), compacted_children);
        if result.name.is_empty() {
            if let Some(text) = extract_sole_static_text(&result.children) {
                result.name = text;
                result.children.retain(|c| c.role != "StaticText");
            }
        }
        return Some(result);
    }

    // Rule 3: Conditional — keep if has interactive descendants, else promote children
    let has_interactive_descendant = compacted_children
        .iter()
        .any(|c| c.uid.is_some() || has_interactive_in_subtree(c));

    if has_interactive_descendant {
        let mut result = node.clone_without_children();
        result.children = compacted_children;
        return Some(result);
    }

    // No interactive descendants — this node is pruned, children promoted
    // But only promote children that themselves survived compaction
    // (most won't since they also lack interactive descendants)
    None
}
```

The top-level `compact_tree()` function calls `compact_node()` on the root, which is always kept (it's a `RootWebArea`/`document`).

### Pre-computation Optimization

To avoid O(n^2) repeated subtree scans for `has_interactive_descendant`, a single bottom-up pass first marks each node as "has interactive descendant" using a `HashSet<*const SnapshotNode>` or by traversing once and caching. However, given trees are typically < 10K nodes and the recursive check short-circuits on first find, the simpler recursive approach should meet the < 5ms target. If profiling shows otherwise, the pre-computation optimization can be added.

---

## Changes by File

| File | Change | Rationale |
|------|--------|-----------|
| `src/snapshot.rs` | Add `COMPACT_KEPT_ROLES`, `COMPACT_EXCLUDED_ROLES` constants; add `compact_tree()` public function and `compact_node()` helper; add unit tests | Core filtering logic |
| `src/cli/mod.rs` | Add `compact: bool` field to `PageSnapshotArgs`; add `compact: bool` to all 12 `*Args` structs that have `include_snapshot` | CLI flag definition |
| `src/page/snapshot.rs` | Call `compact_tree()` on `build.root` when `args.compact` is true, before formatting/serialization | Wire compact into page snapshot |
| `src/interact.rs` | Pass `compact` bool to `take_snapshot()`; apply `compact_tree()` to result when true | Wire compact into interact commands |
| `src/form.rs` | Pass `compact` bool to `take_snapshot()`; apply `compact_tree()` to result when true | Wire compact into form commands |

### Modification to `take_snapshot()` in `interact.rs` and `form.rs`

Both files have an identical `take_snapshot()` helper. The change adds a `compact: bool` parameter:

```rust
async fn take_snapshot(
    session: &mut ManagedSession,
    url: &str,
    compact: bool,  // NEW
) -> Result<serde_json::Value, AppError> {
    // ... existing code to build tree and write state ...

    // Apply compact filtering if requested
    let root = if compact {
        snapshot::compact_tree(&build_result.root)
    } else {
        build_result.root
    };

    let snapshot_json = serde_json::to_value(&root)
        .map_err(|e| AppError::snapshot_failed(&format!("failed to serialize snapshot: {e}")))?;

    Ok(snapshot_json)
}
```

All call sites pass `args.compact` (or `false` where not applicable).

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Filter during tree build** | Integrate compact filtering into `build_subtree()` | Single pass, slightly faster | Mixes concerns; UIDs would skip filtered nodes making them unstable; harder to test | Rejected — UID stability is critical |
| **B: Post-process filter (selected)** | Build full tree, assign UIDs, then prune | Clean separation; stable UIDs; easy to test; no changes to `build_tree()` | Extra tree traversal (< 5ms) | **Selected** |
| **C: Separate CDP query** | Use `Accessibility.getPartialAXTree` with role filters | Reduces CDP payload | CDP doesn't support role-based filtering; would need multiple queries; UID assignment would differ | Rejected — no CDP support |

---

## Performance Considerations

- [x] **Tree traversal overhead**: Compact filtering is O(n) where n = number of nodes. For max 10K nodes, this is < 5ms.
- [x] **Memory**: The compact tree is a new allocation but smaller than the original. The original tree is dropped after compaction.
- [x] **No caching needed**: Each snapshot is a point-in-time capture. No benefit to caching compact results.

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| `snapshot.rs` | Unit | `compact_tree()` with various tree shapes: empty, flat, nested, mixed roles |
| `snapshot.rs` | Unit | Text inlining: StaticText sole child → name absorbed |
| `snapshot.rs` | Unit | UID preservation: all UIDs from full tree present after compaction |
| `snapshot.rs` | Unit | Excluded roles (InlineTextBox, LineBreak) removed |
| `snapshot.rs` | Unit | Kept roles (heading, navigation, main, etc.) preserved |
| `snapshot.rs` | Unit | Generic nodes with interactive descendants preserved |
| `snapshot.rs` | Unit | Generic nodes without interactive descendants pruned |
| CLI | BDD | AC1–AC8 as Gherkin scenarios |
| Integration | BDD | Full CLI invocation with `--compact` flag |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Compact mode loses information an agent needs | Low | High | AC6 ensures all UIDs preserved; kept roles include all semantic elements |
| Performance regression on large trees | Low | Low | O(n) algorithm; unit test with 10K nodes confirms < 5ms |
| UID instability between compact/full modes | Low | High | UIDs assigned from full tree before compaction — by design |

---

## Open Questions

- None

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #162 | 2026-03-16 | Initial feature spec |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] Database/storage changes planned with migrations — N/A
- [x] State management approach is clear — snapshot state always from full tree
- [x] UI components and hierarchy defined — N/A (CLI tool)
- [x] Security considerations addressed — no new attack surface
- [x] Performance impact analyzed — O(n) traversal < 5ms
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
