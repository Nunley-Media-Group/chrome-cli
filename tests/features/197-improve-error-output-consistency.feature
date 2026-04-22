@regression
Feature: Consistent error output on all failure paths
  Every non-zero exit emits a descriptive JSON error on stderr so AI agents
  can programmatically identify what went wrong without guessing at silent
  failures.

  @regression
  Scenario: Unknown UID on form fill produces JSON on stderr
    Given agentchrome is built
    When I run "agentchrome form fill s9999 hello"
    Then the exit code should be nonzero
    And stderr should be valid JSON
    And stderr JSON should have key "error"
    And stderr JSON should have key "code"

  @regression
  Scenario: interact click with --uid flag emits a "Did you mean" hint
    Given agentchrome is built
    When I run "agentchrome interact click --uid s6"
    Then the exit code should be 1
    And stderr should be valid JSON
    And stderr JSON should have key "error"
    And stderr should contain "Did you mean: agentchrome interact click s6"

  @regression
  Scenario: Unrelated clap errors do not receive the interact-click syntax hint
    Given agentchrome is built
    When I run "agentchrome connect --nonexistent-flag"
    Then the exit code should be 1
    And stderr should be valid JSON
    And stderr should not contain "Did you mean: agentchrome interact click"

  @regression
  Scenario: form fill with missing required argument emits JSON
    Given agentchrome is built
    When I run "agentchrome form fill"
    Then the exit code should be 1
    And stderr should be valid JSON
    And stderr JSON should have key "error"
    And stderr JSON should have key "code"

  @regression
  Scenario: interact click with missing positional emits JSON
    Given agentchrome is built
    When I run "agentchrome interact click"
    Then the exit code should be 1
    And stderr should be valid JSON
    And stderr JSON should have key "error"

  @regression
  Scenario: Top-level --help documents the error schema and exit codes
    Given agentchrome is built
    When I run "agentchrome --help"
    Then the exit code should be 0
    And stdout should contain "ERROR HANDLING"
    And stdout should contain "0=success"
    And stdout should contain "1=general"
    And stdout should contain "2=connection"
    And stdout should contain "3=target"
    And stdout should contain "4=timeout"
    And stdout should contain "5=protocol"
    And stdout should contain "exactly one JSON"

  @regression
  Scenario: A failing command emits exactly one JSON line on stderr
    Given agentchrome is built
    When I run "agentchrome form fill s9999 hello"
    Then the exit code should be nonzero
    And stderr should contain exactly one JSON object
