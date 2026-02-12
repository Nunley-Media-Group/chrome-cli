# File: tests/features/session-connection-management.feature
#
# Generated from: .claude/specs/session-connection-management/requirements.md
# Issue: #6

Feature: Session and connection management
  As a developer or automation engineer
  I want commands to automatically find and reuse a Chrome connection
  So that I don't have to pass connection details to every command

  # --- Session File Management ---

  @requires-chrome
  Scenario: Write session file after connect
    Given a Chrome instance is running with remote debugging
    When I run "chrome-cli connect"
    Then a session file should exist at "~/.chrome-cli/session.json"
    And the session file should contain "ws_url" as a string
    And the session file should contain "port" as a number
    And the session file should contain "timestamp" as a string
    And the exit code should be 0

  @requires-chrome
  Scenario: Write session file after launch
    Given Chrome is installed on the system
    When I run "chrome-cli connect --launch --headless"
    Then a session file should exist at "~/.chrome-cli/session.json"
    And the session file should contain "pid" as a number
    And the session file should contain "ws_url" as a string
    And the exit code should be 0

  @requires-chrome
  Scenario: Subsequent commands auto-read session file
    Given a valid session file exists with a reachable Chrome connection
    And no explicit connection flags are provided
    When I run a command that needs Chrome
    Then the command connects using the session file's ws_url
    And the command executes successfully

  @requires-chrome
  Scenario: Explicit flags override session file
    Given a valid session file exists pointing to port 9222
    And Chrome is running on port 9333
    When I run a command with "--port 9333"
    Then the command connects to port 9333
    And the session file is not consulted

  # --- Connection Status ---

  Scenario: Show connection status with valid reachable session
    Given a valid session file exists with ws_url "ws://127.0.0.1:9222/devtools/browser/abc"
    And Chrome is running on port 9222
    When I run "chrome-cli connect --status"
    Then the output should contain "ws_url"
    And the output should contain "reachable": true
    And the exit code should be 0

  Scenario: Show connection status with stale session
    Given a valid session file exists with ws_url "ws://127.0.0.1:9222/devtools/browser/abc"
    And Chrome is not running on port 9222
    When I run "chrome-cli connect --status"
    Then the output should contain "reachable": false
    And the exit code should be 0

  Scenario: Show connection status with no session
    Given no session file exists
    When I run "chrome-cli connect --status"
    Then stderr should contain "No active session"
    And the exit code should be non-zero

  # --- Disconnect ---

  Scenario: Disconnect removes session file
    Given a valid session file exists
    When I run "chrome-cli connect --disconnect"
    Then the session file should not exist
    And the output should contain "disconnected": true
    And the exit code should be 0

  @requires-chrome
  Scenario: Disconnect kills launched Chrome process
    Given a session file exists with pid 12345 from a launched Chrome
    When I run "chrome-cli connect --disconnect"
    Then the Chrome process 12345 should receive a termination signal
    And the session file should not exist
    And the output should contain "killed_pid"

  Scenario: Disconnect with no session is idempotent
    Given no session file exists
    When I run "chrome-cli connect --disconnect"
    Then the output should contain "disconnected": true
    And the exit code should be 0

  # --- Connection Resolution Chain ---

  Scenario: Connection resolution with no Chrome found
    Given no explicit connection flags are provided
    And no session file exists
    And no Chrome instance is running
    When I run a command that needs Chrome
    Then stderr should contain "No Chrome instance found"
    And stderr should contain "chrome-cli connect"
    And the exit code should be non-zero

  @requires-chrome
  Scenario: Health check before command execution
    Given a valid session file exists with a reachable Chrome connection
    When a command starts execution
    Then a health check is performed via "/json/version"
    And the health check completes in under 100ms

  Scenario: Stale session detection
    Given a session file exists but Chrome is not running at the stored address
    When I run a command that needs Chrome via session file
    Then stderr should contain "stale"
    And stderr should contain "chrome-cli connect"
    And the exit code should be non-zero

  # --- Tab Targeting ---

  @requires-chrome
  Scenario: Target tab by CDP target ID
    Given a connected Chrome instance with multiple tabs
    And a tab has target ID "ABCDEF123"
    When I run a command with "--tab ABCDEF123"
    Then the command targets the tab with ID "ABCDEF123"

  @requires-chrome
  Scenario: Target tab by numeric index
    Given a connected Chrome instance with 3 tabs
    When I run a command with "--tab 0"
    Then the command targets the first tab in the target list

  @requires-chrome
  Scenario: Default tab targeting selects first page
    Given a connected Chrome instance with tabs
    And the target list includes a "page" type target
    When I run a command without "--tab"
    Then the command targets the first "page" type target

  Scenario: Invalid tab ID error
    Given a connected Chrome instance
    When I run a command with "--tab nonexistent-id"
    Then stderr should contain "not found"
    And stderr should contain "tabs list"
    And the exit code should be non-zero

  Scenario: No page targets error
    Given a connected Chrome instance with no page-type targets
    When I run a command without "--tab"
    Then stderr should contain "No page targets found"
    And the exit code should be non-zero

  # --- CDP Session Management ---

  @requires-chrome
  Scenario: Create and cleanup CDP session for tab
    Given a connected Chrome instance with a page tab
    When a command targets that tab
    Then a CDP session is created via Target.attachToTarget
    And the command operates within the session context
    And the session is detached when the command completes

  @requires-chrome
  Scenario: Lazy domain enabling
    Given a CDP session is attached to a tab
    When a command requires only the Page domain
    Then only "Page.enable" is sent to the session
    And "Runtime.enable" and "DOM.enable" are not sent

  # --- Error Handling ---

  @requires-chrome
  Scenario: Chrome dies mid-session
    Given a CDP session is active and processing a command
    When Chrome crashes unexpectedly
    Then the tool reports a connection error
    And the tool does not panic or hang
    And the exit code should be non-zero

  Scenario: Corrupted session file handled gracefully
    Given a session file exists with invalid JSON content
    When I run "chrome-cli connect --status"
    Then stderr should contain an error about the session file
    And the exit code should be non-zero
