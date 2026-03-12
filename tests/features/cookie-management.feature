# File: tests/features/cookie-management.feature
#
# Generated from: .claude/specs/feature-add-cookie-management-command-group/requirements.md
# Issue: #164

Feature: Cookie Management
  As an AI agent or automation engineer
  I want commands to read, set, and clear browser cookies
  So that I can manage authentication state and test session handling

  Background:
    Given agentchrome is built

  # --- Happy Path ---

  # AC1: List all cookies for the current page
  Scenario: List all cookies for the current page
    Given a connected Chrome session on a page with cookies
    When I run "agentchrome cookie list"
    Then the output is a JSON array
    And each cookie object contains "name", "value", "domain", "path", "expires", "httpOnly", "secure", and "sameSite" fields
    And the exit code should be 0

  # AC2: Set a cookie
  Scenario: Set a cookie
    Given a connected Chrome session
    When I run "agentchrome cookie set session_id abc123 --domain example.com"
    Then the output JSON should contain "success" equal to true
    And the output JSON should contain "name" equal to "session_id"
    And the exit code should be 0

  # AC2 (cross-validation): Verify set cookie appears in list
  Scenario: Set cookie is visible in subsequent list
    Given a connected Chrome session
    And I have set a cookie "session_id" with value "abc123" on domain "example.com"
    When I run "agentchrome cookie list --all"
    Then the output JSON array should contain a cookie with "name" equal to "session_id"
    And that cookie should have "value" equal to "abc123"

  # AC3: Delete a specific cookie
  Scenario: Delete a specific cookie
    Given a connected Chrome session with a cookie named "session_id"
    When I run "agentchrome cookie delete session_id"
    Then the output JSON should contain "deleted" equal to 1
    And the exit code should be 0

  # AC3 (cross-validation): Verify deleted cookie is gone from list
  Scenario: Deleted cookie no longer appears in list
    Given a connected Chrome session
    And I have set a cookie "session_id" with value "abc123" on domain "example.com"
    And I have deleted the cookie "session_id"
    When I run "agentchrome cookie list --all"
    Then the output JSON array should not contain a cookie with "name" equal to "session_id"

  # AC4: Clear all cookies
  Scenario: Clear all cookies
    Given a connected Chrome session with multiple cookies
    When I run "agentchrome cookie clear"
    Then the output JSON should contain "deleted"
    And the exit code should be 0

  # AC4 (cross-validation): Verify clear removes all cookies
  Scenario: After clear, cookie list returns empty array
    Given a connected Chrome session with multiple cookies
    And I have cleared all cookies
    When I run "agentchrome cookie list --all"
    Then the output is an empty JSON array

  # --- Filtering ---

  # AC5: List cookies filtered by domain
  Scenario: List cookies filtered by domain
    Given a connected Chrome session with cookies from multiple domains
    When I run "agentchrome cookie list --domain example.com"
    Then only cookies matching domain "example.com" are returned
    And cookies from other domains are excluded

  # AC6: Set a cookie with optional flags
  Scenario: Set a cookie with all optional flags
    Given a connected Chrome session
    When I run "agentchrome cookie set secure_token xyz --domain example.com --secure --http-only --same-site Strict --path /api --expires 1735689600"
    Then the output JSON should contain "success" equal to true
    And a subsequent cookie list shows the cookie with secure, httpOnly, sameSite, path, and expires attributes correctly applied

  # AC7: Delete a cookie scoped by domain
  Scenario: Delete a cookie scoped by domain
    Given a connected Chrome session with cookies named "token" on "a.example.com" and "b.example.com"
    When I run "agentchrome cookie delete token --domain a.example.com"
    Then only the cookie on "a.example.com" is deleted
    And the cookie on "b.example.com" remains

  # --- Edge Cases ---

  # AC8: Empty cookie list
  Scenario: Cookie list returns empty array when no cookies exist
    Given a connected Chrome session on a page with no cookies
    When I run "agentchrome cookie list"
    Then the output is an empty JSON array
    And the exit code should be 0

  # AC9: JSON output format compliance
  Scenario: Cookie commands produce JSON on stdout
    Given a connected Chrome session
    When I run "agentchrome cookie list"
    Then the output is valid JSON on stdout

  Scenario: Cookie command errors produce JSON on stderr
    Given no Chrome session is connected
    When I run "agentchrome cookie list"
    Then stderr contains a JSON error object
    And the exit code should be non-zero

  # AC10: Cross-invocation state persistence
  Scenario: Cookies persist across CLI invocations
    Given a connected Chrome session
    And I set a cookie "persist" with value "yes" on domain "example.com" in one invocation
    When I run "agentchrome cookie list --all" in a separate invocation
    Then the output JSON array should contain a cookie with "name" equal to "persist"

  # --- List with --all flag ---

  Scenario: List all cookies regardless of current URL
    Given a connected Chrome session
    When I run "agentchrome cookie list --all"
    Then the output is a JSON array containing cookies from all domains

  # --- CLI Argument Validation (testable without Chrome) ---

  Scenario: Cookie set requires name and value arguments
    When I run "agentchrome cookie set"
    Then the exit code should be non-zero
    And stderr should contain "required"

  Scenario: Cookie delete requires name argument
    When I run "agentchrome cookie delete"
    Then the exit code should be non-zero
    And stderr should contain "required"

  Scenario: Cookie subcommand is required
    When I run "agentchrome cookie"
    Then the exit code should be non-zero
    And stderr should contain "subcommand"
