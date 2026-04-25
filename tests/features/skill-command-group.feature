# File: tests/features/skill-command-group.feature
#
# Generated from: specs/feature-add-agentchrome-skill-command-group/requirements.md
# Issue: #172

Feature: Skill command group for agentic tool integration
  As an AI agent or developer using an agentic coding tool
  I want to run a single command that installs a concise agentchrome skill
  So that the AI agent automatically knows when to use agentchrome

  # --- Happy Path ---

  Scenario: Auto-detected installation (AC1)
    Given an agentic coding tool environment is active with env var "CLAUDE_CODE" set
    When I run "agentchrome skill install"
    Then the exit code is 0
    And stdout contains valid JSON with "tool" equal to "claude-code"
    And stdout contains "action" equal to "installed"
    And stdout contains a "path" field pointing to the skill file location
    And the skill file exists at the reported path

  Scenario: Explicit tool targeting (AC2)
    Given no particular agentic environment is active
    When I run "agentchrome skill install --tool cursor"
    Then the exit code is 0
    And stdout contains valid JSON with "tool" equal to "cursor"
    And stdout contains "action" equal to "installed"
    And the skill file exists at the Cursor install path

  Scenario: List supported tools (AC3)
    Given the skill command is available
    When I run "agentchrome skill list"
    Then the exit code is 0
    And stdout contains valid JSON with a "tools" array
    And the "tools" array contains entries for "claude-code", "windsurf", "aider", "continue", "copilot-jb", "cursor", "gemini", and "codex"
    And each tool entry has "name", "detection", "path", and "installed" fields

  Scenario: Uninstall skill (AC4)
    Given a skill was previously installed for "claude-code"
    When I run "agentchrome skill uninstall --tool claude-code"
    Then the exit code is 0
    And stdout contains valid JSON with "action" equal to "uninstalled"
    And the skill file no longer exists at the Claude Code install path

  Scenario: Update installed skill (AC5)
    Given a skill was previously installed for "claude-code"
    When I run "agentchrome skill update --tool claude-code"
    Then the exit code is 0
    And stdout contains valid JSON with "action" equal to "updated"
    And stdout contains a "version" field matching the current agentchrome version
    And the skill file at the Claude Code path contains the updated version

  # --- Error Handling ---

  Scenario: Unknown environment with no --tool flag (AC6)
    Given no supported agentic tool can be detected
    When I run "agentchrome skill install"
    Then the exit code is non-zero
    And stderr contains valid JSON with an "error" field
    And stderr contains a "supported_tools" array listing all supported tool names

  # --- Cross-Validation ---

  Scenario: Cross-validate install via list (AC7)
    Given a skill was installed via "agentchrome skill install --tool claude-code"
    When I run "agentchrome skill list"
    Then the exit code is 0
    And the Claude Code entry in the tools list shows "installed" equal to true

  # --- Alternative Paths ---

  Scenario: Uninstall with explicit --tool flag (AC8)
    Given a skill was previously installed for "aider"
    When I run "agentchrome skill uninstall --tool aider"
    Then the exit code is 0
    And stdout contains valid JSON with "tool" equal to "aider"
    And stdout contains "action" equal to "uninstalled"
    And the Aider skill file no longer exists

  # --- Idempotency ---

  Scenario: Install idempotency (AC9)
    Given a skill is already installed for "claude-code"
    When I run "agentchrome skill install --tool claude-code" again
    Then the exit code is 0
    And stdout contains "action" equal to "installed"
    And the skill file is overwritten with current version content

  # --- Detection Priority ---

  Scenario: Detection priority order (AC10)
    Given both "CLAUDE_CODE" env var is set and "~/.continue/" directory exists
    When I run "agentchrome skill install"
    Then the exit code is 0
    And stdout contains "tool" equal to "claude-code"
    And the env var detection takes priority over config dir detection

  # --- README Documentation ---

  Scenario: README features skill install in setup (AC11)
    Given the project README.md exists
    When I read the Claude Code Integration section
    Then it contains "agentchrome skill install" as a setup step
    And it contains "agentchrome skill update" as a post-upgrade step

  # --- Output Compliance ---

  Scenario Outline: All subcommand JSON output compliance (AC12)
    Given the skill command is available
    When I run "agentchrome skill <subcommand>"
    Then stdout or stderr contains valid JSON
    And the output conforms to the global JSON output contract

    Examples:
      | subcommand               |
      | list                     |
      | install --tool claude-code |
      | uninstall --tool claude-code |
      | update --tool claude-code  |
      | install --tool gemini      |
      | uninstall --tool gemini    |
      | update --tool gemini       |

  # --- Gemini CLI Support (Added by issue #214) ---

  Scenario: Gemini skill installs successfully (AC13)
    Given no particular agentic environment is active
    When I run "agentchrome skill install --tool gemini"
    Then the exit code is 0
    And stdout contains valid JSON with "tool" equal to "gemini"
    And stdout contains "action" equal to "installed"
    And stdout contains a "path" field pointing to "~/.gemini/instructions/agentchrome.md"
    And the skill file exists at the Gemini install path

  Scenario: Gemini appears in skill list (AC14)
    Given the skill command is available
    When I run "agentchrome skill list"
    Then the exit code is 0
    And the "tools" array contains an entry with "name" equal to "gemini"
    And the gemini entry has "path" equal to "~/.gemini/instructions/agentchrome.md"
    And the gemini entry has "detection" and "installed" fields

  Scenario: Gemini auto-detection via env var (AC15)
    Given an agentic coding tool environment is active with env var "GEMINI_API_KEY" set
    When I run "agentchrome skill install"
    Then the exit code is 0
    And stdout contains valid JSON with "tool" equal to "gemini"
    And stdout contains "action" equal to "installed"
    And the skill file exists at the Gemini install path

  Scenario: Gemini auto-detection via config directory (AC15)
    Given the "~/.gemini/" directory exists
    And no GEMINI_* environment variables are set
    And no higher-priority tool signals are present
    When I run "agentchrome skill install"
    Then the exit code is 0
    And stdout contains valid JSON with "tool" equal to "gemini"

  Scenario: Gemini skill uninstalls cleanly (AC16)
    Given a skill was previously installed for "gemini"
    When I run "agentchrome skill uninstall --tool gemini"
    Then the exit code is 0
    And stdout contains valid JSON with "tool" equal to "gemini"
    And stdout contains "action" equal to "uninstalled"
    And the skill file no longer exists at the Gemini install path

  Scenario: Gemini skill updates in place (AC17)
    Given a skill was previously installed for "gemini"
    When I run "agentchrome skill update --tool gemini"
    Then the exit code is 0
    And stdout contains valid JSON with "action" equal to "updated"
    And stdout contains a "version" field matching the current agentchrome version
    And the skill file at the Gemini path contains the updated version

  Scenario: README lists Gemini as supported tool (AC18)
    Given the project README.md exists
    When I read the skill installer documentation
    Then it lists "gemini" or "Gemini CLI" as a supported tool

  # --- Codex Support (Added by issue #263) ---

  Scenario: Codex skill installs explicitly with CODEX_HOME (AC19)
    Given no particular agentic environment is active
    And CODEX_HOME points to a temp Codex home
    When I run "agentchrome skill install --tool codex"
    Then the exit code is 0
    And stdout contains valid JSON with "tool" equal to "codex"
    And stdout contains "action" equal to "installed"
    And the skill file exists at "$CODEX_HOME/skills/agentchrome/SKILL.md"

  Scenario: Codex skill installs explicitly with default home fallback (AC19)
    Given no particular agentic environment is active
    And CODEX_HOME is not set
    When I run "agentchrome skill install --tool codex"
    Then the exit code is 0
    And stdout contains valid JSON with "tool" equal to "codex"
    And the skill file exists at "~/.codex/skills/agentchrome/SKILL.md"

  Scenario: Codex appears in skill list (AC20)
    Given the skill command is available
    When I run "agentchrome skill list"
    Then the exit code is 0
    And the "tools" array contains an entry with "name" equal to "codex"
    And the codex entry has "path" equal to "$CODEX_HOME/skills/agentchrome/SKILL.md" or "~/.codex/skills/agentchrome/SKILL.md"
    And the codex entry has "detection" and "installed" fields

  Scenario: Codex auto-detection via CODEX_HOME (AC21)
    Given CODEX_HOME points to a temp Codex home
    And no higher-priority tool signals are present
    When I run "agentchrome skill install"
    Then the exit code is 0
    And stdout contains valid JSON with "tool" equal to "codex"
    And stdout contains "action" equal to "installed"
    And the skill file exists at "$CODEX_HOME/skills/agentchrome/SKILL.md"

  Scenario: Codex auto-detection via config directory (AC21)
    Given the "~/.codex/" directory exists
    And CODEX_HOME is not set
    And no higher-priority tool signals are present
    When I run "agentchrome skill install"
    Then the exit code is 0
    And stdout contains valid JSON with "tool" equal to "codex"

  Scenario: Codex skill lifecycle commands work (AC22)
    Given a skill was previously installed for "codex"
    When I run "agentchrome skill update --tool codex"
    Then the exit code is 0
    And stdout contains valid JSON with "action" equal to "updated"
    And the Codex skill file contains the updated version
    When I run "agentchrome skill uninstall --tool codex"
    Then the exit code is 0
    And stdout contains valid JSON with "action" equal to "uninstalled"
    And the Codex skill file no longer exists

  Scenario: Codex stale skill is included in staleness checks (AC23)
    Given an installed skill for codex with version "0.1.0" planted in a temp Codex home
    When I invoke agentchrome with the planted Codex home
    Then stderr contains a line starting with "note: installed agentchrome skill for codex"
    And stderr contains exactly one staleness notice line

  Scenario: Codex documentation and tests are present (AC24)
    Given Codex support is implemented
    When I review the skill installer documentation and BDD tests
    Then README.md documents Codex as a supported skill installer target
    And docs/codex.md shows "agentchrome skill install --tool codex"
    And BDD or unit tests cover Codex install, list, detection, update, uninstall, and staleness behavior

  # =============================================================================
  # Issue #220 — Enrich SKILL.md template with YAML frontmatter and discovery paths
  # =============================================================================

  Scenario: SKILL.md has YAML frontmatter on install (AC1)
    Given no particular agentic environment is active
    When I run "agentchrome skill install --tool claude-code"
    Then the exit code is 0
    And the installed SKILL.md starts with a YAML frontmatter block
    And the frontmatter contains name "agentchrome"
    And the frontmatter description contains "automate a browser"
    And the frontmatter description contains "fill a form"
    And the frontmatter description contains "test a login"
    And the frontmatter description contains "scrape a page"
    And the frontmatter description contains "take a screenshot"
    And the frontmatter description contains "inspect console / network"

  Scenario: SKILL.md names high-leverage discovery paths (AC2)
    Given no particular agentic environment is active
    When I run "agentchrome skill install --tool claude-code"
    Then the exit code is 0
    And the installed SKILL.md body contains "diagnose"
    And the installed SKILL.md body contains "examples strategies"
    And the installed SKILL.md body contains "--include-snapshot"
    And the installed SKILL.md body contains "output_file"

  Scenario: AppendSection install writes version marker inside section markers (AC5 T005)
    Given no particular agentic environment is active
    When I run "agentchrome skill install --tool windsurf"
    Then the exit code is 0
    And the installed windsurf skill file contains "<!-- agentchrome-version: " inside the section markers
