# File: tests/features/capabilities.feature
#
# Generated from: .claude/specs/30-capabilities-manifest-subcommand/requirements.md
# Issue: #30
# Updated by: issue #218 — Progressive Disclosure retrofit

Feature: Machine-Readable Capabilities Manifest Subcommand
  As a developer or AI agent integrating with agentchrome
  I want a capabilities subcommand that outputs a machine-readable manifest
  So that I can programmatically discover the full CLI surface

  # --- Happy Path (listing / progressive disclosure, #218) ---

  Scenario: AC15 — Listing returns summaries only
    Given agentchrome is installed
    When I run "agentchrome capabilities"
    Then the output is valid JSON
    And the JSON has key "name" with value "agentchrome"
    And the JSON has key "version"
    And the JSON has a "commands" array
    And the "commands" array is not empty
    And every command has "name" and "description" fields
    And no command has "subcommands"
    And the exit code is 0

  Scenario: AC16 — Detail path returns full descriptor
    Given agentchrome is installed
    When I run "agentchrome capabilities navigate"
    Then the output is valid JSON
    And the JSON has key "name" with value "navigate"
    And the JSON has key "description"
    And the exit code is 0

  Scenario: Compact output mode (listing)
    Given agentchrome is installed
    When I run "agentchrome capabilities --compact"
    Then the output is valid JSON
    And every command has "name" and "description" fields
    And no command has "subcommands"
    And the JSON does not have key "global_flags"
    And the JSON does not have key "exit_codes"
    And the exit code is 0

  # --- Global Flags (listing carries them) ---

  Scenario: Global flags are included in the listing
    Given agentchrome is installed
    When I run "agentchrome capabilities"
    Then the output is valid JSON
    And the JSON has a "global_flags" array
    And "global_flags" includes "--port"
    And "global_flags" includes "--host"
    And "global_flags" includes "--timeout"
    And "global_flags" includes "--tab"
    And "global_flags" includes "--json"
    And "global_flags" includes "--pretty"
    And "global_flags" includes "--plain"

  # --- Exit Codes (listing carries them) ---

  Scenario: Exit codes are documented in the listing
    Given agentchrome is installed
    When I run "agentchrome capabilities"
    Then the output is valid JSON
    And the JSON has an "exit_codes" array
    And "exit_codes" contains code 0 named "Success"
    And "exit_codes" contains code 1 named "GeneralError"
    And "exit_codes" contains code 2 named "ConnectionError"
    And "exit_codes" contains code 3 named "TargetError"
    And "exit_codes" contains code 4 named "TimeoutError"
    And "exit_codes" contains code 5 named "ProtocolError"

  # --- Output Formats ---

  Scenario: Pretty-printed JSON output
    Given agentchrome is installed
    When I run "agentchrome capabilities --pretty"
    Then the output is valid JSON
    And the output is multi-line
    And the exit code is 0

  # --- Error Handling (AC17) ---

  Scenario: AC17 — Unknown command in detail path is an error
    Given agentchrome is installed
    When I run "agentchrome capabilities nonexistent"
    Then the exit code is 1
    And stderr contains "Unknown command"

  # --- Auto-Sync Coverage ---

  Scenario: Generated manifest covers all CLI commands
    Given agentchrome is installed
    When I run "agentchrome capabilities"
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
