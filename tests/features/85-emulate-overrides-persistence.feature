# File: tests/features/85-emulate-overrides-persistence.feature
#
# Generated from: .claude/specs/85-fix-emulate-overrides-persistence/requirements.md
# Issue: #85
# Type: Defect regression

@regression
Feature: Emulate set overrides persist across commands
  The emulate set overrides (user-agent, device scale factor, geolocation,
  color scheme) previously did not persist across CLI invocations because
  CDP session-scoped overrides were lost when the command's WebSocket
  session closed.
  This was fixed by expanding EmulateState persistence and re-applying
  persisted overrides when new CDP sessions are created.

  # --- Bug Is Fixed ---

  @regression
  Scenario: User agent persists across commands
    Given Chrome is running with CDP enabled
    And I run "chrome-cli emulate set --user-agent \"TestBot/1.0\" --pretty"
    When I run "chrome-cli js exec \"navigator.userAgent\" --pretty"
    Then the output contains "TestBot/1.0"

  @regression
  Scenario: Device scale factor persists and is reported correctly
    Given Chrome is running with CDP enabled
    And I run "chrome-cli emulate set --device-scale 2 --pretty"
    When I run "chrome-cli emulate status --pretty"
    Then the JSON output field "deviceScaleFactor" is 2.0

  @regression
  Scenario: Geolocation is shown in status
    Given Chrome is running with CDP enabled
    And I run "chrome-cli emulate set --geolocation \"37.7749,-122.4194\" --pretty"
    When I run "chrome-cli emulate status --pretty"
    Then the JSON output field "geolocation.latitude" is 37.7749
    And the JSON output field "geolocation.longitude" is -122.4194

  # --- Reset Still Works ---

  @regression
  Scenario: Reset clears all overrides
    Given Chrome is running with CDP enabled
    And I run "chrome-cli emulate set --user-agent \"TestBot/1.0\" --viewport 375x812 --geolocation \"37.7749,-122.4194\" --color-scheme dark --pretty"
    When I run "chrome-cli emulate reset --pretty"
    And I run "chrome-cli js exec \"navigator.userAgent\" --pretty"
    Then the output does not contain "TestBot/1.0"

  # --- Related Behavior Still Works ---

  @regression
  Scenario: Existing mobile/network/cpu persistence still works
    Given Chrome is running with CDP enabled
    And I run "chrome-cli emulate set --mobile --network slow-4g --cpu 4 --pretty"
    When I run "chrome-cli emulate status --pretty"
    Then the JSON output field "mobile" is true
    And the JSON output field "network" is "slow-4g"
    And the JSON output field "cpu" is 4
