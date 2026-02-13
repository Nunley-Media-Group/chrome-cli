# File: tests/features/perf.feature
#
# Generated from: .claude/specs/performance-tracing/requirements.md
# Issue: #22

Feature: Performance tracing and metrics
  As a developer or automation engineer
  I want to capture and analyze Chrome performance traces from the CLI
  So that I can diagnose page performance issues and measure Core Web Vitals

  Background:
    Given Chrome is running with CDP enabled
    And a page is loaded at "https://example.com"

  # --- Happy Path: Start ---

  Scenario: Start a performance trace with default options
    When I run "chrome-cli perf start"
    Then the output JSON has "tracing" set to true
    And the output JSON has a "file" field with a valid file path
    And the exit code should be 0

  Scenario: Start a trace with page reload
    When I run "chrome-cli perf start --reload"
    Then the page is reloaded before tracing begins
    And the output JSON has "tracing" set to true
    And the exit code should be 0

  Scenario: Start a trace with auto-stop
    When I run "chrome-cli perf start --auto-stop"
    Then the trace automatically stops after page load completes
    And the output JSON has a "file" field
    And the output JSON has a "duration_ms" field
    And the output JSON has a "vitals" object
    And the exit code should be 0

  Scenario: Start a trace with a custom output file
    When I run "chrome-cli perf start --file /tmp/my-trace.json"
    Then the output JSON has "file" set to "/tmp/my-trace.json"
    And the exit code should be 0

  Scenario: Start a trace targeting a specific tab
    Given Chrome has multiple tabs open
    When I run "chrome-cli perf start --tab <ID>"
    Then the trace is recorded for the specified tab
    And the exit code should be 0

  # --- Happy Path: Stop ---

  Scenario: Stop an active trace
    Given a performance trace is currently recording
    When I run "chrome-cli perf stop"
    Then the trace stops recording
    And trace data is saved to a file
    And the output JSON has a "file" field
    And the output JSON has a "duration_ms" field
    And the output JSON has a "size_bytes" field
    And the output JSON has a "vitals" object with Core Web Vitals
    And the exit code should be 0

  Scenario: Stop a trace with custom output file
    Given a performance trace is currently recording
    When I run "chrome-cli perf stop --file /tmp/output-trace.json"
    Then the trace data is saved to "/tmp/output-trace.json"
    And the exit code should be 0

  # --- Happy Path: Analyze ---

  Scenario: Analyze a specific performance insight
    Given a trace file exists at "/tmp/trace.json"
    When I run "chrome-cli perf analyze LCPBreakdown --trace-file /tmp/trace.json"
    Then the output JSON has "insight" set to "LCPBreakdown"
    And the output JSON has a "details" object with breakdown data
    And the exit code should be 0

  # --- Happy Path: Vitals ---

  Scenario: Quick Core Web Vitals measurement
    When I run "chrome-cli perf vitals"
    Then a trace is started, the page is reloaded, and the trace is stopped
    And the output JSON has a "lcp_ms" field
    And the output JSON has a "cls" field
    And the output JSON has a "ttfb_ms" field
    And the output JSON has a "url" field
    And the exit code should be 0

  # --- Error Handling ---

  Scenario: Stop when no trace is active
    Given no performance trace is currently recording
    When I run "chrome-cli perf stop"
    Then the error output contains "No active trace"
    And the exit code should be non-zero

  Scenario: Analyze with an invalid insight name
    Given a trace file exists at "/tmp/trace.json"
    When I run "chrome-cli perf analyze InvalidInsight --trace-file /tmp/trace.json"
    Then the error output contains "Unknown insight"
    And the error output lists available insight names
    And the exit code should be non-zero

  # --- Output Formats ---

  Scenario: JSON output format
    When I run "chrome-cli perf vitals --json"
    Then the output is valid compact JSON
    And the exit code should be 0

  Scenario: Plain text output format
    When I run "chrome-cli perf vitals --plain"
    Then the output contains human-readable labeled metrics
    And the output contains "LCP:"
    And the output contains "CLS:"
    And the output contains "TTFB:"
    And the exit code should be 0

  # --- Data-Driven: Insight Types ---

  Scenario Outline: Analyze different insight types
    Given a trace file exists at "/tmp/trace.json"
    When I run "chrome-cli perf analyze <insight> --trace-file /tmp/trace.json"
    Then the output JSON has "insight" set to "<insight>"
    And the output JSON has a "details" object
    And the exit code should be 0

    Examples:
      | insight          |
      | DocumentLatency  |
      | LCPBreakdown     |
      | RenderBlocking   |
      | LongTasks        |
