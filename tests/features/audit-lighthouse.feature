# File: tests/features/audit-lighthouse.feature
#
# Generated from: .claude/specs/feature-add-audit-lighthouse-command/requirements.md
# Issue: #169

Feature: Audit Lighthouse
  As an AI agent or automation engineer
  I want to run Google Lighthouse audits through the CLI
  So that I can get structured performance, accessibility, SEO, and best-practice scores

  Background:
    Given agentchrome is built

  # --- CLI Argument Validation (testable without Chrome) ---

  # AC1 (partial): Verify the audit lighthouse subcommand exists
  Scenario: Audit lighthouse help text is available
    When I run "agentchrome audit lighthouse --help"
    Then the exit code should be 0
    And stdout should contain "lighthouse"
    And stdout should contain "--only"
    And stdout should contain "--output-file"

  # AC2 (partial): Verify --only flag accepts comma-separated categories
  Scenario: Audit lighthouse accepts --only flag
    When I run "agentchrome audit lighthouse --help"
    Then stdout should contain "--only"

  # Subcommand required
  Scenario: Audit without subcommand exits with error
    When I run "agentchrome audit"
    Then the exit code should be nonzero
    And stderr should contain "subcommand"

  # --- Happy Path (requires Chrome + lighthouse binary) ---

  # AC1: Run a full Lighthouse audit on the current page
  # Scenario: Full Lighthouse audit returns all category scores
  #   Given a connected Chrome session on a page
  #   When I run "agentchrome audit lighthouse"
  #   Then the output is valid JSON on stdout
  #   And the output JSON has key "url"
  #   And the output JSON has key "performance"
  #   And the output JSON has key "accessibility"
  #   And the output JSON has key "best-practices"
  #   And the output JSON has key "seo"
  #   And the output JSON has key "pwa"
  #   And the exit code should be 0

  # AC2: Run a targeted audit for specific categories
  # Scenario: Lighthouse audit with --only returns only requested categories
  #   Given a connected Chrome session on a page
  #   When I run "agentchrome audit lighthouse --only performance,accessibility"
  #   Then the output is valid JSON on stdout
  #   And the output JSON has key "url"
  #   And the output JSON has key "performance"
  #   And the output JSON has key "accessibility"
  #   And stdout should not contain "seo"
  #   And stdout should not contain "best-practices"
  #   And stdout should not contain "pwa"
  #   And the exit code should be 0

  # AC3: Save the full report to a file
  # Scenario: Lighthouse audit with --output-file saves report and prints scores
  #   Given a connected Chrome session on a page
  #   When I run "agentchrome audit lighthouse --output-file /tmp/lh-test-report.json"
  #   Then the output is valid JSON on stdout
  #   And the output JSON has key "performance"
  #   And the file "/tmp/lh-test-report.json" exists and contains valid JSON
  #   And the exit code should be 0

  # AC4: Explicit URL argument overrides the active page
  # Scenario: Lighthouse audit with explicit URL overrides active page
  #   Given a connected Chrome session viewing "https://other.com"
  #   When I run "agentchrome audit lighthouse https://example.com"
  #   Then the output JSON "url" field contains "example.com"
  #   And the exit code should be 0

  # AC5: Lighthouse binary not found returns a structured error
  # Scenario: Error when lighthouse binary is not found
  #   Given a connected Chrome session
  #   And lighthouse binary is not in PATH
  #   When I run "agentchrome audit lighthouse"
  #   Then stderr should be valid JSON
  #   And stderr should contain "lighthouse binary not found"
  #   And stderr should contain "npm install -g lighthouse"
  #   And the exit code should be nonzero

  # AC6: No active session returns a connection error
  Scenario: Error when no Chrome session is connected
    When I run "agentchrome audit lighthouse"
    Then the exit code should be nonzero
    And stderr should be valid JSON
    And stderr JSON should have key "error"

  # AC7: Lighthouse execution failure returns a structured error
  # Scenario: Error when Lighthouse execution fails
  #   Given a connected Chrome session
  #   And lighthouse binary is in PATH
  #   When I run "agentchrome audit lighthouse https://invalid-url-that-will-fail"
  #   Then stderr should be valid JSON
  #   And stderr JSON should have key "error"
  #   And the exit code should be nonzero

  # AC8: The --port global flag is respected
  # Scenario: Lighthouse respects --port global flag
  #   Given Chrome is running on port 9333
  #   When I run "agentchrome --port 9333 audit lighthouse"
  #   Then the Lighthouse process is invoked with --port 9333
  #   And the exit code should be 0

  # =======================================================================
  # Issue #231 — Prerequisite handling
  # =======================================================================

  # Added by issue #231 — AC9: prereq surfaced in `audit lighthouse --help`
  Scenario: Audit lighthouse help text states the lighthouse prerequisite above examples
    When I run "agentchrome audit lighthouse --help"
    Then the exit code should be 0
    And stdout should contain "PREREQUISITES"
    And stdout should contain "lighthouse"
    And stdout should contain "npm install -g lighthouse"
    And stdout should contain "--install-prereqs"
    And in stdout "PREREQUISITES" appears before "EXAMPLES"

  # Added by issue #231 — AC12: audit group help mentions the prerequisite
  Scenario: Audit group help references the lighthouse CLI prerequisite
    When I run "agentchrome audit --help"
    Then the exit code should be 0
    And stdout should contain "lighthouse"

  # Added by issue #231 — AC12: top-level help references the prerequisite
  Scenario: Top-level help references the lighthouse CLI prerequisite
    When I run "agentchrome --help"
    Then the exit code should be 0
    And stdout should contain "lighthouse"

  # Added by issue #231 — AC10: --install-prereqs flag is wired through the CLI
  Scenario: Audit lighthouse exposes --install-prereqs flag
    When I run "agentchrome audit lighthouse --help"
    Then the exit code should be 0
    And stdout should contain "--install-prereqs"

  # Added by issue #231 — AC11: not-found error points at both install paths
  # (Scenario requires lighthouse binary to be absent from PATH — verified via
  # manual smoke test when lighthouse is not installed.)
  # Scenario: Not-found error lists both install paths in a single JSON object
  #   Given lighthouse binary is not in PATH
  #   And a connected Chrome session on a page
  #   When I run "agentchrome audit lighthouse https://example.com"
  #   Then the exit code should be nonzero
  #   And stderr should be valid JSON
  #   And stderr should contain "npm install -g lighthouse"
  #   And stderr should contain "--install-prereqs"

  # Added by issue #231 — AC10: --install-prereqs with npm missing (smoke-only;
  # requires controlled PATH with npm absent, verified via manual smoke test)
  # Scenario: --install-prereqs with npm missing emits Node.js error
  #   Given npm is not in PATH
  #   When I run "agentchrome audit lighthouse --install-prereqs"
  #   Then the exit code should be nonzero
  #   And stderr should be valid JSON
  #   And stderr should contain "npm not found on PATH"
  #   And stderr should contain "Node.js"
