# File: tests/features/cli-skeleton.feature
#
# Generated from: .claude/specs/cli-skeleton/requirements.md
# Issue: #3

Feature: CLI skeleton with clap derive macros and top-level help
  As a developer or AI agent using chrome-cli
  I want a well-structured CLI with comprehensive help text, global flags, and subcommand stubs
  So that I can discover all capabilities and understand available commands

  # --- Happy Path ---

  Scenario: Top-level help displays comprehensive tool description
    Given chrome-cli is built
    When I run "chrome-cli --help"
    Then the exit code should be 0
    And stdout should contain "Chrome DevTools Protocol"
    And stdout should contain "connect"
    And stdout should contain "tabs"
    And stdout should contain "navigate"
    And stdout should contain "page"
    And stdout should contain "dom"
    And stdout should contain "js"
    And stdout should contain "console"
    And stdout should contain "network"
    And stdout should contain "interact"
    And stdout should contain "form"
    And stdout should contain "emulate"
    And stdout should contain "perf"

  Scenario: Version flag displays version information
    Given chrome-cli is built
    When I run "chrome-cli --version"
    Then the exit code should be 0
    And stdout should contain "chrome-cli"

  Scenario: Default connection values are applied
    Given chrome-cli is built
    When I run "chrome-cli navigate"
    Then stderr should contain "URL is required"
    And the exit code should be 1

  # --- Output Format Conflicts ---

  Scenario: Conflicting output format flags are rejected
    Given chrome-cli is built
    When I run "chrome-cli --json --plain navigate"
    Then the exit code should be 2
    And stderr should contain "cannot be used with"

  # --- Subcommand Stubs ---

  Scenario Outline: Subcommand stubs return not-yet-implemented error
    Given chrome-cli is built
    When I run "chrome-cli <subcommand>"
    Then the exit code should be 1
    And stderr should contain "error"
    And stderr should contain "not yet implemented"

    Examples:
      | subcommand |
      | dom        |
      | js         |
      | console    |
      | network    |
      | interact   |
      | form       |
      | emulate    |

  # --- Subcommand Help ---

  Scenario: Subcommand help text is descriptive
    Given chrome-cli is built
    When I run "chrome-cli tabs --help"
    Then the exit code should be 0
    And stdout should contain "Tab management"

  # --- Error Output Format ---

  Scenario: Error output is structured JSON on stderr
    Given chrome-cli is built
    When I run "chrome-cli dom"
    Then the exit code should be 1
    And stderr should be valid JSON
    And stderr JSON should have key "error"
    And stderr JSON should have key "code"

  # --- Global Options ---

  Scenario: Custom port and host are accepted
    Given chrome-cli is built
    When I run "chrome-cli --port 9333 --host 192.168.1.100 navigate"
    Then the exit code should be 1
    And stderr should contain "URL is required"

  Scenario: WebSocket URL option is accepted
    Given chrome-cli is built
    When I run "chrome-cli --ws-url ws://localhost:9222/devtools/browser/abc navigate"
    Then the exit code should be 1
    And stderr should contain "URL is required"

  Scenario: Timeout option is accepted
    Given chrome-cli is built
    When I run "chrome-cli --timeout 5000 navigate"
    Then the exit code should be 1
    And stderr should contain "URL is required"

  Scenario: Tab ID option is accepted
    Given chrome-cli is built
    When I run "chrome-cli --tab abc123 js"
    Then the exit code should be 1
    And stderr should contain "not yet implemented"
