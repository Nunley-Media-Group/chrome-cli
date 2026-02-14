# File: tests/features/console.feature
#
# Generated from: .claude/specs/18-console-message-reading-with-filtering/requirements.md
# Issue: #18

Feature: Console Message Reading with Filtering
  As a developer or automation engineer
  I want to read and monitor browser console messages via the CLI
  So that I can debug web applications and monitor for errors from scripts

  # --- CLI Argument Validation (no Chrome required) ---

  Scenario: Console help lists read and follow subcommands
    Given chrome-cli is built
    When I run "chrome-cli console --help"
    Then the exit code should be 0
    And stdout should contain "read"
    And stdout should contain "follow"

  Scenario: Console read help shows all flags
    Given chrome-cli is built
    When I run "chrome-cli console read --help"
    Then the exit code should be 0
    And stdout should contain "--type"
    And stdout should contain "--errors-only"
    And stdout should contain "--limit"
    And stdout should contain "--page"
    And stdout should contain "--include-preserved"

  Scenario: Console follow help shows all flags
    Given chrome-cli is built
    When I run "chrome-cli console follow --help"
    Then the exit code should be 0
    And stdout should contain "--type"
    And stdout should contain "--errors-only"
    And stdout should contain "--timeout"

  Scenario: Conflicting flags --type and --errors-only on read
    Given chrome-cli is built
    When I run "chrome-cli console read --type error --errors-only"
    Then the exit code should be nonzero
    And stderr should contain "cannot be used with"

  Scenario: Conflicting flags --type and --errors-only on follow
    Given chrome-cli is built
    When I run "chrome-cli console follow --type error --errors-only"
    Then the exit code should be nonzero
    And stderr should contain "cannot be used with"

  # --- Console Read: List Mode (requires Chrome) ---

  # Scenario: List console messages from current page
  #   Given Chrome is running with CDP enabled
  #   And a page has generated console messages
  #   When I run "chrome-cli console read"
  #   Then the output is a JSON array
  #   And the exit code should be 0

  # Scenario: Console read with no messages returns empty array
  #   Given Chrome is running with CDP enabled
  #   When I run "chrome-cli console read"
  #   Then the output is "[]"
  #   And the exit code should be 0

  # Scenario: Default limit is 50 messages
  #   Given Chrome is running with CDP enabled
  #   And a page has generated 100 console messages
  #   When I run "chrome-cli console read"
  #   Then at most 50 messages are returned
  #   And the exit code should be 0

  # --- Console Read: Type Filtering (requires Chrome) ---

  # Scenario: Filter console messages by single type
  #   Given Chrome is running with CDP enabled
  #   And a page has generated log, warn, and error messages
  #   When I run "chrome-cli console read --type error"
  #   Then all returned messages have type "error"

  # Scenario: Use errors-only shorthand filter
  #   Given Chrome is running with CDP enabled
  #   And a page has generated log and error messages
  #   When I run "chrome-cli console read --errors-only"
  #   Then all returned messages have type "error" or "assert"

  # --- Console Read: Detail Mode (requires Chrome) ---

  # Scenario: Get detailed information about a specific message
  #   Given Chrome is running with CDP enabled
  #   And console messages exist
  #   When I run "chrome-cli console read 0"
  #   Then the output contains "args" field
  #   And the output contains "stackTrace" field

  # Scenario: Console read with invalid message ID errors
  #   Given Chrome is running with CDP enabled
  #   When I run "chrome-cli console read 9999"
  #   Then the exit code should be nonzero
  #   And stderr should contain "not found"

  # --- Console Follow: Streaming (requires Chrome) ---

  # Scenario: Stream with timeout exits after specified duration
  #   Given Chrome is running with CDP enabled
  #   When I run "chrome-cli console follow --timeout 1000"
  #   Then the exit code should be 0
