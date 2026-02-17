# File: tests/features/114-fix-connect-status-output-flags.feature
#
# Generated from: .claude/specs/114-fix-connect-status-output-flags/requirements.md
# Issue: #114
# Type: Defect regression

@regression
Feature: connect --status respects output format flags
  The `connect --status` command previously ignored `--pretty` and `--plain`
  output format flags, producing identical compact JSON regardless of which
  flag was passed. This was fixed by updating `execute_status()` to inspect
  `global.output` and format accordingly.

  Background:
    Given a valid session file exists

  # --- Bug Is Fixed ---

  @regression
  Scenario: Pretty flag produces indented JSON
    When I run "chrome-cli connect --status --pretty"
    Then the exit code should be 0
    And the output is valid JSON
    And the output contains newlines and indentation

  @regression
  Scenario: Plain flag produces human-readable text
    When I run "chrome-cli connect --status --plain"
    Then the exit code should be 0
    And the output is not valid JSON
    And the output contains key-value pairs for connection details

  # --- Related Behavior Still Works ---

  @regression
  Scenario: Default output is compact JSON
    When I run "chrome-cli connect --status"
    Then the exit code should be 0
    And the output is valid JSON
    And the output is a single line
