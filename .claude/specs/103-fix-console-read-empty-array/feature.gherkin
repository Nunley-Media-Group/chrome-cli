# File: tests/features/103-fix-console-read-empty-array.feature
#
# Generated from: .claude/specs/103-fix-console-read-empty-array/requirements.md
# Issue: #103
# Type: Defect regression

@regression
Feature: console read returns captured messages after page load
  The `console read` command previously always returned an empty array `[]`
  regardless of console activity. This was because each CLI invocation created
  a new CDP connection after console calls had already occurred, and CDP has no
  retrospective API for `Runtime.consoleAPICalled`. This was fixed by triggering
  a page reload after enabling the Runtime domain so console events from page
  scripts are captured live.

  Background:
    Given Chrome is running with CDP enabled

  # --- Bug Is Fixed ---

  @regression @requires-chrome
  Scenario: Console read returns captured messages
    Given a page has generated console messages
    When I run "chrome-cli console read"
    Then the output is a JSON array
    And the array contains at least one entry
    And each entry contains "id", "type", "text", and "timestamp"
    And the exit code should be 0

  @regression @requires-chrome
  Scenario: Errors-only filter works on captured messages
    Given a page has generated log, warn, and error messages
    When I run "chrome-cli console read --errors-only"
    Then the output is a JSON array
    And the array contains at least one entry
    And each entry has type "error"

  @regression @requires-chrome
  Scenario: Console messages from page scripts are captured across invocations
    Given a page has generated console messages via inline scripts
    When I run "chrome-cli console read" in a new CLI invocation
    Then the output is a JSON array
    And the array contains at least one entry

  # --- Related Behavior Still Works ---

  @regression @requires-chrome
  Scenario: Console follow streaming still works
    Given a page is open
    When I run "chrome-cli console follow --timeout 2000"
    And console messages are generated on the page
    Then messages are streamed as JSON lines
    And each line contains "type" and "text"
