# File: tests/features/accessibility-tree-snapshot.feature
#
# Generated from: .claude/specs/accessibility-tree-snapshot/requirements.md
# Issue: #10

Feature: Accessibility tree snapshot
  As a developer / automation engineer
  I want to capture the accessibility tree of a browser page via the CLI
  So that AI agents can understand page structure and reference interactive elements

  Background:
    Given Chrome is running with CDP enabled

  # --- Happy Path ---

  Scenario: Capture full accessibility tree of current page
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli page snapshot"
    Then stdout contains a hierarchical text representation
    And each node shows its role and accessible name
    And the output uses indentation to reflect DOM structure

  Scenario: Interactive elements have unique reference IDs
    Given a page with links, buttons, and input fields
    When I run "chrome-cli page snapshot"
    Then each interactive element is annotated with a uid in brackets
    And each uid is unique within the snapshot
    And uids follow the format "s1", "s2", "s3"

  # --- Alternative Paths ---

  Scenario: Target a specific tab with --tab
    Given multiple tabs are open
    When I run "chrome-cli page snapshot --tab 1"
    Then the accessibility tree is captured from the second tab

  Scenario: Verbose mode shows additional properties
    Given a page with form elements and headings
    When I run "chrome-cli page snapshot --verbose"
    Then elements include extra properties where applicable
    And properties may include level, checked, disabled, expanded, url

  Scenario: Save snapshot to file with --file
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli page snapshot --file /tmp/snapshot-test.txt"
    Then the file "/tmp/snapshot-test.txt" contains the accessibility tree
    And stdout is empty
    And the exit code is 0

  Scenario: JSON output with --json
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli page snapshot --json"
    Then stdout contains valid JSON
    And the JSON has "role", "name", and "children" fields
    And interactive elements have a "uid" field

  Scenario: Pretty JSON output with --pretty
    Given a page is loaded
    When I run "chrome-cli page snapshot --pretty"
    Then stdout contains pretty-printed JSON with indentation

  Scenario: Default output is structured text
    Given a page is loaded
    When I run "chrome-cli page snapshot"
    Then output uses "- role \"name\" [uid]" format with indentation
    And the output is not JSON

  # --- State Management ---

  Scenario: UID-to-backend-node mapping persisted to session
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli page snapshot"
    Then the file "~/.chrome-cli/snapshot.json" exists
    And it contains a "uid_map" with uid-to-backend-node-id entries
    And the "url" field matches "https://example.com/"

  Scenario: UIDs stable across consecutive snapshots of same page
    Given a page is loaded and unchanged
    When I run "chrome-cli page snapshot" twice consecutively
    Then the same elements receive the same uids in both runs

  # --- Edge Cases ---

  Scenario: Large page handling with truncation
    Given a very large page with more than 10000 elements
    When I run "chrome-cli page snapshot"
    Then the snapshot output is truncated
    And a truncation message indicates the total node count

  Scenario: Blank page produces minimal tree
    Given a blank page is loaded at "about:blank"
    When I run "chrome-cli page snapshot"
    Then output shows a minimal tree with just the document root
    And the exit code is 0
