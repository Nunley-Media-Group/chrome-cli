# File: tests/features/js-execution.feature
#
# Generated from: .claude/specs/js-execution/requirements.md
# Issue: #13

Feature: JavaScript execution in page context
  As a developer / automation engineer
  I want to execute arbitrary JavaScript in the browser page context via the CLI
  So that I can perform custom automation and data extraction from scripts

  Background:
    Given Chrome is running with CDP enabled

  # --- Happy Path ---

  Scenario: Execute a JavaScript expression
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli js exec 'document.title'"
    Then stdout contains JSON with keys "result", "type"
    And the "result" field is "Example Domain"
    And the "type" field is "string"

  Scenario: Execute a JavaScript function
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli js exec '() => { return 2 + 2; }'"
    Then the "result" field is 4
    And the "type" field is "number"

  Scenario Outline: Return all JavaScript value types
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli js exec '<expression>'"
    Then the "type" field is "<expected_type>"

    Examples:
      | expression            | expected_type |
      | 'hello'               | string        |
      | 42                    | number        |
      | true                  | boolean       |
      | null                  | object        |
      | undefined             | undefined     |
      | ({key: 'val'})        | object        |
      | [1, 2, 3]             | object        |

  # --- Tab Targeting ---

  Scenario: Target a specific tab with --tab
    Given multiple tabs are open
    And the second tab is loaded at "https://example.org"
    When I run "chrome-cli js exec --tab <SECOND_TAB_ID> 'document.title'"
    Then the "result" field reflects the second tab's title

  # --- Promise Handling ---

  Scenario: Await promise results by default
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli js exec 'new Promise(r => setTimeout(() => r(\"done\"), 100))'"
    Then the "result" field is "done"
    And the "type" field is "string"

  Scenario: Disable promise awaiting with --no-await
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli js exec --no-await 'new Promise(r => r(42))'"
    Then the "type" field is "object"

  # --- Timeout ---

  Scenario: Execution timeout with --timeout
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli js exec --timeout 100 'new Promise(() => {})'"
    Then stderr contains a JSON error indicating timeout
    And the exit code is non-zero

  # --- File and Stdin Input ---

  Scenario: Execute JavaScript from a file
    Given a page is loaded at "https://example.com"
    And a temporary file contains "document.title"
    When I run "chrome-cli js exec --file <TEMP_FILE>"
    Then the "result" field is "Example Domain"
    And the "type" field is "string"

  Scenario: Read code from stdin with dash argument
    Given a page is loaded at "https://example.com"
    When I pipe "document.title" to "chrome-cli js exec -"
    Then the "result" field is "Example Domain"
    And the "type" field is "string"

  # --- Element Context ---

  Scenario: Element context execution with --uid
    Given a page is loaded at "https://example.com"
    And a snapshot has been taken with "chrome-cli page snapshot"
    And element "s1" exists in the snapshot
    When I run "chrome-cli js exec --uid s1 '(el) => el.textContent'"
    Then the "result" field contains the element's text content
    And the "type" field is "string"

  # --- Error Handling ---

  Scenario: JavaScript exception returned as structured error
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli js exec 'throw new Error(\"test error\")'"
    Then stderr contains a JSON error with key "error"
    And the error message contains "Error: test error"
    And the exit code is non-zero

  Scenario: Reference error for undefined variable
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli js exec 'nonExistentVariable'"
    Then stderr contains a JSON error
    And the error message contains "ReferenceError"
    And the exit code is non-zero

  Scenario: UID not found error
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli js exec --uid s999 '(el) => el.textContent'"
    Then stderr contains a JSON error about UID not found
    And the exit code is non-zero

  Scenario: File not found error
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli js exec --file /nonexistent/script.js"
    Then stderr contains a JSON error about file not found
    And the exit code is non-zero

  # --- Truncation ---

  Scenario: Large result truncation with --max-size
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli js exec --max-size 100 \"'x'.repeat(10000)\""
    Then the JSON output contains a "truncated" field set to true
    And the "result" field is shorter than 10000 characters

  # --- Console Capture ---

  Scenario: Console output captured during execution
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli js exec 'console.log(\"hello\"); 42'"
    Then the "result" field is 42
    And the "console" field is an array
    And the "console" array contains an entry with level "log" and text "hello"
