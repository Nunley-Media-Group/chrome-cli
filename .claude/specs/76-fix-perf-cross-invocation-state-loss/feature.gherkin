# File: tests/features/perf-record.feature
#
# Generated from: .claude/specs/76-fix-perf-cross-invocation-state-loss/requirements.md
# Issue: #76
# Type: Defect regression

@regression
Feature: perf record replaces broken perf start/stop workflow
  The `perf start` / `perf stop` two-command workflow previously failed because
  each CLI invocation created a new CDP session, and Chrome's Tracing domain
  state is bound to the session that initiated it. This was fixed by replacing
  the two-command workflow with a single long-running `perf record` command
  that holds the session open until stopped by signal or timeout.

  # --- Bug Is Fixed ---

  @regression
  Scenario: perf record captures a complete trace in a single session
    Given Chrome is running with CDP enabled
    And a page is loaded at "https://example.com"
    When I run "chrome-cli perf record --duration 2000"
    Then the exit code should be 0
    And the trace file should exist
    And the trace file should contain valid Chrome Trace Event Format data
    And the output should include "file" and "duration_ms" and "size_bytes"

  # --- Related Behavior Still Works ---

  @regression
  Scenario: perf record with --reload reloads before recording
    Given Chrome is running with CDP enabled
    And a page is loaded at "https://example.com"
    When I run "chrome-cli perf record --reload --duration 3000"
    Then the exit code should be 0
    And the trace file should exist
    And the trace file should contain valid Chrome Trace Event Format data

  @regression
  Scenario: perf record stops gracefully on Ctrl+C
    Given Chrome is running with CDP enabled
    And a page is loaded at "https://example.com"
    When I start "chrome-cli perf record" in the background
    And I wait 2 seconds
    And I send SIGINT to the process
    Then the exit code should be 0
    And the trace file should exist
    And the trace file should contain valid Chrome Trace Event Format data

  # --- Edge Case ---

  @regression
  Scenario: perf vitals still works as a single-invocation command
    Given Chrome is running with CDP enabled
    And a page is loaded at "https://example.com"
    When I run "chrome-cli perf vitals"
    Then the exit code should be 0
    And the output should include "lcp_ms" and "cls" and "ttfb_ms"
