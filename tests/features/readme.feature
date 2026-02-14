# File: tests/features/readme.feature
#
# Generated from: .claude/specs/28-readme-quickstart-examples-architecture/requirements.md
# Issue: #28

Feature: README documentation
  As a developer discovering chrome-cli for the first time
  I want a comprehensive README with installation, quick-start, examples, and architecture
  So that I can quickly understand, install, and start using the tool

  Background:
    Given the file "README.md" exists in the repository root

  # --- Header & Badges ---

  Scenario: Header with project name and description
    When I read the README content
    Then it starts with a level-1 heading containing "chrome-cli"
    And it contains the text "browser automation via the Chrome DevTools Protocol"

  Scenario: Badges are present
    When I read the README content
    Then it contains a CI badge linking to the GitHub Actions workflow
    And it contains a license badge showing "MIT" and "Apache-2.0"

  # --- Features ---

  Scenario: Features section lists key capabilities
    When I read the "Features" section
    Then it lists at least 8 capabilities as bullet points
    And the capabilities include "tab management"
    And the capabilities include "screenshot"
    And the capabilities include "JavaScript"

  Scenario: Features section includes comparison table
    When I read the "Features" section
    Then it contains a Markdown table comparing chrome-cli with alternatives
    And the table mentions "No Node.js" or "standalone binary"

  # --- Installation ---

  Scenario: Installation via cargo install
    When I read the "Installation" section
    Then it contains "cargo install chrome-cli"

  Scenario: Installation via pre-built binaries
    When I read the "Installation" section
    Then it contains curl commands or download instructions for pre-built binaries
    And it lists supported platforms including "macOS" and "Linux"

  Scenario: Installation from source
    When I read the "Installation" section
    Then it contains "cargo build" instructions for building from source

  # --- Quick Start ---

  Scenario: Quick Start provides a step-by-step guide
    When I read the "Quick Start" section
    Then it contains at least 5 numbered steps
    And it includes "chrome-cli connect"
    And it includes "chrome-cli navigate"
    And it includes a page inspection command

  # --- Usage Examples ---

  Scenario: Usage examples cover common workflows
    When I read the "Usage" section
    Then it contains a screenshot example with "chrome-cli page screenshot"
    And it contains a text extraction example with "chrome-cli page text"
    And it contains a JavaScript execution example with "chrome-cli js exec"

  Scenario: Usage examples include form and network workflows
    When I read the "Usage" section
    Then it contains a form filling example with "chrome-cli form"
    And it contains a network monitoring example with "chrome-cli network"

  Scenario: Lengthy examples use collapsible sections
    When I read the "Usage" section
    Then at least one example uses a "<details>" HTML tag

  # --- Command Reference ---

  Scenario: Command reference table lists all commands
    When I read the "Command Reference" section
    Then it contains a Markdown table
    And the table lists the command "connect"
    And the table lists the command "tabs"
    And the table lists the command "navigate"
    And the table lists the command "page"
    And the table lists the command "js"
    And the table lists the command "console"
    And the table lists the command "network"
    And the table lists the command "interact"
    And the table lists the command "form"
    And the table lists the command "emulate"
    And the table lists the command "perf"
    And the table lists the command "dialog"
    And the table lists the command "config"
    And the table lists the command "completions"
    And the table lists the command "man"

  Scenario: Command reference directs to detailed help
    When I read the "Command Reference" section
    Then it mentions "chrome-cli <command> --help" or "man" for detailed usage

  # --- Architecture ---

  Scenario: Architecture section contains CDP diagram
    When I read the "Architecture" section
    Then it contains a text diagram showing the communication flow
    And it mentions "CDP" or "Chrome DevTools Protocol"
    And it mentions "WebSocket"

  Scenario: Architecture section describes session management
    When I read the "Architecture" section
    Then it describes the session or connection management model

  Scenario: Architecture section mentions performance
    When I read the "Architecture" section
    Then it mentions "Rust" or "native" in the context of performance

  # --- Claude Code Integration ---

  Scenario: Claude Code integration guide
    When I read the "Claude Code" section
    Then it explains how to use chrome-cli with Claude Code
    And it contains a CLAUDE.md example snippet in a code block

  # --- Contributing ---

  Scenario: Contributing section has development setup
    When I read the "Contributing" section
    Then it mentions "cargo build" for building
    And it mentions "cargo test" for running tests
    And it mentions "clippy" or "fmt" for code style

  # --- License ---

  Scenario: License section with dual license
    When I read the "License" section
    Then it mentions "MIT"
    And it mentions "Apache-2.0" or "Apache License"
    And it links to "LICENSE-MIT"
    And it links to "LICENSE-APACHE"
