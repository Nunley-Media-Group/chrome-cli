# Requirements: Session and Connection Management

**Issue**: #6
**Date**: 2026-02-11
**Status**: Draft
**Author**: Claude (spec-driven)

---

## User Story

**As a** developer or automation engineer using chrome-cli
**I want** commands to automatically find and reuse a Chrome connection, manage CDP sessions per tab, and persist connection state across CLI invocations
**So that** I don't have to pass connection details to every command, and I can efficiently target specific tabs

---

## Background

Each chrome-cli invocation needs a connection to Chrome. Today, `chrome-cli connect` discovers or launches Chrome and outputs connection info, but that info is not persisted — the next command would need to rediscover Chrome from scratch. This is inefficient and fragile.

This feature adds a lightweight session file (similar to `docker context` or `kubectl config`) so that after a successful `connect` or `--launch`, subsequent commands can automatically reconnect. It also adds CDP session management for targeting specific tabs, lazy domain enabling, and robust connection health checks before executing commands.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Write session file after successful connect

**Given** Chrome is running with remote debugging enabled
**When** I run `chrome-cli connect`
**Then** the tool connects to Chrome and writes a session file to `~/.chrome-cli/session.json`
**And** the session file contains `{"ws_url": "ws://...", "port": N, "pid": null, "timestamp": "..."}`
**And** the exit code is 0

**Example**:
- Given: Chrome running on port 9222
- When: `chrome-cli connect --port 9222`
- Then: `~/.chrome-cli/session.json` contains `{"ws_url":"ws://127.0.0.1:9222/devtools/browser/abc","port":9222,"pid":null,"timestamp":"2026-02-11T12:00:00Z"}`

### AC2: Write session file after successful launch

**Given** Chrome is installed on the system
**When** I run `chrome-cli connect --launch`
**Then** the tool launches Chrome and writes a session file to `~/.chrome-cli/session.json`
**And** the session file contains the WebSocket URL, port, Chrome PID, and timestamp
**And** the exit code is 0

### AC3: Subsequent commands auto-read session file

**Given** a valid session file exists at `~/.chrome-cli/session.json`
**And** no explicit `--ws-url` or `--port` flags are provided
**When** I run a command that needs Chrome (e.g., `chrome-cli tabs list`)
**Then** the tool reads the session file and connects using the stored WebSocket URL
**And** the command executes successfully

### AC4: Explicit flags override session file

**Given** a valid session file exists at `~/.chrome-cli/session.json` pointing to port 9222
**And** Chrome is also running on port 9333
**When** I run `chrome-cli tabs list --port 9333`
**Then** the tool connects to port 9333, ignoring the session file
**And** the command executes successfully

### AC5: Show connection status

**Given** a valid session file exists
**When** I run `chrome-cli connect --status`
**Then** the tool reads the session file and displays connection info
**And** indicates whether Chrome is still reachable at the stored address
**And** the exit code is 0

### AC6: Show connection status when no session exists

**Given** no session file exists
**When** I run `chrome-cli connect --status`
**Then** the tool reports that no active session was found
**And** the exit code is non-zero

### AC7: Disconnect removes session file

**Given** a valid session file exists
**When** I run `chrome-cli connect --disconnect`
**Then** the session file is removed
**And** the exit code is 0

### AC8: Disconnect kills launched Chrome process

**Given** a session file exists with a PID (from a `--launch`)
**When** I run `chrome-cli connect --disconnect`
**Then** the session file is removed
**And** the Chrome process identified by the PID is sent SIGTERM (or equivalent)
**And** if the process doesn't exist, it is silently ignored

### AC9: Per-command connection resolution chain

**Given** no explicit flags are provided and no session file exists
**When** I run a command that needs Chrome
**Then** the tool follows this resolution chain:
  1. Check for `--ws-url` flag (not present)
  2. Check for `--port` flag (not present)
  3. Check for session file (not present)
  4. Attempt auto-discovery (try port 9222)
  5. Return clear error if no Chrome found
**And** the error message suggests using `chrome-cli connect` or `chrome-cli connect --launch`

### AC10: Connection health check before command execution

**Given** a session file exists with a stored WebSocket URL
**When** a command starts execution
**Then** the tool first performs a fast health check (HTTP GET to `/json/version`)
**And** if the check passes, proceeds with the command
**And** if the check fails, reports a stale session error and suggests reconnecting

### AC11: Stale session file detection

**Given** a session file exists but Chrome is no longer running at the stored address
**When** I run a command that needs Chrome
**Then** the tool detects the stale session
**And** outputs an error indicating the session is stale
**And** suggests running `chrome-cli connect` to establish a new connection

### AC12: Create CDP session for a tab

**Given** a connected Chrome instance with multiple open tabs
**When** a command targets a specific tab
**Then** the tool creates a CDP session for that tab's target ID
**And** the command operates within the tab's session context

### AC13: Lazy CDP domain enabling

**Given** a CDP session is attached to a tab
**When** a command requires specific CDP domains (e.g., `Page`, `Runtime`, `DOM`)
**Then** only the required domains are enabled (e.g., `Page.enable`, `Runtime.enable`)
**And** domains not needed by the command are not enabled

### AC14: CDP session cleanup

**Given** a CDP session is attached to a tab
**When** the command finishes execution
**Then** the CDP session is properly detached/closed
**And** no dangling sessions remain on the Chrome side

### AC15: Target tab by CDP target ID

**Given** a connected Chrome instance with tabs
**When** I run `chrome-cli tabs list` and get target IDs
**And** I run a command with `--tab <target-id>`
**Then** the command targets the tab with that CDP target ID

### AC16: Target tab by numeric index

**Given** a connected Chrome instance with tabs
**When** I run a command with `--tab 0`
**Then** the tool fetches the target list and selects the target at index 0
**And** the command targets that tab

### AC17: Default tab targeting (active tab)

**Given** a connected Chrome instance with tabs
**When** I run a command without `--tab`
**Then** the tool targets the first "page" type target in the target list
**And** this approximates the "active" tab behavior

### AC18: Invalid tab ID error

**Given** a connected Chrome instance
**When** I run a command with `--tab nonexistent-id`
**Then** the tool reports that the specified tab was not found
**And** suggests using `chrome-cli tabs list` to see available tabs
**And** the exit code is non-zero

### AC19: Connection health check is fast

**Given** Chrome is running and reachable
**When** a health check is performed before a command
**Then** the health check completes in under 100ms for local connections

### AC20: Graceful handling when Chrome dies mid-session

**Given** a CDP session is active and Chrome crashes
**When** a command is in progress
**Then** the tool reports a connection-lost error
**And** does not panic or hang indefinitely

### Generated Gherkin Preview

```gherkin
Feature: Session and connection management
  As a developer or automation engineer
  I want commands to automatically find and reuse a Chrome connection
  So that I don't have to pass connection details to every command

  # --- Session File Management ---

  Scenario: Write session file after connect
    Given a Chrome instance is running with remote debugging
    When I run "chrome-cli connect"
    Then a session file should exist at "~/.chrome-cli/session.json"
    And the session file should contain "ws_url" and "port" fields
    And the exit code should be 0

  Scenario: Write session file after launch
    Given Chrome is installed on the system
    When I run "chrome-cli connect --launch"
    Then a session file should exist with a "pid" field
    And the exit code should be 0

  Scenario: Auto-read session file for subsequent commands
    Given a valid session file exists
    When I run a command that needs Chrome without explicit flags
    Then the command connects using the session file

  Scenario: Explicit flags override session file
    Given a valid session file exists pointing to port 9222
    When I run a command with "--port 9333"
    Then the command connects to port 9333

  # --- Status and Disconnect ---

  Scenario: Show connection status
    Given a valid session file exists
    When I run "chrome-cli connect --status"
    Then the output shows connection info and reachability

  Scenario: No active session status
    Given no session file exists
    When I run "chrome-cli connect --status"
    Then the output indicates no active session

  Scenario: Disconnect removes session
    Given a valid session file exists
    When I run "chrome-cli connect --disconnect"
    Then the session file is removed

  Scenario: Disconnect kills launched process
    Given a session file exists with a PID
    When I run "chrome-cli connect --disconnect"
    Then the Chrome process is terminated

  # --- Connection Resolution ---

  Scenario: Connection resolution chain with no Chrome found
    Given no flags, no session file, and no Chrome running
    When I run a command that needs Chrome
    Then the error suggests using "chrome-cli connect"

  Scenario: Health check before command execution
    Given a valid session file exists
    When a command starts execution
    Then a fast health check is performed first

  Scenario: Stale session detection
    Given a session file exists but Chrome is not running
    When I run a command that needs Chrome
    Then the error indicates the session is stale

  # --- Tab Targeting ---

  Scenario: Target tab by CDP target ID
    Given a connected Chrome instance with tabs
    When I run a command with "--tab <target-id>"
    Then the command targets the specified tab

  Scenario: Target tab by numeric index
    Given a connected Chrome instance with tabs
    When I run a command with "--tab 0"
    Then the command targets the first tab

  Scenario: Default tab targeting
    Given a connected Chrome instance with tabs
    When I run a command without "--tab"
    Then the command targets the first page-type tab

  Scenario: Invalid tab ID error
    Given a connected Chrome instance
    When I run a command with "--tab nonexistent"
    Then the error suggests using "tabs list"

  # --- CDP Session Management ---

  Scenario: Create and cleanup CDP session
    Given a connected Chrome instance
    When a command targets a tab
    Then a CDP session is created and cleaned up after

  Scenario: Lazy domain enabling
    Given a CDP session is attached to a tab
    When a command needs only the Page domain
    Then only Page.enable is called

  # --- Error Handling ---

  Scenario: Chrome dies mid-session
    Given a CDP session is active
    When Chrome crashes
    Then the tool reports a connection-lost error without panicking
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Write session file to `~/.chrome-cli/session.json` after successful connect/launch | Must | Contains ws_url, port, pid, timestamp |
| FR2 | Read session file as fallback when no explicit flags given | Must | Auto-reconnect pattern |
| FR3 | `connect --status` shows current session info and reachability | Must | Status inspection |
| FR4 | `connect --disconnect` removes session file and optionally kills Chrome | Must | Clean teardown |
| FR5 | Connection resolution chain: flags → session → auto-discovery → error | Must | Predictable, layered fallback |
| FR6 | Health check via `/json/version` before command execution | Must | Detect stale sessions early |
| FR7 | Create CDP sessions for specific browser targets (tabs) | Must | Per-tab command targeting |
| FR8 | Lazy CDP domain enabling (only what the command needs) | Must | Efficiency, minimal side effects |
| FR9 | CDP session cleanup (detach) when command finishes | Must | No dangling sessions |
| FR10 | `--tab <ID>` flag for targeting by target ID or numeric index | Must | Tab targeting |
| FR11 | Default to first "page" type target when no `--tab` specified | Must | Sensible default |
| FR12 | Graceful error on Chrome process death mid-session | Must | No panics or hangs |
| FR13 | Session file directory creation (`~/.chrome-cli/`) if missing | Should | First-run experience |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Health check < 100ms for local connections; session file read < 5ms |
| **Security** | Session file readable only by the current user (mode 0600); no secrets stored |
| **Reliability** | Graceful degradation when Chrome dies; stale session detection; no panics on invalid session files |
| **Platforms** | macOS, Linux, Windows (session file path adapts per platform) |
| **Error Messages** | Actionable messages: suggest `chrome-cli connect` for stale sessions, `tabs list` for invalid tab IDs |

---

## CLI Requirements

| Element | Requirement |
|---------|-------------|
| **New Flags** | `connect --status`, `connect --disconnect` |
| **Global Flag** | `--tab <ID>` (target tab by CDP target ID or numeric index) |
| **Output** | Session status as JSON to stdout; errors to stderr |
| **Exit Codes** | 0 = success, 2 = connection error, 3 = target error (tab not found), 4 = timeout |

---

## Data Requirements

### Session File Schema

| Field | Type | Description | Required |
|-------|------|-------------|----------|
| `ws_url` | String | WebSocket debugger URL | Yes |
| `port` | u16 | CDP debugging port | Yes |
| `pid` | Option<u32> | Chrome process ID (only for launched instances) | No |
| `timestamp` | String (ISO 8601) | When the session was created | Yes |

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--tab` | String | Non-empty; either a valid CDP target ID or a non-negative integer | No |
| `--status` | bool | Flag | No |
| `--disconnect` | bool | Flag | No |

### Output Data (connect --status)

| Field | Type | Description |
|-------|------|-------------|
| `ws_url` | String | Stored WebSocket URL |
| `port` | u16 | Stored port |
| `pid` | Option<u32> | Stored PID |
| `timestamp` | String | Session creation time |
| `reachable` | bool | Whether Chrome is currently reachable |

---

## Dependencies

### Internal Dependencies
- [x] Issue #1 — Cargo workspace setup (complete)
- [x] Issue #3 — CLI skeleton with global flags (complete)
- [x] Issue #4 — CDP WebSocket client with session multiplexing (complete)
- [x] Issue #5 — Chrome discovery and launch with connect subcommand (complete)

### External Dependencies
- `dirs` or `home` crate for cross-platform home directory resolution
- `chrono` or `time` crate for ISO 8601 timestamps (or use manual formatting)

### Blocked By
- None (all dependencies are resolved)

---

## Out of Scope

- Persistent daemon/long-lived background process for connection management
- Multi-session management (only one active session at a time)
- Tab state caching in the session file (keep it minimal — connection info only)
- Remote (non-localhost) Chrome connections
- File locking for concurrent CLI invocations (deferred — single-user CLI for now)
- Session file encryption or obfuscation

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| All acceptance criteria pass | 20/20 BDD scenarios green | `cargo test --test bdd` |
| Health check latency | < 100ms for local connections | Timing test |
| Session file round-trip | Read + parse < 5ms | Benchmark test |
| No panics on invalid session | 0 panics | Tests with corrupted/missing session files |
| Cross-platform session paths | Works on macOS + Linux + Windows | CI tests |

---

## Open Questions

- [ ] Should the session file location be configurable (e.g., `CHROME_CLI_SESSION` env var)? — Nice to have, can defer
- [x] Should `connect --disconnect` require confirmation before killing Chrome? — No, CLI tools should be non-interactive; the `--disconnect` flag is explicit intent

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified (AC6, AC9, AC10, AC11, AC18, AC20)
- [x] Dependencies are identified (all resolved)
- [x] Out of scope is defined
- [x] Open questions are documented
