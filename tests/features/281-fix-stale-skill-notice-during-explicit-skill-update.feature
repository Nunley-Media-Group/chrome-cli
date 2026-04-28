# File: tests/features/281-fix-stale-skill-notice-during-explicit-skill-update.feature
#
# Generated from: specs/bug-fix-stale-skill-notice-during-explicit-skill-update/requirements.md
# Issue: #281
# Type: Defect regression

@regression
Feature: Explicit skill update does not emit a self-stale notice
  The explicit skill update path previously emitted the global stale-skill notice before updating the selected target.
  This was fixed by suppressing the pre-dispatch stale notice only for explicit single-target skill updates.

  # --- Bug Is Fixed ---

  @regression
  Scenario: Explicit update suppresses self-stale notice
    Given the installed AgentChrome skill for "copilot-jb" has stale version "0.1.0" in a temp home
    When I run "agentchrome skill update --tool copilot-jb"
    Then the exit code is 0
    And stderr does not contain a stale-skill notice naming "copilot-jb"
    And stdout contains valid JSON with "tool" equal to "copilot-jb"
    And stdout contains valid JSON with "action" equal to "updated"
    And stdout contains selected skill result fields "tool", "path", and "version"
    And stdout does not contain a batch "results" array

  @regression
  Scenario: Explicit update preserves successful single-target JSON contract
    Given the installed AgentChrome skill for "claude-code" has stale version "0.1.0" in a temp home
    When I run "agentchrome skill update --tool claude-code"
    Then the exit code is 0
    And stdout contains valid JSON with "tool" equal to "claude-code"
    And stdout contains valid JSON with "action" equal to "updated"
    And stdout contains selected skill result fields "tool", "path", and "version"
    And stdout does not contain a batch "results" array

  # --- Related Behavior Still Works ---

  @regression
  Scenario: Non-update stale notice behavior is preserved
    Given the active agentic tool signal is "claude-code"
    And the installed AgentChrome skill for "claude-code" has stale version "0.1.0" in a temp home
    When any AgentChrome command is invoked with the temp home
    Then stderr contains exactly one staleness notice line
    And the staleness notice names "claude-code"
    And the staleness notice says "run 'agentchrome skill update' to refresh"

  @regression
  Scenario: Bare update flow still updates stale installed skills
    Given installed AgentChrome skills for "claude-code" and "codex" have stale versions in a temp home
    When I run "agentchrome skill update" without a --tool flag
    Then the exit code is 0
    And stdout contains batch JSON results for "claude-code" and "codex"
    And each batch result has status "ok" and action "updated"
    And a subsequent AgentChrome invocation with the same temp home emits no staleness notice
