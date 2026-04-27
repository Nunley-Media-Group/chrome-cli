# File: tests/features/277-fix-same-document-url-navigation-waits-for-fragment-only-navigations.feature
#
# Generated from: specs/bug-fix-same-document-url-navigation-waits-for-fragment-only-navigations/requirements.md
# Issue: #277
# Type: Defect regression

@regression
Feature: Same-document URL navigation respects load-style waits
  Direct URL navigation previously timed out when the requested URL differed
  from the current page only by fragment and the caller used --wait-until load
  or --wait-until domcontentloaded. The fix makes URL navigation treat
  Page.navigatedWithinDocument as completion for same-document navigations
  while preserving cross-document load waits.

  Background:
    Given agentchrome is built
    And a Chrome instance is connected
    And the same-document navigation fixture is loaded

  # --- Bug Is Fixed ---

  @regression
  Scenario: AC1 - same-document URL navigate succeeds with load wait
    Given the active tab is loaded at the same-document navigation fixture base URL
    When I run "agentchrome navigate <fixture-url>#S06 --wait-until load"
    Then the exit code should be 0
    And stdout should contain JSON with "url" ending in "#S06"
    And stdout should contain a "title" field

  @regression
  Scenario: AC2 - same-document URL navigate succeeds with DOMContentLoaded wait
    Given the active tab is loaded at the same-document navigation fixture base URL
    When I run "agentchrome navigate <fixture-url>#S07 --wait-until domcontentloaded"
    Then the exit code should be 0
    And stdout should contain JSON with "url" ending in "#S07"
    And stdout should contain a "title" field

  # --- Related Behavior Still Works ---

  @regression
  Scenario: AC3 - cross-document URL navigate still waits for load completion
    Given Chrome is connected and the active tab can reach "https://example.com/"
    When I run "agentchrome navigate https://example.com/ --wait-until load"
    Then the command should wait for cross-document load completion
    And the exit code should be 0
    And stdout should contain JSON with keys "url", "title", and "status"
