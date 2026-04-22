# File: tests/features/226-session-autodiscovery.feature
#
# Issue: #226 — Windows Auto-Discovery Reliability & Status UX
# Generated from: specs/feature-session-and-connection-management/feature.gherkin

@regression
Feature: Session auto-discovery hardening and --status UX (issue #226)
  After issue #226, `connect --status` returns exit 0 regardless of whether a
  session exists, the session file path + resolution precedence are
  documented in `connect --help`, and `agentchrome capabilities` surfaces a
  `connect.session_file` object for scripted consumers.

  # --- AC39: --status exit-code contract ---

  Scenario: connect --status returns exit 0 when no session exists
    Given no session file exists
    When I run "agentchrome connect --status"
    Then the output should contain "active": false
    And the exit code should be 0

  # --- AC42: connect --help documents session file path + precedence ---

  Scenario: connect --help lists session file paths per platform
    Given agentchrome is built
    When I run "agentchrome connect --help"
    Then stdout should contain "~/.agentchrome/session.json"
    And stdout should contain "%USERPROFILE%\.agentchrome\session.json"

  Scenario: connect --help lists resolution precedence
    Given agentchrome is built
    When I run "agentchrome connect --help"
    Then stdout should contain "--ws-url"
    And stdout should contain "--port"
    And stdout should contain "AGENTCHROME_PORT"
    And stdout should contain "session.json"
    And stdout should contain "9222"

  Scenario: connect --help includes cross-invocation EXAMPLES
    Given agentchrome is built
    When I run "agentchrome connect --help"
    Then stdout should contain "EXAMPLES:"
    And stdout should contain "connect --launch --headless"
    And stdout should contain "tabs list"

  # --- AC43: capabilities manifest surfaces session_file object ---

  Scenario: capabilities manifest includes connect.session_file
    Given agentchrome is built
    When I run "agentchrome capabilities connect"
    Then stdout should contain "session_file"
    And stdout should contain "path_unix"
    And stdout should contain "path_windows"
    And stdout should contain "precedence"
    And stdout should contain "AGENTCHROME_PORT"
