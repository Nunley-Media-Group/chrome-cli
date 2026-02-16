# File: tests/features/102-fix-network-list-empty-array.feature
#
# Generated from: .claude/specs/102-fix-network-list-empty-array/requirements.md
# Issue: #102
# Type: Defect regression

@regression
Feature: network list returns captured requests after page load
  The `network list` command previously always returned an empty array `[]`
  regardless of network activity. This was because each CLI invocation created
  a new CDP connection after requests had already completed, and CDP has no
  retrospective API. This was fixed by triggering a page reload after enabling
  the Network domain so events are captured live.

  Background:
    Given Chrome is running with CDP enabled

  # --- Bug Is Fixed ---

  @regression @requires-chrome
  Scenario: Network list returns requests after page load
    Given a page is loaded that has made network requests
    When I run "chrome-cli network list"
    Then the output is a JSON array
    And the array contains at least one entry
    And each entry contains "id", "method", "url", "status", and "type"
    And the exit code should be 0

  @regression @requires-chrome
  Scenario: Type filter works on captured requests
    Given a page is loaded that has made network requests
    When I run "chrome-cli network list --type document"
    Then the output is a JSON array
    And the array contains at least one entry
    And each entry has type "document"

  @regression @requires-chrome
  Scenario: URL filter works on captured requests
    Given a page at "https://www.google.com" has been loaded
    When I run "chrome-cli network list --url google"
    Then the output is a JSON array
    And the array contains at least one entry
    And each entry URL contains "google"

  @regression @requires-chrome
  Scenario: Network get returns details for a captured request
    Given a page is loaded that has made network requests
    And I have a request ID from "chrome-cli network list"
    When I run "chrome-cli network get <request-id>"
    Then the output is a JSON object with "request", "response", and "timing" sections
    And the exit code should be 0

  # --- Related Behavior Still Works ---

  @regression @requires-chrome
  Scenario: Network follow streaming still works
    Given a page is open
    When I run "chrome-cli network follow --timeout 3000"
    And the page makes new network requests
    Then each completed request is printed as a JSON line
    And each line contains "method", "url", "status"
