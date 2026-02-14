# File: tests/features/capabilities.feature
#
# Generated from: .claude/specs/30-capabilities-manifest-subcommand/requirements.md
# Issue: #30

Feature: Machine-Readable Capabilities Manifest Subcommand
  As a developer or AI agent integrating with chrome-cli
  I want a capabilities subcommand that outputs a machine-readable manifest
  So that I can programmatically discover the full CLI surface

  # --- Happy Path ---

  Scenario: Full capabilities manifest output
    Given chrome-cli is installed
    When I run "chrome-cli capabilities"
    Then the output is valid JSON
    And the JSON has key "name" with value "chrome-cli"
    And the JSON has key "version"
    And the JSON has a "commands" array
    And the "commands" array is not empty
    And the exit code is 0

  Scenario: Command entries include full metadata
    Given chrome-cli is installed
    When I run "chrome-cli capabilities"
    Then the output is valid JSON
    And every command has "name" and "description" fields
    And commands with subcommands have a "subcommands" array

  # --- Filtering ---

  Scenario: Filter by specific command
    Given chrome-cli is installed
    When I run "chrome-cli capabilities --command navigate"
    Then the output is valid JSON
    And the "commands" array has exactly 1 entry
    And the first command has name "navigate"
    And the exit code is 0

  Scenario: Compact output mode
    Given chrome-cli is installed
    When I run "chrome-cli capabilities --compact"
    Then the output is valid JSON
    And every command has "name" and "description" fields
    And no command has "subcommands"
    And the JSON does not have key "global_flags"
    And the JSON does not have key "exit_codes"
    And the exit code is 0

  # --- Global Flags ---

  Scenario: Global flags are included
    Given chrome-cli is installed
    When I run "chrome-cli capabilities"
    Then the output is valid JSON
    And the JSON has a "global_flags" array
    And "global_flags" includes "--port"
    And "global_flags" includes "--host"
    And "global_flags" includes "--timeout"
    And "global_flags" includes "--tab"
    And "global_flags" includes "--json"
    And "global_flags" includes "--pretty"
    And "global_flags" includes "--plain"

  # --- Exit Codes ---

  Scenario: Exit codes are documented
    Given chrome-cli is installed
    When I run "chrome-cli capabilities"
    Then the output is valid JSON
    And the JSON has an "exit_codes" array
    And "exit_codes" contains code 0 named "Success"
    And "exit_codes" contains code 1 named "GeneralError"
    And "exit_codes" contains code 2 named "ConnectionError"
    And "exit_codes" contains code 3 named "TargetError"
    And "exit_codes" contains code 4 named "TimeoutError"
    And "exit_codes" contains code 5 named "ProtocolError"

  # --- Enum Values ---

  Scenario: Enum values are listed for flags
    Given chrome-cli is installed
    When I run "chrome-cli capabilities --command navigate"
    Then the output is valid JSON
    And a subcommand has flag "--wait-until" with type "enum"
    And the "--wait-until" flag has values "load", "domcontentloaded", "networkidle", "none"

  # --- Output Formats ---

  Scenario: Pretty-printed JSON output
    Given chrome-cli is installed
    When I run "chrome-cli capabilities --pretty"
    Then the output is valid JSON
    And the output is multi-line
    And the exit code is 0

  # --- Error Handling ---

  Scenario: Error on unknown command filter
    Given chrome-cli is installed
    When I run "chrome-cli capabilities --command nonexistent"
    Then the exit code is 1
    And stderr contains "Unknown command"

  # --- Auto-Sync Coverage ---

  Scenario: Generated manifest covers all CLI commands
    Given chrome-cli is installed
    When I run "chrome-cli capabilities"
    Then the output is valid JSON
    And the "commands" array contains entry "connect"
    And the "commands" array contains entry "tabs"
    And the "commands" array contains entry "navigate"
    And the "commands" array contains entry "page"
    And the "commands" array contains entry "dom"
    And the "commands" array contains entry "js"
    And the "commands" array contains entry "console"
    And the "commands" array contains entry "network"
    And the "commands" array contains entry "interact"
    And the "commands" array contains entry "form"
    And the "commands" array contains entry "emulate"
    And the "commands" array contains entry "perf"
    And the "commands" array contains entry "dialog"
    And the "commands" array contains entry "config"
    And the "commands" array contains entry "completions"
    And the "commands" array contains entry "examples"
    And the "commands" array contains entry "capabilities"
    And the "commands" array contains entry "man"

  # --- Data-Driven: Per-Command Subcommand Coverage ---

  Scenario Outline: Commands with subcommands list them
    Given chrome-cli is installed
    When I run "chrome-cli capabilities --command <command>"
    Then the output is valid JSON
    And the first command has subcommands
    And the exit code is 0

    Examples:
      | command  |
      | tabs     |
      | navigate |
      | page     |
      | js       |
      | console  |
      | network  |
      | interact |
      | form     |
      | emulate  |
      | perf     |
      | dialog   |
      | config   |
