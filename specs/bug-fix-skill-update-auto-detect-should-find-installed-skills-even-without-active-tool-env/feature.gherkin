# File: tests/features/254-fix-skill-update-auto-detect.feature
#
# Generated from: specs/bug-fix-skill-update-auto-detect-should-find-installed-skills-even-without-active-tool-env/requirements.md
# Issue: #254
# Type: Defect regression

@regression
Feature: Bare skill update finds installed skills without active tool detection
  The bare skill update path previously depended on active agentic-tool detection or treated an empty stale scan as an error.
  This was fixed by scanning supported installed skill paths directly and returning successful no-op JSON when there is nothing to update.

  @regression
  Scenario: Bare update finds stale installed skills without active tool signals
    Given installed AgentChrome skills for "claude-code" and "codex" have stale versions in a temp home
    And no active agentic tool signal is present
    When I run "agentchrome skill update" without a --tool flag
    Then the exit code is 0
    And stdout contains batch JSON results for "claude-code" and "codex"
    And each batch result has status "ok" and action "updated"
    And a subsequent AgentChrome invocation with the same temp home emits no staleness notice

  @regression
  Scenario: Bare update reports all installed skills are already current
    Given a skill installed at the current binary version for "claude-code" in a temp home
    And no active agentic tool signal is present
    When I run "agentchrome skill update" without a --tool flag
    Then the exit code is 0
    And stdout contains an informational skill update message "all installed AgentChrome skills are up to date"
    And stderr does not contain a JSON error object

  @regression
  Scenario: Bare update reports no AgentChrome skills are installed
    Given no AgentChrome skill is installed in a temp home
    And no active agentic tool signal is present
    When I run "agentchrome skill update" without a --tool flag
    Then the exit code is 0
    And stdout contains an informational skill update message "no AgentChrome skills are installed"
    And stderr does not contain a JSON error object

  @regression
  Scenario: Explicit update remains single-target
    Given installed AgentChrome skills for "claude-code" and "codex" have stale versions in a temp home
    When I run "agentchrome skill update --tool claude-code"
    Then the exit code is 0
    And stdout contains valid JSON with "action" equal to "updated"
    And stdout does not contain a batch "results" array
    And the Claude Code skill file contains the current AgentChrome version
    And the Codex skill file remains stale
