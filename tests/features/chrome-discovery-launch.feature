# File: tests/features/chrome-discovery-launch.feature
#
# Generated from: .claude/specs/5-chrome-instance-discovery-and-launch/requirements.md
# Issue: #5

Feature: Chrome instance discovery and launch
  As a developer or automation engineer
  I want chrome-cli to discover running Chrome instances and launch new ones
  So that I can seamlessly connect to Chrome for browser automation

  # --- Connect help ---

  Scenario: Connect help displays all options
    Given chrome-cli is built
    When I run "chrome-cli connect --help"
    Then the exit code should be 0
    And stdout should contain "--launch"
    And stdout should contain "--headless"
    And stdout should contain "--channel"
    And stdout should contain "--chrome-path"
    And stdout should contain "--chrome-arg"

  # --- Direct WebSocket URL (AC7) ---

  Scenario: Connect via direct WebSocket URL
    Given chrome-cli is built
    When I run "chrome-cli --ws-url ws://127.0.0.1:9222/devtools/browser/test-id connect"
    Then the exit code should be 0
    And stdout should be valid JSON
    And stdout JSON should have key "ws_url"
    And stdout JSON should have key "port"

  Scenario: WebSocket URL port is extracted correctly
    Given chrome-cli is built
    When I run "chrome-cli --ws-url ws://127.0.0.1:9333/devtools/browser/abc connect"
    Then the exit code should be 0
    And stdout should be valid JSON
    And stdout should contain "9333"

  # --- Error handling (AC11, AC12, AC13) ---

  Scenario: Connect with no Chrome running produces error JSON (AC13)
    Given chrome-cli is built
    When I run "chrome-cli --port 19222 connect"
    Then the exit code should be nonzero
    And stderr should be valid JSON
    And stderr JSON should have key "error"
    And stderr JSON should have key "code"

  Scenario: Launch with invalid chrome-path produces error
    Given chrome-cli is built
    When I run "chrome-cli connect --launch --chrome-path /nonexistent/chrome"
    Then the exit code should be nonzero
    And stderr should be valid JSON
    And stderr JSON should have key "error"

  Scenario: Chrome not found error suggests --chrome-path (AC11)
    Given chrome-cli is built
    When I run "chrome-cli connect --launch --chrome-path /nonexistent/chrome"
    Then the exit code should be nonzero
    And stderr should contain "error"

  Scenario: Error output includes exit code in JSON
    Given chrome-cli is built
    When I run "chrome-cli --port 19222 connect"
    Then stderr should be valid JSON
    And stderr JSON should have key "code"

  # --- Argument validation ---

  Scenario: Headless requires launch flag
    Given chrome-cli is built
    When I run "chrome-cli connect --headless"
    Then the exit code should be 2
    And stderr should contain "required"

  Scenario: Channel requires launch flag
    Given chrome-cli is built
    When I run "chrome-cli connect --channel canary"
    Then the exit code should be 2
    And stderr should contain "required"

  Scenario: Chrome-path requires launch flag
    Given chrome-cli is built
    When I run "chrome-cli connect --chrome-path /some/path"
    Then the exit code should be 2
    And stderr should contain "required"

  Scenario: Chrome-arg requires launch flag
    Given chrome-cli is built
    When I run "chrome-cli connect --chrome-arg=--disable-gpu"
    Then the exit code should be 2
    And stderr should contain "required"

  # --- Non-localhost warning (security) ---

  Scenario: Non-localhost host emits warning
    Given chrome-cli is built
    When I run "chrome-cli --host 192.168.1.100 --ws-url ws://192.168.1.100:9222/test connect"
    Then stderr should contain "non-localhost"

  # --- Connect with no flags (AC8 auto-discover path) ---

  Scenario: Connect with unused port falls back gracefully
    Given chrome-cli is built
    When I run "chrome-cli --port 19222 connect"
    Then the exit code should be nonzero
    And stderr should be valid JSON
