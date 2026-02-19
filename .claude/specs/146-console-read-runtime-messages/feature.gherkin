# File: tests/features/146-console-read-runtime-messages.feature
#
# Generated from: .claude/specs/146-console-read-runtime-messages/requirements.md
# Issue: #146

Feature: Console Read Runtime Messages
  As an AI agent automating browser workflows
  I want console read to show console messages from runtime interactions
  So that I can detect JavaScript errors or warnings from automation steps

  Background:
    Given Chrome is running with CDP enabled

  # --- Core Feature: Runtime Message Capture ---

  @requires-chrome
  Scenario: Console read captures messages from page interactions
    Given a page with a button that logs to console on click
    And the button has been clicked via interact click
    When I run "chrome-cli console read"
    Then the output is a JSON array
    And the array contains at least one entry
    And each entry contains "id", "type", "text", and "timestamp"
    And the exit code should be 0

  @requires-chrome
  Scenario: Console read captures messages from js exec across invocations
    Given js exec has generated console messages in a prior invocation
    When I run "chrome-cli console read" in a new CLI invocation
    Then the output is a JSON array
    And the array contains entries with text "test" and "oops"
    And the exit code should be 0

  # --- No Regression ---

  @requires-chrome
  Scenario: Console read still returns page-load messages
    Given a page has console output from inline scripts during load
    When I run "chrome-cli console read"
    Then the output is a JSON array
    And the array contains at least one entry
    And each entry contains "id", "type", "text", and "timestamp"

  @requires-chrome
  Scenario: Console read preserves page state
    Given a page has been modified via runtime interactions
    When I run "chrome-cli console read"
    Then the page state is preserved with no reload

  @requires-chrome
  Scenario: Console follow streaming still works
    Given a page is open
    When I run "chrome-cli console follow --timeout 2000"
    And console messages are generated on the page
    Then messages are streamed as JSON lines
    And each line contains "type" and "text"

  # --- Filters ---

  @requires-chrome
  Scenario: Errors-only filter works on runtime interaction messages
    Given runtime interactions have generated log, warn, and error messages
    When I run "chrome-cli console read --errors-only"
    Then the output is a JSON array
    And each entry has type "error"

  # --- Accumulation ---

  @requires-chrome
  Scenario: Accumulated messages from multiple interactions
    Given multiple js exec invocations have each generated console messages
    When I run "chrome-cli console read"
    Then the output is a JSON array
    And the array contains entries from all prior invocations
    And entries are ordered chronologically by timestamp
