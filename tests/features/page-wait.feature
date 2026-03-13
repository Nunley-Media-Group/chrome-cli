# File: tests/features/page-wait.feature
#
# Generated from: .claude/specs/feature-add-page-wait-command/requirements.md
# Issue: #163

Feature: Page Wait Command
  As an AI agent automating multi-step browser workflows
  I want a standalone command to wait until a specified condition is met
  So that I can reliably synchronize with page state changes without arbitrary sleeps or polling loops

  # --- CLI Validation ---

  Scenario: Wait help displays usage
    Given agentchrome is built
    When I run "agentchrome page wait --help"
    Then the exit code should be 0
    And stdout should contain "wait"
    And stdout should contain "--url"
    And stdout should contain "--text"
    And stdout should contain "--selector"
    And stdout should contain "--network-idle"
    And stdout should contain "--interval"

  Scenario: Wait with no condition shows help
    Given agentchrome is built
    When I run "agentchrome page wait"
    Then the exit code should be nonzero

  Scenario: Page help lists wait subcommand
    Given agentchrome is built
    When I run "agentchrome page --help"
    Then the exit code should be 0
    And stdout should contain "wait"

  # --- Happy Paths (requires Chrome) ---

  Scenario: Wait for URL to match a glob pattern
    Given a connected Chrome session at "https://example.com/login"
    When I run page wait with url pattern "*/dashboard*"
    And the page URL changes to "https://example.com/dashboard"
    Then the command exits with code 0
    And stdout is valid JSON with "condition" equal to "url"
    And the JSON has "matched" equal to true
    And the JSON has "url" containing "dashboard"
    And the JSON has "pattern" equal to "*/dashboard*"

  Scenario: Wait for text to appear on page
    Given a connected Chrome session on a page without text "Products"
    When I run page wait with text "Products"
    And the text "Products" appears on the page
    Then the command exits with code 0
    And stdout is valid JSON with "condition" equal to "text"
    And the JSON has "matched" equal to true
    And the JSON has "text" equal to "Products"

  Scenario: Wait for network idle
    Given a connected Chrome session on a page with active network requests
    When I run page wait with network-idle flag
    And all network requests complete
    Then the command exits with code 0
    And stdout is valid JSON with "condition" equal to "network-idle"
    And the JSON has "matched" equal to true
    And the JSON has "url" as a non-empty string

  Scenario: Wait for CSS selector to match an element in the DOM
    Given a connected Chrome session on a page without element "#results-table"
    When I run page wait with selector "#results-table"
    And the element "#results-table" is added to the DOM
    Then the command exits with code 0
    And stdout is valid JSON with "condition" equal to "selector"
    And the JSON has "matched" equal to true
    And the JSON has "selector" equal to "#results-table"

  # --- Error Handling (requires Chrome) ---

  Scenario: Wait times out with descriptive error
    Given a connected Chrome session
    When I run page wait with text "never-appearing-text" and timeout 3000
    And the text never appears within the timeout
    Then the command exits with code 4
    And stderr is valid JSON with "code" equal to 4
    And the stderr JSON "error" contains "timed out"
    And the stderr JSON "error" contains "never-appearing-text"

  # --- Edge Cases (requires Chrome) ---

  Scenario: Network idle returns immediately when network is already idle
    Given a connected Chrome session on a fully loaded page with no active requests
    When I run page wait with network-idle flag
    Then the command exits with code 0 within 1000 milliseconds
    And stdout is valid JSON with "condition" equal to "network-idle"

  Scenario: Condition already satisfied returns immediately
    Given a connected Chrome session at "https://example.com/dashboard"
    When I run page wait with url pattern "*/dashboard*"
    Then the command exits with code 0 within 500 milliseconds
    And stdout is valid JSON with "condition" equal to "url"
    And the JSON has "matched" equal to true
