# File: tests/features/form.feature
#
# Generated from: .claude/specs/16-form-input-and-filling/requirements.md
# Issue: #16

Feature: Form input and filling
  As a developer or automation engineer
  I want to fill form fields, select dropdown options, and clear inputs via the CLI
  So that my automation scripts can programmatically interact with web forms

  # --- CLI Argument Validation (no Chrome required) ---

  Scenario: Fill requires target and value arguments
    Given chrome-cli is built
    When I run "chrome-cli form fill"
    Then the exit code should be nonzero
    And stderr should contain "required"

  Scenario: Clear requires a target argument
    Given chrome-cli is built
    When I run "chrome-cli form clear"
    Then the exit code should be nonzero
    And stderr should contain "required"

  Scenario: Form help displays all subcommands
    Given chrome-cli is built
    When I run "chrome-cli form --help"
    Then the exit code should be 0
    And stdout should contain "fill"
    And stdout should contain "fill-many"
    And stdout should contain "clear"

  Scenario: Fill help displays all options
    Given chrome-cli is built
    When I run "chrome-cli form fill --help"
    Then the exit code should be 0
    And stdout should contain "TARGET"
    And stdout should contain "VALUE"
    And stdout should contain "--include-snapshot"

  Scenario: Fill-many help displays all options
    Given chrome-cli is built
    When I run "chrome-cli form fill-many --help"
    Then the exit code should be 0
    And stdout should contain "--file"
    And stdout should contain "--include-snapshot"

  Scenario: Clear help displays all options
    Given chrome-cli is built
    When I run "chrome-cli form clear --help"
    Then the exit code should be 0
    And stdout should contain "TARGET"
    And stdout should contain "--include-snapshot"

  # --- Chrome-Required Scenarios ---
  # These scenarios require a running Chrome instance with CDP enabled.
  # They are documented here for completeness but run only when integration
  # test infrastructure is available.

  # Scenario: Fill a text input field by UID
  #   Given Chrome is running with CDP enabled
  #   And a page is loaded with a form containing various field types
  #   And an accessibility snapshot has been taken with UIDs assigned
  #   And the page has a text input with UID "s1"
  #   When I run "chrome-cli form fill s1 'John Doe'"
  #   Then the exit code should be 0
  #   And the output JSON "filled" should be "s1"
  #   And the output JSON "value" should be "John Doe"

  # Scenario: Fill a text input field by CSS selector
  #   Given Chrome is running with CDP enabled
  #   And the page has a text input with id "email"
  #   When I run "chrome-cli form fill css:#email user@example.com"
  #   Then the exit code should be 0
  #   And the output JSON "filled" should be "css:#email"
  #   And the output JSON "value" should be "user@example.com"

  # Scenario: Fill a select dropdown
  #   Given Chrome is running with CDP enabled
  #   And an accessibility snapshot has been taken with UIDs assigned
  #   And the page has a select element with UID "s3"
  #   When I run "chrome-cli form fill s3 option2"
  #   Then the exit code should be 0
  #   And the output JSON "filled" should be "s3"
  #   And the output JSON "value" should be "option2"

  # Scenario: Fill a textarea
  #   Given Chrome is running with CDP enabled
  #   And an accessibility snapshot has been taken with UIDs assigned
  #   And the page has a textarea with UID "s4"
  #   When I run "chrome-cli form fill s4 'Multi-line text content'"
  #   Then the exit code should be 0
  #   And the output JSON "value" should be "Multi-line text content"

  # Scenario: Toggle a checkbox to checked
  #   Given Chrome is running with CDP enabled
  #   And an accessibility snapshot has been taken with UIDs assigned
  #   And the page has an unchecked checkbox with UID "s5"
  #   When I run "chrome-cli form fill s5 true"
  #   Then the exit code should be 0
  #   And the checkbox should be checked

  # Scenario: Toggle a checkbox to unchecked
  #   Given Chrome is running with CDP enabled
  #   And an accessibility snapshot has been taken with UIDs assigned
  #   And the page has a checked checkbox with UID "s5"
  #   When I run "chrome-cli form fill s5 false"
  #   Then the exit code should be 0
  #   And the checkbox should be unchecked

  # Scenario: Fill with --include-snapshot flag
  #   Given Chrome is running with CDP enabled
  #   And an accessibility snapshot has been taken with UIDs assigned
  #   When I run "chrome-cli form fill s1 'Jane' --include-snapshot"
  #   Then the exit code should be 0
  #   And the output JSON should contain a "snapshot" object

  # Scenario: Fill multiple fields at once from inline JSON
  #   Given Chrome is running with CDP enabled
  #   And an accessibility snapshot has been taken with UIDs assigned
  #   When I run "chrome-cli form fill-many '[{\"uid\":\"s1\",\"value\":\"John\"},{\"uid\":\"s2\",\"value\":\"Doe\"}]'"
  #   Then the exit code should be 0
  #   And the output should be a JSON array with 2 results

  # Scenario: Fill multiple fields from a JSON file
  #   Given Chrome is running with CDP enabled
  #   And an accessibility snapshot has been taken with UIDs assigned
  #   And a JSON file "fields.json" contains '[{"uid":"s1","value":"John"}]'
  #   When I run "chrome-cli form fill-many --file fields.json"
  #   Then the exit code should be 0

  # Scenario: Fill many with --include-snapshot flag
  #   Given Chrome is running with CDP enabled
  #   And an accessibility snapshot has been taken with UIDs assigned
  #   When I run "chrome-cli form fill-many '[{\"uid\":\"s1\",\"value\":\"John\"}]' --include-snapshot"
  #   Then the exit code should be 0
  #   And the output JSON should contain a "snapshot" object

  # Scenario: Clear a form field
  #   Given Chrome is running with CDP enabled
  #   And an accessibility snapshot has been taken with UIDs assigned
  #   And the page has a text input with UID "s1" containing value "old"
  #   When I run "chrome-cli form clear s1"
  #   Then the exit code should be 0
  #   And the output JSON "cleared" should be "s1"

  # Scenario: Fill dispatches events for framework compatibility
  #   Given Chrome is running with CDP enabled
  #   And an accessibility snapshot has been taken with UIDs assigned
  #   When I run "chrome-cli form fill s1 'test'"
  #   Then an "input" event should have been dispatched with bubbles true
  #   And a "change" event should have been dispatched with bubbles true

  # Scenario: Fill with --tab flag targets specific tab
  #   Given Chrome is running with CDP enabled
  #   And multiple tabs are open
  #   When I run "chrome-cli form fill s1 'value' --tab TAB_ID"
  #   Then the exit code should be 0

  # Scenario: Fill nonexistent UID returns error
  #   Given Chrome is running with CDP enabled
  #   And an accessibility snapshot has been taken with UIDs assigned
  #   When I run "chrome-cli form fill s999 'value'"
  #   Then the exit code should be nonzero
  #   And stderr should contain "UID"
