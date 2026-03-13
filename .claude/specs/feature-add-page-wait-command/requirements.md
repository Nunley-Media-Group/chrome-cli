# Requirements: Add Page Wait Command

**Issues**: #163
**Date**: 2026-03-11
**Status**: Draft
**Author**: Claude

---

## User Story

**As an** AI agent automating multi-step browser workflows
**I want** a standalone command to wait until a specified condition is met
**So that** I can reliably synchronize with page state changes without arbitrary sleeps or polling loops

---

## Background

When automating SPAs or any dynamic web content, agents need to wait for the page to reach a certain state before proceeding — after form submissions, AJAX requests, page transitions, or async content loads. Currently, agents have no built-in way to wait; the workaround is a poll loop (`page snapshot` → check → sleep → repeat), which wastes commands, tokens, and time.

Issue #148 adds `--wait-until` to `interact click`, handling the "click then wait" case. This feature adds a standalone `page wait` command for waiting independently of any click — after `form submit`, after `js exec`, after `navigate`, or simply when waiting for async content.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Wait for URL to match a glob pattern

**Given** a connected Chrome session where the current URL is `https://example.com/login`
**When** I run `agentchrome page wait --url "*/dashboard*"` and the URL changes to `https://example.com/dashboard`
**Then** the command returns successfully with structured JSON containing the matched URL and the condition that was satisfied

**Example**:
- Given: Chrome connected, page at `https://example.com/login`
- When: `agentchrome page wait --url "*/dashboard*"` and navigation occurs
- Then: JSON output `{"condition": "url", "matched": true, "url": "https://example.com/dashboard", "title": "Dashboard"}`

### AC2: Wait for text to appear on page

**Given** a connected Chrome session on a page that is loading content asynchronously
**When** I run `agentchrome page wait --text "Products"` and the text "Products" appears in the page content
**Then** the command returns successfully with structured JSON indicating the text was found

**Example**:
- Given: Chrome connected, page loading async content
- When: `agentchrome page wait --text "Products"`
- Then: JSON output `{"condition": "text", "matched": true, "text": "Products", "url": "https://example.com/products", "title": "Product Listing"}`

### AC3: Wait for network idle

**Given** a connected Chrome session on a page with active network requests
**When** I run `agentchrome page wait --network-idle`
**Then** the command returns once there have been no active network requests for 500ms, with structured JSON confirming network idle state

**Example**:
- Given: Chrome connected, page has active XHR requests
- When: `agentchrome page wait --network-idle`
- Then: JSON output `{"condition": "network-idle", "matched": true, "url": "https://example.com/data", "title": "Data Page"}`

### AC4: Wait for CSS selector to match an element in the DOM

**Given** a connected Chrome session on a page where an element does not yet exist
**When** I run `agentchrome page wait --selector "#results-table"` and the element appears in the DOM
**Then** the command returns successfully with structured JSON indicating the selector matched

**Example**:
- Given: Chrome connected, `#results-table` not yet in DOM
- When: `agentchrome page wait --selector "#results-table"` and JS creates the element
- Then: JSON output `{"condition": "selector", "matched": true, "selector": "#results-table", "url": "https://example.com/search", "title": "Search"}`

### AC5: Wait times out with descriptive error

**Given** a connected Chrome session
**When** I run `agentchrome page wait --text "never-appearing-text" --timeout 3000` and the text never appears within 3 seconds
**Then** the command exits with timeout error (exit code 4) and a structured JSON error on stderr indicating the condition was not met and the timeout duration

**Example**:
- Given: Chrome connected
- When: `agentchrome page wait --text "never-appearing-text" --timeout 3000`
- Then: Exit code 4, stderr: `{"error": "Wait timed out after 3000ms: text \"never-appearing-text\" not found", "code": 4}`

### AC6: Network idle returns immediately when network is already idle

**Given** a connected Chrome session on a fully loaded page with no active network requests
**When** I run `agentchrome page wait --network-idle`
**Then** the command returns within the idle detection window (500ms) without waiting for the full timeout, confirming the network was already idle

### AC7: Condition already satisfied returns immediately

**Given** a connected Chrome session on a page where the URL already matches `*/dashboard*`
**When** I run `agentchrome page wait --url "*/dashboard*"`
**Then** the command checks the condition immediately on startup and returns without further polling, because the condition is already met

### AC8: Exactly one condition must be specified

**Given** a terminal with agentchrome available
**When** I run `agentchrome page wait` with no condition flags (no `--url`, `--text`, `--selector`, or `--network-idle`)
**Then** the command exits with a validation error (exit code 1) indicating that exactly one condition is required
**And** the error is structured JSON on stderr matching the project error contract

### Generated Gherkin Preview

```gherkin
Feature: Page Wait Command
  As an AI agent automating multi-step browser workflows
  I want a standalone command to wait until a specified condition is met
  So that I can reliably synchronize with page state changes

  Scenario: Wait for URL to match a glob pattern
    Given a connected Chrome session at "https://example.com/login"
    When I run page wait with --url "*/dashboard*"
    And the URL changes to "https://example.com/dashboard"
    Then the command succeeds with JSON containing "condition" "url" and "matched" true

  Scenario: Wait for text to appear on page
    Given a connected Chrome session with async loading content
    When I run page wait with --text "Products"
    And the text "Products" appears on the page
    Then the command succeeds with JSON containing "condition" "text" and "matched" true

  Scenario: Wait for network idle
    Given a connected Chrome session with active network requests
    When I run page wait with --network-idle
    And network requests complete and remain idle for 500ms
    Then the command succeeds with JSON containing "condition" "network-idle" and "matched" true

  Scenario: Wait for CSS selector to match
    Given a connected Chrome session where "#results-table" does not exist
    When I run page wait with --selector "#results-table"
    And the element "#results-table" appears in the DOM
    Then the command succeeds with JSON containing "condition" "selector" and "matched" true

  Scenario: Wait times out
    Given a connected Chrome session
    When I run page wait with --text "never-appearing-text" --timeout 3000
    And the text never appears within 3000ms
    Then the command fails with exit code 4
    And stderr contains a timeout error mentioning "never-appearing-text"

  Scenario: Network idle returns immediately when already idle
    Given a connected Chrome session on a fully loaded page
    When I run page wait with --network-idle
    Then the command returns within 1000ms

  Scenario: Condition already satisfied returns immediately
    Given a connected Chrome session at "https://example.com/dashboard"
    When I run page wait with --url "*/dashboard*"
    Then the command returns immediately with the matched URL

  Scenario: No condition specified
    Given a terminal with agentchrome available
    When I run page wait with no condition flags
    Then the command fails with exit code 1
    And stderr contains a validation error about missing condition
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Add `Wait` variant to `PageCommand` enum with `PageWaitArgs` struct | Must | Follows existing page subcommand pattern |
| FR2 | Add `--url <glob>` condition flag for URL glob pattern matching | Must | Use glob matching (e.g., `*/dashboard*`) |
| FR3 | Add `--text <string>` condition flag for page text content matching | Must | Poll via `Runtime.evaluate` with `document.body.innerText.includes(text)` |
| FR4 | Add `--network-idle` boolean flag reusing existing `wait_for_network_idle()` infrastructure | Must | Same 500ms idle threshold as navigate |
| FR5 | Add `--selector <css>` condition flag for DOM element existence | Should | Poll via `Runtime.evaluate` with `document.querySelector(selector) !== null` |
| FR6 | Respect `--timeout` global option for maximum wait duration (default: 30000ms) | Must | Matching navigate timeout default |
| FR7 | Return structured JSON output with condition type, match status, current URL, and title | Must | Follow project JSON output contract on stdout |
| FR8 | Exit with code 4 (TimeoutError) when condition is not met within timeout | Must | Reuse existing `AppError` timeout pattern |
| FR9 | Check condition immediately before entering poll loop; return instantly if already satisfied | Must | Avoids unnecessary waiting when condition is pre-met |
| FR10 | Require exactly one condition flag via clap argument group validation | Must | Error output must be structured JSON per project error contract |
| FR11 | Add `--interval <ms>` option for configurable poll interval (default: 100ms) | Could | Applies to --url, --text, --selector polling; --network-idle is event-driven |
| FR12 | Output structured JSON error on stderr for all error conditions | Must | Single JSON error object per invocation |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Polling conditions (--url, --text, --selector) must use configurable interval (default 100ms); --network-idle is event-driven with no polling overhead |
| **Reliability** | Each polling probe must complete within the CDP command timeout; a stuck probe must not consume the entire wait timeout |
| **Platforms** | macOS, Linux, Windows (same as all agentchrome commands) |
| **Output contract** | JSON on stdout for success, JSON on stderr for errors, exit codes per project convention |

---

## UI/UX Requirements

| Element | Requirement |
|---------|-------------|
| **CLI help** | `agentchrome page wait --help` displays all condition flags with descriptions and defaults |
| **Error messages** | Timeout errors include the condition type and value that was being waited for |
| **Flag naming** | `--url`, `--text`, `--selector`, `--network-idle`, `--interval` — verified no collision with global flags (`--timeout`, `--port`, `--host`, `--config`, `--pretty`, `--no-color`) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--url` | String (glob pattern) | Non-empty string; valid glob syntax | One of --url/--text/--selector/--network-idle required |
| `--text` | String | Non-empty string | One of --url/--text/--selector/--network-idle required |
| `--selector` | String (CSS selector) | Non-empty string | One of --url/--text/--selector/--network-idle required |
| `--network-idle` | Boolean flag | Presence-based | One of --url/--text/--selector/--network-idle required |
| `--interval` | u64 (milliseconds) | > 0 | No (default: 100) |
| `--timeout` | u64 (milliseconds) | > 0 (global option) | No (default: 30000) |

### Output Data (Success — stdout)

| Field | Type | Description |
|-------|------|-------------|
| `condition` | String | The condition type: `"url"`, `"text"`, `"selector"`, or `"network-idle"` |
| `matched` | Boolean | Always `true` on success |
| `url` | String | Current page URL at time of match |
| `title` | String | Current page title at time of match |
| `pattern` | String or null | The glob pattern (present for --url, `null` otherwise) |
| `text` | String or null | The search text (present for --text, `null` otherwise) |
| `selector` | String or null | The CSS selector (present for --selector, `null` otherwise) |

### Output Data (Timeout Error — stderr)

| Field | Type | Description |
|-------|------|-------------|
| `error` | String | Descriptive message including condition type, value, and timeout duration |
| `code` | Integer | Exit code 4 (TimeoutError) |

---

## Dependencies

### Internal Dependencies
- [x] `wait_for_network_idle()` helper in `src/navigate.rs` — reuse for `--network-idle`
- [x] `setup_session()` / `cdp_config()` in `src/page/mod.rs` — reuse session setup pattern
- [x] `get_page_info()` in `src/page/mod.rs` — reuse for URL/title in output
- [x] `AppError` / `ExitCode` in `src/error.rs` — reuse error types
- [x] `print_output()` in `src/page/mod.rs` — reuse JSON output helper

### External Dependencies
- [ ] `glob` or `globset` crate for URL pattern matching (not currently a dependency — must be added)

### Blocked By
- None — all infrastructure exists

---

## Out of Scope

- Combining multiple conditions with AND/OR logic (follow-up)
- Waiting for element visibility or interactability beyond DOM presence
- Replacing `--wait-until` on `navigate` or `interact click`
- Custom JavaScript condition expressions (e.g., `--js "expression"`)
- Waiting for specific network request URL patterns

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Command latency overhead | < 10ms beyond actual wait time | Time between condition match and command exit |
| Poll efficiency | ≤ 10 CDP calls/second at default interval | Count `Runtime.evaluate` calls per second during polling |

---

## Open Questions

- [x] Use glob for URL matching? → Yes, per issue recommendation
- [ ] Should `--network-idle` reuse `wait_for_network_idle()` directly or adapt it into a shared utility? (Recommend reuse with minor refactoring if needed to decouple from navigate-specific setup)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #163 | 2026-03-11 | Initial feature spec |

---

## Validation Checklist

Before moving to PLAN phase:

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified (AC5: timeout, AC6: already idle, AC7: already satisfied, AC8: no condition)
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented
