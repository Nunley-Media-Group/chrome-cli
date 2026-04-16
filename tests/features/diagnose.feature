# File: tests/features/diagnose.feature
#
# Issue: #200 — Diagnose Command for Pre-Automation Challenge Scanning

Feature: Diagnose Command for Pre-Automation Challenge Scanning
  As a browser automation engineer starting work on a new target page
  I want an automated diagnostic scan that identifies potential automation challenges before I begin
  So that I can set expectations and choose the right interaction strategy from the start

  # --- Argument parsing (testable without Chrome) ---

  Scenario: Missing URL without --current is an argument error
    Given the agentchrome binary is built
    When I run the command "agentchrome diagnose"
    Then the command exits with code 1
    And stdout is empty
    And stderr contains a JSON error object

  Scenario: Supplying both URL and --current is mutually exclusive
    Given the agentchrome binary is built
    When I run the command "agentchrome diagnose https://example.com --current"
    Then the command exits with code 1
    And stdout is empty
    And stderr contains a JSON error object

  Scenario: URL argument parses successfully
    Given the agentchrome binary is built
    When I run the command "agentchrome diagnose https://example.com --help"
    Then the command exits with code 0

  Scenario: --current flag parses successfully with help
    Given the agentchrome binary is built
    When I run the command "agentchrome diagnose --help"
    Then the command exits with code 0
    And stdout contains "diagnose"
    And stdout contains "--current"

  Scenario: Documentation and examples are discoverable
    Given the agentchrome binary is built
    When I run the command "agentchrome examples diagnose"
    Then the command exits with code 0
    And stdout contains "diagnose"
    And stdout contains "--current"

  # --- Chrome-dependent scenarios (skipped in CI, verified via manual smoke test) ---
  # The following scenarios require a live Chrome instance and are validated
  # via the manual smoke test procedure in T018.

  # Scenario: Diagnose a URL produces a structured JSON report (AC1)
  # Scenario: Known pattern suggestions include actionable agentchrome commands (AC2)
  # Scenario: Clean page reports straightforward (AC3)
  # Scenario: Diagnose the current page does not issue a navigation (AC4)
  # Scenario: Cross-origin iframe fields serialize as null, not omitted or zero (AC6)
  # Scenario: Undetermined fields serialize as null (AC7)
  # Scenario: Navigation failure is reported with the same exit code shape as navigate (AC10)
  # Scenario: No active Chrome session is reported as a connection error (AC11)
  # Scenario: Unknown patterns or missing signals do not cause failure (AC12)
