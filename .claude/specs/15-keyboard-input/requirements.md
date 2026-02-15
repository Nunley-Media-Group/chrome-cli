# Requirements: Keyboard Input

**Issue**: #15
**Date**: 2026-02-13
**Status**: Approved
**Author**: Claude (writing-specs)

---

## User Story

**As a** developer / automation engineer
**I want** to simulate keyboard input (typing text and pressing keys/shortcuts) via the CLI
**So that** my automation scripts can fill in forms, trigger keyboard shortcuts, and interact with web pages programmatically

---

## Background

AI agents and automation scripts need to type text into form fields and press keyboard shortcuts (e.g., Control+A to select all, Enter to submit). The Chrome DevTools Protocol provides `Input.dispatchKeyEvent` for simulating keyboard events — `char` events for text input, and `keyDown`/`keyUp` sequences for key presses. The MCP server's `press_key` tool supports 200+ keys and key combinations using `+` as a separator. This feature brings equivalent capabilities to the CLI under the existing `interact` subcommand group, adding `type` and `key` commands alongside the mouse interaction commands from issue #14.

---

## Acceptance Criteria

### AC1: Type text into the focused element

**Given** a page with a focused text input
**When** I run `chrome-cli interact type "Hello World"`
**Then** the text "Hello World" is typed character-by-character
**And** JSON output is returned: `{"typed": "Hello World", "length": 11}`
**And** the exit code is 0

**Example**:
- Given: page has `<input>` with focus
- When: `chrome-cli interact type "Hello World"`
- Then: `{"typed": "Hello World", "length": 11}`

### AC2: Type with delay between keystrokes

**Given** a page with a focused text input
**When** I run `chrome-cli interact type "abc" --delay 50`
**Then** each character is typed with a 50ms pause between keystrokes
**And** JSON output is returned: `{"typed": "abc", "length": 3}`

### AC3: Type with include-snapshot flag

**Given** a page with a focused text input
**When** I run `chrome-cli interact type "test" --include-snapshot`
**Then** the text is typed
**And** the JSON output includes an updated accessibility snapshot in the `snapshot` field

### AC4: Type handles Unicode and special characters

**Given** a page with a focused text input
**When** I run `chrome-cli interact type "cafe\u0301"` (or other multi-byte/Unicode text)
**Then** each character is dispatched individually via `char` events
**And** special characters, Unicode, and multi-byte characters are handled correctly

### AC5: Press a single key

**Given** a page with a focused element
**When** I run `chrome-cli interact key Enter`
**Then** the Enter key is pressed (keyDown + keyUp sequence)
**And** JSON output is returned: `{"pressed": "Enter"}`
**And** the exit code is 0

### AC6: Press a key combination with modifiers

**Given** a page with a text input containing selected text
**When** I run `chrome-cli interact key "Control+A"`
**Then** the Control+A combination is pressed (Control keyDown, A keyDown+keyUp, Control keyUp)
**And** JSON output is returned: `{"pressed": "Control+A"}`

**Example**:
- Given: page has `<input value="Hello">` with focus
- When: `chrome-cli interact key "Control+A"`
- Then: `{"pressed": "Control+A"}`

### AC7: Press a key with multiple modifiers

**Given** a page with an element listening for key events
**When** I run `chrome-cli interact key "Control+Shift+ArrowDown"`
**Then** both modifiers are held while ArrowDown is pressed
**And** JSON output is returned: `{"pressed": "Control+Shift+ArrowDown"}`

### AC8: Press a key with repeat flag

**Given** a page with a focused element
**When** I run `chrome-cli interact key ArrowDown --repeat 5`
**Then** the ArrowDown key is pressed 5 times
**And** JSON output is returned: `{"pressed": "ArrowDown", "repeat": 5}`

### AC9: Key press with include-snapshot flag

**Given** a page with a focused element
**When** I run `chrome-cli interact key Enter --include-snapshot`
**Then** the key is pressed
**And** the JSON output includes an updated accessibility snapshot in the `snapshot` field

### AC10: Invalid key name error

**Given** the user provides an invalid key name
**When** I run `chrome-cli interact key "InvalidKey"`
**Then** an error is returned: `Invalid key: 'InvalidKey'`
**And** the exit code is non-zero

### AC11: Duplicate modifier error

**Given** the user provides a key combination with duplicate modifiers
**When** I run `chrome-cli interact key "Control+Control+A"`
**Then** an error is returned: `Duplicate modifier: 'Control'`
**And** the exit code is non-zero

### AC12: Type command requires text argument

**Given** no text argument is provided
**When** I run `chrome-cli interact type`
**Then** an error is returned indicating the required argument is missing
**And** the exit code is non-zero

### AC13: Key command requires keys argument

**Given** no keys argument is provided
**When** I run `chrome-cli interact key`
**Then** an error is returned indicating the required argument is missing
**And** the exit code is non-zero

### AC14: Supported key categories

**Given** the tool supports 100+ keys matching the MCP server
**When** I run `chrome-cli interact key <KEY>` with any supported key name
**Then** the key is pressed correctly via CDP `Input.dispatchKeyEvent`

Supported key categories:
- Letters: a-z, A-Z
- Numbers: 0-9
- Function keys: F1-F24
- Modifiers: Shift, Control, Alt, Meta
- Navigation: ArrowUp/Down/Left/Right, Home, End, PageUp, PageDown
- Editing: Backspace, Delete, Insert, Tab, Enter, Escape
- Whitespace: Space
- Numpad: Numpad0-9, NumpadAdd, NumpadSubtract, NumpadMultiply, NumpadDivide, NumpadDecimal, NumpadEnter
- Media: MediaPlayPause, MediaStop, MediaTrackNext, MediaTrackPrevious, AudioVolumeUp, AudioVolumeDown, AudioVolumeMute
- Symbols: Minus, Equal, BracketLeft, BracketRight, Backslash, Semicolon, Quote, Backquote, Comma, Period, Slash
- Lock keys: CapsLock, NumLock, ScrollLock
- Other: ContextMenu, PrintScreen, Pause

### AC15: Plain text output for type command

**Given** a page with a focused text input
**When** I run `chrome-cli interact type "Hello" --plain`
**Then** plain text output is returned: `Typed 5 characters`

### AC16: Plain text output for key command

**Given** a page with a focused element
**When** I run `chrome-cli interact key Enter --plain`
**Then** plain text output is returned: `Pressed Enter`

### AC17: Tab targeting for type command

**Given** a specific tab with ID "ABC123" contains a focused input
**When** I run `chrome-cli interact type "Hello" --tab ABC123`
**Then** the text is typed in that specific tab

### AC18: Tab targeting for key command

**Given** a specific tab with ID "ABC123" contains a focused element
**When** I run `chrome-cli interact key Enter --tab ABC123`
**Then** the key is pressed in that specific tab

### Generated Gherkin Preview

```gherkin
Feature: Keyboard Input
  As a developer / automation engineer
  I want to simulate keyboard input via the CLI
  So that my automation scripts can type text and press keys programmatically

  Scenario: Type text into the focused element
    Given a page with a focused text input
    When I run "chrome-cli interact type 'Hello World'"
    Then the output JSON should contain "typed" equal to "Hello World"
    And the output JSON should contain "length" equal to 11
    And the exit code should be 0

  Scenario: Type with delay between keystrokes
    Given a page with a focused text input
    When I run "chrome-cli interact type 'abc' --delay 50"
    Then the output JSON should contain "typed" equal to "abc"

  Scenario: Press a single key
    Given a page with a focused element
    When I run "chrome-cli interact key Enter"
    Then the output JSON should contain "pressed" equal to "Enter"

  Scenario: Press a key combination
    When I run "chrome-cli interact key Control+A"
    Then the output JSON should contain "pressed" equal to "Control+A"

  Scenario: Press key with repeat
    When I run "chrome-cli interact key ArrowDown --repeat 5"
    Then the output JSON should contain "pressed" equal to "ArrowDown"
    And the output JSON should contain "repeat" equal to 5

  Scenario: Invalid key name
    When I run "chrome-cli interact key InvalidKey"
    Then stderr should contain "Invalid key"
    And the exit code should be non-zero

  Scenario: Duplicate modifier
    When I run "chrome-cli interact key Control+Control+A"
    Then stderr should contain "Duplicate modifier"
    And the exit code should be non-zero
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `interact type <TEXT>` — type text character-by-character via `char` events | Must | Core text input |
| FR2 | `interact key <KEYS>` — press a key or key combination via `keyDown`/`keyUp` | Must | Core key pressing |
| FR3 | `--delay <MS>` flag for type command — delay between keystrokes | Must | Default 0 for instant |
| FR4 | `--repeat <N>` flag for key command — press multiple times | Must | Default 1 |
| FR5 | `--include-snapshot` flag for both commands | Should | Convenience for agents |
| FR6 | Key combination parsing: split by `+`, validate each part, reject duplicates | Must | Format: "Control+A" |
| FR7 | 100+ supported key names matching MCP server's key set | Must | Letters, digits, function keys, modifiers, navigation, editing, numpad, media, symbols |
| FR8 | CDP key/code mapping: correct `key`, `code`, `modifiers` values | Must | Proper key event dispatch |
| FR9 | Modifier key sequencing: press modifiers down, press key, release key, release modifiers | Must | Correct event order |
| FR10 | Unicode and multi-byte character support in type command | Must | `char` events handle all characters |
| FR11 | Clear error messages for invalid key names | Must | Lists the invalid key |
| FR12 | JSON, pretty-JSON, and plain text output formats | Must | Consistent with all other commands |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Typing and key presses should complete within the command timeout (default 30s); individual key dispatch < 10ms per event |
| **Reliability** | Key validation happens before connecting to Chrome; early fail for invalid keys |
| **Error handling** | Clear messages for: invalid key name, duplicate modifier, missing required arguments |
| **Platforms** | macOS, Linux, Windows (consistent with all other commands) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| text | String | Non-empty, any characters including Unicode | Yes (for type) |
| keys | String | Valid key name(s) separated by `+` | Yes (for key) |
| --delay | u64 | Non-negative milliseconds | No (default: 0) |
| --repeat | u32 | Positive integer | No (default: 1) |
| --include-snapshot | Boolean flag | N/A | No |
| --tab | String | Valid tab ID or index | No (defaults to active tab) |

### Output Data — `interact type`

| Field | Type | Description |
|-------|------|-------------|
| typed | String | The text that was typed |
| length | usize | Number of characters typed |
| snapshot | Object (optional) | Updated accessibility snapshot (when `--include-snapshot`) |

### Output Data — `interact key`

| Field | Type | Description |
|-------|------|-------------|
| pressed | String | The key/combination that was pressed (e.g., "Control+A") |
| repeat | u32 (optional) | Present only when `--repeat` > 1 |
| snapshot | Object (optional) | Updated accessibility snapshot (when `--include-snapshot`) |

---

## Dependencies

### Internal Dependencies
- [x] CDP client (`src/cdp/`) — WebSocket communication
- [x] Connection resolution (`src/connection.rs`) — target selection, session management
- [x] Interact command module (`src/interact.rs`) — extends existing mouse interaction code
- [x] Snapshot system (`src/snapshot.rs`) — for `--include-snapshot`
- [x] Output formatting — JSON/pretty/plain output patterns

### External Dependencies
- [x] Chrome DevTools Protocol — `Input` domain (`dispatchKeyEvent`)

### Blocked By
- [x] Issue #4 (CDP client) — completed
- [x] Issue #6 (session management) — completed

---

## Out of Scope

- Keyboard shortcut interception/monitoring (reading key events, not sending them)
- Text input via `Runtime.evaluate` (element.value = ...) — this feature uses CDP key events
- IME (Input Method Editor) composition events
- Auto-detection of the focused element (user must ensure element is focused)
- Form field filling by selector (separate issue — `form` subcommand group)
- Key repeat held down (continuous keyDown without keyUp) — only discrete presses

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| All key categories work | All 100+ keys press correctly | BDD tests pass |
| Text typing works | Characters are typed via char events | Integration tests |
| Key combinations work | Modifier + key sequences dispatched correctly | BDD test with Control+A |
| Response time | < 100ms per key press (excluding delay) | Manual timing |

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
