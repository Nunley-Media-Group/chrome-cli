# Requirements: Form Input and Filling

**Issue**: #16
**Date**: 2026-02-13
**Status**: Approved
**Author**: Claude (nmg-sdlc)

---

## User Story

**As a** developer or automation engineer
**I want** to fill form fields, select dropdown options, and clear inputs via the CLI
**So that** my automation scripts can programmatically interact with web forms without simulating keystrokes

---

## Background

The MCP server exposes `fill` (single field) and `fill_form` (multiple fields) tools that set values directly on form elements. This is more reliable than simulating keystrokes via the `interact type` command because it directly sets element values and dispatches the appropriate DOM events (`input`, `change`) for framework compatibility (React, Vue, Angular).

The `form` subcommand provides three operations: `fill` (set a single field's value), `fill-many` (set multiple fields at once from JSON), and `clear` (reset a field's value). Fields are targeted by UID (from accessibility snapshot) or CSS selector.

---

## Acceptance Criteria

### AC1: Fill a text input field by UID

**Given** Chrome is running with a page containing a text input field
**And** an accessibility snapshot has been taken with UIDs assigned
**When** I run `chrome-cli form fill <UID> <VALUE>`
**Then** the input field's value is set to the provided value
**And** `input` and `change` events are dispatched on the element
**And** JSON output is returned: `{"filled": "<UID>", "value": "<VALUE>"}`
**And** the exit code is 0

### AC2: Fill a text input field by CSS selector

**Given** Chrome is running with a page containing a text input with id "email"
**When** I run `chrome-cli form fill css:#email user@example.com`
**Then** the input field's value is set to "user@example.com"
**And** JSON output is returned: `{"filled": "css:#email", "value": "user@example.com"}`

### AC3: Fill a select dropdown

**Given** Chrome is running with a page containing a `<select>` element with UID "s3"
**And** the select has options including one with value "option2"
**When** I run `chrome-cli form fill s3 option2`
**Then** the matching `<option>` is selected
**And** `change` events are dispatched
**And** JSON output is returned: `{"filled": "s3", "value": "option2"}`

### AC4: Fill a textarea

**Given** Chrome is running with a page containing a `<textarea>` element
**When** I run `chrome-cli form fill <UID> "Multi-line\ntext content"`
**Then** the textarea's value is set to the provided text
**And** appropriate events are dispatched

### AC5: Toggle a checkbox to checked

**Given** Chrome is running with a page containing an unchecked checkbox with UID "s5"
**When** I run `chrome-cli form fill s5 true`
**Then** the checkbox becomes checked
**And** `input` and `change` events are dispatched
**And** JSON output confirms the value

### AC6: Toggle a checkbox to unchecked

**Given** Chrome is running with a page containing a checked checkbox with UID "s5"
**When** I run `chrome-cli form fill s5 false`
**Then** the checkbox becomes unchecked

### AC7: Fill with --include-snapshot flag

**Given** Chrome is running with a page containing a form field
**When** I run `chrome-cli form fill <UID> <VALUE> --include-snapshot`
**Then** the JSON output includes a `snapshot` field with the updated accessibility tree
**And** the snapshot state file is updated with new UID mappings

### AC8: Fill multiple fields at once

**Given** Chrome is running with a page containing multiple form fields
**When** I run `chrome-cli form fill-many '[{"uid":"s1","value":"John"},{"uid":"s2","value":"Doe"}]'`
**Then** all specified fields are filled with their respective values
**And** JSON array output is returned with results for each field
**And** the exit code is 0

### AC9: Fill multiple fields from a file

**Given** Chrome is running with a page containing form fields
**And** a JSON file exists at `fields.json` with contents `[{"uid":"s1","value":"John"}]`
**When** I run `chrome-cli form fill-many --file fields.json`
**Then** the fields are filled from the file contents

### AC10: Clear a form field

**Given** Chrome is running with a page containing a text input with value "old" and UID "s1"
**When** I run `chrome-cli form clear s1`
**Then** the input field's value is set to empty string
**And** `input` and `change` events are dispatched
**And** JSON output is returned: `{"cleared": "s1"}`
**And** the exit code is 0

### AC11: Fill nonexistent UID returns error

**Given** Chrome is running with a snapshot taken
**When** I run `chrome-cli form fill s999 "value"`
**Then** the exit code is nonzero
**And** stderr contains an error about the UID not being found

### AC12: Fill without required arguments

**Given** chrome-cli is built
**When** I run `chrome-cli form fill`
**Then** the exit code is nonzero
**And** stderr contains usage information about required arguments

### AC13: Fill with --tab flag targets specific tab

**Given** Chrome is running with multiple tabs open
**And** a form field exists in a specific tab
**When** I run `chrome-cli form fill <UID> <VALUE> --tab <TAB_ID>`
**Then** the field is filled in the specified tab

### AC14: Fill dispatches events for framework compatibility

**Given** Chrome is running with a React-controlled input field
**When** I run `chrome-cli form fill <UID> <VALUE>`
**Then** the React state is updated to reflect the new value
**And** the `input` event is dispatched with `bubbles: true`

### AC15: Fill-many with --include-snapshot flag

**Given** Chrome is running with a page containing form fields
**When** I run `chrome-cli form fill-many '<JSON>' --include-snapshot`
**Then** the JSON output includes a `snapshot` field with the updated accessibility tree

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `form fill <UID> <VALUE>` sets field value by UID | Must | Core single-field fill |
| FR2 | `form fill css:<SELECTOR> <VALUE>` sets field value by CSS selector | Must | Alternative targeting |
| FR3 | Handle text inputs, password, email, number, date, etc. | Must | Standard input types |
| FR4 | Handle `<select>` elements by matching option value | Must | Select matching option |
| FR5 | Handle `<textarea>` elements | Must | Multi-line text support |
| FR6 | Handle checkboxes/radio with "true"/"false" values | Must | Boolean toggle |
| FR7 | Dispatch `input` and `change` events after fill | Must | Framework compatibility |
| FR8 | `form fill-many <JSON>` fills multiple fields | Must | Batch fill |
| FR9 | `form fill-many --file <PATH>` reads JSON from file | Must | File input support |
| FR10 | `form clear <UID>` clears a field value | Must | Reset field |
| FR11 | `--include-snapshot` returns updated snapshot | Must | Snapshot integration |
| FR12 | `--tab <ID>` targets specific tab | Must | Tab targeting (via global flag) |
| FR13 | Plain text output mode (`--plain`) | Should | Human-readable output |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Fill operation should complete in < 500ms for a single field |
| **Reliability** | Events must be dispatched so React/Vue/Angular bindings update |
| **Platforms** | macOS, Linux, Windows (all platforms Chrome supports) |
| **Error handling** | Clear error messages for invalid UIDs, selectors, or element types |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| target (uid or selector) | String | Must be valid UID format (s\d+) or css: prefix | Yes |
| value | String | Any string; "true"/"false" for checkboxes | Yes (fill) |
| json | String | Valid JSON array of {uid, value} objects | Yes (fill-many, unless --file) |
| file | PathBuf | Must exist and contain valid JSON | No (fill-many alt) |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| filled | String | Target identifier that was filled |
| value | String | Value that was set |
| cleared | String | Target identifier that was cleared (clear command) |
| snapshot | Object (optional) | Accessibility tree if --include-snapshot |

---

## Dependencies

### Internal Dependencies
- [x] #4 — CDP client (WebSocket communication)
- [x] #6 — Session management (connection resolution)
- [x] #10 — UID system (accessibility snapshot UIDs)

### External Dependencies
- Chrome/Chromium with CDP enabled

---

## Out of Scope

- File upload (`<input type="file">`) — separate feature
- Form submission (`<form>` submit action) — separate feature
- ContentEditable / rich text editors — not standard form elements
- Drag-and-drop into form fields
- Auto-fill / password manager integration

---

## Open Questions

None — all requirements are clear from the issue specification.

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Edge cases and error states specified
- [x] Dependencies identified
- [x] Out of scope defined
