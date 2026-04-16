# File: tests/features/compact-snapshot-mode.feature
#
# Generated from: .claude/specs/feature-add-compact-snapshot-mode-ai-agent-token-efficiency/requirements.md
# Issue: #162

Feature: Compact Snapshot Mode
  As an AI agent consuming agentchrome output
  I want a compact snapshot mode that shows only interactive and semantically meaningful elements
  So that I can understand page structure without consuming excessive context window tokens

  # --- Happy Path (requires Chrome) ---

  Scenario: Compact snapshot returns only interactive and landmark elements
    Given a Chrome session connected to a page with mixed interactive and decorative elements
    When I run page snapshot with the compact flag
    Then the output includes interactive elements with UIDs
    And the output includes landmark elements like headings and navigation
    And the output does not include InlineTextBox nodes
    And the output does not include LineBreak nodes
    And the output does not include purely decorative generic containers

  Scenario: Compact snapshot preserves hierarchy context
    Given a Chrome session connected to a page with nested interactive elements inside landmarks
    When I run page snapshot with the compact flag
    Then interactive elements show correct indentation under their landmark ancestors
    And intermediate generic containers between landmarks and interactive elements are collapsed

  # --- Size Reduction (requires Chrome) ---

  Scenario: Compact snapshot reduces output size significantly
    Given a Chrome session connected to the SauceDemo inventory page
    When I run page snapshot without the compact flag and count the lines
    And I run page snapshot with the compact flag and count the lines
    Then the compact output has at least 50% fewer lines than the full output
    And all UIDs from the full output are present in the compact output

  # --- Backward Compatibility (requires Chrome) ---

  Scenario: Full snapshot remains default
    Given a Chrome session connected to a page
    When I run page snapshot without the compact flag
    Then the output is the full accessibility tree with all nodes
    And the output matches the pre-existing snapshot behavior

  # --- Cross-Command Integration (requires Chrome) ---

  Scenario: Compact mode with include-snapshot on form commands
    Given a Chrome session connected to a page with form fields
    When I run form fill with include-snapshot and compact flags
    Then the response JSON contains a snapshot field
    And the snapshot field contains only interactive and landmark elements

  # --- UID Preservation (requires Chrome) ---

  Scenario: Compact mode preserves all interactive UIDs
    Given a Chrome session connected to a page with interactive elements
    When I run page snapshot without the compact flag and collect all UIDs
    And I run page snapshot with the compact flag and collect all UIDs
    Then both UID sets are identical

  # --- Flag Combinations (requires Chrome) ---

  Scenario: Compact mode with verbose flag
    Given a Chrome session connected to a page with interactive elements that have properties
    When I run page snapshot with compact and verbose flags
    Then the output includes compact-filtered nodes
    And kept nodes include additional properties like checked or disabled or level

  Scenario: Compact mode with JSON output
    Given a Chrome session connected to a page
    When I run page snapshot with compact and json flags
    Then the output is valid JSON
    And the JSON tree contains only interactive and landmark elements
    And the JSON structure uses the standard SnapshotNode schema

  # --- CLI Validation (testable without Chrome) ---

  Scenario: Compact flag is accepted on page snapshot
    Given agentchrome is built
    When I run "agentchrome page snapshot --help"
    Then stdout should contain "--compact"

  Scenario: Compact flag is accepted on interact click
    Given agentchrome is built
    When I run "agentchrome interact click --help"
    Then stdout should contain "--compact"

  Scenario: Compact flag is accepted on form fill
    Given agentchrome is built
    When I run "agentchrome form fill --help"
    Then stdout should contain "--compact"

  Scenario: Compact flag is accepted on form clear
    Given agentchrome is built
    When I run "agentchrome form clear --help"
    Then stdout should contain "--compact"

  Scenario: Compact flag is accepted on interact scroll
    Given agentchrome is built
    When I run "agentchrome interact scroll --help"
    Then stdout should contain "--compact"

  # --- Unit-Level Validation (source-level, testable without Chrome) ---

  Scenario: compact_tree source contains COMPACT_KEPT_ROLES constant
    Given the snapshot source file exists
    When I read the snapshot source
    Then the source contains "COMPACT_KEPT_ROLES"

  Scenario: compact_tree source contains COMPACT_EXCLUDED_ROLES constant
    Given the snapshot source file exists
    When I read the snapshot source
    Then the source contains "COMPACT_EXCLUDED_ROLES"

  Scenario: compact_tree source contains the compact_tree function
    Given the snapshot source file exists
    When I read the snapshot source
    Then the source contains "pub fn compact_tree"
