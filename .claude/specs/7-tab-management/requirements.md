# Requirements: Tab Management Commands

**Issue**: #7
**Date**: 2026-02-11
**Status**: Draft
**Author**: Claude (spec-driven development)

---

## User Story

**As a** developer or automation engineer
**I want** to manage browser tabs from the command line
**So that** I can script tab creation, switching, and cleanup without manual browser interaction

---

## Background

The `tabs` subcommand group provides the foundational tab management operations that most CLI automation workflows require. The MCP server already exposes equivalent functionality via `list_pages`, `new_page`, `close_page`, and `select_page` tools. This feature reimplements those as ergonomic CLI subcommands: `tabs list`, `tabs create`, `tabs close`, and `tabs activate`.

The existing codebase already provides the building blocks: `query_targets()` returns target info via the HTTP API, `CdpClient` can send browser-level CDP commands like `Target.createTarget` and `Target.closeTarget`, and `resolve_connection()` handles the connection priority chain.

---

## Acceptance Criteria

### AC1: List all open tabs

**Given** Chrome is running with CDP enabled and has open tabs
**When** I run `chrome-cli tabs list`
**Then** stdout contains a JSON array of tab objects with `id`, `url`, `title`, and `active` fields
**And** the exit code is 0

**Example**:
- Given: Chrome has two tabs open (Google, GitHub)
- When: `chrome-cli tabs list`
- Then: `[{"id":"ABC123","url":"https://google.com","title":"Google","active":true},{"id":"DEF456","url":"https://github.com","title":"GitHub","active":false}]`

### AC2: List filters internal pages by default

**Given** Chrome is running with tabs including `chrome://extensions/` and `chrome://newtab/`
**When** I run `chrome-cli tabs list`
**Then** tabs with `chrome://` URLs are excluded from the output (except `chrome://newtab/`)
**And** tabs with `chrome-extension://` URLs are excluded from the output

### AC3: List all pages including internal ones

**Given** Chrome is running with tabs including `chrome://extensions/`
**When** I run `chrome-cli tabs list --all`
**Then** all targets of type "page" are included, including `chrome://` and `chrome-extension://` URLs

### AC4: Create a new tab with URL

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli tabs create https://example.com`
**Then** a new tab opens and navigates to `https://example.com`
**And** stdout contains a JSON object with `id`, `url`, and `title` fields
**And** the exit code is 0

### AC5: Create a blank tab

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli tabs create`
**Then** a new blank tab opens
**And** stdout contains a JSON object with `id`, `url`, and `title` fields

### AC6: Create a tab in the background

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli tabs create --background https://example.com`
**Then** a new tab opens navigating to the URL
**And** the previously active tab remains focused

### AC7: Close a tab by ID

**Given** Chrome has multiple tabs open and tab "ABC123" exists
**When** I run `chrome-cli tabs close ABC123`
**Then** the tab with ID "ABC123" is closed
**And** stdout contains a JSON object with `closed` and `remaining` fields
**And** the exit code is 0

### AC8: Close a tab by index

**Given** Chrome has at least 3 tabs open
**When** I run `chrome-cli tabs close 1`
**Then** the tab at index 1 (0-based) is closed
**And** stdout contains a JSON object with `closed` and `remaining` fields

### AC9: Close multiple tabs

**Given** Chrome has tabs "ABC123", "DEF456", and "GHI789" open
**When** I run `chrome-cli tabs close ABC123 DEF456`
**Then** both tabs are closed
**And** stdout contains a JSON object with `closed` (array of closed IDs) and `remaining` count

### AC10: Prevent closing the last tab

**Given** Chrome has exactly one tab open
**When** I run `chrome-cli tabs close <tab_id>`
**Then** the command fails with an error message indicating the last tab cannot be closed
**And** the exit code is non-zero

### AC11: Activate a tab by ID

**Given** Chrome has multiple tabs and tab "DEF456" is not active
**When** I run `chrome-cli tabs activate DEF456`
**Then** tab "DEF456" becomes the foreground tab
**And** stdout contains a JSON object with `activated`, `url`, and `title` fields
**And** the exit code is 0

### AC12: Activate a tab by index

**Given** Chrome has at least 3 tabs open
**When** I run `chrome-cli tabs activate 2`
**Then** the tab at index 2 (0-based) becomes the foreground tab
**And** stdout contains a JSON object with `activated`, `url`, and `title` fields

### AC13: Tab not found error

**Given** Chrome is running
**When** I run `chrome-cli tabs close nonexistent` or `chrome-cli tabs activate nonexistent`
**Then** the command fails with a "tab not found" error
**And** the exit code is 3 (target error)

### AC14: No Chrome connection error

**Given** no Chrome instance is running or reachable
**When** I run any `chrome-cli tabs` subcommand
**Then** the command fails with a connection error
**And** the error message suggests running `chrome-cli connect`
**And** the exit code is 2 (connection error)

### AC15: Plain text output for list

**Given** Chrome has tabs open
**When** I run `chrome-cli tabs list --plain`
**Then** stdout contains a human-readable table of tabs (index, title, URL, active indicator)

### AC16: Tab IDs use CDP target IDs

**Given** Chrome is running with tabs open
**When** I run `chrome-cli tabs list` multiple times within the same browser session
**Then** the `id` field for each tab is the CDP target ID and remains consistent

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `tabs list` returns JSON array of tab info | Must | Uses HTTP `/json/list` or `Target.getTargets` |
| FR2 | `tabs create [URL]` creates a new tab | Must | Uses `Target.createTarget` CDP command |
| FR3 | `tabs close <ID>...` closes one or more tabs | Must | Uses `Target.closeTarget` CDP command |
| FR4 | `tabs activate <ID>` brings tab to foreground | Must | Uses `Target.activateTarget` CDP command |
| FR5 | Default filtering of internal Chrome pages | Must | Filter `chrome://` (except newtab) and `chrome-extension://` |
| FR6 | `--all` flag on `tabs list` | Must | Include all targets of type "page" |
| FR7 | `--background` flag on `tabs create` | Should | Create without activating |
| FR8 | `--timeout <MS>` flag on `tabs create` | Should | Wait for page load |
| FR9 | `--quiet` flag on `tabs activate` | Could | Suppress output |
| FR10 | Multiple tab close support | Must | Accept multiple IDs/indices |
| FR11 | Last-tab protection on close | Must | Prevent closing the only remaining tab |
| FR12 | Tab ID by index or target ID | Must | Reuse existing `select_target` logic |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | All tab commands complete within 2 seconds on local Chrome |
| **Security** | CDP connections only to localhost by default; warn on remote hosts |
| **Reliability** | Graceful error messages when Chrome is unreachable |
| **Platforms** | macOS, Linux, and Windows (cross-platform Rust) |
| **Output** | JSON to stdout, errors to stderr; support `--json`, `--pretty`, `--plain` global flags |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| URL (create) | String | Valid URL format | No (defaults to blank tab) |
| Tab ID (close, activate) | String | Must be valid target ID or numeric index | Yes |
| --all | Boolean flag | N/A | No |
| --background | Boolean flag | N/A | No |
| --timeout | u64 (milliseconds) | Positive integer | No |

### Output Data — `tabs list`

| Field | Type | Description |
|-------|------|-------------|
| id | String | CDP target ID |
| url | String | Current page URL |
| title | String | Page title |
| active | Boolean | Whether this tab is the focused tab |

### Output Data — `tabs create`

| Field | Type | Description |
|-------|------|-------------|
| id | String | CDP target ID of new tab |
| url | String | URL the tab navigated to |
| title | String | Page title |

### Output Data — `tabs close`

| Field | Type | Description |
|-------|------|-------------|
| closed | String or String[] | Target ID(s) that were closed |
| remaining | u32 | Number of tabs remaining |

### Output Data — `tabs activate`

| Field | Type | Description |
|-------|------|-------------|
| activated | String | Target ID of the activated tab |
| url | String | Tab's URL |
| title | String | Tab's title |

---

## Dependencies

### Internal Dependencies
- [x] Issue #4 — CDP WebSocket client (merged)
- [x] Issue #6 — Session/connection management (merged)

### External Dependencies
- Chrome/Chromium browser with `--remote-debugging-port` enabled

---

## Out of Scope

- Tab reordering or moving between windows
- Tab pinning
- Tab groups / group management
- Tab history inspection
- Tab screenshots (covered by separate `page` command)
- Incognito/private window management

---

## Open Questions

- [x] Use HTTP API (`/json/list`) or CDP command (`Target.getTargets`) for listing? → Use HTTP API for simplicity and consistency with existing `query_targets()`
- [x] How to determine the "active" tab? → The first target returned by `/json/list` is typically the active one; confirm via CDP `Target.getTargets` which includes an `attached` field

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Edge cases and error states specified (AC10, AC13, AC14)
- [x] Dependencies identified (all resolved)
- [x] Out of scope is defined
