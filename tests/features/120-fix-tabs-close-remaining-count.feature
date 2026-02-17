# File: tests/features/120-fix-tabs-close-remaining-count.feature
#
# Generated from: .claude/specs/120-fix-tabs-close-remaining-count/requirements.md
# Issue: #120
# Type: Defect regression

@regression
Feature: tabs close reports correct remaining count
  The `tabs close` command previously reported an incorrect `remaining` count
  that was off by 1, including the just-closed tab because Chrome's HTTP
  `/json/list` endpoint hadn't propagated the closure yet.
  This was fixed by adding a polling retry loop (matching `execute_create`)
  to wait for the HTTP endpoint to reflect the tab closure.

  Background:
    Given a headless Chrome instance is running

  # --- Bug Is Fixed ---

  @regression
  Scenario: Remaining count is accurate after single close
    Given I have created 3 additional tabs for a total of 4 tabs
    When I close one tab via "tabs close"
    Then the JSON output field "remaining" is 3

  # --- Sequential Closes ---

  @regression
  Scenario: Multiple sequential closes report correct counts
    Given I have created 3 additional tabs for a total of 4 tabs
    When I close one tab via "tabs close"
    Then the JSON output field "remaining" is 3
    When I close another tab via "tabs close"
    Then the JSON output field "remaining" is 2

  # --- Related Behavior Still Works ---

  @regression
  Scenario: Existing tab close behavior is preserved
    Given I have created 1 additional tab for a total of 2 tabs
    When I close one tab via "tabs close"
    Then the JSON output field "remaining" is 1
    And the closed tab ID appears in the "closed" array
    And the closed tab no longer appears in "tabs list" output
