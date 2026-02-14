Feature: Claude Code Integration Guide
  As a developer using Claude Code for browser automation
  I want a comprehensive integration guide and CLAUDE.md template
  So that Claude Code can immediately discover and use chrome-cli

  # --- File Existence ---

  Scenario: Integration guide exists and covers discovery mechanisms
    Given the file "docs/claude-code.md" exists in the repository
    When I read the integration guide
    Then it contains a "Discovery" or "Setup" section
    And it mentions "chrome-cli capabilities" for machine-readable discovery
    And it mentions "chrome-cli examples" for learning commands
    And it provides a setup checklist

  Scenario: CLAUDE.md template is provided as a drop-in example
    Given the file "examples/CLAUDE.md.example" exists in the repository
    When I read the template file
    Then it contains "chrome-cli connect" for launching Chrome
    And it contains "chrome-cli page snapshot" for page inspection
    And it contains "chrome-cli interact" or "chrome-cli form fill" for interaction
    And it contains a workflow loop description

  # --- Workflow Documentation ---

  Scenario: Common workflow patterns are documented
    Given the file "docs/claude-code.md" exists in the repository
    When I read the "Common Workflows" section of the guide
    Then the guide documents a "Testing Web Apps" workflow
    And the guide documents a "Scraping Data" workflow
    And the guide documents a "Debugging UI Issues" workflow
    And the guide documents a "Form Automation" workflow

  Scenario: Recommended workflow loops are documented
    Given the file "docs/claude-code.md" exists in the repository
    When I read the "Recommended Workflow Loops" section of the guide
    Then the guide mentions "snapshot" in the workflow loop
    And the guide mentions "interact" in the workflow loop
    And the guide mentions "verify" in the workflow loop

  # --- Efficiency and Best Practices ---

  Scenario: Efficient usage tips minimize round-trips
    Given the file "docs/claude-code.md" exists in the repository
    When I read the "Efficiency Tips" section of the guide
    Then the guide mentions "form fill-many" for batch form filling
    And the guide mentions "--wait-until" to avoid race conditions
    And the guide mentions "page text" for content extraction
    And the guide mentions "--timeout" to prevent hangs

  Scenario: Best practices for AI agents are documented
    Given the file "docs/claude-code.md" exists in the repository
    When I read the "Best Practices" section of the guide
    Then the guide recommends "page snapshot" before interaction commands
    And the guide recommends "json" output for reliable parsing
    And the guide recommends checking exit codes
    And the guide recommends "form fill" over "interact type"
    And the guide recommends "console follow" for debugging
    And the guide recommends "network follow" for debugging

  # --- Error Handling ---

  Scenario: Error handling patterns for AI agents
    Given the file "docs/claude-code.md" exists in the repository
    When I read the "Error Handling" section of the guide
    Then the guide documents exit code conventions
    And the guide documents "ConnectionError" failure mode
    And the guide documents "element not found" failure mode
    And the guide documents "TimeoutError" failure mode
    And the guide provides recovery strategies

  # --- Example Conversation ---

  Scenario: Example conversation demonstrates real-world usage
    Given the file "docs/claude-code.md" exists in the repository
    When I read the "Example Conversation" section of the guide
    Then the guide shows "chrome-cli connect" in the example
    And the guide shows "chrome-cli page snapshot" in the example
    And the guide shows a form fill or interaction command in the example
    And the guide shows verification of the result in the example

  # --- README Integration ---

  Scenario: README links to the full integration guide
    Given the file "README.md" exists in the repository
    When I read the "Claude Code Integration" section of the README
    Then the README contains a link to "docs/claude-code.md"
    And the README contains a link to "examples/CLAUDE.md.example"
