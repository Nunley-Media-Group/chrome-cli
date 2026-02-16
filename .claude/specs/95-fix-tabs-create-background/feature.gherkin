# File: tests/features/95-fix-tabs-create-background.feature
#
# Generated from: .claude/specs/95-fix-tabs-create-background/requirements.md
# Issue: #95
# Type: Defect regression

@regression
Feature: tabs create --background preserves active tab
  The `tabs create --background` command previously activated the new tab
  despite the --background flag, because the Target.activateTarget re-activation
  did not reliably propagate to Chrome's /json/list ordering.
  This was fixed by adding a verification loop after re-activation to confirm
  the original tab has returned to the active position.

  # --- Bug Is Fixed ---

  @regression
  Scenario: Background tab creation keeps original tab active
    Given Chrome is running with an active tab "TAB_A"
    When I run "chrome-cli tabs create --background https://example.com"
    Then a new tab is created navigating to "https://example.com"
    And "TAB_A" remains the active tab

  # --- Related Behavior Still Works ---

  @regression
  Scenario: Non-background tab creation still activates the new tab
    Given Chrome is running with an active tab
    When I run "chrome-cli tabs create https://example.com"
    Then the new tab becomes the active tab

  # --- Background Tab Visible in List ---

  @regression
  Scenario: Background tab appears in tab list
    Given Chrome is running with an active tab
    When I run "chrome-cli tabs create --background https://example.com"
    And I run "chrome-cli tabs list"
    Then the tab list contains a tab with URL "https://example.com"
    And the tab with URL "https://example.com" has "active" set to false
