Feature: Batch Script Execution
  As a browser automation engineer running repetitive multi-step workflows
  I want to define agentchrome commands in a JSON script and run them as one operation
  So that I can cut round-trips and context-window usage for long automations

  Background:
    Given agentchrome is built

  # --- Schema Validation ---

  Scenario: AC9 happy — Dry-run validates a correct script
    Given a script file at "tests/fixtures/scripts/simple.json"
    When I run "agentchrome script run --dry-run tests/fixtures/scripts/simple.json"
    Then the exit code should be 0
    And stdout should be valid JSON
    And stdout JSON should have key "dispatched"
    And stdout JSON should have key "ok"
    And stdout JSON should have key "steps"

  Scenario: AC9 alt — Dry-run reports schema errors for unknown subcommands
    Given a script file at "tests/fixtures/scripts/bad-cmd.json"
    When I run "agentchrome script run --dry-run tests/fixtures/scripts/bad-cmd.json"
    Then the exit code should be nonzero
    And stderr should be valid JSON
    And stderr JSON should have key "error"
    And stderr should contain "unknown subcommand"

  Scenario: Empty commands array is rejected at parse time
    Given a script file at "tests/fixtures/scripts/empty-commands.json"
    When I run "agentchrome script run --dry-run tests/fixtures/scripts/empty-commands.json"
    Then the exit code should be nonzero
    And stderr should be valid JSON
    And stderr JSON should have key "error"
    And stderr should contain "must not be empty"

  # --- Help Surface ---

  Scenario: AC10 — script --help mentions key concepts
    Given agentchrome is built
    When I run "agentchrome script --help"
    Then the exit code should be 0
    And stdout should contain "script"

  Scenario: AC10 — script run --help lists flags
    Given agentchrome is built
    When I run "agentchrome script run --help"
    Then the exit code should be 0
    And stdout should contain "--fail-fast"
    And stdout should contain "--dry-run"

  Scenario: AC11 — script run --help long form includes EXAMPLES
    Given agentchrome is built
    When I run "agentchrome script run --help"
    Then the exit code should be 0
    And stdout should contain "EXAMPLES"

  # --- Integration with capabilities ---

  Scenario: AC12 — Capabilities manifest includes the script surface
    Given agentchrome is built
    When I run "agentchrome capabilities --json"
    Then the exit code should be 0
    And stdout should be valid JSON
    And stdout should contain "script"

  # --- Examples subcommand ---

  Scenario: AC13 — examples script prints worked examples
    Given agentchrome is built
    When I run "agentchrome examples script"
    Then the exit code should be 0
    And stdout should contain "script"
