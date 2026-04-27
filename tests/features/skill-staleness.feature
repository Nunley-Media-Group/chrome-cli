# File: tests/features/skill-staleness.feature
#
# Generated from: specs/feature-harden-progressive-disclosure-enrich-skill-md-extend-temp-file-gating-notify-on-stale-skill/requirements.md
# Issue: #220
#
# Covers: AC6, AC7, AC8, AC10
# All scenarios that plant skill files on disk are logic-only (no Chrome required).
# Chrome-dependent scenarios (streaming command with staleness notice) are tagged
# and skipped in CI using the @requires-chrome tag.

Feature: Skill staleness check notifies when installed skill is behind binary
  As a user or AI agent running agentchrome
  I want to be informed when my installed skill file is older than the binary
  So that I can run skill update and avoid acting on stale guidance

  # --- AC6: Single-line stale notice (single tool) ---

  Scenario: Stale single tool emits a note line on stderr (AC6)
    Given an installed skill for claude-code with version "0.1.0" planted in a temp home
    When I invoke agentchrome with the planted home
    Then stderr contains a line starting with "note: installed agentchrome skill for claude-code"
    And that line contains "run 'agentchrome skill update' to refresh"
    And stderr contains exactly one staleness notice line

  Scenario: Staleness notice is suppressed by env var AGENTCHROME_NO_SKILL_CHECK=1 (AC6)
    Given an installed skill for claude-code with version "0.1.0" planted in a temp home
    When I invoke agentchrome with the planted home and env var "AGENTCHROME_NO_SKILL_CHECK" set to "1"
    Then stderr does not contain "note: installed agentchrome skill"

  Scenario: Staleness notice is suppressed by config key skill.check_enabled=false (AC6)
    Given an installed skill for claude-code with version "0.1.0" planted in a temp home
    And a config file with "check_enabled = false" under "[skill]" in the temp home
    When I invoke agentchrome with the planted home
    Then stderr does not contain "note: installed agentchrome skill"

  # --- AC7: Multi-tool aggregation ---

  Scenario: Stale multi-tool notice aggregates all stale tools into one line (AC7)
    Given an installed skill for claude-code with version "0.1.0" planted in a temp home
    And an installed skill for gemini with version "0.1.0" planted in the same temp home
    And an installed skill for codex with version "0.1.0" planted in the same temp home
    When I invoke agentchrome with the planted home
    Then stderr contains a line starting with "note: installed agentchrome skills for"
    And that line contains "claude-code"
    And that line contains "gemini"
    And that line contains "codex"
    And that line contains "stale"
    And that line contains "run 'agentchrome skill update' to refresh"
    And stderr contains exactly one staleness notice line

  Scenario: Bare skill update clears a multi-tool stale notice (AC7)
    Given an installed skill for claude-code with version "0.1.0" planted in a temp home
    And an installed skill for codex with version "0.1.0" planted in the same temp home
    When I invoke agentchrome with the planted home
    Then stderr contains exactly one staleness notice line
    And that line contains "claude-code"
    And that line contains "codex"
    When I run bare skill update with the planted home
    Then the exit code is 0
    And a subsequent staleness check against the same home produces no notice

  Scenario: Stale Codex skill emits a Codex note line on stderr (AC7, AC23)
    Given an installed skill for codex with version "0.1.0" planted in a temp home
    When I invoke agentchrome with the planted home
    Then stderr contains a line starting with "note: installed agentchrome skill for codex"
    And that line contains "run 'agentchrome skill update' to refresh"
    And stderr contains exactly one staleness notice line

  Scenario: Fresh skill produces no staleness notice (AC7)
    Given a skill installed at the current binary version for claude-code in a temp home
    When I invoke agentchrome with the planted home
    Then stderr does not contain "note: installed agentchrome skill"

  Scenario: Missing skill produces no staleness notice (AC7)
    Given no skill is installed in a temp home
    When I invoke agentchrome with the planted home
    Then stderr does not contain "note: installed agentchrome skill"

  # --- AC8: skill update idempotent when versions match ---

  Scenario: skill update is idempotent when installed version already matches binary (AC8)
    Given a skill installed at the current binary version for claude-code in a temp home
    When I run skill update for claude-code with the planted home
    Then the exit code is 0
    And the output contains action "updated"
    And a subsequent staleness check against the same home produces no notice

  # --- AC10: Cross-invocation consistency; check fires before streaming ---

  Scenario: Staleness check runs independently on each invocation (AC10)
    Given an installed skill for claude-code with version "0.1.0" planted in a temp home
    When I invoke agentchrome twice with the planted home
    Then each invocation emits the staleness notice independently

  Scenario: Per-invocation env var suppression works without cross-invocation leakage (AC10)
    Given an installed skill for claude-code with version "0.1.0" planted in a temp home
    When I invoke agentchrome with the planted home and env var "AGENTCHROME_NO_SKILL_CHECK" set to "1"
    Then stderr does not contain "note: installed agentchrome skill"
    When I invoke agentchrome with the planted home without the suppression env var
    Then stderr contains a line starting with "note: installed agentchrome skill for claude-code"

  # --- Issue #255: Active-tool-scoped notices ---

  Scenario: Active tool current suppresses unrelated stale-skill notices (AC31)
    Given the active agentic tool signal is "claude-code"
    And the installed AgentChrome skill for "claude-code" is current in a temp home
    And the installed AgentChrome skill for "cursor" has stale version "0.1.0" in the same temp home
    When any AgentChrome command is invoked with the temp home
    Then stderr does not contain a stale-skill notice

  Scenario: Active stale skill emits notice only for the active tool (AC32)
    Given the active agentic tool signal is "claude-code"
    And the installed AgentChrome skill for "claude-code" has stale version "0.1.0" in a temp home
    And the installed AgentChrome skill for "cursor" has stale version "0.1.0" in the same temp home
    When any AgentChrome command is invoked with the temp home
    Then stderr contains exactly one staleness notice line
    And the staleness notice names "claude-code"
    And the staleness notice does not name "cursor"

  Scenario: No active tool preserves all-tools stale notice fallback (AC33)
    Given no active agentic tool signal is present
    And installed AgentChrome skills for "claude-code" and "cursor" have stale versions in a temp home
    When any AgentChrome command is invoked with the temp home
    Then stderr contains exactly one staleness notice line
    And the staleness notice names "claude-code"
    And the staleness notice names "cursor"

  Scenario: Scoped stale notice includes version details and update guidance (AC34)
    Given the active agentic tool signal is "claude-code"
    And the installed AgentChrome skill for "claude-code" has stale version "0.1.0" in a temp home
    When any AgentChrome command is invoked with the temp home
    Then stderr contains exactly one staleness notice line
    And the staleness notice contains the installed skill version "0.1.0"
    And the staleness notice contains the current AgentChrome binary version
    And the staleness notice says "run 'agentchrome skill update' to refresh"
