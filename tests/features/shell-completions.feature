# File: tests/features/shell-completions.feature
#
# Generated from: .claude/specs/25-shell-completions-generation/requirements.md
# Issue: #25

Feature: Shell completions generation
  As a developer or automation engineer using chrome-cli
  I want tab-completion for all commands, flags, and enum values in my shell
  So that I can discover and use chrome-cli features faster without consulting documentation

  # --- Happy Path: Generate completions for each shell ---

  Scenario: Generate bash completion script
    Given chrome-cli is built
    When I run "chrome-cli completions bash"
    Then the exit code should be 0
    And stdout should contain "chrome-cli"

  Scenario: Generate zsh completion script
    Given chrome-cli is built
    When I run "chrome-cli completions zsh"
    Then the exit code should be 0
    And stdout should contain "chrome-cli"

  Scenario: Generate fish completion script
    Given chrome-cli is built
    When I run "chrome-cli completions fish"
    Then the exit code should be 0
    And stdout should contain "chrome-cli"

  Scenario: Generate powershell completion script
    Given chrome-cli is built
    When I run "chrome-cli completions powershell"
    Then the exit code should be 0
    And stdout should contain "chrome-cli"

  Scenario: Generate elvish completion script
    Given chrome-cli is built
    When I run "chrome-cli completions elvish"
    Then the exit code should be 0
    And stdout should contain "chrome-cli"

  # --- Content Validation ---

  Scenario: Completions contain top-level subcommands
    Given chrome-cli is built
    When I run "chrome-cli completions bash"
    Then the exit code should be 0
    And stdout should contain "navigate"
    And stdout should contain "tabs"
    And stdout should contain "connect"
    And stdout should contain "page"
    And stdout should contain "js"
    And stdout should contain "completions"

  Scenario: Completions contain nested subcommands
    Given chrome-cli is built
    When I run "chrome-cli completions bash"
    Then the exit code should be 0
    And stdout should contain "list"
    And stdout should contain "create"
    And stdout should contain "close"
    And stdout should contain "activate"

  Scenario: Completions contain global flags
    Given chrome-cli is built
    When I run "chrome-cli completions bash"
    Then the exit code should be 0
    And stdout should contain "--port"
    And stdout should contain "--host"
    And stdout should contain "--json"

  # --- Error Handling ---

  Scenario: Invalid shell argument produces error
    Given chrome-cli is built
    When I run "chrome-cli completions invalid-shell"
    Then the exit code should be 2
    And stderr should contain "invalid value"

  # --- Help Text ---

  Scenario: Completions help shows installation instructions
    Given chrome-cli is built
    When I run "chrome-cli completions --help"
    Then the exit code should be 0
    And stdout should contain "bash"
    And stdout should contain "zsh"
    And stdout should contain "fish"
    And stdout should contain "powershell"
    And stdout should contain "elvish"

  Scenario: Completions subcommand appears in top-level help
    Given chrome-cli is built
    When I run "chrome-cli --help"
    Then the exit code should be 0
    And stdout should contain "completions"
