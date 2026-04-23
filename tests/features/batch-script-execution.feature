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

  # --- Issue #247: page find + screenshot in scripts ---

  Scenario: AC17 dry-run — page find passes script schema validation
    Given a script file at "tests/fixtures/scripts/page-find.json"
    When I run "agentchrome script run --dry-run tests/fixtures/scripts/page-find.json"
    Then the exit code should be 0
    And stdout should be valid JSON
    And stdout JSON should have key "ok"

  Scenario: AC18 dry-run — page screenshot passes script schema validation
    Given a script file at "tests/fixtures/scripts/page-screenshot.json"
    When I run "agentchrome script run --dry-run tests/fixtures/scripts/page-screenshot.json"
    Then the exit code should be 0
    And stdout should be valid JSON
    And stdout JSON should have key "ok"

  Scenario: AC19 dry-run — page find then interact click passes validation
    Given a script file at "tests/fixtures/scripts/find-then-click.json"
    When I run "agentchrome script run --dry-run tests/fixtures/scripts/find-then-click.json"
    Then the exit code should be 0
    And stdout should be valid JSON
    And stdout JSON should have key "ok"

  # The following live-execution scenarios require a running Chrome instance
  # and are verified via the manual smoke test (T021), not the BDD harness.

  @smoke
  Scenario: AC17 live — page find returns matches inside a script
    Given an active CDP session on a page containing a button labelled "Submit"
    When I run "agentchrome script run tests/fixtures/scripts/page-find.json"
    Then the exit code should be 0
    And the script result for step 0 has status "ok"
    And $vars.match contains a match with uid, role "button", and name "Submit"

  @smoke
  Scenario: AC18 live — page screenshot writes a PNG inside a script
    Given an active CDP session
    When I run "agentchrome script run tests/fixtures/scripts/page-screenshot.json"
    Then the exit code should be 0
    And the script result for step 0 has status "ok"
    And the file "/tmp/agentchrome-script-smoke.png" is a valid PNG

  @smoke
  Scenario: AC19 live — bind page find then drive interact click
    Given an active CDP session on a page containing a button labelled "Submit"
    When I run "agentchrome script run tests/fixtures/scripts/find-then-click.json"
    Then the exit code should be 0
    And the script result for step 0 has status "ok"
    And the script result for step 1 has status "ok"
    And the click targeted the same element that page find located

  @smoke
  Scenario: AC20 — existing snapshot/text scripts are unchanged
    Given an active CDP session
    When I run "agentchrome script run tests/fixtures/scripts/simple.json"
    Then the exit code should be 0
    And no warnings appear on stderr

  # --- Issue #248: js exec bind auto-unwraps scalar result ---

  @regression
  Scenario: AC21 dry-run — js-exec-bind-scalar script passes schema validation
    Given a script file at "tests/fixtures/scripts/js-exec-bind-scalar.json"
    When I run "agentchrome script run --dry-run tests/fixtures/scripts/js-exec-bind-scalar.json"
    Then the exit code should be 0
    And stdout should be valid JSON
    And stdout JSON should have key "ok"

  @regression
  Scenario: AC22 dry-run — js-exec-bind-object script passes schema validation
    Given a script file at "tests/fixtures/scripts/js-exec-bind-object.json"
    When I run "agentchrome script run --dry-run tests/fixtures/scripts/js-exec-bind-object.json"
    Then the exit code should be 0
    And stdout should be valid JSON
    And stdout JSON should have key "ok"

  @smoke @regression
  Scenario: AC21 live — js exec scalar bind is auto-unwrapped for downstream use
    Given an active CDP session on a page whose document title is "The Internet"
    When I run "agentchrome script run tests/fixtures/scripts/js-exec-bind-scalar.json"
    Then the exit code should be 0
    And the script result for step 0 has status "ok"
    And the script result for step 1 has status "ok"

  @smoke @regression
  Scenario: AC22 live — js exec object bind exposes fields directly
    Given an active CDP session
    When I run "agentchrome script run tests/fixtures/scripts/js-exec-bind-object.json"
    Then the exit code should be 0
    And $vars.obj.a resolves to 1
    And $vars.obj has no "truncated" field
