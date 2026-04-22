# File: tests/features/197-improve-error-output-consistency.feature
#
# Generated from: specs/feature-improve-error-output-consistency/requirements.md
# Issue: #197

@regression
Feature: Consistent error output on all failure paths
  Every non-zero exit emits a descriptive JSON error on stderr so AI agents
  can programmatically identify what went wrong without guessing at silent
  failures.

  # --- AC1: Non-zero exit always emits one JSON object on stderr ---

  @regression
  Scenario: Unknown UID on form fill produces JSON on stderr
    Given agentchrome is built
    When I run "agentchrome form fill s9999 hello"
    Then the exit code should be nonzero
    And stderr should be valid JSON
    And stderr JSON should have key "error"
    And stderr JSON should have key "code"

  # --- AC3: --uid flag on a positional command triggers a "Did you mean" hint ---

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

  # --- AC4: Silent-failure audit — sampled CLI-reachable failure paths ---

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

  # --- AC5: --help documents the error-output contract ---

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

  # --- AC6: Exactly one JSON object per invocation ---

  @regression
  Scenario: A failing command emits exactly one JSON line on stderr
    Given agentchrome is built
    When I run "agentchrome form fill s9999 hello"
    Then the exit code should be nonzero
    And stderr should contain exactly one JSON object

  # --- AC2 / AC7 — Chrome-dependent scenarios (verified by smoke + unit tests) ---
  #
  # The following scenarios exercise the `form_fill_not_fillable` constructor
  # against a live Chrome session and are documented here for completeness.
  # They are verified by unit tests in src/error.rs (constructor shape,
  # custom_json stable fields, suggested alternatives per tag/role) and by
  # the manual smoke test in tasks.md (T010).
  #
  # Scenario: form fill on a <div> returns a descriptive "not fillable" error
  #   (Requires Chrome + fixture improve-error-output-consistency.html)
  #
  # Scenario: form fill on a <canvas> suggests js exec
  #   (Requires Chrome + fixture)
  #
  # Scenario: form fill on a non-editable combobox names the role
  #   (Requires Chrome + fixture)
  #
  # Scenario: Errors with custom_json still contain the stable fields
  #   (Covered by src/error.rs unit test
  #    form_fill_not_fillable_custom_json_preserves_stable_fields)
