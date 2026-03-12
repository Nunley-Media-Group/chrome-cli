# Requirements: Page Element Command

**Issues**: #165
**Date**: 2026-03-11
**Status**: Draft
**Author**: Claude

---

## User Story

**As an** AI agent making decisions based on element state
**I want** to quickly query a specific element's properties by its accessibility UID
**So that** I can check element visibility, enabled state, or bounding box without taking a full page snapshot

---

## Background

Currently, to check whether a button is enabled or an element is visible, an agent must run `page snapshot` (which returns the entire accessibility tree — potentially 100+ lines), parse through the full output to find the relevant element, and infer state from the tree structure and properties. This is expensive in tokens and time.

When an agent needs to answer a simple question like "is the Checkout button enabled?" or "where is element s15 on screen?", a targeted query is far more efficient. This command queries a single element by its accessibility UID (the `sN` identifiers from `page snapshot`) or CSS selector and returns a focused summary of that element's state — role, name, bounding box, accessibility properties, and viewport visibility.

This complements issue #149 (DOM command group): the `page element` command is UID-centric and accessibility-focused, while #149 is CSS/XPath-centric and DOM-focused.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Query element properties by UID

**Given** a connected Chrome session with a fresh accessibility snapshot containing element `s10` (a button)
**When** I run `agentchrome page element s10`
**Then** the command returns JSON on stdout with the element's role, name, tag name, bounding box (`x`, `y`, `width`, `height`), and accessibility properties (`enabled`, `focused`, `checked`, `expanded`, `required`, `readonly`)
**And** the exit code is 0

**Example**:
- Given: Chrome connected, `page snapshot` taken, s10 is a `<button>` labeled "Submit"
- When: `agentchrome page element s10`
- Then: `{"role":"button","name":"Submit","tagName":"BUTTON","boundingBox":{"x":100,"y":200,"width":150,"height":40},"properties":{"enabled":true,"focused":false,"checked":null,"expanded":null,"required":false,"readonly":false},"inViewport":true}`

### AC2: Query reports viewport visibility

**Given** a connected Chrome session where element `s15` is scrolled off-screen (its bounding box y-coordinate exceeds the viewport height)
**When** I run `agentchrome page element s15`
**Then** the output includes an `inViewport` field set to `false`
**And** the bounding box coordinates reflect the element's actual position on the page (not clipped to viewport)

### AC3: Query element that no longer exists in DOM

**Given** a connected Chrome session where the page has changed since the last snapshot
**When** I run `agentchrome page element s10` and element s10 no longer exists in the DOM
**Then** the command returns a JSON error on stderr indicating the element was not found
**And** the exit code is 3 (TargetError)

### AC4: Query element by CSS selector

**Given** a connected Chrome session on a page with a `#checkout` button
**When** I run `agentchrome page element "css:#checkout"`
**Then** the command returns the same structured JSON output as UID-based queries (role, name, tagName, boundingBox, properties, inViewport)
**And** the exit code is 0

### AC5: Query when no snapshot state exists (UID target)

**Given** a connected Chrome session where no `page snapshot` has been taken (no snapshot state persisted)
**When** I run `agentchrome page element s10`
**Then** the command returns a JSON error on stderr indicating that no snapshot state is available and the user should run `page snapshot` first
**And** the exit code is 1 (GeneralError)

### AC6: Non-applicable properties appear as null

**Given** a connected Chrome session with a snapshot containing element `s5` (a link, which has no `checked` or `expanded` semantics)
**When** I run `agentchrome page element s5`
**Then** the `properties.checked` field is `null` and the `properties.expanded` field is `null`
**And** the `properties.enabled` field is a boolean (not null)

### AC7: CSS selector element not found

**Given** a connected Chrome session on a page with no element matching `#nonexistent`
**When** I run `agentchrome page element "css:#nonexistent"`
**Then** the command returns a JSON error on stderr indicating no element matches the selector
**And** the exit code is 3 (TargetError)

### AC8: Plain text output mode

**Given** a connected Chrome session with a fresh snapshot containing element `s10`
**When** I run `agentchrome page element s10 --plain`
**Then** the output is a human-readable text summary (not JSON) containing the element's role, name, bounding box, and properties
**And** the exit code is 0

### Generated Gherkin Preview

```gherkin
Feature: Page element command
  As an AI agent making decisions based on element state
  I want to quickly query a specific element's properties by its accessibility UID
  So that I can check element visibility, enabled state, or bounding box without taking a full page snapshot

  Scenario: Query element properties by UID
    Given a connected Chrome session with a fresh accessibility snapshot containing element "s10" as a button named "Submit"
    When I run the page element command with target "s10"
    Then the output contains role "button" and name "Submit"
    And the output contains a bounding box with x, y, width, and height
    And the output contains accessibility properties enabled, focused, checked, expanded, required, readonly
    And the exit code is 0

  Scenario: Query reports viewport visibility for off-screen element
    Given a connected Chrome session where element "s15" is scrolled off-screen
    When I run the page element command with target "s15"
    Then the output contains inViewport set to false

  Scenario: Query element that no longer exists in DOM
    Given a connected Chrome session where the page has changed since the last snapshot
    When I run the page element command with target "s10" which no longer exists
    Then stderr contains a JSON error about element not found
    And the exit code is 3

  Scenario: Query element by CSS selector
    Given a connected Chrome session on a page with a "#checkout" button
    When I run the page element command with target "css:#checkout"
    Then the output contains structured JSON with role, name, tagName, boundingBox, properties, and inViewport
    And the exit code is 0

  Scenario: Query when no snapshot state exists
    Given a connected Chrome session where no snapshot has been taken
    When I run the page element command with target "s10"
    Then stderr contains a JSON error about missing snapshot state
    And the exit code is 1

  Scenario: Non-applicable properties appear as null
    Given a connected Chrome session with a snapshot containing element "s5" as a link
    When I run the page element command with target "s5"
    Then properties.checked is null
    And properties.expanded is null
    And properties.enabled is a boolean

  Scenario: CSS selector element not found
    Given a connected Chrome session on a page with no element matching "#nonexistent"
    When I run the page element command with target "css:#nonexistent"
    Then stderr contains a JSON error about no matching element
    And the exit code is 3

  Scenario: Plain text output mode
    Given a connected Chrome session with a fresh snapshot containing element "s10"
    When I run the page element command with target "s10" and the plain flag
    Then the output is human-readable text with role, name, bounding box, and properties
    And the exit code is 0
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Add `page element <target>` subcommand accepting UIDs (`s1`, `s2`, ...) | Must | New variant in `PageCommand` enum |
| FR2 | Return: role, name, bounding box (`x`, `y`, `width`, `height`), tag name | Must | Use `DOM.getBoxModel` with `backendNodeId` and `Accessibility.getPartialAXTree` |
| FR3 | Return accessibility properties: `enabled`, `focused`, `checked`, `expanded`, `required`, `readonly` | Must | From `Accessibility.getPartialAXTree` node properties |
| FR4 | Return `inViewport` boolean comparing bounding box to viewport dimensions (`window.innerWidth`/`innerHeight`) | Should | Element is in viewport if any part of its bounding box overlaps the viewport rect |
| FR5 | Support CSS selector targets (`css:#id`, `css:.class`) via existing `is_css_selector()` / `resolve_target_to_backend_node_id()` pattern | Should | Reuse `form.rs` CSS selector resolution path |
| FR6 | Structured JSON output on stdout, JSON errors on stderr | Must | Follow existing output format conventions |
| FR7 | Support `--plain` flag for human-readable text output | Should | Consistent with other `page` subcommands |
| FR8 | Properties that don't apply to an element's role appear as `null` (not omitted) | Must | Distinguishes "not applicable" (`null`) from "measured as false" (`false`) |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Single-element query should complete in < 200ms (faster than full `page snapshot`) |
| **Reliability** | Graceful error handling when element no longer exists in DOM, when snapshot state is missing, or when CSS selector matches nothing |
| **Platforms** | macOS, Linux, Windows (per `tech.md`) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `target` | String | Must be a UID (`sN` format) or CSS selector (`css:...` prefix). No collision with global flags or framework-reserved names. | Yes |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| `role` | String | Accessibility role (e.g., "button", "link", "textbox"). Source: `Accessibility.getPartialAXTree` → `nodes[0].role.value` |
| `name` | String | Accessible name (e.g., "Submit", "Home"). Source: `Accessibility.getPartialAXTree` → `nodes[0].name.value` |
| `tagName` | String | HTML tag name (e.g., "BUTTON", "A", "INPUT"). Source: `DOM.describeNode` → `node.nodeName` |
| `boundingBox` | Object | `{ x: f64, y: f64, width: f64, height: f64 }`. Source: `DOM.getBoxModel` → `model.content` quad, computed as `x=content[0], y=content[1], width=content[4]-content[0], height=content[5]-content[1]` |
| `properties.enabled` | Boolean | Whether the element is enabled (not disabled). Source: absence of `disabled` property in AX tree means `true` |
| `properties.focused` | Boolean | Whether the element currently has focus. Source: `focused` property in AX tree |
| `properties.checked` | Boolean or null | Check state for checkboxes/radios. `null` if not applicable to this role |
| `properties.expanded` | Boolean or null | Expanded state for expandable elements. `null` if not applicable |
| `properties.required` | Boolean | Whether the element is marked as required. Default `false` if absent |
| `properties.readonly` | Boolean | Whether the element is read-only. Default `false` if absent |
| `inViewport` | Boolean | `true` if any part of the bounding box overlaps the viewport rectangle `(0, 0, window.innerWidth, window.innerHeight)` |

---

## Dependencies

### Internal Dependencies
- [x] `page snapshot` command (provides UID mapping via `snapshot.rs`)
- [x] `resolve_target_to_backend_node_id()` pattern (from `form.rs`)
- [x] CSS selector resolution (from `form.rs` / `interact.rs`)

### External Dependencies
- [x] Chrome DevTools Protocol: `DOM.getBoxModel`, `DOM.describeNode`, `Accessibility.getPartialAXTree`, `Runtime.evaluate`

### Blocked By
- None

---

## Out of Scope

- DOM manipulation (covered by #149)
- Getting/setting HTML attributes (covered by #149)
- innerHTML/outerHTML extraction (covered by #149)
- Batch querying multiple elements in one call
- XPath selector support (CSS selectors only for this issue)
- Querying non-interactive elements by UID (UIDs are only assigned to interactive elements)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Query latency | < 200ms for single element | Time from command invocation to JSON output |
| Token savings | > 80% fewer output tokens vs `page snapshot` for single-element queries | Compare output size of `page element s10` vs `page snapshot` |

---

## Open Questions

- [x] Should `inViewport` account for CSS `visibility: hidden` or `opacity: 0`? — No, `inViewport` is purely geometric (bounding box vs viewport). CSS visibility is a separate concern.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #165 | 2026-03-11 | Initial feature spec |

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
