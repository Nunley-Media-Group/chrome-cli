# File: tests/features/emulate.feature
#
# Generated from: .claude/specs/21-device-network-viewport-emulation/requirements.md
# Issue: #21

Feature: Device, network, and viewport emulation
  As a developer or automation engineer
  I want to emulate different devices, network conditions, geolocations, and color schemes
  So that I can test web page behavior under various device and network constraints

  # --- CLI Argument Validation (testable without Chrome) ---

  Scenario: Emulate help displays all subcommands
    Given chrome-cli is built
    When I run "chrome-cli emulate --help"
    Then the exit code should be 0
    And stdout should contain "set"
    And stdout should contain "reset"
    And stdout should contain "status"

  Scenario: Emulate set help displays all flags
    Given chrome-cli is built
    When I run "chrome-cli emulate set --help"
    Then the exit code should be 0
    And stdout should contain "--network"
    And stdout should contain "--cpu"
    And stdout should contain "--geolocation"
    And stdout should contain "--no-geolocation"
    And stdout should contain "--user-agent"
    And stdout should contain "--no-user-agent"
    And stdout should contain "--color-scheme"
    And stdout should contain "--viewport"
    And stdout should contain "--device-scale"
    And stdout should contain "--mobile"

  Scenario: Invalid network profile produces error
    Given chrome-cli is built
    When I run "chrome-cli emulate set --network invalid-profile"
    Then the exit code should be nonzero
    And stderr should contain "possible values"

  Scenario: CPU throttling rate out of range produces error
    Given chrome-cli is built
    When I run "chrome-cli emulate set --cpu 0"
    Then the exit code should be nonzero
    And stderr should contain "not in 1..=20"

  Scenario: Geolocation and no-geolocation are mutually exclusive
    Given chrome-cli is built
    When I run "chrome-cli emulate set --geolocation 37.7,-122.4 --no-geolocation"
    Then the exit code should be nonzero
    And stderr should contain "cannot be used with"

  Scenario: User-agent and no-user-agent are mutually exclusive
    Given chrome-cli is built
    When I run "chrome-cli emulate set --user-agent test --no-user-agent"
    Then the exit code should be nonzero
    And stderr should contain "cannot be used with"

  Scenario: Page resize help displays size argument
    Given chrome-cli is built
    When I run "chrome-cli page resize --help"
    Then the exit code should be 0
    And stdout should contain "SIZE"

  Scenario: Page resize with invalid format produces error
    Given chrome-cli is built
    When I run "chrome-cli page resize badformat"
    Then the exit code should be nonzero
    And stderr should contain "WIDTHxHEIGHT"

  # --- CDP Integration Tests (require Chrome) ---

  # Background:
  #   Given Chrome is running with CDP enabled

  # Scenario Outline: Set network emulation profile
  #   When I run "chrome-cli emulate set --network <profile>"
  #   Then the JSON output should contain "network" set to "<profile>"
  #   And the exit code should be 0
  #
  #   Examples:
  #     | profile  |
  #     | offline  |
  #     | slow-4g  |
  #     | 4g       |
  #     | 3g       |
  #     | none     |

  # Scenario: Set CPU throttling rate
  #   When I run "chrome-cli emulate set --cpu 4"
  #   Then the JSON output should contain "cpu" set to 4
  #   And the exit code should be 0

  # Scenario: Set geolocation override
  #   When I run "chrome-cli emulate set --geolocation 37.7749,-122.4194"
  #   Then the JSON output should contain "geolocation" with "latitude" 37.7749
  #   And the JSON output should contain "geolocation" with "longitude" -122.4194

  # Scenario: Clear geolocation override
  #   Given geolocation is overridden to "37.7749,-122.4194"
  #   When I run "chrome-cli emulate set --no-geolocation"
  #   Then the JSON output should contain "geolocation" set to null

  # Scenario: Set custom user agent
  #   When I run "chrome-cli emulate set --user-agent 'Mozilla/5.0 Custom'"
  #   Then the JSON output should contain "userAgent" set to "Mozilla/5.0 Custom"

  # Scenario: Reset user agent to default
  #   Given user agent is overridden
  #   When I run "chrome-cli emulate set --no-user-agent"
  #   Then the JSON output should contain "userAgent" set to null

  # Scenario Outline: Set color scheme emulation
  #   When I run "chrome-cli emulate set --color-scheme <scheme>"
  #   Then the JSON output should contain "colorScheme" set to "<scheme>"
  #
  #   Examples:
  #     | scheme |
  #     | dark   |
  #     | light  |
  #     | auto   |

  # Scenario: Set viewport dimensions
  #   When I run "chrome-cli emulate set --viewport 375x667"
  #   Then the JSON output should contain "viewport" with "width" 375
  #   And the JSON output should contain "viewport" with "height" 667

  # Scenario: Set device scale factor
  #   When I run "chrome-cli emulate set --device-scale 2"
  #   Then the JSON output should contain "deviceScaleFactor" set to 2.0

  # Scenario: Enable mobile emulation
  #   When I run "chrome-cli emulate set --mobile --viewport 375x667"
  #   Then the JSON output should contain "mobile" set to true

  # Scenario: Combine multiple emulation settings
  #   When I run "chrome-cli emulate set --network slow-4g --viewport 375x667 --mobile --color-scheme dark"
  #   Then the JSON output should contain "network" set to "slow-4g"
  #   And the JSON output should contain "viewport" with "width" 375
  #   And the JSON output should contain "mobile" set to true
  #   And the JSON output should contain "colorScheme" set to "dark"

  # Scenario: Reset all emulation overrides
  #   Given emulation overrides are active
  #   When I run "chrome-cli emulate reset"
  #   Then the JSON output should contain "reset" set to true

  # Scenario: Show current emulation status
  #   When I run "chrome-cli emulate status"
  #   Then the JSON output should contain "viewport"
  #   And the JSON output should contain "userAgent"

  # Scenario: Page resize shorthand
  #   When I run "chrome-cli page resize 1280x720"
  #   Then the JSON output should contain "width" set to 1280
  #   And the JSON output should contain "height" set to 720
