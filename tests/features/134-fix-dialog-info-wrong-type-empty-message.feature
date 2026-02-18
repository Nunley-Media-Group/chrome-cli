# File: tests/features/134-fix-dialog-info-wrong-type-empty-message.feature
#
# Generated from: .claude/specs/134-fix-dialog-info-wrong-type-empty-message/requirements.md
# Issue: #134
# Type: Defect regression

@regression
Feature: Dialog info and handle work correctly for open dialogs
  The dialog commands previously failed for pre-existing dialogs because
  CDP events are ephemeral and never replayed to new sessions. This was
  fixed by using cookie-based interceptors for dialog metadata and a
  Page.navigate fallback for dialog dismissal.

  Background:
    Given chrome-cli is built

  # --- Bug Is Fixed ---

  @regression
  Scenario: AC1 — dialog info returns correct type and message for alert
    Given a page has triggered an alert dialog with message "hello"
    When I run "chrome-cli dialog info"
    Then the output JSON should contain "open" equal to true
    And the output JSON should contain "type" equal to "alert"
    And the output JSON should contain "message" equal to "hello"
    And the exit code should be 0

  @regression
  Scenario: AC2 — dialog info reports confirm dialogs correctly
    Given a page has triggered a confirm dialog with message "proceed?"
    When I run "chrome-cli dialog info"
    Then the output JSON should contain "open" equal to true
    And the output JSON should contain "type" equal to "confirm"
    And the output JSON should contain "message" equal to "proceed?"
    And the exit code should be 0

  @regression
  Scenario: AC4 — dialog handle returns correct type and message
    Given a page has triggered an alert dialog with message "test"
    When I run "chrome-cli dialog handle accept"
    Then the output JSON should contain "dialog_type" equal to "alert"
    And the output JSON should contain "message" equal to "test"
    And the exit code should be 0

  @regression
  Scenario: AC5 — dialog handle dismisses pre-existing dialogs
    Given a page has triggered an alert dialog with message "dismiss me"
    When I run "chrome-cli dialog handle accept"
    Then the exit code should be 0
    When I run "chrome-cli dialog info"
    Then the output JSON should contain "open" equal to false

  # --- Related Behavior Still Works ---

  @regression
  Scenario: AC3 — dialog info still works when no dialog is open
    Given no dialog is currently open
    When I run "chrome-cli dialog info"
    Then the output JSON should contain "open" equal to false
    And the exit code should be 0

  @regression
  Scenario: AC6 — dialog handle returns error when no dialog is open
    Given no dialog is currently open
    When I run "chrome-cli dialog handle accept"
    Then the exit code should be non-zero
    And the output should contain "No dialog is currently open"
