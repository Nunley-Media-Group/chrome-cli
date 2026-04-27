# File: tests/features/278-detect-codex-runtime-env-vars-for-active-skill-notice-scoping.feature
#
# Generated from: specs/bug-detect-codex-runtime-env-vars-for-active-skill-notice-scoping/requirements.md
# Issue: #278
# Type: Defect regression

@regression
Feature: Codex runtime env vars scope active skill notices
  The active tool detector previously recognized Codex only through CODEX_HOME.
  This was fixed by treating observed Codex runtime environment variables as active Codex signals while preserving plain-terminal fallback behavior.

  # --- Bug Is Fixed ---

  @regression
  Scenario: Codex runtime env vars mark Codex active without CODEX_HOME
    Given a Codex runtime environment variable "CODEX_CI" is present
    And CODEX_HOME is not set
    When active-tool detection runs
    Then the active tool is classified as "codex"

  # --- Related Behavior Still Works ---

  @regression
  Scenario: Current active Codex skill suppresses inactive stale notices
    Given a Codex runtime environment variable "CODEX_MANAGED_BY_NPM" is present
    And CODEX_HOME is not set
    And a skill installed at the current binary version for "codex" in a temp home
    And an installed AgentChrome skill for "claude-code" has a stale version in the same temp home
    When any AgentChrome command is invoked with the temp home
    Then no staleness notice is emitted

  @regression
  Scenario: Stale active Codex notice names only Codex
    Given a Codex runtime environment variable "CODEX_THREAD_ID" is present
    And CODEX_HOME is not set
    And installed AgentChrome skills for "codex" and "claude-code" have stale versions in a temp home
    When any AgentChrome command is invoked with the temp home
    Then stderr contains exactly one staleness notice line
    And stderr contains a line starting with "note: installed agentchrome skill for codex"
    And stderr does not contain "claude-code"
