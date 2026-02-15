# File: tests/features/help-text.feature
#
# Generated from: .claude/specs/26-comprehensive-help-text/requirements.md
# Issue: #26

Feature: Comprehensive help text
  As a developer or AI agent using chrome-cli
  I want rich, detailed --help documentation for every command, subcommand, and flag
  So that I can fully understand all capabilities without external documentation

  # --- Top-Level Help ---

  Scenario: Top-level help displays tool description
    Given chrome-cli is built
    When I run "chrome-cli --help"
    Then stdout should contain "browser automation via the Chrome DevTools Protocol"

  Scenario: Top-level help lists all command groups
    Given chrome-cli is built
    When I run "chrome-cli --help"
    Then stdout should contain "connect"
    And stdout should contain "tabs"
    And stdout should contain "navigate"
    And stdout should contain "page"
    And stdout should contain "js"
    And stdout should contain "console"
    And stdout should contain "network"
    And stdout should contain "interact"
    And stdout should contain "form"
    And stdout should contain "emulate"
    And stdout should contain "perf"
    And stdout should contain "dialog"
    And stdout should contain "config"
    And stdout should contain "completions"

  Scenario: Top-level help includes quick-start examples
    Given chrome-cli is built
    When I run "chrome-cli --help"
    Then stdout should contain "QUICK START"
    And stdout should contain "chrome-cli connect"
    And stdout should contain "chrome-cli tabs list"

  Scenario: Top-level help documents exit codes
    Given chrome-cli is built
    When I run "chrome-cli --help"
    Then stdout should contain "EXIT CODES"
    And stdout should contain "Success"
    And stdout should contain "Connection error"
    And stdout should contain "Timeout error"

  Scenario: Top-level help documents environment variables
    Given chrome-cli is built
    When I run "chrome-cli --help"
    Then stdout should contain "ENVIRONMENT VARIABLES"
    And stdout should contain "CHROME_CLI_PORT"
    And stdout should contain "CHROME_CLI_TIMEOUT"

  # --- Command Group Help ---

  Scenario: Tabs group help includes usage examples
    Given chrome-cli is built
    When I run "chrome-cli tabs --help"
    Then stdout should contain "EXAMPLES"
    And stdout should contain "chrome-cli tabs"

  Scenario: Navigate group help includes usage examples
    Given chrome-cli is built
    When I run "chrome-cli navigate --help"
    Then stdout should contain "EXAMPLES"
    And stdout should contain "chrome-cli navigate"

  Scenario: Page group help includes usage examples
    Given chrome-cli is built
    When I run "chrome-cli page --help"
    Then stdout should contain "EXAMPLES"
    And stdout should contain "chrome-cli page"

  Scenario: Js group help includes usage examples
    Given chrome-cli is built
    When I run "chrome-cli js --help"
    Then stdout should contain "EXAMPLES"
    And stdout should contain "chrome-cli js"

  Scenario: Console group help includes usage examples
    Given chrome-cli is built
    When I run "chrome-cli console --help"
    Then stdout should contain "EXAMPLES"
    And stdout should contain "chrome-cli console"

  Scenario: Network group help includes usage examples
    Given chrome-cli is built
    When I run "chrome-cli network --help"
    Then stdout should contain "EXAMPLES"
    And stdout should contain "chrome-cli network"

  Scenario: Interact group help includes usage examples
    Given chrome-cli is built
    When I run "chrome-cli interact --help"
    Then stdout should contain "EXAMPLES"
    And stdout should contain "chrome-cli interact"

  Scenario: Form group help includes usage examples
    Given chrome-cli is built
    When I run "chrome-cli form --help"
    Then stdout should contain "EXAMPLES"
    And stdout should contain "chrome-cli form"

  Scenario: Emulate group help includes usage examples
    Given chrome-cli is built
    When I run "chrome-cli emulate --help"
    Then stdout should contain "EXAMPLES"
    And stdout should contain "chrome-cli emulate"

  Scenario: Perf group help includes usage examples
    Given chrome-cli is built
    When I run "chrome-cli perf --help"
    Then stdout should contain "EXAMPLES"
    And stdout should contain "chrome-cli perf"

  Scenario: Dialog group help includes usage examples
    Given chrome-cli is built
    When I run "chrome-cli dialog --help"
    Then stdout should contain "EXAMPLES"
    And stdout should contain "chrome-cli dialog"

  Scenario: Config group help includes usage examples
    Given chrome-cli is built
    When I run "chrome-cli config --help"
    Then stdout should contain "EXAMPLES"
    And stdout should contain "chrome-cli config"

  # --- Leaf Command Help ---

  Scenario: tabs list help includes examples
    Given chrome-cli is built
    When I run "chrome-cli tabs list --help"
    Then stdout should contain "EXAMPLES"

  Scenario: tabs create help includes examples
    Given chrome-cli is built
    When I run "chrome-cli tabs create --help"
    Then stdout should contain "EXAMPLES"

  Scenario: tabs close help includes examples
    Given chrome-cli is built
    When I run "chrome-cli tabs close --help"
    Then stdout should contain "EXAMPLES"

  Scenario: tabs activate help includes examples
    Given chrome-cli is built
    When I run "chrome-cli tabs activate --help"
    Then stdout should contain "EXAMPLES"

  Scenario: navigate back help includes examples
    Given chrome-cli is built
    When I run "chrome-cli navigate back --help"
    Then stdout should contain "EXAMPLES"

  Scenario: navigate forward help includes examples
    Given chrome-cli is built
    When I run "chrome-cli navigate forward --help"
    Then stdout should contain "EXAMPLES"

  Scenario: navigate reload help includes examples
    Given chrome-cli is built
    When I run "chrome-cli navigate reload --help"
    Then stdout should contain "EXAMPLES"

  Scenario: page text help includes examples
    Given chrome-cli is built
    When I run "chrome-cli page text --help"
    Then stdout should contain "EXAMPLES"

  Scenario: page snapshot help includes examples
    Given chrome-cli is built
    When I run "chrome-cli page snapshot --help"
    Then stdout should contain "EXAMPLES"

  Scenario: page find help includes examples
    Given chrome-cli is built
    When I run "chrome-cli page find --help"
    Then stdout should contain "EXAMPLES"

  Scenario: page screenshot help includes examples
    Given chrome-cli is built
    When I run "chrome-cli page screenshot --help"
    Then stdout should contain "EXAMPLES"

  Scenario: page resize help includes examples
    Given chrome-cli is built
    When I run "chrome-cli page resize --help"
    Then stdout should contain "EXAMPLES"

  Scenario: js exec help includes examples
    Given chrome-cli is built
    When I run "chrome-cli js exec --help"
    Then stdout should contain "EXAMPLES"

  Scenario: console read help includes examples
    Given chrome-cli is built
    When I run "chrome-cli console read --help"
    Then stdout should contain "EXAMPLES"

  Scenario: console follow help includes examples
    Given chrome-cli is built
    When I run "chrome-cli console follow --help"
    Then stdout should contain "EXAMPLES"

  Scenario: network list help includes examples
    Given chrome-cli is built
    When I run "chrome-cli network list --help"
    Then stdout should contain "EXAMPLES"

  Scenario: network get help includes examples
    Given chrome-cli is built
    When I run "chrome-cli network get --help"
    Then stdout should contain "EXAMPLES"

  Scenario: network follow help includes examples
    Given chrome-cli is built
    When I run "chrome-cli network follow --help"
    Then stdout should contain "EXAMPLES"

  Scenario: interact click help includes examples
    Given chrome-cli is built
    When I run "chrome-cli interact click --help"
    Then stdout should contain "EXAMPLES"

  Scenario: interact click-at help includes examples
    Given chrome-cli is built
    When I run "chrome-cli interact click-at --help"
    Then stdout should contain "EXAMPLES"

  Scenario: interact hover help includes examples
    Given chrome-cli is built
    When I run "chrome-cli interact hover --help"
    Then stdout should contain "EXAMPLES"

  Scenario: interact drag help includes examples
    Given chrome-cli is built
    When I run "chrome-cli interact drag --help"
    Then stdout should contain "EXAMPLES"

  Scenario: interact type help includes examples
    Given chrome-cli is built
    When I run "chrome-cli interact type --help"
    Then stdout should contain "EXAMPLES"

  Scenario: interact key help includes examples
    Given chrome-cli is built
    When I run "chrome-cli interact key --help"
    Then stdout should contain "EXAMPLES"

  Scenario: interact scroll help includes examples
    Given chrome-cli is built
    When I run "chrome-cli interact scroll --help"
    Then stdout should contain "EXAMPLES"

  Scenario: form fill help includes examples
    Given chrome-cli is built
    When I run "chrome-cli form fill --help"
    Then stdout should contain "EXAMPLES"

  Scenario: form fill-many help includes examples
    Given chrome-cli is built
    When I run "chrome-cli form fill-many --help"
    Then stdout should contain "EXAMPLES"

  Scenario: form clear help includes examples
    Given chrome-cli is built
    When I run "chrome-cli form clear --help"
    Then stdout should contain "EXAMPLES"

  Scenario: form upload help includes examples
    Given chrome-cli is built
    When I run "chrome-cli form upload --help"
    Then stdout should contain "EXAMPLES"

  Scenario: emulate set help includes examples
    Given chrome-cli is built
    When I run "chrome-cli emulate set --help"
    Then stdout should contain "EXAMPLES"

  Scenario: emulate reset help includes examples
    Given chrome-cli is built
    When I run "chrome-cli emulate reset --help"
    Then stdout should contain "EXAMPLES"

  Scenario: emulate status help includes examples
    Given chrome-cli is built
    When I run "chrome-cli emulate status --help"
    Then stdout should contain "EXAMPLES"

  Scenario: perf record help includes examples
    Given chrome-cli is built
    When I run "chrome-cli perf record --help"
    Then stdout should contain "EXAMPLES"

  Scenario: perf analyze help includes examples
    Given chrome-cli is built
    When I run "chrome-cli perf analyze --help"
    Then stdout should contain "EXAMPLES"

  Scenario: perf vitals help includes examples
    Given chrome-cli is built
    When I run "chrome-cli perf vitals --help"
    Then stdout should contain "EXAMPLES"

  Scenario: dialog handle help includes examples
    Given chrome-cli is built
    When I run "chrome-cli dialog handle --help"
    Then stdout should contain "EXAMPLES"

  Scenario: dialog info help includes examples
    Given chrome-cli is built
    When I run "chrome-cli dialog info --help"
    Then stdout should contain "EXAMPLES"

  Scenario: config show help includes examples
    Given chrome-cli is built
    When I run "chrome-cli config show --help"
    Then stdout should contain "EXAMPLES"

  Scenario: config init help includes examples
    Given chrome-cli is built
    When I run "chrome-cli config init --help"
    Then stdout should contain "EXAMPLES"

  Scenario: config path help includes examples
    Given chrome-cli is built
    When I run "chrome-cli config path --help"
    Then stdout should contain "EXAMPLES"

  # --- Help Text Quality ---

  Scenario: No placeholder text in top-level help
    Given chrome-cli is built
    When I run "chrome-cli --help"
    Then stdout should not contain "TODO"
    And stdout should not contain "FIXME"

  Scenario: Top-level help exits with code 0
    Given chrome-cli is built
    When I run "chrome-cli --help"
    Then the exit code should be 0
