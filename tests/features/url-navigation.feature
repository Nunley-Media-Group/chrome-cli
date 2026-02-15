# File: tests/features/url-navigation.feature
#
# Generated from: .claude/specs/url-navigation/requirements.md
# Issue: #8

Feature: URL Navigation
  As a developer or automation engineer
  I want to navigate Chrome to URLs and traverse browser history from the CLI
  So that I can script browser navigation workflows without manual interaction

  Background:
    Given Chrome is running with CDP enabled

  # --- URL Navigation: Happy Path ---

  Scenario: AC1 - Navigate to a valid URL
    Given a tab is open with "about:blank"
    When I run "chrome-cli navigate https://example.com"
    Then the exit code is 0
    And the JSON output has key "url" containing "example.com"
    And the JSON output has key "title" with a non-empty string
    And the JSON output has key "status" with a numeric value

  Scenario: AC2 - Navigate with --tab to target a specific tab
    Given two tabs are open
    And the second tab has ID "<TAB_ID>"
    When I run "chrome-cli navigate https://example.com --tab <TAB_ID>"
    Then the exit code is 0
    And the JSON output has key "url" containing "example.com"
    And the first tab URL is unchanged

  # --- Wait Strategies ---

  Scenario: AC3 - Navigate with --wait-until load (default)
    Given a tab is open
    When I run "chrome-cli navigate https://example.com --wait-until load"
    Then the exit code is 0
    And the command waited for the page load event
    And the JSON output has key "url"
    And the JSON output has key "title"
    And the JSON output has key "status"

  Scenario: AC4 - Navigate with --wait-until domcontentloaded
    Given a tab is open
    When I run "chrome-cli navigate https://example.com --wait-until domcontentloaded"
    Then the exit code is 0
    And the command returned after DOMContentLoaded fired
    And the JSON output has key "url"
    And the JSON output has key "title"
    And the JSON output has key "status"

  Scenario: AC5 - Navigate with --wait-until networkidle
    Given a tab is open
    When I run "chrome-cli navigate https://example.com --wait-until networkidle"
    Then the exit code is 0
    And the command waited until network was idle for 500ms
    And the JSON output has key "url"
    And the JSON output has key "title"
    And the JSON output has key "status"

  Scenario: AC6 - Navigate with --wait-until none
    Given a tab is open
    When I run "chrome-cli navigate https://example.com --wait-until none"
    Then the exit code is 0
    And the command returned immediately after initiating navigation
    And the JSON output has key "url"

  # --- Timeout ---

  Scenario: AC7 - Navigate with --timeout
    Given a tab is open
    When I run "chrome-cli navigate https://httpbin.org/delay/60 --timeout 1000"
    Then the exit code is 4
    And the error message contains "timed out"
    And the error message contains "1000ms"

  # --- Cache Bypass ---

  Scenario: AC8 - Navigate with --ignore-cache
    Given a tab is open
    When I run "chrome-cli navigate https://example.com --ignore-cache"
    Then the exit code is 0
    And the navigation bypassed the browser cache
    And the JSON output has key "url"

  # --- History: Back ---

  Scenario: AC9 - Navigate back in browser history
    Given a tab is open with "https://example.com"
    And the tab has navigated to "https://www.iana.org/domains/reserved"
    When I run "chrome-cli navigate back"
    Then the exit code is 0
    And the JSON output has key "url" containing "example.com"
    And the JSON output has key "title"

  Scenario: AC10 - Navigate back with --tab
    Given two tabs are open
    And the second tab has navigated to two pages
    When I run "chrome-cli navigate back --tab <TAB_ID>"
    Then the exit code is 0
    And the JSON output has key "url"
    And the specified tab navigated back

  @regression
  Scenario: AC9b - Cross-origin navigate back succeeds
    Given a tab is open with "https://example.com"
    And the tab has navigated to "https://www.iana.org/domains/reserved"
    When I run "chrome-cli navigate back"
    Then the exit code is 0
    And the JSON output has key "url" containing "example.com"

  # --- History: Forward ---

  Scenario: AC11 - Navigate forward in browser history
    Given a tab has navigated to two pages and then gone back
    When I run "chrome-cli navigate forward"
    Then the exit code is 0
    And the JSON output has key "url" containing the second page URL
    And the JSON output has key "title"

  Scenario: AC12 - Navigate forward with --tab
    Given two tabs are open
    And the second tab has gone back and has forward history
    When I run "chrome-cli navigate forward --tab <TAB_ID>"
    Then the exit code is 0
    And the specified tab navigated forward

  @regression
  Scenario: AC12b - Cross-origin navigate forward succeeds
    Given a tab is open with "https://example.com"
    And the tab has navigated to "https://www.iana.org/domains/reserved"
    And the tab has navigated back
    When I run "chrome-cli navigate forward"
    Then the exit code is 0
    And the JSON output has key "url" containing "iana.org"

  # --- Reload ---

  Scenario: AC13 - Reload the current page
    Given a tab is open with "https://example.com"
    When I run "chrome-cli navigate reload"
    Then the exit code is 0
    And the JSON output has key "url" containing "example.com"
    And the JSON output has key "title"

  Scenario: AC14 - Reload with --ignore-cache
    Given a tab is open with "https://example.com"
    When I run "chrome-cli navigate reload --ignore-cache"
    Then the exit code is 0
    And the page was reloaded bypassing the cache

  Scenario: AC15 - Reload with --tab
    Given two tabs are open with pages loaded
    When I run "chrome-cli navigate reload --tab <TAB_ID>"
    Then the exit code is 0
    And the specified tab was reloaded

  # --- Error Handling ---

  Scenario: AC16 - DNS resolution failure
    Given a tab is open
    When I run "chrome-cli navigate https://this-domain-does-not-exist.invalid"
    Then the exit code is not 0
    And the error message contains "Navigation failed"
    And the error message contains "ERR_NAME_NOT_RESOLVED" or a DNS-related message

  Scenario: AC17 - Navigation timeout
    Given a tab is open
    When I run "chrome-cli navigate https://httpbin.org/delay/60 --timeout 1000"
    Then the exit code is 4
    And the error message contains "timed out"

  Scenario: AC18 - No Chrome connection
    Given no Chrome instance is running
    When I run "chrome-cli navigate https://example.com"
    Then the exit code is 2
    And the error message contains "No Chrome instance found" or "chrome-cli connect"
