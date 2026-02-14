# File: tests/features/man-page-generation.feature
#
# Generated from: .claude/specs/27-man-page-generation/requirements.md
# Issue: #27

Feature: Man Page Generation
  As a developer or automation engineer using chrome-cli
  I want Unix man pages generated from the CLI definition
  So that I can access documentation through the standard man command or inline

  # --- Happy Path ---

  Scenario: Display top-level man page inline
    Given chrome-cli is built
    When I run "chrome-cli man"
    Then stdout should contain "chrome-cli"
    And the exit code should be 0

  Scenario: Top-level man page contains standard sections
    Given chrome-cli is built
    When I run "chrome-cli man"
    Then stdout should contain "SYNOPSIS"
    And stdout should contain "OPTIONS"

  # --- Subcommand Man Pages ---

  Scenario Outline: Display subcommand man page inline
    Given chrome-cli is built
    When I run "chrome-cli man <subcommand>"
    Then stdout should contain "chrome-cli-<subcommand>"
    And the exit code should be 0

    Examples:
      | subcommand  |
      | connect     |
      | tabs        |
      | navigate    |
      | page        |
      | js          |
      | console     |
      | network     |
      | interact    |
      | form        |
      | emulate     |
      | perf        |
      | dialog      |
      | config      |
      | completions |

  # --- Error Handling ---

  Scenario: Invalid subcommand produces error
    Given chrome-cli is built
    When I run "chrome-cli man nonexistent"
    Then the exit code should be 1

  # --- Help Text ---

  Scenario: Man subcommand help text describes usage
    Given chrome-cli is built
    When I run "chrome-cli man --help"
    Then stdout should contain "man"
    And the exit code should be 0
