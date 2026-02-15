# File: tests/features/75-perf-vitals-missing-metrics.feature
#
# Generated from: .claude/specs/75-fix-perf-vitals-missing-metrics/requirements.md
# Issue: #75
# Type: Defect regression

@regression
Feature: perf vitals returns performance metrics
  The perf vitals command previously returned only the page URL with no
  performance metrics (lcp_ms, cls, ttfb_ms) because the trace was stopped
  too early and null fields were omitted from JSON serialization.
  This was fixed by adding a post-load stabilization delay and always
  serializing metric fields (as null when unavailable).

  # --- Bug Is Fixed ---

  @regression
  Scenario: perf vitals returns metrics after page reload
    Given Chrome is connected and navigated to "https://www.google.com/"
    When I run "chrome-cli perf vitals"
    Then the JSON output contains the key "lcp_ms" with a numeric value
    And the JSON output contains the key "ttfb_ms" with a numeric value
    And the JSON output contains the key "cls"
    And the JSON output contains the key "url"
    And the exit code should be 0

  # --- Related Behavior Still Works ---

  @regression
  Scenario: null metrics are serialized as null instead of omitted
    Given Chrome is connected to a page with no layout shifts
    When I run "chrome-cli perf vitals"
    Then the JSON output contains the key "lcp_ms"
    And the JSON output contains the key "cls"
    And the JSON output contains the key "ttfb_ms"
    And all three metric keys are present even if their values are null

  @regression
  Scenario: non-zero exit code when all metrics are unavailable
    Given Chrome is connected to a page where no vitals can be collected
    When I run "chrome-cli perf vitals"
    Then the exit code should be non-zero
    And stderr contains a warning about missing metrics
