# File: tests/features/122-fix-tabs-activate-state-propagation.feature
#
# Generated from: .claude/specs/122-fix-tabs-activate-state-propagation/requirements.md
# Issue: #122
# Type: Defect regression

@regression
Feature: tabs activate state propagation
  The `tabs activate` command previously returned success but Chrome's
  `/json/list` endpoint did not reflect the activation, causing subsequent
  `tabs list` to show the wrong tab as active. This was fixed by adding
  a polling loop after `Target.activateTarget` to wait for propagation.

  # --- Bug Is Fixed ---

  @regression
  Scenario: Activate is reflected in tabs list
    Given a headless Chrome instance with multiple tabs
    When I activate a non-active tab
    And I list all tabs
    Then the activated tab shows as active

  # --- Correct Output ---

  @regression
  Scenario: Activate returns correct tab info
    Given a headless Chrome instance with multiple tabs
    When I activate a specific tab
    Then the JSON output contains the correct activated id, url, and title

  # --- Related Behavior Still Works ---

  @regression
  Scenario: Activating an already-active tab succeeds
    Given a headless Chrome instance with an active tab
    When I activate the already-active tab
    Then the command succeeds
    And the tab remains active in the list
