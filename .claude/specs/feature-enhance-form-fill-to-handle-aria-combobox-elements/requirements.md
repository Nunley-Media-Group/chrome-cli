# Requirements: Enhance Form Fill — ARIA Combobox Support

**Issues**: #196
**Date**: 2026-04-16
**Status**: Draft
**Author**: Claude (nmg-sdlc)

---

## User Story

**As a** browser automation engineer working with modern web applications
**I want** `form fill` to automatically handle ARIA combobox elements
**So that** I can fill combobox fields with a single command instead of manually composing click-type-confirm sequences

---

## Background

Salesforce Lightning, Material UI, Ant Design, and many modern web applications use ARIA combobox elements (`role="combobox"`) for search and selection fields instead of standard `<select>` elements. The current `form fill` command in `src/form.rs` handles text inputs, textareas, `<select>` dropdowns (by value or textContent matching), and checkbox/radio toggles — but has no ARIA role detection. When `form fill` is used on a combobox element, the `fill_element` function falls through to the keyboard-typing path (since the element's `nodeName` is typically `input`), which types the value but never opens the dropdown or confirms a selection. The workaround is a manual 3-step sequence: `interact click` to open the dropdown, `interact type` with the value, then `interact key Enter` to confirm. This enhancement detects `role="combobox"` and executes that sequence automatically.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Auto-handle combobox elements

**Given** an element with `role="combobox"` identified by UID or CSS selector
**When** `form fill` is used with a value
**Then** the click-type-confirm sequence is executed automatically: click to open/focus, type the value, press Enter to confirm selection
**And** the JSON output on stdout includes the filled target and value

**Example**:
- Given: A Salesforce-style combobox with `role="combobox"` at UID s5
- When: `agentchrome form fill s5 "Acme Corp"`
- Then: The combobox opens, "Acme Corp" is typed, Enter is pressed, and `{"filled":"s5","value":"Acme Corp"}` is returned on stdout

### AC2: Preserve existing select behavior

**Given** a standard `<select>` element
**When** `form fill` is used with a value
**Then** existing behavior is preserved: direct value/textContent matching via `selectedIndex`

**Example**:
- Given: A `<select>` dropdown with options "Red", "Green", "Blue" at UID s3
- When: `agentchrome form fill s3 "Green"`
- Then: "Green" is selected via the existing JS-setter path (no click-type-confirm sequence)

### AC3: Combobox value not found

**Given** a combobox element where the typed value produces no matching options visible in the listbox
**When** `form fill` is used
**Then** a descriptive JSON error is returned on stderr with a message explaining that no matching option was found
**And** the exit code is 1 (general error)

**Example**:
- Given: A combobox at UID s5 whose listbox shows no options after typing "Nonexistent"
- When: `agentchrome form fill s5 "Nonexistent"`
- Then: stderr outputs `{"error":"No matching option found in combobox for value: Nonexistent","code":1}`

### AC4: Configurable confirmation key

**Given** a combobox that uses a different confirmation mechanism (e.g., Tab instead of Enter)
**When** `form fill` is used with `--confirm-key <key>`
**Then** the specified key is used for the confirmation step instead of Enter

**Example**:
- Given: A combobox at UID s5 that confirms selection on Tab
- When: `agentchrome form fill --confirm-key Tab s5 "Acme Corp"`
- Then: The sequence is click → type → Tab (instead of Enter)

### AC5: Documentation updated

**Given** the enhanced combobox support
**When** `examples form` is run
**Then** combobox examples are included in the output alongside existing form examples

**Example**:
- When: `agentchrome examples form`
- Then: Output includes an example like `agentchrome form fill s5 "Acme Corp"` with description "Fill an ARIA combobox field"

### AC6: Combobox in fill-many batch

**Given** a fill-many JSON array that includes a combobox element alongside standard inputs
**When** `form fill-many` is used
**Then** each element is filled using its appropriate strategy: combobox elements use the click-type-confirm sequence, standard inputs use their existing paths

**Example**:
- Given: s3 is a text input, s5 is a combobox
- When: `agentchrome form fill-many '[{"uid":"s3","value":"John"},{"uid":"s5","value":"Acme Corp"}]'`
- Then: s3 is filled via keyboard typing, s5 is filled via click-type-confirm, and both results are returned

### AC7: Combobox with dropdown render delay

**Given** a combobox where the dropdown options take time to render after the click/type (e.g., async search)
**When** `form fill` is used
**Then** the implementation waits for the associated listbox to become visible before pressing the confirmation key
**And** times out with a descriptive error if the listbox never appears

**Example**:
- Given: A combobox that fetches search results asynchronously after typing
- When: `agentchrome form fill s5 "Acme Corp"`
- Then: After typing, the implementation polls for a visible `role="listbox"` or `role="option"` before pressing Enter

### Generated Gherkin Preview

```gherkin
Feature: Form Fill ARIA Combobox Support
  As a browser automation engineer working with modern web applications
  I want form fill to automatically handle ARIA combobox elements
  So that I can fill combobox fields with a single command

  Scenario: Auto-handle combobox elements
    Given an element with role "combobox" identified by UID "s5"
    When form fill is used with value "Acme Corp"
    Then the click-type-confirm sequence is executed automatically
    And the JSON output includes filled target and value

  Scenario: Preserve existing select behavior
    Given a standard select element at UID "s3"
    When form fill is used with value "Green"
    Then existing select behavior is preserved

  Scenario: Combobox value not found
    Given a combobox at UID "s5" with no matching options for "Nonexistent"
    When form fill is used with value "Nonexistent"
    Then a descriptive JSON error is returned on stderr

  Scenario: Configurable confirmation key
    Given a combobox at UID "s5" that confirms on Tab
    When form fill is used with confirm-key "Tab" and value "Acme Corp"
    Then the Tab key is used for confirmation instead of Enter

  Scenario: Documentation updated
    When examples for form are requested
    Then combobox examples are included in the output

  Scenario: Combobox in fill-many batch
    Given a text input at UID "s3" and a combobox at UID "s5"
    When form fill-many is used with both elements
    Then each element is filled using its appropriate strategy

  Scenario: Combobox with dropdown render delay
    Given a combobox with async option loading at UID "s5"
    When form fill is used with value "Acme Corp"
    Then the implementation waits for the listbox before confirming
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Detect `role="combobox"` attribute on target elements via `DOM.describeNode` attributes parsing | Must | Extend existing `describe_element` function |
| FR2 | Implement click-type-confirm sequence: DOM.focus + click simulation, keyboard character input, confirmation key press | Must | Reuse existing keyboard simulation from `fill_element_keyboard` |
| FR3 | Wait for listbox/option visibility after typing before pressing confirmation key | Must | Poll for `aria-expanded="true"` or visible `role="listbox"` descendant |
| FR4 | Return descriptive JSON error on stderr when no matching combobox option is found | Must | Follow existing `AppError` pattern with exit code 1 |
| FR5 | Add `--confirm-key` option to `FormFillArgs` (default: "Enter") | Should | Verify no collision with global CLI flags |
| FR6 | Propagate combobox detection through `fill-many` path (each element uses appropriate strategy) | Must | The shared `fill_element` function handles routing |
| FR7 | Update `examples.rs` form command group with combobox example | Must | |
| FR8 | Update `form fill` long help text to mention combobox support | Must | In `cli/mod.rs` |
| FR9 | Support `role="searchbox"` elements that act as combobox inputs | Could | Some component libraries use searchbox role for similar patterns |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Combobox fill sequence should complete within the global `--timeout` budget; individual step delays should not exceed 2s each |
| **Reliability** | Listbox visibility polling should have a reasonable timeout (default ~3s) with descriptive error on timeout |
| **Platforms** | macOS, Linux, and Windows — same cross-platform support as existing form fill |
| **Output contract** | All output on stdout is structured JSON; all errors on stderr are structured JSON with exit code per existing convention |

---

## UI/UX Requirements

| Element | Requirement |
|---------|-------------|
| **CLI interface** | `--confirm-key` flag is optional; default behavior (Enter) requires no flag |
| **Help text** | `agentchrome form fill --help` mentions combobox support and `--confirm-key` option |
| **Error messages** | Combobox-specific errors are distinct from generic fill errors (user can tell it was a combobox interaction that failed) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| target | String | UID (s\d+) or CSS selector (css:...) | Yes |
| value | String | Non-empty string to type into combobox | Yes |
| --confirm-key | String | Valid key name (Enter, Tab, etc.) | No (default: Enter) |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| filled | String | The target identifier (UID or CSS selector) |
| value | String | The value that was typed into the combobox |
| snapshot | Object \| null | Optional accessibility snapshot if `--include-snapshot` was used |

---

## Dependencies

### Internal Dependencies
- [x] `src/form.rs` — `fill_element`, `describe_element`, `fill_element_keyboard` functions
- [x] `src/cli/mod.rs` — `FormFillArgs` struct, `FormCommand` enum
- [x] `src/examples.rs` — form command group examples
- [x] `src/interact.rs` — keyboard dispatch patterns (reference for key press implementation)

### External Dependencies
- [x] Chrome DevTools Protocol — `DOM.describeNode`, `Input.dispatchKeyEvent`, `Runtime.evaluate`

### Blocked By
- None

---

## Out of Scope

- Custom dropdown components without ARIA roles (no reliable detection mechanism)
- Multi-select combobox (selecting multiple values in a single fill command)
- Combobox option verification (checking that the selected option text exactly matches the typed value)
- `role="listbox"` standalone elements (not part of a combobox pattern)
- Configurable delay between click and type steps (the listbox polling handles timing)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Combobox detection accuracy | 100% for elements with explicit `role="combobox"` | BDD test scenarios |
| Backward compatibility | 0 regressions in existing form fill behavior | Existing BDD tests pass |
| Sequence completion time | < 5s for typical combobox interactions | Manual smoke test against Salesforce-style combobox |

---

## Open Questions

- [x] ~~Should `aria-autocomplete` attribute influence the fill strategy?~~ — No, the click-type-confirm sequence works regardless of autocomplete mode
- [ ] Should the listbox polling timeout be configurable via a flag, or is a fixed 3s reasonable? — Fixed 3s for now; can add flag later if needed

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #196 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
