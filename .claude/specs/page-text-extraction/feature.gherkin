# File: tests/features/page-text-extraction.feature
#
# Generated from: .claude/specs/page-text-extraction/requirements.md
# Issue: #9

Feature: Page text extraction
  As a developer / automation engineer
  I want to extract readable text content from a browser page via the CLI
  So that I can process page content in scripts and AI pipelines without parsing HTML

  Background:
    Given Chrome is running with CDP enabled

  # --- Happy Path ---

  Scenario: Extract all visible text from current page (AC1)
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli page text"
    Then stdout is valid JSON with keys "text", "url", "title"
    And the "text" field contains "Example Domain"
    And the "url" field starts with "https://example.com"
    And the "title" field is "Example Domain"
    And the exit code is 0

  Scenario: Target a specific tab with --tab (AC2)
    Given a tab is open at "https://example.com" with ID "TAB_ID"
    And another tab is open at "https://www.iana.org"
    When I run "chrome-cli page text --tab TAB_ID"
    Then the "url" field starts with "https://example.com"
    And the "text" field contains "Example Domain"

  Scenario: Plain text output with --plain (AC3)
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli page text --plain"
    Then stdout contains "Example Domain"
    And stdout is not valid JSON

  Scenario: Extract text from specific element with --selector (AC4)
    Given a page is loaded with an element matching "#content"
    When I run "chrome-cli page text --selector '#content'"
    Then the "text" field contains only content from that element
    And the "text" field does not contain text from outside the element

  # --- Error Handling ---

  Scenario: Selector targets non-existent element (AC5)
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli page text --selector '#does-not-exist'"
    Then stderr contains a JSON error with "not found"
    And the exit code is non-zero

  # --- Edge Cases ---

  Scenario: Script and style content excluded (AC6)
    Given a page is loaded with inline script and style elements
    When I run "chrome-cli page text"
    Then the "text" field does not contain "function()"
    And the "text" field does not contain "background-color"

  Scenario: Basic structure preserved with newlines (AC7)
    Given a page is loaded with headings and paragraphs
    When I run "chrome-cli page text"
    Then the "text" field contains newline-separated blocks
    And headings and paragraphs are distinguishable by whitespace

  Scenario: Page with no content returns empty text (AC8)
    Given a blank page is loaded at "about:blank"
    When I run "chrome-cli page text"
    Then stdout is valid JSON
    And the "text" field is ""
    And the exit code is 0

  Scenario: Pretty JSON output with --pretty (AC9)
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli page text --pretty"
    Then stdout is valid JSON with keys "text", "url", "title"
    And stdout contains newlines and indentation

  Scenario: Iframe content excluded by default (AC10)
    Given a page is loaded with an iframe containing "iframe text"
    When I run "chrome-cli page text"
    Then the "text" field does not contain "iframe text"
