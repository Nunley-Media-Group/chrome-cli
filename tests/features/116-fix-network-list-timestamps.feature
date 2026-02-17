# File: tests/features/116-fix-network-list-timestamps.feature
#
# Generated from: .claude/specs/116-fix-network-list-timestamps/requirements.md
# Issue: #116
# Type: Defect regression

@regression
Feature: Network list timestamps show real wall-clock time
  The network list command previously displayed timestamps as 1970-01-01
  dates because CDP monotonic timestamps were treated as Unix epoch values.
  This was fixed by using the wallTime field from CDP Network events.

  Background:
    Given Chrome is running with CDP enabled

  # --- Bug Is Fixed ---

  @regression @requires-chrome
  Scenario: Network timestamps reflect real wall-clock time
    Given a page is loaded that has made network requests
    When I run "chrome-cli network list"
    Then the timestamp fields show dates from the current year
    And the timestamps are within the last few minutes

  # --- Timestamps Are Valid ISO 8601 ---

  @regression @requires-chrome
  Scenario: Network timestamps are valid ISO 8601 in UTC
    Given a page is loaded that has made network requests
    When I run "chrome-cli network list"
    Then all timestamp values match the ISO 8601 format
    And all timestamps end with Z indicating UTC

  # --- Console Timestamps Still Work ---

  @regression @requires-chrome
  Scenario: Console timestamps are not regressed
    Given a page is loaded that has made network requests
    When I run "chrome-cli console read"
    Then console timestamps show correct wall-clock times
