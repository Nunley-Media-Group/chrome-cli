Feature: Browser Dialog Handling
  As a developer / automation engineer
  I want to detect and handle browser dialogs from the CLI
  So that my automation scripts can respond to dialogs programmatically

  Background:
    Given chrome-cli is built

  # AC1: Accept an alert dialog
  Scenario: Accept an alert dialog
    Given a page has triggered an alert dialog with message "Hello"
    When I run "chrome-cli dialog handle accept"
    Then the output JSON should contain "action" equal to "accept"
    And the output JSON should contain "dialog_type" equal to "alert"
    And the output JSON should contain "message" equal to "Hello"
    And the exit code should be 0

  # AC2: Dismiss a confirm dialog
  Scenario: Dismiss a confirm dialog
    Given a page has triggered a confirm dialog with message "Are you sure?"
    When I run "chrome-cli dialog handle dismiss"
    Then the output JSON should contain "action" equal to "dismiss"
    And the output JSON should contain "dialog_type" equal to "confirm"
    And the output JSON should contain "message" equal to "Are you sure?"

  # AC3: Accept a prompt dialog with text
  Scenario: Accept a prompt dialog with text
    Given a page has triggered a prompt dialog with message "Enter name:"
    When I run "chrome-cli dialog handle accept --text Alice"
    Then the output JSON should contain "action" equal to "accept"
    And the output JSON should contain "dialog_type" equal to "prompt"
    And the output JSON should contain "text" equal to "Alice"

  # AC4: Handle a beforeunload dialog
  Scenario: Handle a beforeunload dialog
    Given a page has registered a beforeunload handler
    When a navigation is triggered and a beforeunload dialog appears
    And I run "chrome-cli dialog handle accept"
    Then the output JSON should contain "dialog_type" equal to "beforeunload"
    And the output JSON should contain "action" equal to "accept"

  # AC5: Query dialog info when open
  Scenario: Query dialog info when open
    Given a page has triggered a prompt dialog with message "Enter name:" and default "default"
    When I run "chrome-cli dialog info"
    Then the output JSON should contain "open" equal to true
    And the output JSON should contain "type" equal to "prompt"
    And the output JSON should contain "message" equal to "Enter name:"
    And the output JSON should contain "default_value" equal to "default"

  # AC6: Query dialog info when no dialog is open
  Scenario: Query dialog info when no dialog is open
    Given no dialog is currently open
    When I run "chrome-cli dialog info"
    Then the output JSON should contain "open" equal to false

  # AC7: Handle dialog with tab targeting
  Scenario: Handle dialog with tab targeting
    Given a dialog is open on tab "ABC123"
    When I run "chrome-cli dialog handle accept --tab ABC123"
    Then the dialog on that tab is accepted

  # AC8: Auto-dismiss dialogs during a command
  Scenario: Auto-dismiss dialogs during a command
    Given a page will trigger an alert during navigation
    When I run "chrome-cli navigate https://example.com --auto-dismiss-dialogs"
    Then the navigation completes without blocking

  # AC9: Handle dialog when none is open (error)
  Scenario: Handle dialog when none is open
    Given no dialog is currently open
    When I run "chrome-cli dialog handle accept"
    Then stderr should contain an error about no dialog being open
    And the exit code should be non-zero

  # AC10: Plain text output for dialog handle
  Scenario: Plain text output for dialog handle
    Given a page has triggered an alert dialog with message "Hello"
    When I run "chrome-cli dialog handle accept --plain"
    Then the output should be plain text containing "Accepted" and "alert"

  # AC11: Plain text output for dialog info
  Scenario: Plain text output for dialog info
    Given a page has triggered a confirm dialog with message "Continue?"
    When I run "chrome-cli dialog info --plain"
    Then the output should be plain text containing "confirm" and "Continue?"

  # CLI argument validation (testable without Chrome)
  Scenario: Dialog handle requires an action argument
    When I run "chrome-cli dialog handle"
    Then the exit code should be non-zero
    And stderr should contain "required"

  Scenario: Dialog handle rejects invalid action
    When I run "chrome-cli dialog handle invalid"
    Then the exit code should be non-zero
    And stderr should contain "invalid"
