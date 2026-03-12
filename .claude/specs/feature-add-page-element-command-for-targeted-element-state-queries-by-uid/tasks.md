# Tasks: Page Element Command

**Issues**: #165
**Date**: 2026-03-11
**Status**: Planning
**Author**: Claude

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 4 | [ ] |
| Integration | 1 | [ ] |
| Testing | 3 | [ ] |
| **Total** | **10** | |

---

## Phase 1: Setup

### T001: Add Element variant to PageCommand enum

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `PageCommand::Element(PageElementArgs)` variant added to the enum
- [ ] `PageElementArgs` struct defined with `target: String` positional argument
- [ ] Help text: `"Query a single element's properties by UID or CSS selector"`
- [ ] `cargo build` succeeds with no errors

**Notes**: Follow the existing pattern of `PageFindArgs`, `PageScreenshotArgs`, etc. The `target` argument should have help text explaining both UID (`s1`, `s2`) and CSS selector (`css:#id`, `css:.class`) formats.

### T002: Add error variant for target-not-found with exit code 3

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] New error constructor (or variant) that produces exit code 3 (TargetError) with a descriptive message about the element not being found
- [ ] Distinct from existing `uid_not_found` (which uses exit code 1) — this is for "target element does not exist in the DOM"
- [ ] Error message includes the target identifier (UID or selector) for debuggability
- [ ] `cargo build` succeeds

**Notes**: Check existing `AppError` constructors. If there's already a `target_not_found` variant mapping to exit code 3, reuse it. Otherwise, add one. The key requirement is exit code 3, not exit code 1.

---

## Phase 2: Backend Implementation

### T003: Implement target resolution in page.rs

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] `resolve_element_target()` function added to `page.rs`
- [ ] UID path: reads snapshot state via `snapshot::read_snapshot_state()`, looks up `uid_map.get(target)`, returns `backendNodeId`
- [ ] CSS path: uses `DOM.getDocument` → `DOM.querySelector` → `DOM.describeNode` → returns `backendNodeId`
- [ ] Returns `AppError::no_snapshot_state()` (exit 1) when no snapshot state exists and target is a UID
- [ ] Returns target-not-found error (exit 3) when UID is not in uid_map or CSS selector matches nothing
- [ ] `cargo build` succeeds

**Notes**: Follow the UID/CSS detection pattern from `form.rs` (`is_uid()` checks `sN` format, `is_css_selector()` checks `css:` prefix). The `page.rs` module already has inline UID resolution for screenshots — follow that pattern.

### T004: Implement element data retrieval via CDP

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] Given a `backendNodeId`, retrieves accessibility data via `Accessibility.getPartialAXTree` with `fetchRelatives: false`
- [ ] Extracts `role` from `nodes[0].role.value`, defaults to `"none"` if absent
- [ ] Extracts `name` from `nodes[0].name.value`, defaults to `""` if absent
- [ ] Extracts properties (`disabled` → inverted to `enabled`, `focused`, `checked`, `expanded`, `required`, `readonly`) from `nodes[0].properties` array
- [ ] `checked` and `expanded` are `Option<bool>`: `Some(value)` if present in properties array, `None` if absent
- [ ] `enabled`, `focused`, `required`, `readonly` default to `true`/`false` as specified in design.md when absent
- [ ] Retrieves bounding box via `DOM.getBoxModel` with `backendNodeId` (not nodeId)
- [ ] Computes `x`, `y`, `width`, `height` from content quad
- [ ] Retrieves tag name via `DOM.describeNode` with `backendNodeId` → `node.nodeName`
- [ ] Handles CDP errors gracefully (e.g., element removed from DOM → TargetError exit 3)
- [ ] `cargo build` succeeds

**Notes**: Use `backendNodeId` directly with `DOM.getBoxModel` — do not resolve to intermediate nodeId first (per bug documented at `page.rs:711-713`). If `DOM.getBoxModel` fails (e.g., `display: none` element), return zero bounding box with `inViewport: false`.

### T005: Implement viewport visibility computation

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] Fetches viewport dimensions via `Runtime.evaluate` with `JSON.stringify({ width: window.innerWidth, height: window.innerHeight })`
- [ ] Computes `inViewport = (x + width > 0) && (x < viewportWidth) && (y + height > 0) && (y < viewportHeight)`
- [ ] Returns `false` when bounding box is zero (element has no layout)
- [ ] `cargo build` succeeds

**Notes**: Reuse the existing `get_viewport_dimensions()` helper if available in `page.rs`, or follow the same `Runtime.evaluate` pattern used for `get_page_dimensions()`.

### T006: Implement execute_element and wire into dispatcher

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T003, T004, T005
**Acceptance**:
- [ ] `execute_element()` async function orchestrates: session setup → domain enabling → target resolution → data retrieval → viewport computation → output
- [ ] Enables required CDP domains: `Accessibility`, `DOM`, `Runtime`
- [ ] Defines `ElementInfo`, `BoundingBoxInfo`, `ElementProperties` output structs with `#[derive(Serialize)]` and `#[serde(rename_all = "camelCase")]`
- [ ] JSON output on stdout via existing `print_output()` or `println!("{}", serde_json::to_string(...))` pattern
- [ ] Plain text output when `global.output.plain` is true, formatted per design.md spec
- [ ] `PageCommand::Element` match arm added to `execute_page()` dispatcher
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` passes with no warnings

---

## Phase 3: Integration

### T007: Manual smoke test against real Chrome

**File(s)**: N/A (manual verification)
**Type**: Verify
**Depends**: T006
**Acceptance**:
- [ ] Build debug binary: `cargo build`
- [ ] Launch headless Chrome: `./target/debug/agentchrome connect --launch --headless`
- [ ] Navigate to https://www.saucedemo.com/
- [ ] Take snapshot: `./target/debug/agentchrome page snapshot`
- [ ] Run `./target/debug/agentchrome page element s1` (or first interactive element) — verify JSON output with role, name, tagName, boundingBox, properties, inViewport
- [ ] Run `./target/debug/agentchrome page element "css:#user-name"` — verify same structured output for CSS selector
- [ ] Run `./target/debug/agentchrome page element s999` — verify exit code 3 and JSON error on stderr
- [ ] Run `./target/debug/agentchrome page element "css:#nonexistent"` — verify exit code 3
- [ ] Run `./target/debug/agentchrome page element s1 --plain` — verify human-readable text output
- [ ] Disconnect and kill Chrome: `./target/debug/agentchrome connect disconnect && pkill -f 'chrome.*--remote-debugging' || true`

---

## Phase 4: Testing

### T008: Create BDD feature file

**File(s)**: `tests/features/page-element.feature`
**Type**: Create
**Depends**: T006
**Acceptance**:
- [ ] Feature file contains scenarios for all 8 acceptance criteria from requirements.md
- [ ] Scenarios use Given/When/Then format
- [ ] File is valid Gherkin syntax
- [ ] Scenarios are independent and self-contained

### T009: Implement BDD step definitions

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T008
**Acceptance**:
- [ ] Step definitions added for all page element scenarios
- [ ] Steps follow existing patterns in `bdd.rs` (World struct, step functions)
- [ ] `cargo test --test bdd` compiles successfully
- [ ] Non-Chrome-dependent scenarios pass (Chrome-dependent scenarios may be skipped in CI)

### T010: Verify no regressions

**File(s)**: N/A (existing test files)
**Type**: Verify
**Depends**: T006, T009
**Acceptance**:
- [ ] `cargo test --lib` passes (unit tests)
- [ ] `cargo test --test bdd` passes (BDD tests)
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt --check` passes
- [ ] No side effects on existing `page` subcommands (snapshot, find, screenshot, text, resize)

---

## Dependency Graph

```
T001 ──┐
       ├──▶ T003 ──▶ T004 ──▶ T005 ──▶ T006 ──▶ T007
T002 ──┘                                  │
                                          ├──▶ T008 ──▶ T009
                                          │
                                          └──▶ T010
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #165 | 2026-03-11 | Initial feature spec |

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included for each layer
- [x] No circular dependencies
- [x] Tasks are in logical execution order
