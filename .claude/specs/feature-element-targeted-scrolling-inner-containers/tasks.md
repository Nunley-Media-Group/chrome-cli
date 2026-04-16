# Tasks: Element-Targeted Scrolling for Inner Containers

**Issues**: #182
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
| Integration | 1 | [ ] |
| Testing | 3 | [ ] |
| **Total** | **9** | |

---

## Phase 1: Setup

### T001: Add `element_not_scrollable` error constructor

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] New `pub fn element_not_scrollable(descriptor: &str) -> Self` constructor on `AppError`
- [ ] Message format: `"Element '<descriptor>' is not scrollable: content does not overflow the container"`
- [ ] Exit code: `ExitCode::GeneralError`
- [ ] Unit test added for the new constructor
- [ ] `cargo test --lib` passes

**Notes**: Follow the pattern of existing constructors like `element_not_found` and `uid_not_found` at lines 126-178.

### T002: Add `--selector` and `--uid` flags to `ScrollArgs`

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `pub selector: Option<String>` field added with `#[arg(long)]`
- [ ] `pub uid: Option<String>` field added with `#[arg(long)]`
- [ ] `--selector` conflicts with: `uid`, `to_element`, `to_top`, `to_bottom`
- [ ] `--uid` conflicts with: `selector`, `to_element`, `to_top`, `to_bottom`
- [ ] Help text for `--selector`: "CSS selector to target a scrollable container (e.g., '.stage', '#panel')"
- [ ] Help text for `--uid`: "Accessibility UID to target a scrollable container (e.g., 's42', requires prior snapshot)"
- [ ] Update command `long_about` and `after_long_help` examples to include `--selector` and `--uid` usage
- [ ] `cargo build` succeeds with no clippy warnings

**Notes**: Add fields to `ScrollArgs` struct at `src/cli/mod.rs:2179`. Follow conflict patterns of existing `--container` field at line 2206. Also update `--container` conflicts to include `selector` and `uid`.

---

## Phase 2: Backend Implementation

### T003: Add `check_element_scrollable` helper

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] New async function `check_element_scrollable(session: &ManagedSession, backend_node_id: i64, descriptor: &str) -> Result<(), AppError>`
- [ ] Uses `Runtime.callFunctionOn` to read `scrollHeight`, `clientHeight`, `scrollWidth`, `clientWidth`
- [ ] Returns `Ok(())` if `scrollHeight > clientHeight` OR `scrollWidth > clientWidth`
- [ ] Returns `Err(AppError::element_not_scrollable(descriptor))` otherwise
- [ ] Calls `resolve_to_object_id` to get the objectId (reuse existing helper at line 1131)

**Notes**: Place near the existing `dispatch_container_scroll` helper (around line 1148). Pattern the `Runtime.callFunctionOn` call after `get_container_scroll_position` at line 1173.

### T004: Add `resolve_selector_to_backend_node_id` helper

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] New async function `resolve_selector_to_backend_node_id(session: &ManagedSession, selector: &str) -> Result<i64, AppError>`
- [ ] Resolves a raw CSS selector (no `css:` prefix) to a `backend_node_id`
- [ ] Internally calls `resolve_target_to_backend_node_id(session, &format!("css:{selector}"))`
- [ ] This is a thin wrapper for clarity and to avoid leaking the `css:` prefix convention to callers

**Notes**: Alternative approach: call `resolve_target_to_backend_node_id` directly with `format!("css:{}", selector)` inline in `execute_scroll`. The wrapper is preferred for readability.

### T005: Wire `--selector` and `--uid` into `execute_scroll`

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: T002, T003, T004
**Acceptance**:
- [ ] New branch in `execute_scroll` that handles `args.selector.is_some() || args.uid.is_some()`
- [ ] `--selector` resolves via `resolve_selector_to_backend_node_id`
- [ ] `--uid` resolves via `resolve_target_to_backend_node_id` (existing, UID path)
- [ ] After resolution, calls `check_element_scrollable` with the `backend_node_id`
- [ ] Then follows existing container scroll path: `get_container_scroll_position`, `compute_scroll_delta`, `dispatch_container_scroll`, optional `wait_for_smooth_container_scroll`
- [ ] `mode_label` set to `"selector"` or `"uid"` respectively
- [ ] New branch placed before the existing `--container` branch (line 1312)
- [ ] JSON output matches existing scroll format: `{ scrolled: {x, y}, position: {x, y} }`
- [ ] `cargo build` and `cargo clippy` pass

**Notes**: The new branch closely mirrors the existing container branch at lines 1312-1322, with the addition of the scrollability check. Keep the existing `--container` path unchanged.

---

## Phase 3: Frontend Implementation

*N/A — this is a CLI tool with no frontend.*

---

## Phase 4: Integration

### T006: Update command help text and examples

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `long_about` for `Scroll` variant updated to mention `--selector` and `--uid` flags
- [ ] `after_long_help` examples section includes:
  - `agentchrome interact scroll --selector ".stage" --direction down`
  - `agentchrome interact scroll --uid s42 --direction down --amount 300`
- [ ] Existing examples preserved
- [ ] `cargo build` succeeds

**Notes**: Update the `#[command(...)]` attribute on `InteractCommand::Scroll` at line 2001.

---

## Phase 5: BDD Testing (Required)

### T007: Create BDD feature file

**File(s)**: `tests/features/element-targeted-scrolling.feature`
**Type**: Create
**Depends**: T005
**Acceptance**:
- [ ] Feature file covers all 6 acceptance criteria from requirements.md
- [ ] Scenario for AC1: scroll by CSS selector
- [ ] Scenario for AC2: scroll by UID
- [ ] Scenario for AC3: error on non-scrollable target
- [ ] Scenario for AC4: selector/uid conflict
- [ ] Scenario for AC5: smooth scroll with targeted container
- [ ] Scenario Outline for AC6: all four directions
- [ ] Valid Gherkin syntax

### T008: Implement BDD step definitions

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T007
**Acceptance**:
- [ ] Step definitions for all scenarios in T007
- [ ] Steps use the project's cucumber-rs 0.21 patterns
- [ ] `cargo test --test bdd` compiles (tests may skip Chrome-dependent scenarios in CI)

### T009: Smoke test against real Chrome

**File(s)**: `tests/fixtures/element-targeted-scrolling.html`
**Type**: Create
**Depends**: T005
**Acceptance**:
- [ ] HTML fixture includes:
  - Scrollable container with class `.stage` (vertical overflow, 200px height, 1000px content)
  - Scrollable container with class `.panel` (both horizontal and vertical overflow)
  - Non-scrollable `#static-div` element (no overflow)
- [ ] Manual smoke test verifies:
  - `--selector ".stage" --direction down` scrolls and returns positive `scrolled.y`
  - `--uid <uid> --direction down --amount 300` scrolls by ~300px
  - `--selector "#static-div" --direction down` returns JSON error
  - `--selector ".panel" --direction down --smooth` scrolls smoothly
  - All four directions work with `--selector ".panel"`
- [ ] Chrome processes cleaned up after test

**Notes**: Build with `cargo build`, launch headless Chrome with `./target/debug/agentchrome connect --launch --headless`, navigate to `file://` path of fixture, exercise each AC.

---

## Dependency Graph

```
T001 (error type) ----+
                      |
T002 (CLI flags) -----+---> T005 (wire execute_scroll) ---> T007 (feature file) ---> T008 (step defs)
                      |                                  |
T003 (scrollable chk)-+                                  +---> T009 (smoke test)
                      |
T004 (selector helper)+

T002 ---> T006 (help text)
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #182 | 2026-04-16 | Initial feature spec |

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
