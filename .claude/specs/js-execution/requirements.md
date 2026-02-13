# Requirements: JavaScript Execution in Page Context

**Issue**: #13
**Date**: 2026-02-12
**Status**: Draft
**Author**: Claude (writing-specs)

---

## User Story

**As a** developer / automation engineer
**I want** to execute arbitrary JavaScript in the browser page context via the CLI
**So that** I can perform custom automation, data extraction, and DOM manipulation from scripts and pipelines

---

## Background

JavaScript execution is a fundamental building block for browser automation. While dedicated commands like `page text` and `element find` cover common patterns, many automation tasks require running arbitrary JavaScript — evaluating expressions, calling functions, extracting computed values, or triggering UI interactions. The `js exec` command exposes `Runtime.evaluate` and `Runtime.callFunctionOn` from the Chrome DevTools Protocol, giving power users direct access to the page's JavaScript execution context. This is an MVP feature identified in the product steering document.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Execute a JavaScript expression

**Given** Chrome is running with a page loaded at "https://example.com"
**When** I run `chrome-cli js exec "document.title"`
**Then** stdout contains JSON with `result` and `type` fields
**And** the `result` field contains the page title string
**And** the `type` field is `"string"`

**Example**:
- Given: Page loaded at https://example.com
- When: `chrome-cli js exec "document.title"`
- Then: `{"result":"Example Domain","type":"string"}`

### AC2: Execute a JavaScript function

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli js exec "() => { return 2 + 2; }"`
**Then** the `result` field is `4`
**And** the `type` field is `"number"`

### AC3: Return all JavaScript value types

**Given** Chrome is running with a page loaded
**When** I execute JavaScript that returns a string, number, boolean, null, undefined, object, or array
**Then** each type is correctly represented in JSON output
**And** the `type` field accurately reflects the JavaScript type

### AC4: Target a specific tab with --tab

**Given** Chrome is running with multiple tabs open
**When** I run `chrome-cli js exec --tab <ID> "document.title"`
**Then** the JavaScript is executed in the specified tab
**And** the `result` reflects that tab's page context

### AC5: Await promise results (default behavior)

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli js exec "new Promise(r => setTimeout(() => r('done'), 100))"`
**Then** the `result` field is `"done"`
**And** the promise was awaited before returning

### AC6: Disable promise awaiting with --no-await

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli js exec --no-await "new Promise(r => setTimeout(() => r('done'), 100))"`
**Then** the `result` field represents the unresolved promise object
**And** the `type` field is `"object"`

### AC7: Execution timeout with --timeout

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli js exec --timeout 100 "new Promise(() => {})"`
**Then** stderr contains a JSON error indicating a timeout
**And** the exit code is non-zero

### AC8: Execute JavaScript from a file with --file

**Given** Chrome is running with a page loaded
**And** a file `/tmp/script.js` contains `document.title`
**When** I run `chrome-cli js exec --file /tmp/script.js`
**Then** the `result` field contains the page title
**And** the behavior is identical to inline code execution

### AC9: Read code from stdin with `-`

**Given** Chrome is running with a page loaded
**When** I run `echo "document.title" | chrome-cli js exec -`
**Then** the `result` field contains the page title
**And** stdin is read as the JavaScript code to execute

### AC10: Element context execution with --uid

**Given** Chrome is running with a page loaded
**And** a snapshot has been taken with `chrome-cli page snapshot`
**And** element `s1` exists in the snapshot
**When** I run `chrome-cli js exec --uid s1 "(el) => el.textContent"`
**Then** the function receives the DOM element as its first argument
**And** the `result` field contains the element's text content

### AC11: JavaScript exception handling

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli js exec "throw new Error('test error')"`
**Then** stderr contains a JSON error with `error` and `stack` fields
**And** the `error` field contains `"Error: test error"`
**And** the exit code is non-zero

### AC12: Reference error handling

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli js exec "nonExistentVariable"`
**Then** stderr contains a JSON error indicating a ReferenceError
**And** the exit code is non-zero

### AC13: Large result truncation with --max-size

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli js exec --max-size 100 "'x'.repeat(10000)"`
**Then** the `result` field is truncated to approximately 100 bytes
**And** a `truncated` field is set to `true` in the JSON output

### AC14: UID not found error

**Given** Chrome is running with a page loaded
**And** no snapshot exists or the UID is invalid
**When** I run `chrome-cli js exec --uid s999 "(el) => el.textContent"`
**Then** stderr contains a JSON error indicating the UID was not found
**And** the exit code is non-zero

### AC15: File not found error

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli js exec --file /nonexistent/script.js`
**Then** stderr contains a JSON error indicating the file was not found
**And** the exit code is non-zero

### AC16: Console output capture

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli js exec "console.log('hello'); 42"`
**Then** the `result` field is `42`
**And** a `console` field in the JSON output contains the captured console messages

### Generated Gherkin Preview

```gherkin
Feature: JavaScript execution in page context
  As a developer / automation engineer
  I want to execute arbitrary JavaScript in the browser page context via the CLI
  So that I can perform custom automation and data extraction from scripts

  Background:
    Given Chrome is running with CDP enabled

  Scenario: Execute a JavaScript expression
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli js exec 'document.title'"
    Then stdout contains JSON with keys "result", "type"
    And the "result" field is "Example Domain"
    And the "type" field is "string"

  Scenario: Execute a JavaScript function
    Given a page is loaded
    When I run "chrome-cli js exec '() => { return 2 + 2; }'"
    Then the "result" field is 4
    And the "type" field is "number"

  Scenario Outline: Return all JavaScript value types
    Given a page is loaded
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

  Scenario: Target a specific tab
    Given multiple tabs are open
    When I run "chrome-cli js exec --tab <ID> 'document.title'"
    Then the result reflects the targeted tab's context

  Scenario: Await promise results by default
    Given a page is loaded
    When I run "chrome-cli js exec 'new Promise(r => setTimeout(() => r(\"done\"), 100))'"
    Then the "result" field is "done"

  Scenario: Disable promise awaiting
    Given a page is loaded
    When I run "chrome-cli js exec --no-await 'new Promise(r => r(42))'"
    Then the "type" field is "object"

  Scenario: Execution timeout
    Given a page is loaded
    When I run "chrome-cli js exec --timeout 100 'new Promise(() => {})'"
    Then stderr contains a JSON error indicating timeout
    And the exit code is non-zero

  Scenario: Execute JavaScript from a file
    Given a page is loaded
    And a file "/tmp/script.js" contains "document.title"
    When I run "chrome-cli js exec --file /tmp/script.js"
    Then the "result" field contains the page title

  Scenario: Read code from stdin
    Given a page is loaded
    When I pipe "document.title" to "chrome-cli js exec -"
    Then the "result" field contains the page title

  Scenario: Element context execution with UID
    Given a page is loaded
    And a snapshot has been taken
    And element "s1" exists in the snapshot
    When I run "chrome-cli js exec --uid s1 '(el) => el.textContent'"
    Then the result contains the element's text content

  Scenario: JavaScript exception returned as structured error
    Given a page is loaded
    When I run "chrome-cli js exec 'throw new Error(\"test error\")'"
    Then stderr contains a JSON error with "error" and "stack" fields
    And the exit code is non-zero

  Scenario: Reference error handling
    Given a page is loaded
    When I run "chrome-cli js exec 'nonExistentVariable'"
    Then stderr contains a JSON error indicating ReferenceError
    And the exit code is non-zero

  Scenario: Large result truncation
    Given a page is loaded
    When I run "chrome-cli js exec --max-size 100 \"'x'.repeat(10000)\""
    Then the "result" field is truncated
    And a "truncated" field is true

  Scenario: UID not found error
    Given a page is loaded
    When I run "chrome-cli js exec --uid s999 '(el) => el.textContent'"
    Then stderr contains a JSON error about UID not found
    And the exit code is non-zero

  Scenario: File not found error
    Given a page is loaded
    When I run "chrome-cli js exec --file /nonexistent/script.js"
    Then stderr contains a JSON error about file not found
    And the exit code is non-zero

  Scenario: Console output capture
    Given a page is loaded
    When I run "chrome-cli js exec 'console.log(\"hello\"); 42'"
    Then the "result" field is 42
    And the "console" field contains "hello"
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `js exec <CODE>` executes JavaScript via `Runtime.evaluate` | Must | Core functionality |
| FR2 | JSON output with `result` and `type` fields | Must | Default output format |
| FR3 | `--tab <ID>` targets a specific tab | Must | Consistent with other commands |
| FR4 | `--await` / `--no-await` controls promise handling (default: await) | Must | Async JavaScript support |
| FR5 | `--timeout <MS>` overrides execution timeout | Must | Long-running script control |
| FR6 | `--file <PATH>` reads JavaScript from a file | Must | Complex script support |
| FR7 | `-` as code argument reads from stdin | Must | Pipeline integration |
| FR8 | `--uid <UID>` passes element reference to function via `Runtime.callFunctionOn` | Must | Element context execution |
| FR9 | JavaScript exceptions returned as structured errors with stack trace | Must | Debugging support |
| FR10 | `--max-size <BYTES>` truncates large results | Should | Prevent overwhelming output |
| FR11 | Console output captured during execution | Should | Debugging support |
| FR12 | Handles all JS return types: string, number, boolean, null, undefined, object, array | Must | Complete type coverage |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Expression evaluation completes within the `--timeout` window (default from global `--timeout` or 30s) |
| **Security** | Power-user tool — no sandboxing needed since the user controls the browser. Local CDP only. |
| **Reliability** | Graceful error on disconnected tabs, crashed pages, or pages mid-load |
| **Platforms** | macOS, Linux, Windows (same as project baseline) |
| **Output** | Errors to stderr as JSON, data to stdout; exit codes per `error.rs` conventions |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `<CODE>` | String (positional) | Must be valid JavaScript or `-` for stdin | Yes (unless `--file`) |
| `--file` | String (file path) | File must exist and be readable | No (mutually exclusive with `<CODE>`) |
| `--tab` | String (tab ID) | Must resolve to a valid target | No (defaults to first page target) |
| `--uid` | String (snapshot UID) | Must exist in current snapshot state | No |
| `--timeout` | u64 (milliseconds) | Positive integer | No (inherits global timeout) |
| `--max-size` | usize (bytes) | Positive integer | No (default: no truncation) |
| `--no-await` | Boolean flag | N/A | No (default: await promises) |

### Output Data (Success)

| Field | Type | Description |
|-------|------|-------------|
| `result` | serde_json::Value | The JavaScript return value, serialized to JSON |
| `type` | String | JavaScript type of the result (`"string"`, `"number"`, `"boolean"`, `"object"`, `"undefined"`) |
| `console` | Array of objects | Console messages captured during execution (if any) |
| `truncated` | Boolean | Present and `true` only when result was truncated by `--max-size` |

### Output Data (Error)

| Field | Type | Description |
|-------|------|-------------|
| `error` | String | Error description (e.g., `"ReferenceError: foo is not defined"`) |
| `stack` | String | JavaScript stack trace (when available) |
| `code` | u8 | Exit code |

---

## Dependencies

### Internal Dependencies
- [x] Issue #4 — CDP client (merged)
- [x] Issue #6 — Session/connection management (merged)
- [x] Issue #10 — Accessibility tree / snapshot (merged, needed for `--uid`)

### Blocked By
- None (all dependencies resolved)

---

## Out of Scope

- Multi-expression batching (execute multiple scripts in sequence)
- REPL / interactive JavaScript console mode
- Source maps or TypeScript transpilation
- Injecting scripts that persist across navigations (use `Page.addScriptToEvaluateOnNewDocument` later)
- Execution in iframe contexts (main frame only for now)
- Custom serialization of DOM elements in return values (elements returned as `{}`)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Type coverage | All 7 JS types handled | Test with each type |
| Error fidelity | Exception message + stack trace returned | Test with throwing code |
| Pipeline support | stdin and file input work | Test with `echo ... \| chrome-cli js exec -` |

---

## Open Questions

- [x] ~~Should `--await` be the default?~~ — Yes, per the issue spec. Use `--no-await` to disable.
- [x] ~~Should console capture be opt-in?~~ — Include by default when console messages are present; omit the field when empty.

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Edge cases and error states are specified
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
