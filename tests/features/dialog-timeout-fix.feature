# File: tests/features/dialog-timeout-fix.feature
#
# Generated from: .claude/specs/86-fix-dialog-commands-timeout-with-open-dialog/requirements.md
# Issue: #86
# Type: Defect regression

@regression
Feature: Dialog commands work when a dialog is already open
  The dialog info and dialog handle commands previously timed out on
  Page.enable when a JavaScript dialog was already open, making them
  non-functional in their primary use case.
  This was fixed by skipping blocking domain enablement in dialog commands.

  Background:
    Given chrome-cli is built

  # --- Bug Is Fixed ---

  @regression
  Scenario: AC1 — dialog info works with open alert dialog
    Given a page has triggered an alert dialog with message "test"
    When I run "chrome-cli dialog info"
    Then the output JSON should contain "open" equal to true
    And the output JSON should contain "type" equal to "alert"
    And the output JSON should contain "message" equal to "test"
    And the exit code should be 0

  @regression
  Scenario: AC2 — dialog handle accept works with open alert dialog
    Given a page has triggered an alert dialog with message "test"
    When I run "chrome-cli dialog handle accept"
    Then the output JSON should contain "action" equal to "accept"
    And the exit code should be 0

  @regression
  Scenario: AC3 — dialog handle dismiss works with open confirm dialog
    Given a page has triggered a confirm dialog with message "Are you sure?"
    When I run "chrome-cli dialog handle dismiss"
    Then the output JSON should contain "action" equal to "dismiss"
    And the exit code should be 0

  @regression
  Scenario: AC4 — dialog handle accept with text works for prompt dialog
    Given a page has triggered a prompt dialog with message "Enter name:"
    When I run "chrome-cli dialog handle accept --text answer"
    Then the output JSON should contain "action" equal to "accept"
    And the output JSON should contain "text" equal to "answer"
    And the exit code should be 0

  # --- Related Behavior Still Works ---

  @regression
  Scenario: AC5 — dialog info still reports no dialog when none is open
    Given no dialog is currently open
    When I run "chrome-cli dialog info"
    Then the output JSON should contain "open" equal to false
    And the exit code should be 0

  @regression
  Scenario: AC6 — dialog handle still errors when no dialog is open
    Given no dialog is currently open
    When I run "chrome-cli dialog handle accept"
    Then the exit code should be non-zero
    And stderr should contain an error about no dialog being open
