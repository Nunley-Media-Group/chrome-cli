# Tasks: Compact Snapshot Mode for AI Agent Token Efficiency

**Issues**: #162
**Date**: 2026-03-16
**Status**: Planning
**Author**: Claude

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 3 | [ ] |
| Integration | 2 | [ ] |
| Testing | 3 | [ ] |
| **Total** | **10** | |

---

## Phase 1: Setup

### T001: Add compact constants and `compact_tree()` function to `snapshot.rs`

**File(s)**: `src/snapshot.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `COMPACT_KEPT_ROLES` constant defined with: `banner`, `complementary`, `contentinfo`, `form`, `main`, `navigation`, `region`, `search`, `heading`, `list`, `listitem`, `table`, `row`, `cell`, `columnheader`, `rowheader`, `RootWebArea`, `document`
- [ ] `COMPACT_EXCLUDED_ROLES` constant defined with: `InlineTextBox`, `LineBreak`
- [ ] Public `compact_tree(root: &SnapshotNode) -> SnapshotNode` function implemented
- [ ] Private `compact_node(node: &SnapshotNode) -> Option<SnapshotNode>` helper implemented
- [ ] Private `has_interactive_in_subtree(node: &SnapshotNode) -> bool` helper implemented
- [ ] Pruning rules applied: always-exclude, always-keep (interactive OR kept role), conditional-keep (has interactive descendant)
- [ ] Text inlining: when a `StaticText` node is the sole child of a kept node with empty name, the text is absorbed into the parent name and the `StaticText` child removed
- [ ] `cargo clippy` passes with no new warnings

**Notes**: The function operates on the already-built `SnapshotNode` tree. It clones kept nodes and builds a new pruned tree. The root node is always kept (RootWebArea/document is in `COMPACT_KEPT_ROLES`).

### T002: Add `--compact` flag to CLI argument structs

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `PageSnapshotArgs` gains `pub compact: bool` field with `#[arg(long)]`
- [ ] Help text for `page snapshot` examples updated to include `--compact` usage
- [ ] All 12 `*Args` structs with `include_snapshot` gain `pub compact: bool` field with `#[arg(long)]`: `ClickArgs`, `ClickAtArgs`, `HoverArgs`, `DragArgs`, `TypeArgs`, `KeyArgs`, `ScrollArgs`, `FormFillArgs`, `FormFillManyArgs`, `FormClearArgs`, `FormUploadArgs`, `FormSubmitArgs`
- [ ] `cargo clippy` passes with no new warnings

**Notes**: The `--compact` flag on include-snapshot commands is independent — it silently does nothing when `--include-snapshot` is not also set. No `conflicts_with` or `requires` annotation needed.

---

## Phase 2: Backend Implementation

### T003: Wire `--compact` into `page/snapshot.rs`

**File(s)**: `src/page/snapshot.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] When `args.compact` is true, `compact_tree()` is called on `build.root` before formatting
- [ ] Compact filtering happens after `write_snapshot_state()` (UID map always from full tree)
- [ ] Plain text output path uses compacted tree when compact is true
- [ ] JSON output path uses compacted tree when compact is true
- [ ] `--file` output path uses compacted tree when compact is true
- [ ] Large-response gate summary uses compacted tree node count when compact is true
- [ ] `--verbose` combined with `--compact` works (properties on compacted nodes)
- [ ] Without `--compact`, behavior is identical to before (no code path changes for default)

**Notes**: The key ordering is: `build_tree()` → `write_snapshot_state()` → `compact_tree()` → format/emit.

### T004: Wire `--compact` into `interact.rs` and `form.rs` `take_snapshot()` helpers

**File(s)**: `src/interact.rs`, `src/form.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] `take_snapshot()` in `interact.rs` gains a `compact: bool` parameter
- [ ] `take_snapshot()` in `form.rs` gains a `compact: bool` parameter
- [ ] When `compact` is true, `compact_tree()` is applied to `build_result.root` before serialization
- [ ] Compact filtering happens after `write_snapshot_state()` in both files
- [ ] All call sites in `interact.rs` pass `args.compact` (7 commands: click, click-at, hover, drag, type, key, scroll)
- [ ] All call sites in `form.rs` pass `args.compact` (5 commands: fill, fill-many, clear, upload, submit)
- [ ] When `include_snapshot` is false, `take_snapshot()` is not called regardless of `compact` value

**Notes**: Both `take_snapshot()` functions are nearly identical. The change is the same in both: add parameter, conditionally apply `compact_tree()`.

### T005: Update built-in help documentation and examples

**File(s)**: `src/examples.rs`, `docs/claude-code.md`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `src/examples.rs` — add a compact snapshot example entry (e.g., `"agentchrome page snapshot --compact"` with description `"Compact snapshot — interactive and landmark elements only"`)
- [ ] `docs/claude-code.md` — "Efficiency Tips" section updated to mention `--compact` for reducing token consumption
- [ ] `docs/claude-code.md` — "Best Practices" section updated: recommend `--compact` for AI agents, mention `--include-snapshot --compact` combination
- [ ] `docs/claude-code.md` — "Recommended Workflow Loops" section updated: show `page snapshot --compact` in the snapshot-interact-snapshot loop
- [ ] `docs/claude-code.md` — at least one example conversation snippet uses `--compact`
- [ ] No existing examples broken or removed

**Notes**: The Claude Code integration guide is the primary documentation AI agents read. Recommending `--compact` there directly impacts adoption.

---

## Phase 3: Integration

### T006: Unit tests for `compact_tree()` in `snapshot.rs`

**File(s)**: `src/snapshot.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] Test: empty tree (document root only) returns document root with no children
- [ ] Test: tree with only interactive elements returns all of them unchanged
- [ ] Test: `InlineTextBox` and `LineBreak` nodes are removed from output
- [ ] Test: `heading`, `navigation`, `main`, `form`, `list` nodes are preserved
- [ ] Test: `generic` node with an interactive descendant is preserved
- [ ] Test: `generic` node with NO interactive descendants is pruned (children not promoted since they also have no interactive descendants)
- [ ] Test: `StaticText` sole child of a kept node with empty name is inlined into parent name
- [ ] Test: `StaticText` sole child of a kept node with non-empty name is NOT inlined (parent already has a name)
- [ ] Test: all UIDs from input tree appear in compacted output
- [ ] Test: hierarchy context preserved — interactive element inside `main > form` appears nested under both
- [ ] All tests pass with `cargo test --lib`

**Notes**: Build test trees manually using `SnapshotNode` struct literals, similar to existing `build_tree_*` tests.

### T007: BDD feature file and step definitions

**File(s)**: `tests/features/compact-snapshot-mode.feature`, `tests/bdd.rs`
**Type**: Create (feature file), Modify (bdd.rs)
**Depends**: T003, T004
**Acceptance**:
- [ ] Feature file contains scenarios for all 8 acceptance criteria (AC1–AC8)
- [ ] Step definitions added to `tests/bdd.rs` for compact snapshot scenarios
- [ ] Scenarios use `Given a Chrome session` / `When I run ... --compact` / `Then` patterns consistent with existing feature files
- [ ] `cargo test --test bdd` passes (Chrome-dependent scenarios may be skipped in CI)

**Notes**: Follow existing BDD patterns in `tests/bdd.rs`. Chrome-dependent scenarios are marked to skip when Chrome is unavailable.

---

## Phase 4: Testing & Verification

### T008: Manual smoke test against real Chrome

**File(s)**: N/A (manual verification)
**Type**: Verify
**Depends**: T003, T004, T005, T006, T007
**Acceptance**:
- [ ] `cargo build` succeeds
- [ ] `./target/debug/agentchrome connect --launch --headless` connects
- [ ] `./target/debug/agentchrome navigate https://www.saucedemo.com/` succeeds
- [ ] `./target/debug/agentchrome page snapshot` returns full tree (backward compat)
- [ ] `./target/debug/agentchrome page snapshot --compact` returns filtered tree
- [ ] Compact output is at least 50% smaller (line count) than full output
- [ ] All UIDs from full snapshot appear in compact snapshot
- [ ] `./target/debug/agentchrome page snapshot --compact --verbose` includes properties on kept nodes
- [ ] `./target/debug/agentchrome page snapshot --compact --json` returns valid JSON with filtered tree
- [ ] `./target/debug/agentchrome page snapshot --compact --pretty` returns pretty-printed filtered JSON
- [ ] Login to SauceDemo: `form fill` username/password, then `form submit` with `--include-snapshot --compact` returns compact snapshot
- [ ] `./target/debug/agentchrome connect disconnect` cleans up
- [ ] Kill any orphaned Chrome processes

### T009: SauceDemo smoke test

**File(s)**: N/A (manual verification)
**Type**: Verify
**Depends**: T008
**Acceptance**:
- [ ] Navigate to `https://www.saucedemo.com/`
- [ ] `page snapshot` returns full tree (baseline)
- [ ] `page snapshot --compact` returns compact tree
- [ ] Login with `form fill` + `form submit`
- [ ] Navigate to inventory page
- [ ] `page snapshot --compact` on inventory page shows all product links, buttons, sort combobox
- [ ] Compact output lacks InlineTextBox, LineBreak, decorative generic nodes
- [ ] Line count reduction is >= 50%

### T010: Verify no regressions

**File(s)**: N/A
**Type**: Verify
**Depends**: T003, T004, T006, T007
**Acceptance**:
- [ ] `cargo test --lib` passes (all unit tests)
- [ ] `cargo test --test bdd` passes (all BDD tests)
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt --check` passes
- [ ] Existing `page snapshot` behavior unchanged (no `--compact` = full tree)
- [ ] Existing `--include-snapshot` behavior unchanged (no `--compact` = full tree in response)

---

## Dependency Graph

```
T001 (compact_tree logic) ──┬──▶ T003 (page/snapshot.rs wiring) ──┐
                            │                                       │
                            ├──▶ T004 (interact/form wiring) ──────┤
                            │                                       │
                            └──▶ T006 (unit tests)                 │
                                                                    │
T002 (CLI flags) ──────────┬──▶ T003                               │
                           ├──▶ T004                               │
                           └──▶ T005 (docs/examples)               │
                                                                    │
                            T003, T004 ──▶ T007 (BDD tests)       │
                                                                    │
                  T003, T004, T005, T006, T007 ──▶ T008 (smoke test)
                                                          │
                                                          ▼
                                                    T009 (SauceDemo)
                                                          │
                                                          ▼
                                                    T010 (regressions)
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #162 | 2026-03-16 | Initial feature spec |

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included for each layer
- [x] No circular dependencies
- [x] Tasks are in logical execution order
