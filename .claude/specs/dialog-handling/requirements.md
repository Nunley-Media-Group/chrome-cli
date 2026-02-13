# Requirements: Browser Dialog Handling

**Issue**: #20
**Date**: 2026-02-13
**Status**: Draft
**Author**: Claude (writing-specs)

---

## User Story

**As a** developer / automation engineer
**I want** to detect and handle browser dialogs (alerts, confirms, prompts, beforeunload) from the CLI
**So that** my automation scripts can respond to dialogs programmatically without being blocked by unexpected popups

---

## Background

Browser dialogs (`alert()`, `confirm()`, `prompt()`, `beforeunload`) block all further page interaction until they are handled. In automation scenarios, an unhandled dialog can stall an entire pipeline. The Chrome DevTools Protocol exposes the `Page.javascriptDialogOpening` event and `Page.handleJavaScriptDialog` method for this purpose. This feature adds a `dialog` command group to chrome-cli that lets users query open dialogs and accept/dismiss them, as well as a global `--auto-dismiss-dialogs` flag for fire-and-forget automation.

---

## Acceptance Criteria

### AC1: Accept an alert dialog

**Given** a page has triggered an `alert()` dialog
**When** I run `chrome-cli dialog handle accept`
**Then** the dialog is dismissed (accepted)
**And** JSON output is returned: `{"action": "accept", "dialog_type": "alert", "message": "..."}`
**And** the exit code is 0

**Example**:
- Given: page runs `alert("Hello")`
- When: `chrome-cli dialog handle accept`
- Then: `{"action": "accept", "dialog_type": "alert", "message": "Hello"}`

### AC2: Dismiss a confirm dialog

**Given** a page has triggered a `confirm()` dialog
**When** I run `chrome-cli dialog handle dismiss`
**Then** the dialog is dismissed (cancelled)
**And** JSON output is returned: `{"action": "dismiss", "dialog_type": "confirm", "message": "..."}`

**Example**:
- Given: page runs `confirm("Are you sure?")`
- When: `chrome-cli dialog handle dismiss`
- Then: `{"action": "dismiss", "dialog_type": "confirm", "message": "Are you sure?"}`

### AC3: Accept a prompt dialog with text

**Given** a page has triggered a `prompt()` dialog
**When** I run `chrome-cli dialog handle accept --text "my input"`
**Then** the dialog is accepted with the provided text as the prompt value
**And** JSON output is returned: `{"action": "accept", "dialog_type": "prompt", "message": "...", "text": "my input"}`

**Example**:
- Given: page runs `prompt("Enter name:", "default")`
- When: `chrome-cli dialog handle accept --text "Alice"`
- Then: `{"action": "accept", "dialog_type": "prompt", "message": "Enter name:", "text": "Alice"}`

### AC4: Handle a beforeunload dialog

**Given** a page has registered a `beforeunload` handler and navigation is triggered
**When** I run `chrome-cli dialog handle accept`
**Then** the beforeunload dialog is accepted (allowing navigation)
**And** JSON output is returned: `{"action": "accept", "dialog_type": "beforeunload", "message": "..."}`

### AC5: Query dialog info when a dialog is open

**Given** a page has triggered a `prompt()` dialog with message "Enter name:" and default value "default"
**When** I run `chrome-cli dialog info`
**Then** JSON output is returned: `{"open": true, "type": "prompt", "message": "Enter name:", "default_value": "default"}`

### AC6: Query dialog info when no dialog is open

**Given** no dialog is currently open on the page
**When** I run `chrome-cli dialog info`
**Then** JSON output is returned: `{"open": false}`

### AC7: Handle dialog with --tab flag

**Given** a dialog is open on a specific tab with ID "ABC123"
**When** I run `chrome-cli dialog handle accept --tab ABC123`
**Then** the dialog on that specific tab is handled
**And** the correct JSON output is returned

### AC8: Auto-dismiss dialogs flag

**Given** a page triggers one or more dialogs during a command
**When** I run any chrome-cli command with `--auto-dismiss-dialogs`
**Then** all dialogs that appear are automatically dismissed without blocking
**And** the primary command completes normally

### AC9: Handle dialog when no dialog is open (error)

**Given** no dialog is currently open on the page
**When** I run `chrome-cli dialog handle accept`
**Then** an error is returned to stderr: `{"error": "No dialog is currently open", "code": 1}`
**And** the exit code is non-zero

### AC10: Plain text output for dialog handle

**Given** a page has triggered an `alert()` dialog
**When** I run `chrome-cli dialog handle accept --plain`
**Then** plain text output is returned (e.g., `Accepted alert: "Hello"`)

### AC11: Plain text output for dialog info

**Given** a page has triggered a `confirm()` dialog with message "Continue?"
**When** I run `chrome-cli dialog info --plain`
**Then** plain text output is returned (e.g., `Dialog open: confirm — "Continue?"`)

### Generated Gherkin Preview

```gherkin
Feature: Browser Dialog Handling
  As a developer / automation engineer
  I want to detect and handle browser dialogs from the CLI
  So that my automation scripts can respond to dialogs programmatically

  Scenario: Accept an alert dialog
    Given a page has triggered an alert dialog with message "Hello"
    When I run "chrome-cli dialog handle accept"
    Then the output JSON should contain "action" equal to "accept"
    And the output JSON should contain "dialog_type" equal to "alert"
    And the output JSON should contain "message" equal to "Hello"
    And the exit code should be 0

  Scenario: Dismiss a confirm dialog
    Given a page has triggered a confirm dialog with message "Are you sure?"
    When I run "chrome-cli dialog handle dismiss"
    Then the output JSON should contain "action" equal to "dismiss"
    And the output JSON should contain "dialog_type" equal to "confirm"

  Scenario: Accept a prompt dialog with text
    Given a page has triggered a prompt dialog with message "Enter name:"
    When I run "chrome-cli dialog handle accept --text Alice"
    Then the output JSON should contain "action" equal to "accept"
    And the output JSON should contain "text" equal to "Alice"

  Scenario: Handle a beforeunload dialog
    Given a page has registered a beforeunload handler
    When a navigation is triggered and a beforeunload dialog appears
    And I run "chrome-cli dialog handle accept"
    Then the output JSON should contain "dialog_type" equal to "beforeunload"

  Scenario: Query dialog info when open
    Given a page has triggered a prompt dialog with message "Enter name:" and default "default"
    When I run "chrome-cli dialog info"
    Then the output JSON should contain "open" equal to true
    And the output JSON should contain "type" equal to "prompt"
    And the output JSON should contain "default_value" equal to "default"

  Scenario: Query dialog info when no dialog is open
    Given no dialog is currently open
    When I run "chrome-cli dialog info"
    Then the output JSON should contain "open" equal to false

  Scenario: Handle dialog with tab targeting
    Given a dialog is open on tab "ABC123"
    When I run "chrome-cli dialog handle accept --tab ABC123"
    Then the dialog on that tab is accepted

  Scenario: Auto-dismiss dialogs during a command
    Given a page will trigger an alert during navigation
    When I run "chrome-cli navigate https://example.com --auto-dismiss-dialogs"
    Then the navigation completes without blocking

  Scenario: Handle dialog when none is open
    Given no dialog is currently open
    When I run "chrome-cli dialog handle accept"
    Then stderr should contain an error about no dialog being open
    And the exit code should be non-zero

  Scenario: Plain text output for dialog handle
    Given a page has triggered an alert dialog with message "Hello"
    When I run "chrome-cli dialog handle accept --plain"
    Then the output should be plain text "Accepted alert: \"Hello\""

  Scenario: Plain text output for dialog info
    Given a page has triggered a confirm dialog with message "Continue?"
    When I run "chrome-cli dialog info --plain"
    Then the output should be plain text containing "confirm" and "Continue?"
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `dialog handle accept` — accept the current dialog | Must | Core CDP `Page.handleJavaScriptDialog(accept: true)` |
| FR2 | `dialog handle dismiss` — dismiss the current dialog | Must | CDP `Page.handleJavaScriptDialog(accept: false)` |
| FR3 | `dialog handle accept --text <TEXT>` — provide prompt text | Must | CDP `promptText` parameter |
| FR4 | `dialog info` — query whether a dialog is open and its details | Must | Track via `Page.javascriptDialogOpening` events |
| FR5 | `--auto-dismiss-dialogs` global flag | Should | Subscribe to `Page.javascriptDialogOpening` and auto-dismiss |
| FR6 | Support all dialog types: alert, confirm, prompt, beforeunload | Must | CDP `type` field in `Page.javascriptDialogOpening` |
| FR7 | JSON, pretty-JSON, and plain text output formats | Must | Consistent with all other commands |
| FR8 | `--tab <ID>` targeting for dialog commands | Must | Uses existing `GlobalOpts.tab` resolution |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Dialog handling should complete within the command timeout (default 30s) |
| **Reliability** | If a dialog event is missed (race condition), `dialog info` should probe the current state rather than relying solely on cached events |
| **Error handling** | Clear error message when attempting to handle a dialog that isn't open |
| **Platforms** | macOS, Linux, Windows (consistent with all other commands) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| action | `accept` or `dismiss` | Must be one of two values | Yes (for `handle`) |
| --text | String | Any UTF-8 string | No (only meaningful for prompt dialogs) |
| --tab | String | Valid tab ID or index | No (defaults to active tab) |
| --auto-dismiss-dialogs | Boolean flag | N/A | No |

### Output Data — `dialog handle`

| Field | Type | Description |
|-------|------|-------------|
| action | String | "accept" or "dismiss" |
| dialog_type | String | "alert", "confirm", "prompt", or "beforeunload" |
| message | String | The dialog message text |
| text | String (optional) | The text provided for prompt dialogs (only present when --text used) |

### Output Data — `dialog info`

| Field | Type | Description |
|-------|------|-------------|
| open | Boolean | Whether a dialog is currently open |
| type | String (optional) | Dialog type (only when open=true) |
| message | String (optional) | Dialog message (only when open=true) |
| default_value | String (optional) | Default prompt value (only when open=true and type=prompt) |

---

## Dependencies

### Internal Dependencies
- [x] CDP client (`src/cdp/`) — WebSocket communication, event subscription
- [x] Connection resolution (`src/connection.rs`) — target selection, session management
- [x] Output formatting — JSON/pretty/plain output patterns

### External Dependencies
- [x] Chrome DevTools Protocol — `Page` domain (`javascriptDialogOpening`, `handleJavaScriptDialog`)

### Blocked By
- [x] Issue #4 (CDP client) — completed
- [x] Issue #6 (session management) — completed

---

## Out of Scope

- Dialog **creation/triggering** from the CLI (use `js exec` for that)
- Custom dialog UI replacement (this is browser-native only)
- File upload dialogs (these are not JavaScript dialogs; they use `Page.fileChooserOpened`)
- HTTP authentication dialogs (these use `Fetch.authRequired`, a separate CDP domain)
- Persistent dialog auto-handling across CLI invocations (auto-dismiss is per-command only)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| All dialog types handled | 4/4 (alert, confirm, prompt, beforeunload) | BDD tests pass |
| Auto-dismiss works in automation | Dialogs don't block scripts | Integration test with auto-dismiss flag |
| Response time | < 200ms for dialog handle/info | Manual timing of CDP round-trip |

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
