Feature: Mouse Interactions
  As a developer / automation engineer
  I want to simulate mouse interactions on page elements via the CLI
  So that my automation scripts can interact with web pages programmatically

  Background:
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements

  Scenario: Click an element by UID
    Given the page has a button with snapshot UID "s1"
    When I run "chrome-cli interact click s1"
    Then the output JSON should contain "clicked" equal to "s1"
    And the output JSON should contain "navigated" equal to false
    And the exit code should be 0

  Scenario: Click an element by CSS selector
    Given the page has a button matching "css:#submit-btn"
    When I run "chrome-cli interact click css:#submit-btn"
    Then the output JSON should contain "clicked" equal to "css:#submit-btn"
    And the exit code should be 0

  Scenario: Click triggers navigation
    Given the page has a link that navigates away
    When I run "chrome-cli interact click s1"
    Then the output JSON should contain "clicked" equal to "s1"
    And the exit code should be 0

  Scenario: Double-click an element
    Given the page has a button with snapshot UID "s1"
    When I run "chrome-cli interact click s1 --double"
    Then the output JSON should contain "double_click" equal to true
    And the exit code should be 0

  Scenario: Right-click an element
    Given the page has a button with snapshot UID "s1"
    When I run "chrome-cli interact click s1 --right"
    Then the output JSON should contain "right_click" equal to true
    And the exit code should be 0

  Scenario: Click with include-snapshot
    Given the page has a button with snapshot UID "s1"
    When I run "chrome-cli interact click s1 --include-snapshot"
    Then the output JSON should contain a "snapshot" field
    And the exit code should be 0

  Scenario: Click at viewport coordinates
    Given a page is loaded
    When I run "chrome-cli interact click-at 100 200"
    Then the output JSON "clicked_at.x" should be 100
    And the output JSON "clicked_at.y" should be 200
    And the exit code should be 0

  Scenario: Click-at with double flag
    Given a page is loaded
    When I run "chrome-cli interact click-at 100 200 --double"
    Then the output JSON should contain "double_click" equal to true
    And the exit code should be 0

  Scenario: Click-at with right flag
    Given a page is loaded
    When I run "chrome-cli interact click-at 100 200 --right"
    Then the output JSON should contain "right_click" equal to true
    And the exit code should be 0

  Scenario: Hover over an element by UID
    Given the page has an element with snapshot UID "s3"
    When I run "chrome-cli interact hover s3"
    Then the output JSON should contain "hovered" equal to "s3"
    And the exit code should be 0

  Scenario: Hover with CSS selector
    Given the page has a menu item matching "css:.dropdown-trigger"
    When I run "chrome-cli interact hover css:.dropdown-trigger"
    Then the output JSON should contain "hovered" equal to "css:.dropdown-trigger"
    And the exit code should be 0

  Scenario: Hover with include-snapshot
    Given the page has an element with snapshot UID "s3"
    When I run "chrome-cli interact hover s3 --include-snapshot"
    Then the output JSON should contain a "snapshot" field
    And the exit code should be 0

  Scenario: Drag from one element to another
    Given the page has draggable elements with UIDs "s1" and "s2"
    When I run "chrome-cli interact drag s1 s2"
    Then the output JSON "dragged.from" should be "s1"
    And the output JSON "dragged.to" should be "s2"
    And the exit code should be 0

  Scenario: Drag with CSS selectors
    Given the page has elements matching "css:#item" and "css:#target"
    When I run "chrome-cli interact drag css:#item css:#target"
    Then the output JSON "dragged.from" should be "css:#item"
    And the output JSON "dragged.to" should be "css:#target"
    And the exit code should be 0

  Scenario: Drag with include-snapshot
    Given the page has draggable elements with UIDs "s1" and "s2"
    When I run "chrome-cli interact drag s1 s2 --include-snapshot"
    Then the output JSON should contain a "snapshot" field
    And the exit code should be 0

  Scenario: UID not found error
    Given no snapshot has been taken
    When I run "chrome-cli interact click s99"
    Then stderr should contain "No snapshot state found"
    And the exit code should be non-zero

  Scenario: CSS selector not found error
    Given a page is loaded
    When I run "chrome-cli interact click css:#nonexistent"
    Then stderr should contain "Element not found"
    And the exit code should be non-zero

  Scenario: No snapshot state error for UID
    Given no snapshot has been taken
    When I run "chrome-cli interact click s1"
    Then stderr should contain "page snapshot"
    And the exit code should be non-zero

  Scenario: Element scrolled into view before click
    Given an element with UID "s5" is not visible in the viewport
    When I run "chrome-cli interact click s5"
    Then the element is scrolled into view and clicked
    And the exit code should be 0

  Scenario: Plain text output for click
    Given the page has a button with snapshot UID "s1"
    When I run "chrome-cli interact click s1 --plain"
    Then the output should be plain text "Clicked s1"
    And the exit code should be 0

  Scenario: Plain text output for hover
    Given the page has an element with snapshot UID "s3"
    When I run "chrome-cli interact hover s3 --plain"
    Then the output should be plain text "Hovered s3"
    And the exit code should be 0

  Scenario: Plain text output for drag
    Given the page has draggable elements with UIDs "s1" and "s2"
    When I run "chrome-cli interact drag s1 s2 --plain"
    Then the output should be plain text "Dragged s1 to s2"
    And the exit code should be 0
