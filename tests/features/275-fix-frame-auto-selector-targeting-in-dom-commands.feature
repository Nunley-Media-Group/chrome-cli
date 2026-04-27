# File: tests/features/275-fix-frame-auto-selector-targeting-in-dom-commands.feature
#
# Generated from: specs/bug-fix-frame-auto-selector-targeting-in-dom-commands/requirements.md
# Issue: #275
# Type: Defect regression

@regression
Feature: DOM selector targets resolve frames automatically
  The DOM select command previously passed no selector hint when resolving
  --frame auto, so frame auto detection searched only UID maps and failed
  before selectors could be evaluated in child frames. The fix makes DOM
  selector targets participate in auto frame resolution while preserving
  explicit frame targeting and UID-based auto targeting.

  Background:
    Given agentchrome is built
    And a Chrome instance is connected
    And the test fixture "iframe-frame-targeting.html" is loaded

  @regression
  Scenario: AC1 - selector-based DOM auto frame targeting finds child-frame elements
    Given a child iframe contains a body element
    When I run "agentchrome dom --frame auto select body"
    Then the exit code should be 0
    And stdout should contain an element from the child frame
    And stdout should include frame context for the selected frame

  @regression
  Scenario: AC2 - explicit DOM frame targeting still works
    Given the child iframe is available at frame index 1
    When I run "agentchrome dom --frame 1 select body"
    Then the exit code should be 0
    And stdout should contain the child frame body

  @regression
  Scenario: AC3 - UID-based auto frame targeting still works
    Given a page snapshot has assigned a UID to an element inside the child iframe
    When I run "agentchrome interact --frame auto click <uid>"
    Then the exit code should be 0
    And the command acts in the frame that owns the UID

  @regression
  Scenario: AC4 - missing selector preserves target-error contract
    Given no frame contains an element matching "css:#missing-auto-selector"
    When I run "agentchrome dom --frame auto select css:#missing-auto-selector"
    Then stderr contains a JSON error with "Element not found in any frame"
    And stderr JSON contains "code" equal to 3
    And the exit code should be 3
