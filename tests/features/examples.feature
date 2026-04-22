# File: tests/features/examples.feature
#
# Generated from: .claude/specs/29-built-in-examples-subcommand/requirements.md
# Issue: #29

Feature: Built-in Examples Subcommand
  As a developer or AI agent using agentchrome
  I want a dedicated examples subcommand that prints usage examples
  So that I can discover working CLI invocations without parsing --help output

  # --- Happy Path ---

  Scenario: List all command groups with summary examples
    Given the agentchrome binary is available
    When I run "agentchrome examples"
    Then stdout should contain "connect"
    And stdout should contain "tabs"
    And stdout should contain "navigate"
    And stdout should contain "page"
    And stdout should contain "js"
    And stdout should contain "console"
    And stdout should contain "network"
    And stdout should contain "interact"
    And stdout should contain "form"
    And stdout should contain "emulate"
    And stdout should contain "perf"
    And stdout should contain "dialog"
    And stdout should contain "config"
    And the exit code should be 0

  Scenario: Show detailed examples for a specific command group
    Given the agentchrome binary is available
    When I run "agentchrome examples navigate"
    Then stdout should contain "agentchrome navigate"
    And stdout should contain "#"
    And the output should have at least 3 example commands
    And the exit code should be 0

  # --- Output Formats ---

  Scenario: Plain text output is the default
    Given the agentchrome binary is available
    When I run "agentchrome examples"
    Then stdout should not start with "["
    And stdout should not start with "{"
    And the exit code should be 0

  Scenario: JSON output for summary listing (progressive disclosure, AC13)
    Given the agentchrome binary is available
    When I run "agentchrome examples --json"
    Then stdout should be a valid JSON array
    And each JSON entry should have a "command" field
    And each JSON entry should have a "description" field
    And no JSON entry should have a "examples" field
    And the JSON payload size should be less than 4096 bytes
    And the exit code should be 0

  Scenario: JSON output for a command group still carries examples (AC14)
    Given the agentchrome binary is available
    When I run "agentchrome examples navigate --json"
    Then stdout should be a valid JSON object
    And the JSON "examples" array should have at least 1 entries
    And the exit code should be 0

  Scenario: JSON output for a specific command group
    Given the agentchrome binary is available
    When I run "agentchrome examples navigate --json"
    Then stdout should be a valid JSON object
    And the JSON "command" field should be "navigate"
    And the JSON "examples" array should have at least 3 entries
    And the exit code should be 0

  Scenario: Pretty-printed JSON output
    Given the agentchrome binary is available
    When I run "agentchrome examples --pretty"
    Then stdout should be a valid JSON array
    And stdout should be multi-line
    And the exit code should be 0

  # --- Error Handling ---

  Scenario: Error on unknown command group
    Given the agentchrome binary is available
    When I run "agentchrome examples nonexistent"
    Then the exit code should be 1
    And stderr should contain "Unknown command group"

  # --- Coverage ---

  Scenario Outline: Each command group has at least 3 examples
    Given the agentchrome binary is available
    When I run "agentchrome examples <group> --json"
    Then stdout should be a valid JSON object
    And the JSON "examples" array should have at least 3 entries
    And the exit code should be 0

    Examples:
      | group    |
      | connect  |
      | tabs     |
      | navigate |
      | page     |
      | dom      |
      | js       |
      | console  |
      | network  |
      | interact |
      | form     |
      | emulate  |
      | perf     |
      | dialog   |
      | config   |
