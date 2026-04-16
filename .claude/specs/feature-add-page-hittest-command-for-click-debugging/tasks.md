# Tasks: Page Hit Test Command for Click Debugging

**Issues**: #191
**Date**: 2026-04-16
**Status**: Planning
**Author**: Rich Nunley

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 3 | [ ] |
| Frontend | 0 | [ ] |
| Integration | 3 | [ ] |
| Testing | 3 | [ ] |
| **Total** | **11** | |

---

## Phase 1: Setup

### T001: Define CLI argument types for page hittest

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `PageHitTestArgs` struct defined with positional `x: u32` and `y: u32` fields
- [ ] `PageCommand::HitTest(PageHitTestArgs)` variant added to enum
- [ ] Subcommand has `long_about` and `after_long_help` with usage examples
- [ ] `--help` text describes both positional arguments
- [ ] No parameter name collides with existing global flags

**Notes**: Follow the pattern of `PageElementArgs` and other `Page*Args` structs. The `--frame` argument is already inherited from `PageArgs`.

### T002: Define output types for page hittest

**File(s)**: `src/page/hittest.rs` (new file)
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `HitTestResult` struct with fields: `frame` (String), `hit_target` (ElementInfo), `intercepted_by` (Option<ElementInfo>), `stack` (Vec<StackElement>), `suggestion` (Option<String>)
- [ ] `ElementInfo` struct with fields: `tag` (String), `id` (Option<String>), `class` (Option<String>), `uid` (Option<String>)
- [ ] `StackElement` struct extends `ElementInfo` with `z_index` (String)
- [ ] All structs derive `Serialize` with `#[serde(rename_all = "camelCase")]`
- [ ] `Option<T>` fields serialize as `null` (not omitted) — no `skip_serializing_if`
- [ ] Compiles with `cargo check`

---

## Phase 2: Backend Implementation

### T003: Implement core hit test logic

**File(s)**: `src/page/hittest.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] `execute_hittest(global, args, frame)` function implemented
- [ ] Establishes session via `setup_session(global)`
- [ ] Fetches viewport dimensions via `get_viewport_dimensions()`
- [ ] Returns structured error (exit code 3) if coordinates exceed viewport bounds
- [ ] Calls `DOM.getNodeForLocation` with (x, y) to identify the hit target
- [ ] Calls `DOM.describeNode` to get tag, attributes of hit target
- [ ] Calls `Runtime.evaluate` with `document.elementsFromPoint(x, y)` to enumerate stack
- [ ] For each stack element: extracts tag, id, class, computed z-index via JS
- [ ] Attempts UID lookup from cached snapshot state (falls back to null)
- [ ] Serializes `HitTestResult` and calls `print_output()`

**Notes**: Use `DOM.getNodeForLocation` with `includeUserAgentShadowDOM: false`. For the stack enumeration, use a single `Runtime.evaluate` call that returns all element data as JSON to minimize CDP round-trips.

### T004: Implement overlay detection and suggestion generation

**File(s)**: `src/page/hittest.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `detect_overlay()` compares `DOM.getNodeForLocation` hit target against `elementsFromPoint()[0]`
- [ ] If the hit target is NOT the first interactive/meaningful element in the stack, the topmost non-interactive element is flagged as an intercepting overlay
- [ ] `interceptedBy` is set to the overlay element info, or `null` if no overlay
- [ ] `generate_suggestion()` produces actionable text when overlay detected
- [ ] Suggestion includes the overlay's selector (tag#id or tag.class) and the intended target's selector/UID
- [ ] When no overlay, suggestion is `null`

**Notes**: Overlay detection heuristic: if `stack[0]` differs from the first element with an interactive role or event listener, flag stack[0] as an overlay. Keep it simple — compare the `backendNodeId` from `DOM.getNodeForLocation` against each stack element.

### T005: Implement frame-scoped hit testing

**File(s)**: `src/page/hittest.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] When `--frame` is provided, resolves frame via `agentchrome::frame::resolve_frame()`
- [ ] CDP calls use the correct execution context for the resolved frame
- [ ] `frame` field in output reflects the resolved frame (URL or index, not "main")
- [ ] When `--frame` is not provided, `frame` field is `"main"`
- [ ] Invalid frame index returns structured error with appropriate exit code

---

## Phase 3: Frontend Implementation

N/A — CLI tool, no frontend components.

---

## Phase 4: Integration

### T006: Wire hittest into page command dispatcher

**File(s)**: `src/page/mod.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `mod hittest;` added to module declarations
- [ ] `PageCommand::HitTest` match arm routes to `hittest::execute_hittest(global, args, frame)`
- [ ] Frame argument is passed through from `PageArgs.frame`

### T007: Add page hittest examples to built-in examples

**File(s)**: `src/examples.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] At least two `page hittest` examples added to the `page` command group
- [ ] First example: basic `agentchrome page hittest 100 200` — "Hit test at viewport coordinates"
- [ ] Second example: `agentchrome page hittest 50 50 --frame 1` — "Hit test within a specific iframe"
- [ ] Examples follow the `ExampleEntry` struct pattern with `cmd`, `description`, and optional `flags`

### T008: Manual smoke test against real Chrome

**File(s)**: `tests/fixtures/page-hittest.html` (new file)
**Type**: Create
**Depends**: T003, T004, T005, T006
**Acceptance**:
- [ ] Test fixture HTML file created with: a button at known coordinates, an invisible overlay div above the button, a bare area with only document root, and an iframe with elements
- [ ] `cargo build` succeeds
- [ ] `agentchrome connect --launch --headless` connects
- [ ] `agentchrome navigate file://<path-to-fixture>` loads the test page
- [ ] `page hittest` at overlay-covered coordinates returns correct `hitTarget`, `interceptedBy`, `stack`, and `suggestion`
- [ ] `page hittest` at bare coordinates returns stack with document elements and `interceptedBy: null`
- [ ] `page hittest` with out-of-bounds coordinates returns structured error
- [ ] `agentchrome connect disconnect` and Chrome process cleaned up

---

## Phase 5: BDD Testing

### T009: Create BDD feature file for page hittest

**File(s)**: `tests/features/page-hittest.feature`
**Type**: Create
**Depends**: T006
**Acceptance**:
- [ ] All 8 acceptance criteria (AC1–AC8) mapped to Gherkin scenarios
- [ ] Feature header includes user story
- [ ] Scenarios use Given/When/Then format
- [ ] Valid Gherkin syntax
- [ ] File follows project naming convention (kebab-case)

### T010: Implement BDD step definitions for page hittest

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T009
**Acceptance**:
- [ ] Step definitions implemented for all page hittest scenarios
- [ ] Steps follow existing `cucumber-rs` patterns in `bdd.rs`
- [ ] CLI invocation steps test `page hittest` with various arguments
- [ ] JSON output parsing steps verify `hitTarget`, `interceptedBy`, `stack`, `suggestion` fields
- [ ] Error scenarios test stderr output and exit codes
- [ ] `cargo test --test bdd` passes for page-hittest features

### T011: Verify no regressions

**File(s)**: (no file changes)
**Type**: Verify
**Depends**: T006, T007, T009, T010
**Acceptance**:
- [ ] `cargo test --lib` passes (unit tests)
- [ ] `cargo test --test bdd` passes (all BDD tests, not just page-hittest)
- [ ] `cargo clippy --all-targets` passes with zero warnings
- [ ] `cargo fmt --check` passes
- [ ] Existing `page` subcommands (text, snapshot, find, screenshot, element, wait, resize, frames, workers) still function correctly

---

## Dependency Graph

```
T001 ──────────────────────────────────┐
  │                                    │
  ▼                                    ▼
T002 ──▶ T003 ──┬──▶ T004            T007
           │    │
           │    └──▶ T005
           │
           ▼
         T006 ──┬──▶ T008
                │
                ├──▶ T009 ──▶ T010
                │
                └──▶ T011
```

Critical path: T001 → T002 → T003 → T006 → T009 → T010 → T011

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #191 | 2026-04-16 | Initial feature spec |

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
