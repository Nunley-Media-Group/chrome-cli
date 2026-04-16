# File: tests/features/dom-events.feature
#
# Generated from: .claude/specs/feature-add-dom-events-command-for-event-listener-introspection/requirements.md
# Issue: #192

Feature: DOM Events Command
  As a browser automation engineer debugging interaction failures
  I want to see what event listeners are attached to a DOM element
  So that I can understand how events are handled and choose the correct interaction approach

  Background:
    Given agentchrome is built

  # --- Happy Path ---

  # AC1: List event listeners on an element via addEventListener
  Scenario: List event listeners registered via addEventListener
    Given a connected Chrome session on a page with event listeners
    When I run "agentchrome dom events css:#btn-addeventlistener"
    Then the output is valid JSON
    And the output JSON contains a "listeners" array
    And each listener object contains "type", "useCapture", "once", "passive" fields
    And each listener object contains a "handler" object
    And each handler contains "description" as a non-empty string
    And each handler contains "scriptId" as a string or null
    And each handler contains "lineNumber" as an integer or null
    And each handler contains "columnNumber" as an integer or null
    And the exit code should be 0

  # AC2: Inline event handlers included
  Scenario: Inline event handlers are included in listeners
    Given a connected Chrome session on a page with an inline onclick handler
    When I run "agentchrome dom events css:#btn-inline"
    Then the output JSON "listeners" array is not empty
    And at least one listener has "type" equal to "click"
    And the exit code should be 0

  # --- Alternative Paths ---

  # AC3: Frame-scoped event introspection
  Scenario: Event introspection within a frame
    Given a connected Chrome session on a page with an iframe containing event listeners
    When I run "agentchrome dom --frame 0 events css:#framed-btn"
    Then the output JSON contains a "listeners" array
    And the listeners reflect the handlers attached in the frame context
    And the exit code should be 0

  # --- Edge Cases ---

  # AC4: Element with no listeners
  Scenario: Element with no event listeners returns empty array
    Given a connected Chrome session on a page with a plain element
    When I run "agentchrome dom events css:#no-listeners"
    Then the output JSON contains "listeners" as an empty array
    And the exit code should be 0

  # AC5: Handler source location unavailable
  Scenario: Handler source location fields are null when unavailable
    Given a connected Chrome session on a page with event listeners
    When I run "agentchrome dom events css:#btn-addeventlistener"
    Then for any listener where source location is unavailable, "handler.scriptId" is null
    And "handler.lineNumber" is null
    And "handler.columnNumber" is null
    And "handler.description" is still a non-empty string

  # --- Output Format ---

  # AC6: Output format compliance
  Scenario: Plain text output format
    Given a connected Chrome session on a page with event listeners
    When I run "agentchrome dom events css:#btn-addeventlistener --plain"
    Then the output is human-readable plain text with one listener per line
    And each line contains the event type, capture, once, passive flags, and handler description
    And the exit code should be 0

  Scenario: JSON output format
    Given a connected Chrome session on a page with event listeners
    When I run "agentchrome dom events css:#btn-addeventlistener --json"
    Then the output is valid JSON on stdout
    And the exit code should be 0

  # --- CLI Testable ---

  # AC6 (partial): Help text validation
  Scenario: Help text includes event listener description
    When I run "agentchrome dom events --help"
    Then the exit code should be 0
    And stdout should contain "event listeners"
    And stdout should contain "addEventListener"

  # AC7: Documentation updated
  Scenario: Examples include dom events
    When I run "agentchrome examples dom"
    Then stdout should contain "dom events"
    And the exit code should be 0
