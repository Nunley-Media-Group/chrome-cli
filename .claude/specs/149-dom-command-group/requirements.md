# Requirements: DOM Command Group

**Issue**: #149
**Date**: 2026-02-19
**Status**: Draft
**Author**: Claude (writing-specs)

---

## User Story

**As a** developer or AI agent automating Chrome via the CLI
**I want** a `dom` command group with subcommands for querying, navigating, inspecting, styling, and manipulating DOM elements
**So that** I can interact with page structure directly without falling back to raw JavaScript execution

---

## Background

The `dom` command is listed in `chrome-cli --help` with a full description but returns `{"error":"dom: not yet implemented","code":1}` when invoked. The placeholder was originally defined in the CLI skeleton (spec `.claude/specs/3-cli-skeleton/`) and given help text in spec `.claude/specs/26-comprehensive-help-text/`. The closely related `page find` command (spec `.claude/specs/11-element-finding/`) provides accessibility-tree-based element finding but not raw DOM manipulation.

During hands-on testing of https://www.saucedemo.com/, agents had to use `js exec "document.querySelector(...)"` for all DOM queries. The CDP protocol provides direct DOM methods (`DOM.querySelector`, `DOM.querySelectorAll`, `DOM.getAttributes`, `DOM.getOuterHTML`, `DOM.setAttributeValue`, `DOM.removeNode`, `CSS.getComputedStyleForNode`) that map naturally to CLI subcommands. Implementing the `dom` group fills the gap between the accessibility-tree-based `page find`/`page snapshot` commands and raw `js exec`, giving agents structured, type-safe DOM operations.

### Related Specs

| Spec | Relationship |
|------|-------------|
| `.claude/specs/3-cli-skeleton/` | Originally defined the `Dom` command variant as a placeholder |
| `.claude/specs/11-element-finding/` | Implements `page find` — accessibility-tree-based element finding; complementary to DOM queries |
| `.claude/specs/26-comprehensive-help-text/` | Defined the `dom` command help text (to be updated by this spec) |

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Select elements by CSS selector

**Given** a connected Chrome session on a page with DOM elements (e.g., `https://example.com`)
**When** I run `chrome-cli dom select "h1"`
**Then** the command returns a JSON array of matching elements on stdout
**And** each element includes `nodeId` (integer), `tag` (string), `attributes` (object), and `textContent` (string)
**And** the exit code is 0

### AC2: Select elements by XPath

**Given** a connected Chrome session on a page with DOM elements
**When** I run `chrome-cli dom select --xpath "//h1"`
**Then** the command returns matching elements in the same structured JSON format as CSS selection
**And** the exit code is 0

### AC3: Get element attribute

**Given** a connected Chrome session on a page with a link element `<a href="https://www.iana.org/domains/example">More information...</a>`
**When** I run `chrome-cli dom get-attribute <nodeId> href`
**Then** the command returns `{"attribute":"href","value":"https://www.iana.org/domains/example"}` on stdout
**And** the exit code is 0

### AC4: Get element text content

**Given** a connected Chrome session on a page with heading `<h1>Example Domain</h1>`
**When** I run `chrome-cli dom get-text <nodeId>`
**Then** the command returns `{"textContent":"Example Domain"}` on stdout
**And** the exit code is 0

### AC5: Get element HTML

**Given** a connected Chrome session on a page with elements
**When** I run `chrome-cli dom get-html <nodeId>`
**Then** the command returns `{"outerHTML":"<h1>Example Domain</h1>"}` on stdout
**And** the exit code is 0

### AC6: Set element attribute

**Given** a connected Chrome session on a page with an element
**When** I run `chrome-cli dom set-attribute <nodeId> class "new-class"`
**Then** the element's attribute is updated in the DOM
**And** the command returns `{"success":true,"nodeId":<nodeId>,"attribute":"class","value":"new-class"}` on stdout
**And** a subsequent `dom get-attribute <nodeId> class` confirms the change

### AC7: Set element text content

**Given** a connected Chrome session on a page with a text element
**When** I run `chrome-cli dom set-text <nodeId> "new text"`
**Then** the element's text content is updated in the DOM
**And** the command returns `{"success":true,"nodeId":<nodeId>,"textContent":"new text"}` on stdout
**And** a subsequent `dom get-text <nodeId>` confirms the change

### AC8: Remove element

**Given** a connected Chrome session on a page with an element
**When** I run `chrome-cli dom remove <nodeId>`
**Then** the element is removed from the DOM
**And** the command returns `{"success":true,"nodeId":<nodeId>,"removed":true}` on stdout
**And** a subsequent `dom select` for that element returns an empty array

### AC9: Select with no matches returns empty array

**Given** a connected Chrome session on a page
**When** I run `chrome-cli dom select ".nonexistent-class"`
**Then** the command returns `[]` on stdout (empty JSON array)
**And** the exit code is 0 (not an error)

### AC10: Invalid nodeId returns target error

**Given** a connected Chrome session
**When** I run `chrome-cli dom get-attribute 999999 href`
**Then** the command returns a descriptive error on stderr with exit code 3 (target error)

### AC11: UID-based targeting

**Given** a connected Chrome session on a page where `page snapshot` has been run, assigning UIDs to interactive elements
**When** I run `chrome-cli dom get-text s1` (using a snapshot UID instead of a raw nodeId)
**Then** the UID is resolved to its backend DOM node and the text content is returned
**And** the behavior is identical to using a raw nodeId

### AC12: Cross-validate mutation via independent read

**Given** a connected Chrome session on a page with an element
**When** I run `chrome-cli dom set-attribute <nodeId> data-test "hello"`
**And** I then run `chrome-cli dom get-attribute <nodeId> data-test`
**Then** the get-attribute command returns `{"attribute":"data-test","value":"hello"}`

### AC13: Get computed CSS styles

**Given** a connected Chrome session on a page with a styled element (e.g., `<h1>` on `https://example.com`)
**When** I run `chrome-cli dom get-style <nodeId>`
**Then** the command returns a JSON object with computed CSS property key-value pairs on stdout
**And** the output includes common properties such as `color`, `font-size`, `display`
**And** the exit code is 0

### AC14: Get specific CSS property

**Given** a connected Chrome session on a page with a styled element
**When** I run `chrome-cli dom get-style <nodeId> display`
**Then** the command returns only the requested property: `{"property":"display","value":"block"}` on stdout
**And** the exit code is 0

### AC15: Set inline CSS style

**Given** a connected Chrome session on a page with an element
**When** I run `chrome-cli dom set-style <nodeId> "color: red; font-weight: bold"`
**Then** the element's inline style is updated in the DOM
**And** the command returns `{"success":true,"nodeId":<nodeId>,"style":"color: red; font-weight: bold"}` on stdout
**And** a subsequent `dom get-style <nodeId> color` reflects the change

### AC16: Get parent element

**Given** a connected Chrome session on a page where `<h1>` is a child of `<div><h1>Example Domain</h1></div>`
**When** I run `chrome-cli dom parent <nodeId>` (where nodeId refers to the `<h1>`)
**Then** the command returns the parent element in the same structured format as `dom select` (nodeId, tag, attributes, textContent)
**And** the exit code is 0

### AC17: Get child elements

**Given** a connected Chrome session on a page with a `<div>` containing multiple child elements
**When** I run `chrome-cli dom children <nodeId>` (where nodeId refers to the `<div>`)
**Then** the command returns a JSON array of direct child elements in document order
**And** each child includes `nodeId`, `tag`, `attributes`, and `textContent`
**And** the exit code is 0

### AC18: Get sibling elements

**Given** a connected Chrome session on a page with an element that has siblings
**When** I run `chrome-cli dom siblings <nodeId>`
**Then** the command returns a JSON array of sibling elements (same parent, excluding the target element itself)
**And** each sibling includes `nodeId`, `tag`, `attributes`, and `textContent`
**And** the exit code is 0

### AC19: DOM tree visualization

**Given** a connected Chrome session on a page loaded at `https://example.com`
**When** I run `chrome-cli dom tree`
**Then** the command returns an indented text representation of the DOM tree on stdout
**And** each node shows its tag name, key attributes (id, class), and a truncated text preview
**And** the tree is indented to reflect nesting depth
**And** the exit code is 0

### AC20: DOM tree with depth limit

**Given** a connected Chrome session on a page
**When** I run `chrome-cli dom tree --depth 2`
**Then** the tree output includes only nodes up to 2 levels deep from the root
**And** deeper subtrees are indicated with an ellipsis marker (e.g., `...`)

### AC21: DOM tree rooted at a specific element

**Given** a connected Chrome session on a page
**When** I run `chrome-cli dom tree --root <nodeId>` (or `--root "div.container"` as a CSS selector)
**Then** the tree output starts from the specified element instead of `<html>`
**And** the output structure is identical to a full tree but scoped to that subtree

### AC22: Parent of root element returns error

**Given** a connected Chrome session on a page
**When** I run `chrome-cli dom parent <nodeId>` where nodeId is the `<html>` root element
**Then** the command returns a descriptive error on stderr indicating the element has no parent
**And** the exit code is 3 (target error)

### Generated Gherkin Preview

```gherkin
Feature: DOM command group
  As a developer or AI agent automating Chrome via the CLI
  I want a dom command group for querying, navigating, and manipulating DOM elements
  So that I can interact with page structure without raw JavaScript

  Background:
    Given Chrome is connected with a page loaded

  Scenario: Select elements by CSS selector
    When I run "chrome-cli dom select \"h1\""
    Then stdout is a JSON array of matching elements
    And each element has nodeId, tag, attributes, and textContent

  Scenario: Select elements by XPath
    When I run "chrome-cli dom select --xpath \"//h1\""
    Then stdout is a JSON array in the same format as CSS selection

  # ... all 22 ACs become scenarios
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Implement `dom select` subcommand with CSS selector support via `DOM.querySelectorAll` | Must | Default selection mode |
| FR2 | Implement `dom select --xpath` for XPath queries via `DOM.performSearch` + `DOM.getSearchResults` | Must | Alternative to CSS |
| FR3 | Implement `dom get-attribute` to read a single element attribute via `DOM.getAttributes` | Must | |
| FR4 | Implement `dom get-text` to read element text content via `Runtime.evaluate` on the node | Must | Uses `node.textContent` |
| FR5 | Implement `dom get-html` to read element outerHTML via `DOM.getOuterHTML` | Must | |
| FR6 | Implement `dom set-attribute` to modify an attribute via `DOM.setAttributeValue` | Must | |
| FR7 | Implement `dom set-text` via `Runtime.evaluate` to set `node.textContent` | Must | |
| FR8 | Implement `dom remove` to remove an element via `DOM.removeNode` | Must | |
| FR9 | Support UID-based targeting (e.g., `s1`) consistent with `form fill`, `js exec --uid` | Should | Reuse `is_uid` + snapshot `uid_map` pattern from `src/form.rs` |
| FR10 | All output follows existing JSON stdout / JSON stderr conventions | Must | Structured output per `tech.md` |
| FR11 | Add `DomArgs` / `DomCommand` subcommand enum to `src/cli/mod.rs` replacing bare `Dom` variant | Must | Follow pattern of `PageArgs`/`PageCommand` |
| FR12 | Implement `dom get-style` to read computed CSS styles via `CSS.getComputedStyleForNode` | Must | Optional single-property filter |
| FR13 | Implement `dom set-style` to set inline styles via `DOM.setAttributeValue` on `style` attribute | Must | Accepts CSS text string |
| FR14 | Implement `dom parent` to navigate to a node's parent via `DOM.describeNode` | Must | Returns parent in same format as `select` |
| FR15 | Implement `dom children` to list direct child elements via `DOM.requestChildNodes` / `DOM.describeNode` | Must | Returns array in document order |
| FR16 | Implement `dom siblings` to list sibling elements (derive from parent's children, excluding self) | Must | |
| FR17 | Implement `dom tree` to display a pretty-printed DOM tree rooted at `<html>` or a given node | Must | Indented text output, not JSON |
| FR18 | `dom tree` supports `--depth N` to limit traversal depth | Should | Ellipsis marker for truncated subtrees |
| FR19 | `dom tree` supports `--root <nodeId-or-selector>` to scope the tree to a subtree | Should | Accepts nodeId, UID, or CSS selector |
| FR20 | Update `dom` command `long_about` and `after_long_help` in `src/cli/mod.rs` to list all subcommands with examples | Must | Remove "not yet implemented" caveat |
| FR21 | Update `dom` entry in `src/examples.rs` to replace placeholder examples with working subcommand examples | Must | |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `selector` (positional, `dom select`) | String | Valid CSS selector or XPath expression | Yes |
| `--xpath` flag (`dom select`) | Boolean | N/A | No (default: CSS) |
| `node-id` (positional, get/set/remove/nav) | String | Integer nodeId or UID format `s\d+` | Yes |
| `attribute` (positional, get/set-attribute) | String | Non-empty attribute name | Yes |
| `value` (positional, set-attribute/set-text) | String | Any string | Yes |
| `property` (positional, `dom get-style`, optional) | String | CSS property name (e.g., `display`, `color`) | No (omit for all computed styles) |
| `style` (positional, `dom set-style`) | String | CSS declarations text (e.g., `"color: red; font-weight: bold"`) | Yes |
| `--depth` flag (`dom tree`) | Integer | Positive integer | No (default: unlimited) |
| `--root` flag (`dom tree`) | String | nodeId, UID, or CSS selector | No (default: document root) |

### Output Data — `dom select`

| Field | Type | Description |
|-------|------|-------------|
| `nodeId` | Integer | CDP backend node ID for subsequent commands |
| `tag` | String | HTML tag name (lowercase, e.g., "h1", "div") |
| `attributes` | Object | Key-value map of HTML attributes |
| `textContent` | String | Element's text content (trimmed) |

### Output Data — `dom get-attribute`

| Field | Type | Description |
|-------|------|-------------|
| `attribute` | String | The requested attribute name |
| `value` | String | The attribute value |

### Output Data — `dom get-text`

| Field | Type | Description |
|-------|------|-------------|
| `textContent` | String | The element's text content |

### Output Data — `dom get-html`

| Field | Type | Description |
|-------|------|-------------|
| `outerHTML` | String | The element's outer HTML markup |

### Output Data — Mutation commands (set-attribute, set-text, set-style, remove)

| Field | Type | Description |
|-------|------|-------------|
| `success` | Boolean | Always `true` on success (errors go to stderr) |
| `nodeId` | Integer | The target node's ID |
| *(varies)* | *(varies)* | Command-specific confirmation fields |

### Output Data — `dom get-style` (all properties)

| Field | Type | Description |
|-------|------|-------------|
| `styles` | Object | Key-value map of computed CSS properties |

### Output Data — `dom get-style <property>` (single property)

| Field | Type | Description |
|-------|------|-------------|
| `property` | String | The requested CSS property name |
| `value` | String | The computed value of the property |

### Output Data — `dom parent` / `dom children` / `dom siblings`

Same element format as `dom select`:

| Field | Type | Description |
|-------|------|-------------|
| `nodeId` | Integer | CDP backend node ID |
| `tag` | String | HTML tag name (lowercase) |
| `attributes` | Object | Key-value map of HTML attributes |
| `textContent` | String | Element's text content (trimmed) |

`parent` returns a single object; `children` and `siblings` return arrays.

### Output Data — `dom tree`

Plain text (not JSON) indented tree representation. Each line:

```
<indent><tag>#<id>.<class> "truncated text..."
```

Example:
```
html
  head
    title "Example Domain"
  body
    div
      h1 "Example Domain"
      p "This domain is for use..."
      p
        a.href="https://www.iana.org/..." "More information..."
```

---

## Dependencies

### Internal Dependencies
- [x] CDP client (`src/cdp/client.rs`) — `send_command` for DOM/CSS/Runtime methods
- [x] Session management (`src/session.rs`) — connection resolution
- [x] Snapshot UID map (`src/snapshot.rs`) — UID-to-backendNodeId resolution for `s\d+` targeting
- [x] Error types (`src/error.rs`) — `AppError` with exit codes

### External Dependencies
- [x] Chrome DevTools Protocol — DOM, CSS, and Runtime domains

### Blocked By
- None — all infrastructure exists

---

## Out of Scope

- DOM event listeners or mutation observers
- Shadow DOM traversal
- Batch/bulk DOM operations in a single invocation
- DOM change watching/following (analogous to `console follow`)

---

## Validation Checklist

Before moving to PLAN phase:

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements (CDP methods noted for traceability, not as mandates)
- [x] All criteria are testable and unambiguous
- [x] Edge cases and error states specified (AC9, AC10, AC22)
- [x] Dependencies identified
- [x] Out of scope is defined
- [x] Cross-validation AC included per retrospective learning (AC12)
- [x] UID-based targeting specified for consistency with existing commands (AC11)
- [x] CLI help documentation updates required (FR20, FR21)
