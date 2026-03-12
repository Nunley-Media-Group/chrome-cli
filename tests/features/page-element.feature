# File: tests/features/page-element.feature
#
# Generated from: .claude/specs/feature-add-page-element-command-for-targeted-element-state-queries-by-uid/requirements.md
# Issue: #165

Feature: Page element command
  As an AI agent making decisions based on element state
  I want to quickly query a specific element's properties by its accessibility UID
  So that I can check element visibility, enabled state, or bounding box without taking a full page snapshot

  # --- CLI Validation ---

  Scenario: Element help displays usage
    Given agentchrome is built
    When I run "agentchrome page element --help"
    Then the exit code should be 0
    And stdout should contain "element"
    And stdout should contain "target"

  Scenario: Element without required target argument
    Given agentchrome is built
    When I run "agentchrome page element"
    Then the exit code should be nonzero
    And stderr should contain "TARGET"

  Scenario: Page help lists element subcommand
    Given agentchrome is built
    When I run "agentchrome page --help"
    Then the exit code should be 0
    And stdout should contain "element"

  # --- Happy Path (requires Chrome) ---

  Scenario: Query element properties by UID
    Given a connected Chrome session with a fresh accessibility snapshot containing interactive elements
    When I run the page element command with a valid UID target
    Then the output is valid JSON containing "role" as a string
    And the output contains "name" as a string
    And the output contains "tagName" as a string
    And the output contains "boundingBox" with numeric "x", "y", "width", and "height" fields
    And the output contains "properties" with boolean "enabled" and "focused" fields
    And the output contains "inViewport" as a boolean
    And the exit code is 0

  # --- Viewport Visibility (requires Chrome) ---

  Scenario: Query reports viewport visibility for off-screen element
    Given a connected Chrome session with a snapshot containing an element scrolled off-screen
    When I run the page element command with the off-screen element's UID
    Then the output contains "inViewport" set to false
    And the "boundingBox" reflects the element's actual page position
    And the exit code is 0

  # --- CSS Selector (requires Chrome) ---

  Scenario: Query element by CSS selector
    Given a connected Chrome session on a page with identifiable elements
    When I run the page element command with a CSS selector target
    Then the output is valid JSON containing "role", "name", "tagName", "boundingBox", "properties", and "inViewport"
    And the exit code is 0

  # --- Plain Text Output (requires Chrome) ---

  Scenario: Plain text output mode
    Given a connected Chrome session with a fresh accessibility snapshot containing interactive elements
    When I run the page element command with a valid UID target and the plain flag
    Then the output is human-readable text containing the element's role and name
    And the output contains bounding box coordinates
    And the output contains property values
    And the exit code is 0
