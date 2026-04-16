# Requirements: Page Hit Test Command for Click Debugging

**Issues**: #191
**Date**: 2026-04-16
**Status**: Draft
**Author**: Rich Nunley

---

## User Story

**As a** browser automation engineer debugging failed click interactions
**I want** to see the full event delivery path for a click at specific coordinates
**So that** I can identify overlays intercepting my clicks and find the correct interaction target on the first attempt

---

## Background

Users report burning 20+ attempts clicking at coordinates that visually appear correct but are intercepted by invisible overlay elements (e.g., Storyline's "acc-blocker" div). Currently, `interact click-at` dispatches `Input.dispatchMouseEvent` at viewport coordinates but provides no feedback about which element receives the event. `page element` queries element state by UID/selector but doesn't show what's at a given coordinate. No CDP-based hit testing (`DOM.getNodeForLocation`) or z-index stack inspection is exposed to users.

A `page hittest` command that reveals the actual hit target, intercepting overlays, and stacked elements would identify these problems immediately and suggest workarounds. This command pairs well with `dom events` for full interaction debugging.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Hit test at coordinates

**Given** a page is loaded with known DOM elements at specific positions
**When** `page hittest X Y` is run with valid viewport coordinates
**Then** the JSON output includes:
- `frame`: which frame received the event (main frame or frame index)
- `hitTarget`: the actual hit target element with `tag`, `id`, `class`, and `uid` (null if no UID available)
- `interceptedBy`: the overlay element if an overlay intercepts the coordinates, or `null` if no overlay
- `stack`: all stacked elements at those coordinates ordered by z-index (highest first)

**Example**:
- Given: a page with a button at (100, 200) covered by an invisible overlay div
- When: `page hittest 100 200`
- Then: `hitTarget` is the overlay div, `interceptedBy` contains overlay details, `stack` lists both overlay and button

### AC2: Workaround suggestions for overlays

**Given** coordinates that hit an overlay element intercepting the intended target
**When** `page hittest` is run
**Then** the output includes a `suggestion` field with actionable advice (e.g., "use --frame to bypass overlay" or "target the accessibility element at <selector>")

**Example**:
- Given: coordinates (100, 200) hit an "acc-blocker" overlay above a button
- When: `page hittest 100 200`
- Then: `suggestion` contains a string like "Element intercepted by overlay div.acc-blocker — try targeting the underlying element via selector 'button#submit'"

### AC3: Frame-scoped hit test

**Given** a `--frame <index>` argument is provided
**When** `page hittest X Y --frame 1` is run
**Then** coordinates are resolved within that frame's context, and the output reflects elements within the specified frame

**Example**:
- Given: frame index 1 contains a form with an input at (50, 50) relative to the frame
- When: `page hittest 50 50 --frame 1`
- Then: `hitTarget` is the input element within frame 1, and `frame` shows the frame identifier

### AC4: Documentation updated

**Given** the new `page hittest` command exists
**When** `examples page` is run
**Then** the output includes at least one `page hittest` example showing basic usage

### AC5: Coordinates outside viewport

**Given** coordinates X, Y that are outside the current viewport bounds
**When** `page hittest X Y` is run
**Then** a structured JSON error is written to stderr with a descriptive message indicating the coordinates are out of bounds, and the exit code is non-zero

**Example**:
- Given: viewport is 1280x720
- When: `page hittest 5000 5000`
- Then: stderr contains `{"error": "Coordinates (5000, 5000) are outside the viewport bounds (1280x720)", "code": 3}`

### AC6: No connection error handling

**Given** no active Chrome connection exists
**When** `page hittest X Y` is run
**Then** a structured JSON error is written to stderr indicating no connection, and exit code 2 is returned

### AC7: Null UID semantics for non-accessible elements

**Given** the hit target is an element without an accessibility UID (e.g., a plain `<div>` with no ARIA role)
**When** `page hittest X Y` is run
**Then** the `uid` field in `hitTarget` is `null` (not omitted), distinguishing "no UID assigned" from "UID lookup failed"

**Example**:
- Given: coordinates hit a bare `<div class="blocker">` with no accessibility role
- When: `page hittest 100 200`
- Then: `hitTarget` contains `{"tag": "div", "id": null, "class": "blocker", "uid": null}`

### AC8: Empty stack at coordinates

**Given** coordinates that hit only the document root with no meaningful stacked elements
**When** `page hittest X Y` is run in an area with only `<html>` and `<body>`
**Then** the `stack` array contains the document-level elements and `interceptedBy` is `null`

### Generated Gherkin Preview

```gherkin
Feature: Page Hit Test Command
  As a browser automation engineer debugging failed click interactions
  I want to see the full event delivery path for a click at specific coordinates
  So that I can identify overlays intercepting my clicks and find the correct interaction target

  Scenario: Hit test at coordinates returns structured element info
    Given a page is loaded with known DOM elements at specific positions
    When "page hittest 100 200" is run
    Then the JSON output includes hitTarget, interceptedBy, frame, and stack fields

  Scenario: Workaround suggestions for overlay interception
    Given coordinates that hit an overlay element
    When "page hittest" is run at those coordinates
    Then the output includes a suggestion field with actionable advice

  Scenario: Frame-scoped hit test
    Given a --frame argument targeting frame index 1
    When "page hittest 50 50 --frame 1" is run
    Then coordinates are resolved within that frame's context

  Scenario: Documentation includes page hittest examples
    Given the page hittest command is available
    When "examples page" is run
    Then page hittest examples appear in the output

  Scenario: Coordinates outside viewport return error
    Given coordinates outside the viewport bounds
    When "page hittest 5000 5000" is run
    Then a JSON error is written to stderr with a non-zero exit code

  Scenario: No connection returns error
    Given no active Chrome connection
    When "page hittest 100 200" is run
    Then a connection error is written to stderr with exit code 2

  Scenario: Null UID for non-accessible elements
    Given the hit target has no accessibility UID
    When "page hittest" is run at those coordinates
    Then the uid field is null, not omitted

  Scenario: Empty stack at bare coordinates
    Given coordinates hit only document root elements
    When "page hittest" is run
    Then the stack contains document-level elements and interceptedBy is null
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | New `page hittest X Y` subcommand under the `page` command group with structured JSON output on stdout | Must | Subcommand of existing `page` group |
| FR2 | Hit target identification via CDP `DOM.getNodeForLocation` returning tag, id, class, and UID | Must | UID via accessibility tree lookup |
| FR3 | Z-index stack enumeration at coordinates via `Runtime.evaluate` calling `document.elementsFromPoint()` | Must | Returns elements ordered highest z-index first |
| FR4 | Overlay detection by comparing `DOM.getNodeForLocation` result against the intended target stack | Should | Detects transparent/invisible overlays |
| FR5 | Workaround suggestion generation when an overlay is detected | Should | Actionable text referencing selectors or frame targeting |
| FR6 | Frame-scoped coordinate resolution with `--frame <index>` option | Should | Reuses frame targeting from iframe support |
| FR7 | Help documentation and built-in examples updated in `examples.rs` | Must | At least one `page hittest` example |
| FR8 | BDD test scenarios covering all acceptance criteria | Must | Gherkin feature file + step definitions |
| FR9 | Optional `null` fields use explicit `null` serialization, never omitted | Must | Applies to `uid`, `id`, `interceptedBy` |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Hit test response < 500ms including CDP round-trips and JS evaluation |
| **Output format** | JSON on stdout, JSON errors on stderr, exit codes per project convention (0=success, 2=connection, 3=target, 4=timeout) |
| **Platforms** | macOS, Linux, Windows (all platforms where agentchrome runs) |
| **Reliability** | Graceful error on no connection, invalid coordinates, missing frames |

---

## UI/UX Requirements

| Element | Requirement |
|---------|-------------|
| **CLI interface** | `page hittest <X> <Y> [--frame <index>]` — positional args for coordinates, optional flag for frame |
| **Error states** | Structured JSON error on stderr for: no connection, out-of-bounds coordinates, invalid frame index |
| **Empty states** | When stack contains only document root elements, return them (do not return empty array) |
| **Help text** | `page hittest --help` shows usage, argument descriptions, and one example |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| X | u32 | Must be non-negative integer within viewport width | Yes |
| Y | u32 | Must be non-negative integer within viewport height | Yes |
| --frame | usize | Must be a valid frame index (0-based) | No |

### Output Data

| Field | Type | Description | Null Semantics |
|-------|------|-------------|----------------|
| `frame` | string | Frame identifier (e.g., "main" or frame URL/index) | Never null |
| `hitTarget` | object | Element that receives the hit: `{tag, id, class, uid}` | Never null (always an element at any coordinate) |
| `hitTarget.tag` | string | HTML tag name (lowercase) | Never null |
| `hitTarget.id` | string \| null | Element `id` attribute | `null` if element has no id |
| `hitTarget.class` | string \| null | Element `class` attribute (space-separated) | `null` if element has no class |
| `hitTarget.uid` | string \| null | Accessibility tree UID | `null` if no UID assigned |
| `interceptedBy` | object \| null | Overlay element if one intercepts, same shape as hitTarget | `null` if no overlay detected |
| `stack` | array | All elements at coordinates, ordered by z-index (highest first) | Empty array only if no elements found (unlikely) |
| `stack[n]` | object | Element in stack: `{tag, id, class, uid, zIndex}` | Individual fields follow same null semantics |
| `stack[n].zIndex` | string | Computed z-index value (e.g., "auto", "10") | Never null |
| `suggestion` | string \| null | Workaround suggestion when overlay detected | `null` when no overlay or no actionable suggestion |

---

## Dependencies

### Internal Dependencies
- [x] `page` command group (`src/page.rs`) — existing parent command
- [x] CDP client (`src/cdp/client.rs`) — for `DOM.getNodeForLocation` and `Runtime.evaluate`
- [x] CLI argument parsing (`src/cli/mod.rs`) — add `HitTest` subcommand variant
- [x] Accessibility tree lookup (`src/snapshot.rs`) — for UID resolution
- [ ] Frame targeting infrastructure — for `--frame` support (may need to reference iframe feature patterns)

### External Dependencies
- [x] Chrome DevTools Protocol — `DOM.getNodeForLocation`, `DOM.describeNode`, `Runtime.evaluate`

### Blocked By
- None

---

## Out of Scope

- Automatic click rerouting — this is diagnostic only, user decides the action
- Event listener enumeration at hit target — covered by `dom events` (issue #192)
- Visual overlay highlighting or screenshot annotation
- Recursive frame traversal (only explicit `--frame` targeting)
- CSS selector generation for hit targets beyond what's in the output

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Click debugging time | Reduce from 20+ attempts to 1-2 | User can identify overlay on first `page hittest` invocation |
| Command response time | < 500ms | Time from invocation to JSON output |
| Overlay detection accuracy | 100% for z-index-based overlays | Hit target matches what Chrome would dispatch a click event to |

---

## Open Questions

- None (all technical approaches are well-established via CDP APIs)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #191 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

Before moving to PLAN phase:

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified (AC5, AC6, AC7, AC8)
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
