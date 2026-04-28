# File: tests/features/286-iframe-guidance-advertises-frame-command-shapes-the-parser-rejects.feature
#
# Generated from: specs/bug-iframe-guidance-advertises-frame-command-shapes-the-parser-rejects/requirements.md
# Issue: #286
# Type: Defect regression

@regression
Feature: Iframe guidance advertises parser-accepted frame command shapes
  Iframe strategy and diagnose guidance previously advertised --frame command
  shapes that the clap parser rejected. The fix keeps guidance aligned with the
  accepted parser contract and validates advertised frame commands by parsing
  them, not only by checking that text is present.

  Background:
    Given agentchrome is built

  @regression
  Scenario: AC1 - iframe strategy guide commands parse successfully
    When I run "agentchrome examples strategies iframes --json"
    Then every iframe strategy command using --frame parses successfully
    And no iframe strategy command uses rejected page subcommand frame placement

  @regression
  Scenario: AC2 - diagnose iframe suggestions parse successfully
    When I inspect diagnose iframe guidance
    Then every iframe-related diagnose suggestion command using --frame parses successfully
    And no diagnose suggestion uses rejected page subcommand frame placement

  @regression
  Scenario: AC3 - help examples and generated man pages use accepted frame placement
    When I inspect frame-targeting guidance in help, examples, and man pages
    Then each advertised frame command parses successfully
    And page command guidance uses group-scoped frame placement
    And network list guidance keeps subcommand-scoped frame placement

  @regression
  Scenario: AC4 - existing accepted frame targeting commands remain valid
    When I validate accepted frame-targeting commands with the parser
    Then every accepted frame-targeting command parses successfully
