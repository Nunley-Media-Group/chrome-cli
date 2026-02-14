# Requirements: Console Message Reading with Filtering

**Issue**: #18
**Date**: 2026-02-14
**Status**: Approved
**Author**: Claude (writing-specs)

---

## User Story

**As a** developer or automation engineer
**I want** to read and monitor browser console messages via the CLI with type filtering and pagination
**So that** I can debug web applications and monitor for errors directly from scripts and CI pipelines

---

## Background

Browser console messages are critical for debugging and monitoring web applications. The MCP server already provides `list_console_messages` and `get_console_message` tools with navigation-aware collection (preserving messages from the last 3 navigations). This feature exposes that capability through the CLI as a `console` command group with `read` and `follow` subcommands. The `read` subcommand lists/retrieves messages, while `follow` provides real-time streaming (tail -f style) for interactive debugging and CI monitoring.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: List console messages from current page

**Given** Chrome is running with CDP enabled and a page has generated console messages
**When** I run `chrome-cli console read`
**Then** the output is a JSON array of console messages
**And** each message contains `id`, `type`, `text`, `timestamp`, `url`, `line`, and `column` fields
**And** the exit code is 0

**Example**:
- Given: a page that has called `console.log("hello")`
- When: `chrome-cli console read`
- Then: `[{"id": 0, "type": "log", "text": "hello", "timestamp": "...", "url": "...", "line": 1, "column": 1}]`

### AC2: Filter console messages by type

**Given** Chrome is running with CDP enabled and a page has generated log, warn, and error messages
**When** I run `chrome-cli console read --type error,warn`
**Then** only messages with type "error" or "warn" are returned
**And** messages with type "log" are excluded

### AC3: Use errors-only shorthand filter

**Given** Chrome is running with CDP enabled and a page has generated log and error messages
**When** I run `chrome-cli console read --errors-only`
**Then** only messages with type "error" or "assert" are returned

### AC4: Limit number of returned messages

**Given** Chrome is running with CDP enabled and a page has generated more than 10 console messages
**When** I run `chrome-cli console read --limit 5`
**Then** at most 5 messages are returned
**And** the exit code is 0

### AC5: Paginate through console messages

**Given** Chrome is running with CDP enabled and a page has generated 20 console messages
**When** I run `chrome-cli console read --limit 10 --page 1`
**Then** messages 10-19 are returned (0-based page indexing)

### AC6: Include messages from previous navigations

**Given** Chrome is running with CDP enabled and console messages exist from a previous navigation
**When** I run `chrome-cli console read --include-preserved`
**Then** messages from both the current and previous navigations are included

### AC7: Target a specific tab

**Given** Chrome is running with CDP enabled and multiple tabs are open
**When** I run `chrome-cli console read --tab 2`
**Then** console messages from tab 2 are returned

### AC8: Get detailed information about a specific console message

**Given** Chrome is running with CDP enabled and console messages exist
**When** I run `chrome-cli console read 0`
**Then** the output contains detailed information including full args, stack trace, and source location
**And** the stack trace includes file, line, column, and function name for each frame

### AC9: Stack traces limited to 50 frames

**Given** Chrome is running with CDP enabled and a console message has a stack trace exceeding 50 frames
**When** I run `chrome-cli console read <MSG_ID>`
**Then** the stack trace is truncated to 50 frames

### AC10: Stream console messages in real-time

**Given** Chrome is running with CDP enabled and a page is loaded
**When** I run `chrome-cli console follow`
**And** the page generates a console.log("streaming")
**Then** the message is printed to stdout as it arrives (one per line)

### AC11: Stream with type filter

**Given** Chrome is running with CDP enabled and a page is loaded
**When** I run `chrome-cli console follow --type error`
**And** the page generates a console.log and a console.error
**Then** only the error message is printed to stdout

### AC12: Stream with errors-only filter

**Given** Chrome is running with CDP enabled and a page is loaded
**When** I run `chrome-cli console follow --errors-only`
**And** the page generates a console.log and a console.error
**Then** only the error message is printed to stdout

### AC13: Stream with timeout

**Given** Chrome is running with CDP enabled and a page is loaded
**When** I run `chrome-cli console follow --timeout 1000`
**Then** the command exits after 1000ms
**And** the exit code is 0 if no error-level messages were seen

### AC14: Stream returns non-zero exit code on errors

**Given** Chrome is running with CDP enabled and a page is loaded
**When** I run `chrome-cli console follow --timeout 2000`
**And** the page generates a console.error("failure")
**Then** the exit code is non-zero (indicating error-level messages were seen)

### AC15: Error cause chains for uncaught exceptions

**Given** Chrome is running with CDP enabled and a page throws an uncaught exception with a cause chain
**When** I run `chrome-cli console read`
**Then** the error message includes the full cause chain

### AC16: Default limit is 50 messages

**Given** Chrome is running with CDP enabled and a page has generated 100 console messages
**When** I run `chrome-cli console read` (without `--limit`)
**Then** at most 50 messages are returned

### AC17: Console read with no messages returns empty array

**Given** Chrome is running with CDP enabled and a page has generated no console messages
**When** I run `chrome-cli console read`
**Then** the output is an empty JSON array `[]`
**And** the exit code is 0

### AC18: Console read with invalid message ID errors

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli console read 9999`
**Then** the exit code is nonzero
**And** stderr contains an error that message ID 9999 was not found

### Generated Gherkin Preview

```gherkin
Feature: Console Message Reading with Filtering
  As a developer or automation engineer
  I want to read and monitor browser console messages via the CLI
  So that I can debug web applications and monitor for errors from scripts

  Scenario: List console messages from current page
    Given Chrome is running with CDP enabled
    And a page has generated console messages
    When I run "chrome-cli console read"
    Then the output is a JSON array of console messages
    And the exit code should be 0

  Scenario: Filter by message type
    Given Chrome is running with CDP enabled
    When I run "chrome-cli console read --type error,warn"
    Then only error and warn messages are returned

  Scenario: Stream console messages in real-time
    Given Chrome is running with CDP enabled
    When I run "chrome-cli console follow --timeout 2000"
    Then messages are printed as they arrive

  # ... all ACs become scenarios
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `console read` lists console messages with JSON output | Must | Paginated, filtered |
| FR2 | `console read <MSG_ID>` retrieves detailed single message | Must | Full args, stack trace |
| FR3 | `--type <TYPES>` filter by comma-separated message types | Must | log, error, warn, info, debug, dir, table, trace, assert, count, timeEnd |
| FR4 | `--errors-only` shorthand for `--type error,assert` | Must | |
| FR5 | `--limit <N>` maximum messages (default: 50) | Must | |
| FR6 | `--page <N>` pagination (0-based) | Must | |
| FR7 | `--include-preserved` include previous navigation messages | Must | |
| FR8 | `--tab <ID>` target a specific tab | Must | Follows existing tab targeting pattern |
| FR9 | `console follow` real-time streaming | Must | One message per line |
| FR10 | `console follow --timeout <MS>` auto-exit after timeout | Must | |
| FR11 | `console follow` non-zero exit on error messages | Must | Useful for CI |
| FR12 | Stack traces limited to 50 frames | Must | Match MCP behavior |
| FR13 | Error cause chains for uncaught exceptions | Should | Match MCP behavior |
| FR14 | Plain text output mode for `console read` | Must | Follows existing `--plain` pattern |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | `console read` should respond in < 500ms for up to 50 messages |
| **Performance** | `console follow` should have < 100ms latency from event to output |
| **Reliability** | Message collection must be navigation-aware (last 3 navigations) |
| **Reliability** | `console follow` must handle WebSocket disconnection gracefully |
| **Platforms** | macOS, Linux, Windows (all platforms supported by chrome-cli) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `MSG_ID` | positive integer | Must be a valid message index | No (positional) |
| `--type` | comma-separated string | Each value must be a valid console type | No |
| `--errors-only` | boolean flag | Conflicts with `--type` | No |
| `--limit` | positive integer | Must be > 0, default 50 | No |
| `--page` | non-negative integer | Must be >= 0, default 0 | No |
| `--include-preserved` | boolean flag | None | No |
| `--tab` | string (tab ID or index) | Must be a valid tab | No |
| `--timeout` | positive integer (ms) | Must be > 0 | No (follow only) |

### Output Data — List (console read)

| Field | Type | Description |
|-------|------|-------------|
| `id` | integer | Message index within the collection |
| `type` | string | Console message type (log, error, warn, etc.) |
| `text` | string | Formatted message text |
| `timestamp` | string | ISO 8601 timestamp |
| `url` | string | Source URL where message originated |
| `line` | integer | Source line number |
| `column` | integer | Source column number |

### Output Data — Detail (console read <ID>)

| Field | Type | Description |
|-------|------|-------------|
| `id` | integer | Message index |
| `type` | string | Console message type |
| `text` | string | Formatted message text |
| `timestamp` | string | ISO 8601 timestamp |
| `url` | string | Source URL |
| `line` | integer | Source line number |
| `column` | integer | Source column number |
| `args` | array | Full argument objects from the console call |
| `stackTrace` | array | Stack frames: `{file, line, column, functionName}` |

---

## Dependencies

### Internal Dependencies
- [x] Issue #4 — CDP client (implemented)
- [x] Issue #6 — Session management (implemented)

### External Dependencies
- Chrome/Chromium with CDP support (Runtime domain)

### Blocked By
- None (all dependencies resolved)

---

## Out of Scope

- Console message clearing / `console.clear()` passthrough
- Console message grouping (`console.group`/`console.groupEnd`)
- Network error messages (covered by future Network feature)
- Performance console entries (`console.time`/`console.timeEnd` aggregation beyond raw messages)
- Console message interception or modification
- Console message persistence across CLI invocations (messages live only in the CDP session)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| All BDD scenarios pass | 100% | `cargo test --test bdd` |
| Message format accuracy | All fields present and correctly typed | Compare output against CDP event data |
| Follow latency | < 100ms from console call to CLI output | Timestamp comparison |
| CI exit code reliability | Non-zero on any error message | Automated test |

---

## Open Questions

- (none — all resolved from issue context)

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
