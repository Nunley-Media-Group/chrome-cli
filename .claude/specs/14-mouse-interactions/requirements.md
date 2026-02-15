# Requirements: Mouse Interactions

**Issue**: #14
**Date**: 2026-02-13
**Status**: Draft
**Author**: Claude (writing-specs)

---

## User Story

**As a** developer / automation engineer
**I want** to simulate mouse interactions (click, double-click, hover, drag) on page elements via the CLI
**So that** my automation scripts can interact with web pages programmatically without requiring a GUI

---

## Background

AI agents and automation scripts need to interact with page elements — clicking buttons, hovering over menus, dragging items. The Chrome DevTools Protocol provides `Input.dispatchMouseEvent` for low-level mouse simulation and `DOM.getBoxModel`/`DOM.getContentQuads` for computing element coordinates. The MCP server already exposes `click`, `clickAt`, `hover`, and `drag` tools; this feature brings equivalent capabilities to the CLI under an `interact` subcommand group.

Targets are identified either by accessibility UID (from `page snapshot`, e.g., `s1`) or by CSS selector (prefixed with `css:`). The UID system is already implemented in `snapshot.rs` and persisted to `~/.chrome-cli/snapshot.json`.

---

## Acceptance Criteria

### AC1: Click an element by UID

**Given** a page with a button that has snapshot UID `s1`
**When** I run `chrome-cli interact click s1`
**Then** the button is clicked
**And** JSON output is returned: `{"clicked": "s1", "url": "...", "navigated": false}`
**And** the exit code is 0

**Example**:
- Given: page has `<button>Submit</button>` with UID `s1`
- When: `chrome-cli interact click s1`
- Then: `{"clicked": "s1", "url": "https://example.com", "navigated": false}`

### AC2: Click an element by CSS selector

**Given** a page with a button matching `css:#submit-btn`
**When** I run `chrome-cli interact click "css:#submit-btn"`
**Then** the button is clicked
**And** JSON output is returned: `{"clicked": "css:#submit-btn", "url": "...", "navigated": false}`

**Example**:
- Given: page has `<button id="submit-btn">Submit</button>`
- When: `chrome-cli interact click "css:#submit-btn"`
- Then: `{"clicked": "css:#submit-btn", "url": "https://example.com", "navigated": false}`

### AC3: Click triggers navigation

**Given** a page with a link that navigates to another page
**When** I run `chrome-cli interact click s1` and the click triggers navigation
**Then** the command waits for navigation to complete
**And** JSON output contains `"navigated": true` with the new URL

**Example**:
- Given: page has `<a href="/about">About</a>` with UID `s1`
- When: `chrome-cli interact click s1`
- Then: `{"clicked": "s1", "url": "https://example.com/about", "navigated": true}`

### AC4: Double-click an element

**Given** a page with an element that has snapshot UID `s1`
**When** I run `chrome-cli interact click s1 --double`
**Then** a double-click is performed on the element
**And** JSON output includes `"double_click": true`

### AC5: Right-click an element

**Given** a page with an element that has snapshot UID `s1`
**When** I run `chrome-cli interact click s1 --right`
**Then** a right-click (context menu click) is performed on the element
**And** JSON output includes `"right_click": true`

### AC6: Click with include-snapshot flag

**Given** a page with a button that has snapshot UID `s1`
**When** I run `chrome-cli interact click s1 --include-snapshot`
**Then** the button is clicked
**And** the JSON output includes an updated accessibility snapshot in the `snapshot` field

### AC7: Click at viewport coordinates

**Given** a page is loaded
**When** I run `chrome-cli interact click-at 100 200`
**Then** a click is dispatched at viewport coordinates (100, 200)
**And** JSON output is returned: `{"clicked_at": {"x": 100, "y": 200}}`
**And** the exit code is 0

### AC8: Click-at with double and right flags

**Given** a page is loaded
**When** I run `chrome-cli interact click-at 100 200 --double`
**Then** a double-click is dispatched at coordinates (100, 200)
**And** JSON output includes `"double_click": true`

### AC9: Hover over an element

**Given** a page with a menu item that has snapshot UID `s3`
**When** I run `chrome-cli interact hover s3`
**Then** the cursor is moved over the element
**And** JSON output is returned: `{"hovered": "s3"}`
**And** the exit code is 0

### AC10: Hover with CSS selector

**Given** a page with a menu item matching `css:.dropdown-trigger`
**When** I run `chrome-cli interact hover "css:.dropdown-trigger"`
**Then** the cursor is moved over the element
**And** JSON output is returned: `{"hovered": "css:.dropdown-trigger"}`

### AC11: Hover with include-snapshot flag

**Given** a page with an element that has snapshot UID `s3`
**When** I run `chrome-cli interact hover s3 --include-snapshot`
**Then** the cursor is moved over the element
**And** the JSON output includes an updated accessibility snapshot in the `snapshot` field

### AC12: Drag from one element to another

**Given** a page with a draggable item (UID `s1`) and a drop target (UID `s2`)
**When** I run `chrome-cli interact drag s1 s2`
**Then** a drag operation is performed from `s1` to `s2`
**And** JSON output is returned: `{"dragged": {"from": "s1", "to": "s2"}}`
**And** the exit code is 0

### AC13: Drag with CSS selectors

**Given** a page with elements matching `css:#item` and `css:#target`
**When** I run `chrome-cli interact drag "css:#item" "css:#target"`
**Then** a drag operation is performed between the elements
**And** JSON output is returned: `{"dragged": {"from": "css:#item", "to": "css:#target"}}`

### AC14: Drag with include-snapshot flag

**Given** a page with draggable elements
**When** I run `chrome-cli interact drag s1 s2 --include-snapshot`
**Then** the drag is performed
**And** the JSON output includes an updated accessibility snapshot in the `snapshot` field

### AC15: Target element by --tab flag

**Given** a specific tab with ID "ABC123" contains an element with UID `s1`
**When** I run `chrome-cli interact click s1 --tab ABC123`
**Then** the click is performed on the element in that specific tab

### AC16: Element not found error

**Given** no element matches UID `s99`
**When** I run `chrome-cli interact click s99`
**Then** an error is returned to stderr: `{"error": "UID 's99' not found. Run 'chrome-cli page snapshot' first.", "code": 1}`
**And** the exit code is non-zero

### AC17: CSS selector not found error

**Given** no element matches `css:#nonexistent`
**When** I run `chrome-cli interact click "css:#nonexistent"`
**Then** an error is returned to stderr: `{"error": "Element not found for selector: #nonexistent", "code": 1}`
**And** the exit code is non-zero

### AC18: No snapshot state error

**Given** no snapshot has been taken (no `~/.chrome-cli/snapshot.json`)
**When** I run `chrome-cli interact click s1`
**Then** an error is returned advising the user to run `chrome-cli page snapshot` first
**And** the exit code is non-zero

### AC19: Element scrolled into view before click

**Given** an element with UID `s5` that is not currently visible in the viewport
**When** I run `chrome-cli interact click s5`
**Then** the element is scrolled into view before the click is dispatched
**And** the click succeeds

### AC20: Plain text output for click

**Given** a page with a button that has UID `s1`
**When** I run `chrome-cli interact click s1 --plain`
**Then** plain text output is returned (e.g., `Clicked s1`)

### AC21: Plain text output for hover

**Given** a page with an element that has UID `s3`
**When** I run `chrome-cli interact hover s3 --plain`
**Then** plain text output is returned (e.g., `Hovered s3`)

### AC22: Plain text output for drag

**Given** a page with draggable elements
**When** I run `chrome-cli interact drag s1 s2 --plain`
**Then** plain text output is returned (e.g., `Dragged s1 to s2`)

### Generated Gherkin Preview

```gherkin
Feature: Mouse Interactions
  As a developer / automation engineer
  I want to simulate mouse interactions on page elements via the CLI
  So that my automation scripts can interact with web pages programmatically

  Background:
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements

  Scenario: Click an element by UID
    Given the page has a button with snapshot UID "s1"
    When I run "chrome-cli interact click s1"
    Then the output JSON should contain "clicked" equal to "s1"
    And the output JSON should contain "navigated" equal to false
    And the exit code should be 0

  Scenario: Click an element by CSS selector
    Given the page has a button matching "css:#submit-btn"
    When I run "chrome-cli interact click css:#submit-btn"
    Then the output JSON should contain "clicked" equal to "css:#submit-btn"

  Scenario: Click triggers navigation
    Given the page has a link that navigates away
    When I run "chrome-cli interact click s1"
    Then the output JSON should contain "navigated" equal to true
    And the output JSON "url" should be the new page URL

  Scenario: Double-click an element
    When I run "chrome-cli interact click s1 --double"
    Then the output JSON should contain "double_click" equal to true

  Scenario: Right-click an element
    When I run "chrome-cli interact click s1 --right"
    Then the output JSON should contain "right_click" equal to true

  Scenario: Click with include-snapshot
    When I run "chrome-cli interact click s1 --include-snapshot"
    Then the output JSON should contain a "snapshot" field

  Scenario: Click at viewport coordinates
    When I run "chrome-cli interact click-at 100 200"
    Then the output JSON "clicked_at.x" should be 100
    And the output JSON "clicked_at.y" should be 200

  Scenario: Click-at with double flag
    When I run "chrome-cli interact click-at 100 200 --double"
    Then the output JSON should contain "double_click" equal to true

  Scenario: Hover over an element by UID
    When I run "chrome-cli interact hover s3"
    Then the output JSON should contain "hovered" equal to "s3"

  Scenario: Hover with CSS selector
    When I run "chrome-cli interact hover css:.dropdown-trigger"
    Then the output JSON should contain "hovered" equal to "css:.dropdown-trigger"

  Scenario: Hover with include-snapshot
    When I run "chrome-cli interact hover s3 --include-snapshot"
    Then the output JSON should contain a "snapshot" field

  Scenario: Drag from one element to another
    When I run "chrome-cli interact drag s1 s2"
    Then the output JSON "dragged.from" should be "s1"
    And the output JSON "dragged.to" should be "s2"

  Scenario: Drag with CSS selectors
    When I run "chrome-cli interact drag css:#item css:#target"
    Then the output JSON "dragged.from" should be "css:#item"

  Scenario: Drag with include-snapshot
    When I run "chrome-cli interact drag s1 s2 --include-snapshot"
    Then the output JSON should contain a "snapshot" field

  Scenario: Click with tab targeting
    When I run "chrome-cli interact click s1 --tab ABC123"
    Then the click is performed on the specified tab

  Scenario: UID not found error
    When I run "chrome-cli interact click s99"
    Then stderr should contain "UID 's99' not found"
    And the exit code should be non-zero

  Scenario: CSS selector not found error
    When I run "chrome-cli interact click css:#nonexistent"
    Then stderr should contain "Element not found for selector"
    And the exit code should be non-zero

  Scenario: No snapshot state error
    Given no snapshot has been taken
    When I run "chrome-cli interact click s1"
    Then stderr should contain "page snapshot"
    And the exit code should be non-zero

  Scenario: Element scrolled into view before click
    Given an element is not visible in the viewport
    When I run "chrome-cli interact click s5"
    Then the element is scrolled into view and clicked

  Scenario: Plain text output for click
    When I run "chrome-cli interact click s1 --plain"
    Then the output should be plain text "Clicked s1"

  Scenario: Plain text output for hover
    When I run "chrome-cli interact hover s3 --plain"
    Then the output should be plain text "Hovered s3"

  Scenario: Plain text output for drag
    When I run "chrome-cli interact drag s1 s2 --plain"
    Then the output should be plain text "Dragged s1 to s2"
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `interact click <TARGET>` — click an element by UID or CSS selector | Must | Core mouse interaction |
| FR2 | `interact click-at <X> <Y>` — click at viewport coordinates | Must | Coordinate-based clicking |
| FR3 | `interact hover <TARGET>` — move cursor over element | Must | Hover triggers tooltips, menus |
| FR4 | `interact drag <FROM> <TO>` — drag between elements | Must | Drag-and-drop automation |
| FR5 | `--double` flag for double-click | Must | CDP `clickCount: 2` |
| FR6 | `--right` flag for right-click (context menu) | Must | CDP button parameter `2` |
| FR7 | `--include-snapshot` flag to return updated snapshot after interaction | Should | Convenience for agents |
| FR8 | Target resolution: `css:` prefix → CSS selector; UID pattern → snapshot lookup | Must | Dual target system |
| FR9 | Scroll element into view before interaction (`DOM.scrollIntoViewIfNeeded`) | Must | Elements may be off-screen |
| FR10 | Wait for navigation after click if navigation occurs | Must | Similar to navigate command |
| FR11 | Wait for DOM stability after interactions | Should | Similar to MCP's WaitForHelper |
| FR12 | JSON, pretty-JSON, and plain text output formats | Must | Consistent with all other commands |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Mouse interactions should complete within the command timeout (default 30s); coordinate resolution < 100ms |
| **Reliability** | Element must be scrolled into view and have non-zero dimensions before dispatching events |
| **Error handling** | Clear messages for: UID not found, CSS selector not found, no snapshot state, element has zero size |
| **Platforms** | macOS, Linux, Windows (consistent with all other commands) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| target | String | UID (e.g., `s1`) or CSS selector (prefixed `css:`) | Yes (for click, hover) |
| from | String | UID or CSS selector | Yes (for drag) |
| to | String | UID or CSS selector | Yes (for drag) |
| x | f64 | Positive number | Yes (for click-at) |
| y | f64 | Positive number | Yes (for click-at) |
| --double | Boolean flag | N/A | No |
| --right | Boolean flag | N/A | No |
| --include-snapshot | Boolean flag | N/A | No |
| --tab | String | Valid tab ID or index | No (defaults to active tab) |

### Output Data — `interact click`

| Field | Type | Description |
|-------|------|-------------|
| clicked | String | The target that was clicked (UID or CSS selector) |
| url | String | Current page URL after click |
| navigated | Boolean | Whether the click triggered a navigation |
| double_click | Boolean (optional) | Present and true when `--double` was used |
| right_click | Boolean (optional) | Present and true when `--right` was used |
| snapshot | Object (optional) | Updated accessibility snapshot (when `--include-snapshot`) |

### Output Data — `interact click-at`

| Field | Type | Description |
|-------|------|-------------|
| clicked_at | Object | `{"x": N, "y": N}` with the click coordinates |
| double_click | Boolean (optional) | Present and true when `--double` was used |
| right_click | Boolean (optional) | Present and true when `--right` was used |
| snapshot | Object (optional) | Updated accessibility snapshot (when `--include-snapshot`) |

### Output Data — `interact hover`

| Field | Type | Description |
|-------|------|-------------|
| hovered | String | The target that was hovered (UID or CSS selector) |
| snapshot | Object (optional) | Updated accessibility snapshot (when `--include-snapshot`) |

### Output Data — `interact drag`

| Field | Type | Description |
|-------|------|-------------|
| dragged | Object | `{"from": "...", "to": "..."}` identifying both targets |
| snapshot | Object (optional) | Updated accessibility snapshot (when `--include-snapshot`) |

---

## Dependencies

### Internal Dependencies
- [x] CDP client (`src/cdp/`) — WebSocket communication, event subscription
- [x] Connection resolution (`src/connection.rs`) — target selection, session management
- [x] Snapshot/UID system (`src/snapshot.rs`) — UID-to-backendDOMNodeId mapping
- [x] Output formatting — JSON/pretty/plain output patterns

### External Dependencies
- [x] Chrome DevTools Protocol — `Input` domain (`dispatchMouseEvent`), `DOM` domain (`getBoxModel`, `getContentQuads`, `scrollIntoViewIfNeeded`, `querySelector`, `resolveNode`, `describeNode`)

### Blocked By
- [x] Issue #4 (CDP client) — completed
- [x] Issue #6 (session management) — completed
- [x] Issue #10 (UID system) — completed

---

## Out of Scope

- Keyboard interactions (type text, key press) — separate issue
- Scroll commands (scroll page, scroll to element) — separate issue
- Touch/gesture events (tap, swipe, pinch) — not in this issue
- Form filling (select dropdown, toggle checkbox) — separate issue (#15)
- Mouse move without hover semantics (raw `Input.dispatchMouseEvent` with `mouseMoved`)
- Multi-step interaction sequences (click + wait + click) — users compose commands in scripts

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| All interaction types work | 4/4 (click, click-at, hover, drag) | BDD tests pass |
| UID and CSS selector targets | Both resolve correctly | Integration tests |
| Navigation detection | Click-triggered navigations are detected and waited for | BDD test with navigating link |
| Response time | < 500ms for element resolution + click dispatch | Manual timing |

---

## Open Questions

- (None — all requirements are clear from the issue and CDP documentation)

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
