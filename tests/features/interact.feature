# File: tests/features/interact.feature
#
# Generated from: .claude/specs/mouse-interactions/requirements.md
# Issue: #14

Feature: Mouse Interactions
  As a developer / automation engineer
  I want to simulate mouse interactions on page elements via the CLI
  So that my automation scripts can interact with web pages programmatically

  # --- CLI Argument Validation (no Chrome required) ---

  Scenario: Click requires a target argument
    Given chrome-cli is built
    When I run "chrome-cli interact click"
    Then the exit code should be nonzero
    And stderr should contain "required"

  Scenario: Click-at requires x and y arguments
    Given chrome-cli is built
    When I run "chrome-cli interact click-at"
    Then the exit code should be nonzero
    And stderr should contain "required"

  Scenario: Hover requires a target argument
    Given chrome-cli is built
    When I run "chrome-cli interact hover"
    Then the exit code should be nonzero
    And stderr should contain "required"

  Scenario: Drag requires from and to arguments
    Given chrome-cli is built
    When I run "chrome-cli interact drag"
    Then the exit code should be nonzero
    And stderr should contain "required"

  Scenario: Double and right flags are mutually exclusive
    Given chrome-cli is built
    When I run "chrome-cli interact click s1 --double --right"
    Then the exit code should be nonzero
    And stderr should contain "cannot be used with"

  Scenario: Interact help displays all subcommands
    Given chrome-cli is built
    When I run "chrome-cli interact --help"
    Then the exit code should be 0
    And stdout should contain "click"
    And stdout should contain "click-at"
    And stdout should contain "hover"
    And stdout should contain "drag"
    And stdout should contain "type"
    And stdout should contain "key"

  Scenario: Click help displays all options
    Given chrome-cli is built
    When I run "chrome-cli interact click --help"
    Then the exit code should be 0
    And stdout should contain "--double"
    And stdout should contain "--right"
    And stdout should contain "--include-snapshot"

  # --- Click: Happy Paths (require Chrome) ---

  Scenario: Click an element by UID
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And the page has a button with snapshot UID "s1"
    When I run "chrome-cli interact click s1"
    Then the output JSON should contain "clicked" equal to "s1"
    And the output JSON should contain "navigated" equal to false
    And the output JSON should contain "url"
    And the exit code should be 0

  Scenario: Click an element by CSS selector
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And the page has a button with id "submit-btn"
    When I run "chrome-cli interact click css:#submit-btn"
    Then the output JSON should contain "clicked" equal to "css:#submit-btn"
    And the output JSON should contain "navigated" equal to false
    And the exit code should be 0

  Scenario: Click triggers navigation
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And the page has a link with UID "s1" that navigates to "/about"
    When I run "chrome-cli interact click s1"
    Then the output JSON should contain "navigated" equal to true
    And the output JSON "url" should contain "/about"

  Scenario: Double-click an element
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And the page has an element with snapshot UID "s1"
    When I run "chrome-cli interact click s1 --double"
    Then the output JSON should contain "double_click" equal to true
    And the output JSON should contain "clicked" equal to "s1"

  Scenario: Right-click an element
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And the page has an element with snapshot UID "s1"
    When I run "chrome-cli interact click s1 --right"
    Then the output JSON should contain "right_click" equal to true
    And the output JSON should contain "clicked" equal to "s1"

  Scenario: Click with include-snapshot flag
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And the page has a button with snapshot UID "s1"
    When I run "chrome-cli interact click s1 --include-snapshot"
    Then the output JSON should contain "clicked" equal to "s1"
    And the output JSON should contain a "snapshot" field
    And the snapshot should be a valid accessibility tree

  # --- Click-At ---

  Scenario: Click at viewport coordinates
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    When I run "chrome-cli interact click-at 100 200"
    Then the output JSON "clicked_at.x" should be 100
    And the output JSON "clicked_at.y" should be 200
    And the exit code should be 0

  Scenario: Click-at with double flag
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    When I run "chrome-cli interact click-at 100 200 --double"
    Then the output JSON should contain "double_click" equal to true
    And the output JSON "clicked_at.x" should be 100

  # --- Hover ---

  Scenario: Hover over an element by UID
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And the page has a menu item with snapshot UID "s3"
    When I run "chrome-cli interact hover s3"
    Then the output JSON should contain "hovered" equal to "s3"
    And the exit code should be 0

  Scenario: Hover with CSS selector
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And the page has an element matching "css:.dropdown-trigger"
    When I run "chrome-cli interact hover css:.dropdown-trigger"
    Then the output JSON should contain "hovered" equal to "css:.dropdown-trigger"

  Scenario: Hover with include-snapshot flag
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And the page has an element with snapshot UID "s3"
    When I run "chrome-cli interact hover s3 --include-snapshot"
    Then the output JSON should contain "hovered" equal to "s3"
    And the output JSON should contain a "snapshot" field

  # --- Drag ---

  Scenario: Drag from one element to another by UID
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And the page has a draggable item with UID "s1" and a drop target with UID "s2"
    When I run "chrome-cli interact drag s1 s2"
    Then the output JSON "dragged.from" should be "s1"
    And the output JSON "dragged.to" should be "s2"
    And the exit code should be 0

  Scenario: Drag with CSS selectors
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And the page has elements matching "css:#item" and "css:#target"
    When I run "chrome-cli interact drag css:#item css:#target"
    Then the output JSON "dragged.from" should be "css:#item"
    And the output JSON "dragged.to" should be "css:#target"

  Scenario: Drag with include-snapshot flag
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And the page has draggable elements with UIDs "s1" and "s2"
    When I run "chrome-cli interact drag s1 s2 --include-snapshot"
    Then the output JSON "dragged.from" should be "s1"
    And the output JSON should contain a "snapshot" field

  # --- Tab Targeting ---

  Scenario: Click with tab targeting
    Given Chrome is running with CDP enabled
    And a specific tab with ID "ABC123" contains an element with UID "s1"
    When I run "chrome-cli interact click s1 --tab ABC123"
    Then the click is performed on the element in tab "ABC123"
    And the exit code should be 0

  # --- Error Handling (require Chrome for snapshot/UID errors) ---

  Scenario: UID not found error
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And no element matches UID "s99" in the snapshot state
    When I run "chrome-cli interact click s99"
    Then stderr should contain "UID 's99' not found"
    And stderr should contain "page snapshot"
    And the exit code should be nonzero

  Scenario: CSS selector not found error
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And no element matches the selector "#nonexistent"
    When I run "chrome-cli interact click css:#nonexistent"
    Then stderr should contain "Element not found for selector"
    And the exit code should be nonzero

  Scenario: No snapshot state error
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And no snapshot has been taken
    When I run "chrome-cli interact click s1"
    Then stderr should contain "No snapshot state found"
    And stderr should contain "page snapshot"
    And the exit code should be nonzero

  # --- Edge Cases ---

  Scenario: Element scrolled into view before click
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And an element with UID "s5" is below the viewport fold
    When I run "chrome-cli interact click s5"
    Then the element is scrolled into view
    And the click succeeds
    And the output JSON should contain "clicked" equal to "s5"

  # --- Plain Text Output ---

  Scenario: Plain text output for click
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And the page has a button with snapshot UID "s1"
    When I run "chrome-cli interact click s1 --plain"
    Then the output should be plain text "Clicked s1"

  Scenario: Plain text output for hover
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And the page has an element with snapshot UID "s3"
    When I run "chrome-cli interact hover s3 --plain"
    Then the output should be plain text "Hovered s3"

  Scenario: Plain text output for drag
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    And a snapshot has been taken with UIDs assigned
    And the page has draggable elements with UIDs "s1" and "s2"
    When I run "chrome-cli interact drag s1 s2 --plain"
    Then the output should be plain text "Dragged s1 to s2"
