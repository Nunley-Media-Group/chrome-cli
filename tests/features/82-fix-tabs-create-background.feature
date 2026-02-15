# File: tests/features/82-fix-tabs-create-background.feature
#
# Generated from: .claude/specs/82-fix-tabs-create-background/requirements.md
# Issue: #82
# Type: Defect regression

@regression
Feature: tabs create --background keeps previously active tab focused
  The `tabs create --background` command previously activated the new tab
  despite the --background flag, because Chrome does not reliably honor
  the background parameter in Target.createTarget.
  This was fixed by re-activating the original tab via Target.activateTarget
  after background tab creation.

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

  # --- Output Format Preserved ---

  @regression
  Scenario: Background tab creation output contains expected fields
    Given Chrome is running
    When I run "chrome-cli tabs create --background https://example.com"
    Then stdout contains a JSON object with "id", "url", and "title" fields
    And the exit code is 0
