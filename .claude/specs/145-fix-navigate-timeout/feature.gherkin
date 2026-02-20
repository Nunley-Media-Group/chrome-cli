# File: tests/features/145-fix-navigate-timeout.feature
#
# Generated from: .claude/specs/145-fix-navigate-timeout/requirements.md
# Issue: #145
# Type: Defect regression

@regression
Feature: Navigate back/forward/reload respects global --timeout option
  The navigate back, navigate forward, and navigate reload commands previously
  ignored the global --timeout flag and CHROME_CLI_TIMEOUT environment variable,
  always using a hardcoded 30-second timeout. This was fixed by reading
  global.timeout with a fallback to DEFAULT_NAVIGATE_TIMEOUT_MS.

  Background:
    Given chrome-cli is built

  # --- Bug Is Fixed ---

  @regression
  Scenario: AC1 — navigate back respects --timeout
    Given a tab with navigation history
    When I run "chrome-cli --timeout 5000 navigate back" and the navigation event is not detected
    Then the command times out after approximately 5 seconds

  @regression
  Scenario: AC2 — navigate forward respects --timeout
    Given a tab with forward navigation history
    When I run "chrome-cli --timeout 5000 navigate forward" and the navigation event is not detected
    Then the command times out after approximately 5 seconds

  @regression
  Scenario: AC3 — navigate reload respects --timeout
    Given a tab with a loaded page
    When I run "chrome-cli --timeout 5000 navigate reload" and the load event is not detected
    Then the command times out after approximately 5 seconds

  @regression
  Scenario: AC4 — CHROME_CLI_TIMEOUT environment variable works for history navigation
    Given a tab with navigation history
    And CHROME_CLI_TIMEOUT is set to "5000"
    When I run "chrome-cli navigate back" and the navigation event is not detected
    Then the command times out after approximately 5 seconds

  # --- No Regression ---

  @regression
  Scenario: AC5 — default timeout preserved when no override specified
    Given a tab with navigation history
    When I run "chrome-cli navigate back" with no --timeout flag or CHROME_CLI_TIMEOUT set
    Then the command uses the default 30-second timeout

  @regression
  Scenario: AC6 — navigate URL still respects per-command --timeout
    Given a connected Chrome instance
    When I run "chrome-cli navigate https://example.com --timeout 5000"
    Then the exit code should be 0
    And the per-command timeout is used
