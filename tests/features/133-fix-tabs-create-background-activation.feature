# File: tests/features/133-fix-tabs-create-background-activation.feature
#
# Generated from: .claude/specs/133-fix-tabs-create-background-activation/requirements.md
# Issue: #133
# Type: Defect regression

@regression
Feature: tabs create --background does not activate the new tab
  The `tabs create --background` command previously caused the new tab to become
  active despite the --background flag. The root cause was that /json/list
  ordering does not reflect activation state in headless Chrome. This was fixed
  by replacing the `i == 0` positional heuristic in `execute_list` with CDP
  `document.visibilityState` queries, and using HTTP /json/activate with CDP
  visibility verification in the background creation path.

  # --- Bug Is Fixed ---

  @regression
  Scenario: Background tab does not become active
    Given a headless Chrome instance is running
    And a tab is open at "https://www.google.com" as the active tab
    When I run "tabs create https://example.com --background"
    And I run "tabs list"
    Then the tab at "https://www.google.com" is shown as active
    And the tab at "https://example.com" is shown as not active

  # --- Related Behavior Still Works ---

  @regression
  Scenario: Non-background create still activates the new tab
    Given a headless Chrome instance is running
    And a tab is open at "https://www.google.com" as the active tab
    When I run "tabs create https://example.com"
    And I run "tabs list"
    Then the tab at "https://example.com" is shown as active
    And the tab at "https://www.google.com" is shown as not active
