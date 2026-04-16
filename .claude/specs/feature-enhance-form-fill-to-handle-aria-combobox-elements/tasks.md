# Tasks: Enhance Form Fill — ARIA Combobox Support

**Issues**: #196
**Date**: 2026-04-16
**Status**: Planning
**Author**: Claude (nmg-sdlc)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 4 | [ ] |
| Integration | 2 | [ ] |
| Testing | 3 | [ ] |
| **Total** | **11** | |

---

## Phase 1: Setup

### T001: Extend describe_element to return ARIA role

**File(s)**: `src/form.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `describe_element` returns a 3-tuple `(String, Option<String>, Option<String>)` for `(node_name, input_type, role)`
- [ ] The `role` attribute is parsed from the flat `attributes` array in `DOM.describeNode` response, same scan as `type`
- [ ] All existing callers of `describe_element` (`fill_element`, `clear_element`) are updated to destructure the 3-tuple
- [ ] Existing behavior is unchanged for elements without a `role` attribute (role returns `None`)
- [ ] Unit test: `describe_element` attribute parsing handles `role` attribute correctly

**Notes**: The `attributes` array from `DOM.describeNode` is flat: `[name1, val1, name2, val2, ...]`. Extend the existing `chunks(2)` scan to also match `"role"`.

### T002: Add confirm-key flag to FormFillArgs

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `FormFillArgs` has a new field: `pub confirm_key: Option<String>`
- [ ] The field is annotated with `#[arg(long)]` and has a help string: "Key to confirm combobox selection (default: Enter)"
- [ ] No collision with existing global flags
- [ ] `form fill --help` output shows the new flag with description

**Notes**: The flag is only meaningful for combobox elements; it is silently ignored for other element types.

---

## Phase 2: Backend Implementation

### T003: Add fill_element_combobox function

**File(s)**: `src/form.rs`
**Type**: Create (new function in existing file)
**Depends**: T001
**Acceptance**:
- [ ] New async function `fill_element_combobox(session, backend_node_id, object_id, value, confirm_key)` exists
- [ ] Step 1: Focuses the element via `DOM.focus` using `backend_node_id`
- [ ] Step 2: Clicks the element via `Runtime.callFunctionOn` with `this.click()` using `object_id`
- [ ] Step 3: Waits 50ms for dropdown to start rendering
- [ ] Step 4: Types the value character-by-character via `Input.dispatchKeyEvent` (char events, same pattern as `fill_element_keyboard`)
- [ ] Step 5: Polls for listbox visibility via `Runtime.evaluate` JS (checks `aria-expanded="true"` and `[role="option"]` presence), every 100ms for up to 3000ms
- [ ] Step 6: Dispatches confirmation key (keyDown + keyUp) via `Input.dispatchKeyEvent`
- [ ] Returns `Ok(())` on success
- [ ] Returns `AppError::interaction_failed("combobox_fill", ...)` if listbox never appears (timeout)

**Notes**: The click JS is `function() { this.click(); }`. The poll JS checks `aria-owns`/`aria-controls` first, then falls back to generic `[role="listbox"]`. The confirmation key dispatch follows the `dispatch_key_press` pattern from interact.rs but is inlined since that function is private to interact.rs.

### T004: Update fill_element routing to detect combobox

**File(s)**: `src/form.rs`
**Type**: Modify
**Depends**: T001, T003
**Acceptance**:
- [ ] `fill_element` signature changes to accept `confirm_key: Option<&str>`
- [ ] After calling `describe_element`, checks `role.as_deref() == Some("combobox")` before `is_text_input` check
- [ ] If combobox: resolves object_id, calls `fill_element_combobox`
- [ ] If not combobox: follows existing routing (text-input keyboard path or JS setter path)
- [ ] Existing behavior for text inputs, selects, checkboxes, and radios is unchanged

**Notes**: The combobox check must come before `is_text_input` because a combobox element is typically an `<input type="text">` with `role="combobox"`, which would match `is_text_input` if checked first.

### T005: Update execute_fill to pass confirm_key

**File(s)**: `src/form.rs`
**Type**: Modify
**Depends**: T002, T004
**Acceptance**:
- [ ] `execute_fill` passes `args.confirm_key.as_deref()` to `fill_element`
- [ ] Compiles and existing fill behavior is preserved

### T006: Update execute_fill_many to pass default confirm_key

**File(s)**: `src/form.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `execute_fill_many` passes `None` as `confirm_key` to `fill_element` (uses default Enter for any combobox elements in the batch)
- [ ] Combobox elements in a fill-many batch are filled using the click-type-confirm sequence
- [ ] Non-combobox elements in the same batch use their existing fill paths

---

## Phase 3: Integration

### T007: Update examples.rs with combobox example

**File(s)**: `src/examples.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] The `form` command group in `build_examples()` includes a combobox example
- [ ] Example command: `agentchrome form fill s5 "Acme Corp"`
- [ ] Example description: "Fill an ARIA combobox field (auto click-type-confirm)"
- [ ] A confirm-key example is also included: `agentchrome form fill --confirm-key Tab s5 "Acme Corp"` with description "Fill combobox with custom confirmation key"
- [ ] Existing examples are not modified

### T008: Update form fill help text in CLI

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `FormCommand::Fill` long_about mentions ARIA combobox support
- [ ] `after_long_help` EXAMPLES section includes combobox examples
- [ ] Example shows: `# Fill an ARIA combobox` followed by `agentchrome form fill s5 "Acme Corp"`
- [ ] Example shows: `# Custom confirmation key for combobox` followed by `agentchrome form fill --confirm-key Tab s5 "Acme Corp"`

---

## Phase 4: Testing

### T009: Create BDD feature file

**File(s)**: `tests/features/form-fill-aria-combobox.feature`
**Type**: Create
**Depends**: T004
**Acceptance**:
- [ ] Feature file covers all 7 acceptance criteria (AC1-AC7) as individual scenarios
- [ ] Uses Given/When/Then format consistent with existing feature files
- [ ] Valid Gherkin syntax
- [ ] File is named `form-fill-aria-combobox.feature` per naming convention

### T010: Implement BDD step definitions

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T009
**Acceptance**:
- [ ] Step definitions for all 7 scenarios are implemented in `tests/bdd.rs`
- [ ] Steps follow existing cucumber-rs World patterns in the project
- [ ] CLI invocations use the agentchrome binary
- [ ] Steps that require Chrome interaction are gated (skip in CI if no Chrome available)
- [ ] `cargo test --test bdd` passes (including new scenarios in non-Chrome-gated mode)

### T011: Manual smoke test against headless Chrome

**File(s)**: `tests/fixtures/form-fill-aria-combobox.html`
**Type**: Create
**Depends**: T003, T004
**Acceptance**:
- [ ] HTML fixture includes: (1) an ARIA combobox with role combobox, associated role listbox with role option items, and aria-expanded toggling; (2) a standard select element; (3) a standard text input; (4) a combobox with delayed option rendering (setTimeout-based)
- [ ] Fixture is self-contained: no external dependencies, CDNs, or network requests
- [ ] HTML comment at top documents which ACs each section covers
- [ ] Smoke test procedure: build, connect headless, navigate to fixture, exercise AC1 through AC7, verify outputs, disconnect, cleanup

**Notes**: The fixture should use JavaScript to implement the combobox behavior: clicking toggles aria-expanded, typing filters options in the listbox, Enter selects the first matching option. The delayed combobox variant uses setTimeout(200) to simulate async search.

---

## Dependency Graph

```
T001 ---+---> T003 ---> T004 ---+--> T005
        |                       +--> T006
        |                       +--> T007
        |                       +--> T009 ---> T010
        |                       +--> T011
        |
T002 ---+--> T005
        +--> T008
```

Critical path: T001 -> T003 -> T004 -> T005 (then T009 -> T010 for testing)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #196 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per structure.md)
- [x] Test tasks are included for each layer
- [x] No circular dependencies
- [x] Tasks are in logical execution order
