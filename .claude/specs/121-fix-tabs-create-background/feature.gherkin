# File: tests/features/121-fix-tabs-create-background.feature
#
# Generated from: .claude/specs/121-fix-tabs-create-background/requirements.md
# Issue: #121
# Type: Defect regression

@regression
Feature: tabs create --background reliably keeps original tab active
  The `tabs create --background` command made the newly created tab active
  despite the --background flag. The existing workaround (re-activating the
  original tab via Target.activateTarget with a polling verification loop)
  had an insufficient timeout budget (100ms), causing it to return before
  Chrome propagated the activation state to /json/list.
  This was fixed by increasing the polling budget from 100ms to 500ms.

  # --- Bug Is Fixed ---

  @regression
  Scenario: Background tab creation keeps original tab active
    Given Chrome is running with an active tab at "https://www.google.com"
    When I run "chrome-cli tabs create --background https://example.com"
    And I run "chrome-cli tabs list"
    Then the first tab in the list has "active" set to true
    And the first tab in the list has a URL containing "google.com"

  # --- Background Tab Is Created ---

  @regression
  Scenario: Background tab is created and appears in tab list
    Given Chrome is running with an active tab
    When I run "chrome-cli tabs create --background https://example.com"
    And I run "chrome-cli tabs list"
    Then the tab list contains a tab with URL containing "example.com"
    And the tab with URL containing "example.com" has "active" set to false

  # --- Related Behavior Still Works ---

  @regression
  Scenario: Non-background tab creation still activates the new tab
    Given Chrome is running with an active tab
    When I run "chrome-cli tabs create https://example.com"
    And I run "chrome-cli tabs list"
    Then the first tab in the list has "active" set to true
    And the first tab in the list has a URL containing "example.com"
