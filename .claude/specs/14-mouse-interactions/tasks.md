# Tasks: Mouse Interactions

**Issue**: #14
**Date**: 2026-02-13
**Status**: Planning
**Author**: Claude (writing-specs)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 4 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **9** | |

---

## Phase 1: Setup

### T001: Define CLI argument types for interact commands

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `InteractArgs` struct with `#[command(subcommand)]` field
- [ ] `InteractCommand` enum with `Click(ClickArgs)`, `ClickAt(ClickAtArgs)`, `Hover(HoverArgs)`, `Drag(DragArgs)` variants
- [ ] `ClickArgs` struct with:
  - `target: String` positional arg (UID or css: selector)
  - `--double` bool flag for double-click
  - `--right` bool flag for right-click
  - `--include-snapshot` bool flag
- [ ] `ClickAtArgs` struct with:
  - `x: f64` positional arg
  - `y: f64` positional arg
  - `--double` bool flag
  - `--right` bool flag
  - `--include-snapshot` bool flag
- [ ] `HoverArgs` struct with:
  - `target: String` positional arg
  - `--include-snapshot` bool flag
- [ ] `DragArgs` struct with:
  - `from: String` positional arg
  - `to: String` positional arg
  - `--include-snapshot` bool flag
- [ ] `Command::Interact` variant changed from unit to `Interact(InteractArgs)`
- [ ] `cargo check` passes with no errors

**Notes**: Follow the exact pattern used by `DialogArgs`/`DialogCommand`. The `Interact` variant currently exists as a unit variant — change it to hold `InteractArgs`. The `--double` and `--right` flags are mutually exclusive (`#[group(multiple = false)]` or `conflicts_with`).

### T002: Add error constructors for interact errors

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AppError::no_snapshot_state()` constructor returning `ExitCode::GeneralError` with message "No snapshot state found. Run 'chrome-cli page snapshot' first to assign UIDs to interactive elements."
- [ ] `AppError::element_zero_size(target: &str)` constructor returning `ExitCode::GeneralError` with message "Element '{target}' has zero-size bounding box and cannot be clicked."
- [ ] `AppError::interaction_failed(action: &str, reason: &str)` constructor returning `ExitCode::ProtocolError`
- [ ] `AppError::stale_uid(uid: &str)` constructor returning `ExitCode::GeneralError` with message "UID '{uid}' refers to an element that no longer exists. Run 'chrome-cli page snapshot' to refresh."
- [ ] `cargo check` passes with no errors

---

## Phase 2: Backend Implementation

### T003: Implement target resolution helpers

**File(s)**: `src/interact.rs` (create)
**Type**: Create
**Depends**: T001, T002
**Acceptance**:
- [ ] `is_uid(target: &str) -> bool` — returns true if target matches UID pattern (`s` + digits)
- [ ] `is_css_selector(target: &str) -> bool` — returns true if target starts with `css:`
- [ ] `resolve_target_to_backend_node_id(session, target) -> Result<i64>` — resolves UID via snapshot state or CSS selector via `DOM.querySelector` + `DOM.describeNode`
- [ ] `get_element_center(session, backend_node_id) -> Result<(f64, f64)>` — calls `DOM.getBoxModel` and computes center from content quad
- [ ] `scroll_into_view(session, backend_node_id) -> Result<()>` — calls `DOM.scrollIntoViewIfNeeded`
- [ ] `resolve_target_coords(session, target) -> Result<(f64, f64)>` — high-level function that resolves target → scroll into view → get center coordinates
- [ ] For UID targets: reads `snapshot.rs::read_snapshot_state()`, looks up backendDOMNodeId, calls `DOM.describeNode` to get nodeId
- [ ] For CSS targets: strips `css:` prefix, calls `DOM.querySelector` on document root, handles not-found
- [ ] `cargo check` passes with no errors

**Notes**: `DOM.querySelector` requires a `nodeId` for the parent — use `DOM.getDocument` to get the root nodeId first. `DOM.getBoxModel` returns `content` as `[x1,y1,x2,y2,x3,y3,x4,y4]` — compute center as `((x1+x3)/2, (y1+y3)/2)`. `DOM.scrollIntoViewIfNeeded` takes `backendNodeId` directly.

### T004: Implement mouse dispatch helpers

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `dispatch_click(session, x, y, button, click_count) -> Result<()>` — sends `mousePressed` + `mouseReleased` via `Input.dispatchMouseEvent`
  - `button` is `"left"` (value 0) or `"right"` (value 2)
  - `clickCount` is 1 (normal) or 2 (double)
  - For double-click: send two mousePressed+mouseReleased pairs (clickCount 1 then clickCount 2)
- [ ] `dispatch_hover(session, x, y) -> Result<()>` — sends `mouseMoved` via `Input.dispatchMouseEvent`
- [ ] `dispatch_drag(session, from_x, from_y, to_x, to_y) -> Result<()>` — sends:
  - `mousePressed` at (from_x, from_y)
  - `mouseMoved` to (to_x, to_y)
  - `mouseReleased` at (to_x, to_y)
- [ ] `cargo check` passes with no errors

**Notes**: `Input.dispatchMouseEvent` does not require `Input.enable`. The `type` parameter values are: `mousePressed`, `mouseReleased`, `mouseMoved`. For right-click, use `button: "right"`. For double-click, Chrome expects: press(count=1) → release(count=1) → press(count=2) → release(count=2).

### T005: Implement interact command functions — click, click-at, hover, drag

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: T003, T004
**Acceptance**:
- [ ] Output structs (all `#[derive(Serialize)]`):
  - `ClickResult` with fields: `clicked`, `url`, `navigated`, optional `double_click`, `right_click`, `snapshot`
  - `ClickAtResult` with fields: `clicked_at: Coords { x, y }`, optional `double_click`, `right_click`, `snapshot`
  - `HoverResult` with fields: `hovered`, optional `snapshot`
  - `DragResult` with fields: `dragged: DragTargets { from, to }`, optional `snapshot`
- [ ] `execute_click(global, args) -> Result<()>`:
  - Sets up session, enables DOM + Page domains
  - Resolves target coords via `resolve_target_coords()`
  - Subscribes to `Page.frameNavigated` for navigation detection
  - Dispatches click via `dispatch_click()`
  - Waits briefly (~100ms) for potential navigation event
  - Gets current URL via `Runtime.evaluate("window.location.href")`
  - If `--include-snapshot`: takes new snapshot, writes to snapshot state
  - Builds and outputs `ClickResult`
- [ ] `execute_click_at(global, args) -> Result<()>`:
  - Sets up session
  - Dispatches click directly at (x, y) coordinates
  - If `--include-snapshot`: takes new snapshot
  - Builds and outputs `ClickAtResult`
- [ ] `execute_hover(global, args) -> Result<()>`:
  - Sets up session, enables DOM domain
  - Resolves target coords
  - Dispatches hover via `dispatch_hover()`
  - If `--include-snapshot`: takes new snapshot
  - Builds and outputs `HoverResult`
- [ ] `execute_drag(global, args) -> Result<()>`:
  - Sets up session, enables DOM domain
  - Resolves "from" target coords
  - Resolves "to" target coords
  - Scrolls "from" element into view
  - Dispatches drag via `dispatch_drag()`
  - If `--include-snapshot`: takes new snapshot
  - Builds and outputs `DragResult`
- [ ] `print_output()` helper for JSON/pretty formatting (same pattern as dialog.rs)
- [ ] Plain text formatters for all four result types
- [ ] `execute_interact()` dispatcher that matches `InteractCommand` variants
- [ ] `cargo check` passes with no errors

### T006: Implement snapshot refresh for --include-snapshot

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] `take_snapshot(session) -> Result<serde_json::Value>` helper that:
  - Enables `Accessibility` domain
  - Calls `Accessibility.getFullAXTree`
  - Builds tree via `snapshot::build_tree()`
  - Writes updated snapshot state via `snapshot::write_snapshot_state()`
  - Returns the root node as `serde_json::Value`
- [ ] All four command functions include snapshot in output when `--include-snapshot` is set
- [ ] Updated snapshot state file is written so subsequent UID lookups use fresh data
- [ ] `cargo check` passes with no errors

---

## Phase 3: Integration

### T007: Register interact commands in main dispatcher

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] `mod interact;` added to module declarations
- [ ] `Command::Interact(args) => interact::execute_interact(&cli.global, args).await` replaces the `Err(AppError::not_implemented("interact"))` arm
- [ ] `cargo check` passes with no errors
- [ ] `cargo clippy` passes with no warnings

---

## Phase 4: BDD Testing

### T008: Create BDD feature file

**File(s)**: `tests/features/interact.feature` (create)
**Type**: Create
**Depends**: T007
**Acceptance**:
- [ ] All 22 acceptance criteria from requirements.md are scenarios
- [ ] Uses Given/When/Then format
- [ ] Includes happy paths (click by UID, click by CSS, click-at, hover, drag)
- [ ] Includes modifier scenarios (double-click, right-click, include-snapshot)
- [ ] Includes error cases (UID not found, CSS not found, no snapshot state)
- [ ] Includes navigation detection scenario
- [ ] Includes scroll-into-view scenario
- [ ] Includes plain text output scenarios
- [ ] Feature file is valid Gherkin syntax

### T009: Implement step definitions and unit tests

**File(s)**: `tests/bdd.rs` (modify), `src/interact.rs` (add unit tests)
**Type**: Modify
**Depends**: T008
**Acceptance**:
- [ ] Step definitions for all interact scenarios in BDD test harness
- [ ] Unit tests for `is_uid()` and `is_css_selector()` helper functions
- [ ] Unit tests for `ClickResult`, `ClickAtResult`, `HoverResult`, `DragResult` serialization
- [ ] Unit tests for plain text formatting of all result types
- [ ] `cargo test --lib` passes
- [ ] `cargo test` passes (all tests including BDD)

---

## Dependency Graph

```
T001 (CLI args) ──┐
                   ├──▶ T003 (target resolution) ──▶ T004 (mouse dispatch)
T002 (errors) ────┘                                          │
                                                              ▼
                                               T005 (command functions) ──▶ T006 (snapshot refresh)
                                                              │
                                                              ▼
                                                    T007 (integration)
                                                              │
                                                              ▼
                                                    T008 (feature file)
                                                              │
                                                              ▼
                                                    T009 (step defs + unit tests)
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
