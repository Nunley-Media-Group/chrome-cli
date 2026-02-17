# File: tests/features/119-fix-perf-vitals-null-cls-ttfb.feature
#
# Generated from: .claude/specs/119-fix-perf-vitals-null-cls-ttfb/requirements.md
# Issue: #119
# Type: Defect regression

@regression
Feature: perf vitals returns null for CLS and TTFB metrics
  The `perf vitals` command previously returned `cls: null` when no layout
  shifts occurred and `ttfb_ms: null` when CDP resource tracing events were
  absent. This was fixed by returning CLS 0.0 for zero-shift pages and adding
  a fallback TTFB extraction mechanism.

  Background:
    Given a Chrome instance is running
    And a page has been navigated to

  # --- Bug Is Fixed ---

  @regression
  Scenario: CLS returns 0.0 when no layout shifts occur
    Given the loaded page has no layout shifts
    When I run "chrome-cli perf vitals"
    Then the JSON output should contain key "cls"
    And the "cls" field should be a number
    And the "cls" field should equal 0.0

  @regression
  Scenario: TTFB is measured for loaded pages
    When I run "chrome-cli perf vitals"
    Then the JSON output should contain key "ttfb_ms"
    And the "ttfb_ms" field should be a number
    And the "ttfb_ms" field should be greater than 0

  # --- Related Behavior Still Works ---

  @regression
  Scenario: LCP continues to work correctly
    When I run "chrome-cli perf vitals"
    Then the JSON output should contain key "lcp_ms"
    And the "lcp_ms" field should be a number
    And the "lcp_ms" field should be greater than 0
