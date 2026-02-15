# Requirements: URL Navigation

**Issue**: #8
**Date**: 2026-02-11
**Status**: Draft
**Author**: Claude (spec-driven development)

---

## User Story

**As a** developer or automation engineer
**I want** to navigate Chrome to URLs, reload pages, and traverse browser history from the command line
**So that** I can script browser navigation workflows without manual interaction

---

## Background

The `navigate` subcommand group implements URL navigation and browser history traversal for chrome-cli. This maps to the MCP server's `navigate_page` tool, which supports URL navigation, back/forward/reload, and configurable wait strategies. The navigate feature is a core MVP capability — it enables the fundamental "go to a URL and wait for it to load" workflow that underpins all other page inspection and interaction commands.

Navigation requires session-level CDP communication (attached to a specific tab target) because the Page and Network domains are per-target, unlike the browser-level Target domain commands used by `tabs`. This introduces the first use of `CdpSession`, `ManagedSession`, and event subscriptions in the command layer.

---

## Acceptance Criteria

### AC1: Navigate to a valid URL

**Given** Chrome is running with CDP enabled and a tab is open
**When** I run `chrome-cli navigate https://example.com`
**Then** the active tab navigates to `https://example.com`
**And** the command waits for the `load` event (default wait strategy)
**And** the output is JSON: `{"url": "https://example.com/", "title": "Example Domain", "status": 200}`
**And** the exit code is 0

### AC2: Navigate with --tab to target a specific tab

**Given** Chrome has multiple tabs open
**When** I run `chrome-cli navigate https://example.com --tab <ID>`
**Then** the specified tab navigates to the URL
**And** other tabs are unaffected

### AC3: Navigate with --wait-until load (default)

**Given** Chrome is running with a tab open
**When** I run `chrome-cli navigate https://example.com --wait-until load`
**Then** the command waits for `Page.loadEventFired` before returning
**And** the JSON output includes the final URL, title, and HTTP status

### AC4: Navigate with --wait-until domcontentloaded

**Given** Chrome is running with a tab open
**When** I run `chrome-cli navigate https://example.com --wait-until domcontentloaded`
**Then** the command waits for `Page.domContentEventFired` before returning
**And** the JSON output includes the final URL, title, and HTTP status

### AC5: Navigate with --wait-until networkidle

**Given** Chrome is running with a tab open
**When** I run `chrome-cli navigate https://example.com --wait-until networkidle`
**Then** the command waits until there are 0 in-flight network requests for 500ms
**And** the JSON output includes the final URL, title, and HTTP status

### AC6: Navigate with --wait-until none

**Given** Chrome is running with a tab open
**When** I run `chrome-cli navigate https://example.com --wait-until none`
**Then** the command returns immediately after initiating navigation
**And** the JSON output includes the URL that was navigated to

### AC7: Navigate with --timeout

**Given** Chrome is running with a tab open
**When** I run `chrome-cli navigate https://example.com --timeout 5000`
**Then** the navigation timeout is set to 5000ms
**And** if the wait strategy does not complete within 5000ms, the command fails with exit code 4

### AC8: Navigate with --ignore-cache

**Given** Chrome is running with a tab open
**When** I run `chrome-cli navigate https://example.com --ignore-cache`
**Then** the navigation bypasses the browser cache
**And** the page is fetched fresh from the server

### AC9: Navigate back in browser history

**Given** Chrome has navigated to at least two pages in the active tab
**When** I run `chrome-cli navigate back`
**Then** the tab navigates back one step in history
**And** the output is JSON with the new URL and title
**And** the exit code is 0

### AC10: Navigate back with --tab

**Given** Chrome has multiple tabs, and a specific tab has history
**When** I run `chrome-cli navigate back --tab <ID>`
**Then** the specified tab navigates back in its history

### AC11: Navigate forward in browser history

**Given** Chrome has navigated back from a page
**When** I run `chrome-cli navigate forward`
**Then** the tab navigates forward one step in history
**And** the output is JSON with the new URL and title
**And** the exit code is 0

### AC12: Navigate forward with --tab

**Given** Chrome has multiple tabs, and a specific tab has forward history
**When** I run `chrome-cli navigate forward --tab <ID>`
**Then** the specified tab navigates forward in its history

### AC13: Reload the current page

**Given** Chrome has a page loaded in the active tab
**When** I run `chrome-cli navigate reload`
**Then** the page reloads
**And** the output is JSON with the URL and title
**And** the exit code is 0

### AC14: Reload with --ignore-cache

**Given** Chrome has a page loaded in the active tab
**When** I run `chrome-cli navigate reload --ignore-cache`
**Then** the page performs a hard reload, bypassing the cache

### AC15: Reload with --tab

**Given** Chrome has multiple tabs with pages loaded
**When** I run `chrome-cli navigate reload --tab <ID>`
**Then** the specified tab reloads

### AC16: DNS resolution failure

**Given** Chrome is running with a tab open
**When** I run `chrome-cli navigate https://this-domain-does-not-exist.invalid`
**Then** the command fails with a meaningful error message mentioning DNS resolution
**And** the exit code is non-zero

### AC17: Navigation timeout

**Given** Chrome is running with a tab open
**When** I run `chrome-cli navigate https://httpbin.org/delay/60 --timeout 1000`
**Then** the command fails with a timeout error message
**And** the exit code is 4

### AC18: No Chrome connection

**Given** no Chrome instance is running or connected
**When** I run `chrome-cli navigate https://example.com`
**Then** the command fails with exit code 2
**And** the error message suggests running `chrome-cli connect`

### Generated Gherkin Preview

```gherkin
Feature: URL Navigation
  As a developer or automation engineer
  I want to navigate Chrome to URLs and traverse browser history
  So that I can script browser navigation workflows

  Background:
    Given Chrome is running with CDP enabled

  Scenario: Navigate to a valid URL
    Given a tab is open
    When I run "chrome-cli navigate https://example.com"
    Then the output JSON has key "url" with value "https://example.com/"
    And the output JSON has key "title"
    And the output JSON has key "status" with a numeric value
    And the exit code is 0

  Scenario: Navigate with --tab targets specific tab
    Given multiple tabs are open
    When I run "chrome-cli navigate https://example.com --tab <ID>"
    Then the specified tab URL changes to "https://example.com/"

  Scenario: Wait until load (default)
    ...

  Scenario: Wait until domcontentloaded
    ...

  # ... remaining scenarios
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `navigate <URL>` navigates the active or specified tab to a URL | Must | Core navigation |
| FR2 | `--wait-until` supports `load`, `domcontentloaded`, `networkidle`, `none` | Must | Default: `load` |
| FR3 | `--timeout` sets navigation timeout in milliseconds | Must | Default: 30000ms |
| FR4 | `--ignore-cache` bypasses browser cache during navigation | Must | |
| FR5 | `--tab <ID>` targets a specific tab by ID or index | Must | Uses existing `select_target()` |
| FR6 | `navigate back` goes back one step in browser history | Must | Via `Page.navigateToHistoryEntry` or JS `history.back()` |
| FR7 | `navigate forward` goes forward one step in browser history | Must | Via `Page.navigateToHistoryEntry` or JS `history.forward()` |
| FR8 | `navigate reload` reloads the current page | Must | Via `Page.reload` |
| FR9 | `reload --ignore-cache` performs a hard reload | Must | `Page.reload` with `ignoreCache: true` |
| FR10 | JSON output includes `url`, `title`, and `status` (for URL navigation) | Must | Status from `Page.frameNavigated` or navigate response |
| FR11 | JSON output includes `url` and `title` for back/forward/reload | Must | |
| FR12 | Network idle detection: 0 in-flight requests for 500ms | Should | Track via Network.requestWillBeSent / loadingFinished / loadingFailed |
| FR13 | Meaningful error messages for DNS failures | Should | Parse `Page.navigate` errorText |
| FR14 | Meaningful error messages for SSL errors | Should | Parse `Page.navigate` errorText |
| FR15 | Global `--tab` flag also works for navigate commands | Must | Consistent with tabs command |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Navigation commands should add minimal overhead beyond Chrome's own load time |
| **Reliability** | Wait strategies must handle edge cases (pages that never fire load, infinite redirects) via timeout |
| **Platforms** | macOS, Linux, Windows — all via CDP, no platform-specific code needed |
| **Error handling** | All CDP errors converted to AppError with appropriate exit codes |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| url | String | Must be a URL (Chrome validates) | Yes (for `navigate <URL>`) |
| --tab | String | Tab ID or numeric index | No (defaults to active tab) |
| --wait-until | Enum | One of: load, domcontentloaded, networkidle, none | No (default: load) |
| --timeout | u64 | Positive integer, milliseconds | No (default: 30000) |
| --ignore-cache | bool | Flag | No (default: false) |

### Output Data

#### URL Navigation (`navigate <URL>`)

| Field | Type | Description |
|-------|------|-------------|
| url | String | The final URL after navigation (may differ from input due to redirects) |
| title | String | Page title after navigation |
| status | u16 | HTTP response status code |

#### History Navigation (`navigate back`, `navigate forward`, `navigate reload`)

| Field | Type | Description |
|-------|------|-------------|
| url | String | The URL after navigation |
| title | String | Page title after navigation |

---

## Dependencies

### Internal Dependencies
- [x] CDP client (Issue #4) — CdpClient, CdpSession, event subscriptions
- [x] Session/connection management (Issue #6) — resolve_connection, ManagedSession, select_target
- [x] Tab management (Issue #7) — establishes command module pattern

### External Dependencies
- [x] Chrome DevTools Protocol — Page domain, Network domain

### Blocked By
- [x] Issue #4 (CDP client) — Completed
- [x] Issue #6 (Session management) — Completed

---

## Out of Scope

- **Script injection before/after navigation** — will be handled by the `js` command
- **Network request interception** — will be handled by the `network` command
- **Unload handling / beforeunload dialogs** — deferred to a later issue
- **Multiple URL navigation in sequence** — users can call the command multiple times
- **Custom referrer or extra headers** — deferred to a later issue
- **Navigation to `javascript:` URLs** — not supported for security reasons

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| All 18 acceptance criteria pass | 100% | BDD test suite |
| Navigation overhead | < 100ms above Chrome's own load time | Benchmark against raw CDP |
| Error message clarity | Users understand what went wrong | Error messages include context and suggestions |

---

## Open Questions

- [x] Should `navigate back` wait for page load? — Yes, default wait strategy applies
- [x] How to get HTTP status code? — From `Page.navigate` response `errorText` (absence = success) and `Network.responseReceived` for status code
- [x] Should `--wait-until` apply to back/forward/reload? — Not as a flag (they use default load wait), but the implementation should wait for the page to settle

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
