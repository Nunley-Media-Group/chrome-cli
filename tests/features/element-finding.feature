# File: tests/features/element-finding.feature
#
# Generated from: .claude/specs/11-element-finding/requirements.md
# Issue: #11

Feature: Element finding by text, CSS selector, and accessibility attributes
  As a developer or automation engineer
  I want to find elements on a page by text, CSS selector, or accessibility attributes
  So that I can locate specific interactive elements for clicking, filling, or inspecting from scripts

  Background:
    Given Chrome is running with CDP enabled
    And a page is loaded with known interactive elements

  # --- Happy Path ---

  Scenario: Find elements by text query
    Given the page contains a button named "Submit" and a heading "Submit Your Application"
    When I run "chrome-cli page find Submit"
    Then a JSON array is returned
    And each element includes "uid", "role", "name", and "boundingBox" fields
    And elements are in document order

  Scenario: Find elements by CSS selector
    Given the page contains a button with class "primary"
    When I run "chrome-cli page find --selector button.primary"
    Then a JSON array of matching elements is returned
    And each element includes "uid", "role", "name", and "boundingBox" fields

  Scenario: Filter by accessibility role
    Given the page contains a button "Click me" and a link "Click here"
    When I run "chrome-cli page find Click --role button"
    Then only elements with role "button" are returned
    And the link "Click here" is not in the results

  Scenario: Exact text match
    Given the page contains buttons named "Log" and "Login"
    When I run "chrome-cli page find Log --exact"
    Then only the element with name exactly "Log" is returned
    And "Login" is not in the results

  # --- Configuration ---

  Scenario: Limit results
    Given the page contains 50 links
    When I run "chrome-cli page find link --limit 5"
    Then at most 5 results are returned
    And they are in document order

  Scenario: Default limit of 10
    Given the page contains 20 matching elements
    When I run "chrome-cli page find item"
    Then at most 10 results are returned

  Scenario: Target a specific tab
    Given multiple tabs are open
    When I run "chrome-cli page find Submit --tab <TAB_ID>"
    Then the search is performed on the specified tab only

  # --- Bounding Box ---

  Scenario: Bounding box information included
    Given the page contains a visible button "Submit"
    When I run "chrome-cli page find Submit"
    Then each result includes a "boundingBox" object
    And the bounding box has numeric "x", "y", "width", and "height" values

  # --- Snapshot Integration ---

  Scenario: Snapshot triggered automatically if needed
    Given the page is loaded but no prior snapshot exists
    When I run "chrome-cli page find Submit"
    Then a snapshot is automatically captured
    And UIDs are assigned and persisted to snapshot state

  # --- CSS Selector Without Text ---

  Scenario: CSS selector search without text query
    Given the page contains an email input field
    When I run "chrome-cli page find --selector input[type=email]"
    Then matching elements are returned without requiring a text query

  # --- Combined Filters ---

  Scenario: Combined role and text query
    Given the page contains a link "Next" and a button "Next"
    When I run "chrome-cli page find Next --role link"
    Then only the link element with name "Next" is returned
    And the button "Next" is not in the results

  # --- Role-Only Search (Issue #97) ---

  @regression
  Scenario: Role-only search returns matching elements
    Given the page contains a textbox and a button
    When I run "chrome-cli page find --role textbox"
    Then a JSON array is returned
    And each element has role "textbox"
    And each element includes "uid", "role", "name", and "boundingBox" fields
    And the exit code is 0

  @regression
  Scenario: Role-only search with no matches returns empty array
    Given the page is loaded
    When I run "chrome-cli page find --role nonexistent-role"
    Then an empty JSON array "[]" is returned
    And the exit code is 0

  # --- Edge Cases ---

  Scenario: No matches found returns empty array
    Given the page is loaded
    When I run "chrome-cli page find nonexistent-element-xyz"
    Then an empty JSON array "[]" is returned
    And the exit code is 0

  # --- Error Cases ---

  Scenario: Neither query, selector, nor role provided
    When I run "chrome-cli page find"
    Then an error is returned with message "a text query, --selector, or --role is required"
    And the exit code is 1
