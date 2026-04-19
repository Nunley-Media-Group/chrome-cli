# Requirements: Session and Connection Management

**Issues**: #6, #185
**Date**: 2026-04-18
**Status**: Draft
**Author**: Claude (spec-driven)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #6 | 2026-02-11 | Initial feature spec — session file, health check, tab targeting, CDP session mgmt |
| #185 | 2026-04-18 | Adds auto-reconnect on stale session, WebSocket keep-alive ping, and graceful distinction between recoverable and unrecoverable connection loss |

---

## User Story

**As a** developer or automation engineer using agentchrome
**I want** commands to automatically find and reuse a Chrome connection, manage CDP sessions per tab, and persist connection state across CLI invocations
**So that** I don't have to pass connection details to every command, and I can efficiently target specific tabs

---

## Background

Each agentchrome invocation needs a connection to Chrome. Today, `agentchrome connect` discovers or launches Chrome and outputs connection info, but that info is not persisted — the next command would need to rediscover Chrome from scratch. This is inefficient and fragile.

This feature adds a lightweight session file (similar to `docker context` or `kubectl config`) so that after a successful `connect` or `--launch`, subsequent commands can automatically reconnect. It also adds CDP session management for targeting specific tabs, lazy domain enabling, and robust connection health checks before executing commands.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Write session file after successful connect

**Given** Chrome is running with remote debugging enabled
**When** I run `agentchrome connect`
**Then** the tool connects to Chrome and writes a session file to `~/.agentchrome/session.json`
**And** the session file contains `{"ws_url": "ws://...", "port": N, "pid": null, "timestamp": "..."}`
**And** the exit code is 0

**Example**:
- Given: Chrome running on port 9222
- When: `agentchrome connect --port 9222`
- Then: `~/.agentchrome/session.json` contains `{"ws_url":"ws://127.0.0.1:9222/devtools/browser/abc","port":9222,"pid":null,"timestamp":"2026-02-11T12:00:00Z"}`

### AC2: Write session file after successful launch

**Given** Chrome is installed on the system
**When** I run `agentchrome connect --launch`
**Then** the tool launches Chrome and writes a session file to `~/.agentchrome/session.json`
**And** the session file contains the WebSocket URL, port, Chrome PID, and timestamp
**And** the exit code is 0

### AC3: Subsequent commands auto-read session file

**Given** a valid session file exists at `~/.agentchrome/session.json`
**And** no explicit `--ws-url` or `--port` flags are provided
**When** I run a command that needs Chrome (e.g., `agentchrome tabs list`)
**Then** the tool reads the session file and connects using the stored WebSocket URL
**And** the command executes successfully

### AC4: Explicit flags override session file

**Given** a valid session file exists at `~/.agentchrome/session.json` pointing to port 9222
**And** Chrome is also running on port 9333
**When** I run `agentchrome tabs list --port 9333`
**Then** the tool connects to port 9333, ignoring the session file
**And** the command executes successfully

### AC5: Show connection status

**Given** a valid session file exists
**When** I run `agentchrome connect --status`
**Then** the tool reads the session file and displays connection info
**And** indicates whether Chrome is still reachable at the stored address
**And** the exit code is 0

### AC6: Show connection status when no session exists

**Given** no session file exists
**When** I run `agentchrome connect --status`
**Then** the tool reports that no active session was found
**And** the exit code is non-zero

### AC7: Disconnect removes session file

**Given** a valid session file exists
**When** I run `agentchrome connect --disconnect`
**Then** the session file is removed
**And** the exit code is 0

### AC8: Disconnect kills launched Chrome process

**Given** a session file exists with a PID (from a `--launch`)
**When** I run `agentchrome connect --disconnect`
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
**And** the error message suggests using `agentchrome connect` or `agentchrome connect --launch`

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
**And** suggests running `agentchrome connect` to establish a new connection

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
**When** I run `agentchrome tabs list` and get target IDs
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
**And** suggests using `agentchrome tabs list` to see available tabs
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

---

## Enhancement: Session Reconnection & Keep-Alive (Issue #185)

The following acceptance criteria extend AC10, AC11, and AC20 with transparent auto-reconnect and a keep-alive mechanism so long-running automation sessions survive transient WebSocket drops and idle timeouts.

### AC21: Auto-reconnect on stale WebSocket URL

**Given** a session file exists with a `ws_url` that is no longer reachable
**And** Chrome is still running on the stored port
**When** I run any command that needs Chrome (e.g., `agentchrome page snapshot`)
**Then** the tool detects the dead WebSocket via the health check
**And** transparently re-discovers the current browser `ws_url` on the stored port
**And** updates the session file with the new `ws_url`
**And** retries the original command within the same CLI invocation
**And** the command returns its normal JSON payload on stdout
**And** the exit code is 0

**Example**:
- Given: `session.json` with `ws_url: ws://127.0.0.1:9222/devtools/browser/OLD_ID` but Chrome has rotated to `/NEW_ID`
- When: `agentchrome page snapshot`
- Then: session file is updated to `/NEW_ID`, snapshot is returned, exit code 0

### AC22: Auto-reconnect applies uniformly across all commands

**Given** a stale session file (Chrome reachable, ws_url rotated)
**When** I run any of: `tabs list`, `navigate`, `page snapshot`, `page screenshot`, `js exec`, `form fill`, `interact click`, `network list`, `console read`, `emulate status`, `perf vitals`, `dialog info`
**Then** each command independently exhibits the auto-reconnect behavior from AC21
**And** no command bypasses the reconnect path by opening its own direct connection

> **Retrospective note (path audit):** Every command that needs Chrome must route through the same reconnect-aware resolution layer. Commands that construct their own CDP client must not skip reconnect.

### AC23: Auto-reconnect preserves session file fields

**Given** a session file with `pid: 12345` (from a prior `--launch`) and a stale `ws_url`
**When** auto-reconnect succeeds and rewrites the session file
**Then** the new file contains the new `ws_url` and a refreshed `timestamp`
**And** the `pid` and `port` fields are preserved unchanged
**And** no unrelated field is reset to a default value

> **Retrospective note (write-over-existing-state):** Explicitly name which fields are preserved versus rewritten, so a subsequent `connect --disconnect` can still terminate the correct Chrome process.

### AC24: Per-attempt reconnect latency budget

**Given** auto-reconnect is triggered
**When** each individual probe of Chrome (health check + WebSocket handshake) is attempted
**Then** each probe completes or is aborted within a bounded per-attempt budget (default 500 ms for local connections)
**And** a blocking probe does not consume the entire overall reconnect window
**And** the per-attempt budget is configurable via `reconnect.probe_timeout_ms`

### AC25: Bounded reconnect attempts with backoff

**Given** Chrome is transiently unreachable
**When** auto-reconnect runs
**Then** the tool retries the probe using the exponential backoff defined by `CdpConfig.reconnect` (initial delay, multiplier, max delay)
**And** stops after at most `reconnect.max_attempts` attempts (default 3)
**And** reports an error when the attempt budget is exhausted

### AC26: Graceful error on unrecoverable session loss

**Given** Chrome has been terminated (no process responding on the stored port and no auto-discoverable Chrome)
**When** I run any command
**And** auto-reconnect has exhausted its attempts
**Then** the tool emits a structured JSON error on stderr of the shape `{"error": "<msg>", "code": 2, "kind": "chrome_terminated", "recoverable": false}`
**And** the human-readable message suggests running `agentchrome connect --launch` to start a new session
**And** the session file is not silently deleted (so the user can inspect it)
**And** the exit code is 2 (connection error)

### AC27: Recoverable-loss error is distinguished from unrecoverable

**Given** auto-reconnect fails for a reason other than Chrome termination (e.g., port moved, permission error)
**When** the tool reports the error
**Then** the structured error has `"kind": "transient"` and `"recoverable": true`
**And** the suggested remediation references `agentchrome connect` (rediscovery) rather than `--launch`

### AC28: WebSocket keep-alive ping during long-running commands

**Given** a command holds a CDP session open for longer than the configured keep-alive interval (e.g., `console follow`, long `interact wait`, long `perf record`)
**When** the keep-alive interval elapses with no outbound CDP traffic
**Then** the client sends a WebSocket `Ping` frame
**And** Chrome's `Pong` response is received within the configured pong timeout
**And** the connection is not closed by idle intermediaries or OS-level TCP keepalive before the command completes

### AC29: Keep-alive interval is configurable

**Given** the user configures the keep-alive interval through any supported channel
**When** the CLI runs a command
**Then** the configured interval is honored, resolved in this order (highest precedence first): `--keepalive-interval <ms>` flag, `AGENTCHROME_KEEPALIVE_INTERVAL` env var, `config.toml` `keepalive.interval_ms`, built-in default (30000 ms)
**And** `agentchrome connect --status` includes the effective keep-alive interval in its JSON output

### AC30: Keep-alive can be disabled

**Given** the user passes `--no-keepalive` or sets the interval to `0`
**When** the CLI runs any command
**Then** no WebSocket ping frames are sent by the client
**And** the command still succeeds under normal conditions (short commands are unaffected)

### AC31: Keep-alive does not interfere with in-flight CDP requests

**Given** keep-alive is active and a CDP JSON-RPC request is in flight
**When** the keep-alive interval elapses during the request
**Then** the ping frame is sent concurrently (via the WebSocket control frame path, not the JSON-RPC channel)
**And** the JSON-RPC request completes successfully with its original response correlated by `id`
**And** no ping frame is misinterpreted as a JSON-RPC message

### AC32: Reconnect is silent on stdout

**Given** a command succeeds after an auto-reconnect
**When** the command produces its output
**Then** stdout contains only the command's normal JSON payload
**And** reconnect diagnostics (attempt counts, backoff delays) appear on stderr only when `--verbose` or an equivalent log level is enabled

### AC33: Reconnect behavior is observable via `connect --status`

**Given** a previous reconnect has occurred (session file `ws_url` has been rewritten)
**When** I run `agentchrome connect --status`
**Then** the JSON output includes a `last_reconnect_at` timestamp (ISO 8601) and a `reconnect_count` integer for the current session
**And** these fields reflect cumulative reconnects within the life of the session file

> **Retrospective note (cross-invocation state):** `last_reconnect_at` and `reconnect_count` are stored in the session file so they are visible across separate CLI invocations, not reset per-invocation.

### AC34: Clap help documents the new flags

**Given** the `--keepalive-interval <ms>` and `--no-keepalive` flags are added as global options (or on the `connect` subcommand, whichever applies)
**When** I run `agentchrome --help` (short form)
**Then** the flags appear with a one-line description
**And** `agentchrome --help` long form (`long_about` / `after_long_help`) includes at least one worked `EXAMPLES:` invocation showing each flag, including a `--json` variant where applicable
**And** `--keepalive-interval` uses a `value_parser` / `value_enum` for numeric validation (no free-form string)
**And** `--no-keepalive` declares `conflicts_with = ["keepalive_interval"]`

### AC35: Capabilities manifest and man pages reflect the new flags

**Given** the new flags exist
**When** I run `agentchrome capabilities`
**Then** the JSON manifest lists `--keepalive-interval` and `--no-keepalive` under the appropriate command(s) with their descriptions
**And** `cargo xtask man connect` (or the aggregated man flow) renders a man page section that mentions both flags and their defaults

### AC36: README documents session reconnection and keep-alive

**Given** the feature is shipped
**When** a reader opens the project `README.md`
**Then** a short section (new or under an existing session-management section) explains:
  - That commands auto-reconnect when Chrome is still running but the WebSocket has rotated or dropped
  - The keep-alive flag, env var, config key, and default interval
  - How to disable keep-alive (`--no-keepalive`)
  - How to distinguish the `chrome_terminated` error from the `transient` error in scripts (by `kind` / `recoverable` fields)
**And** the section includes at least one copy-pasteable example command using `--keepalive-interval`

### Generated Gherkin Preview

```gherkin
Feature: Session and connection management
  As a developer or automation engineer
  I want commands to automatically find and reuse a Chrome connection
  So that I don't have to pass connection details to every command

  # --- Session File Management ---

  Scenario: Write session file after connect
    Given a Chrome instance is running with remote debugging
    When I run "agentchrome connect"
    Then a session file should exist at "~/.agentchrome/session.json"
    And the session file should contain "ws_url" and "port" fields
    And the exit code should be 0

  Scenario: Write session file after launch
    Given Chrome is installed on the system
    When I run "agentchrome connect --launch"
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
    When I run "agentchrome connect --status"
    Then the output shows connection info and reachability

  Scenario: No active session status
    Given no session file exists
    When I run "agentchrome connect --status"
    Then the output indicates no active session

  Scenario: Disconnect removes session
    Given a valid session file exists
    When I run "agentchrome connect --disconnect"
    Then the session file is removed

  Scenario: Disconnect kills launched process
    Given a session file exists with a PID
    When I run "agentchrome connect --disconnect"
    Then the Chrome process is terminated

  # --- Connection Resolution ---

  Scenario: Connection resolution chain with no Chrome found
    Given no flags, no session file, and no Chrome running
    When I run a command that needs Chrome
    Then the error suggests using "agentchrome connect"

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

  # --- Reconnection & Keep-Alive (Issue #185) ---

  Scenario: Auto-reconnect on stale WebSocket URL
    Given a session file with a stale ws_url and Chrome running on the stored port
    When I run "agentchrome page snapshot"
    Then the tool rediscovers the current ws_url
    And the session file is updated
    And the command returns its normal JSON payload
    And the exit code is 0

  Scenario Outline: Auto-reconnect applies to every command
    Given a session file with a stale ws_url and Chrome still running
    When I run "<command>"
    Then the tool auto-reconnects and the command succeeds

    Examples:
      | command                         |
      | agentchrome tabs list           |
      | agentchrome page snapshot       |
      | agentchrome js exec "1+1"       |
      | agentchrome network list        |
      | agentchrome console read        |

  Scenario: Auto-reconnect preserves pid
    Given a session file with pid=12345 and a stale ws_url
    When auto-reconnect rewrites the session file
    Then the new session file still has pid=12345 and the new ws_url

  Scenario: Per-attempt probe latency budget enforced
    Given auto-reconnect is triggered
    When each probe is attempted
    Then each probe completes or aborts within the configured probe_timeout_ms

  Scenario: Reconnect respects bounded attempts
    Given Chrome is unreachable
    When auto-reconnect runs
    Then at most reconnect.max_attempts probes are performed

  Scenario: Unrecoverable loss emits structured error
    Given Chrome has been terminated and cannot be rediscovered
    When I run a command
    Then stderr contains a JSON error with kind="chrome_terminated" and recoverable=false
    And the message suggests "agentchrome connect --launch"
    And the exit code is 2

  Scenario: Transient loss is distinguished from termination
    Given auto-reconnect fails for a non-termination reason
    When the tool reports the error
    Then the JSON error has kind="transient" and recoverable=true
    And the suggested remediation references "agentchrome connect"

  Scenario: Keep-alive prevents idle disconnect
    Given a command holds the CDP session longer than the keep-alive interval
    When the keep-alive interval elapses
    Then a WebSocket Ping frame is sent
    And a Pong is received before the pong timeout

  Scenario Outline: Keep-alive interval resolution precedence
    Given <source> sets the keep-alive interval
    When a command runs
    Then the configured interval is honored

    Examples:
      | source                                           |
      | the --keepalive-interval flag                    |
      | the AGENTCHROME_KEEPALIVE_INTERVAL env var       |
      | the config.toml keepalive.interval_ms entry      |

  Scenario: Keep-alive disabled via flag
    Given I pass "--no-keepalive"
    When a command runs
    Then no WebSocket Ping frames are sent

  Scenario: Keep-alive does not collide with JSON-RPC
    Given keep-alive is active and a CDP request is in flight
    When the keep-alive interval elapses
    Then the Ping frame is sent as a WebSocket control frame
    And the JSON-RPC response is correctly correlated to the original request

  Scenario: Reconnect is silent on stdout
    Given a command succeeds after auto-reconnect
    When the command emits output
    Then stdout contains only the normal JSON payload
    And reconnect diagnostics appear on stderr only with --verbose

  Scenario: Reconnect telemetry visible via connect --status
    Given a reconnect has occurred earlier in the session
    When I run "agentchrome connect --status"
    Then the JSON output includes "last_reconnect_at" and "reconnect_count"

  Scenario: Clap help lists the new flags
    When I run "agentchrome --help"
    Then the output mentions "--keepalive-interval" and "--no-keepalive"
    And the long help includes an EXAMPLES section with at least one worked invocation per flag

  Scenario: Capabilities manifest includes keep-alive flags
    When I run "agentchrome capabilities"
    Then the JSON manifest lists "--keepalive-interval" and "--no-keepalive" with descriptions and defaults

  Scenario: Man page covers keep-alive flags
    When I run "cargo xtask man connect"
    Then the rendered man page mentions "--keepalive-interval" and "--no-keepalive"

  Scenario: README documents session resilience
    When I read the project README
    Then it contains a section covering auto-reconnect, keep-alive flag/env/config, disable mechanism, and error-kind scripting guidance
    And the section includes at least one copy-pasteable "--keepalive-interval" example
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Write session file to `~/.agentchrome/session.json` after successful connect/launch | Must | Contains ws_url, port, pid, timestamp |
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
| FR13 | Session file directory creation (`~/.agentchrome/`) if missing | Should | First-run experience |
| FR14 | Auto-reconnect on detected WebSocket closure or stale `ws_url`, with bounded retry (per-attempt latency budget + exponential backoff) | Must | Added by #185 — transparent retry inside the current invocation |
| FR15 | WebSocket keep-alive ping frames at a configurable interval (default 30 s); disabled when interval == 0 or `--no-keepalive` | Must | Added by #185 — prevents idle-timeout drop during long commands |
| FR16 | Structured JSON error distinguishing `kind: chrome_terminated` (unrecoverable) from `kind: transient` (recoverable) connection loss, with `recoverable: bool` | Must | Added by #185 — actionable remediation guidance |
| FR17 | Auto-reconnect preserves the `pid`, `port`, and any unrelated session-file fields when rewriting `ws_url` | Must | Added by #185 — prevents losing the handle to the launched Chrome process |
| FR18 | All commands that need Chrome route through the same reconnect-aware resolution layer (no command constructs its own bypass connection) | Must | Added by #185 — path audit requirement |
| FR19 | Session file records `last_reconnect_at` (ISO 8601) and `reconnect_count` (integer), visible via `connect --status` | Should | Added by #185 — cross-invocation telemetry |
| FR20 | Reconnect diagnostics appear only on stderr, gated on `--verbose` or equivalent log level; stdout remains pure JSON payload | Must | Added by #185 — preserves AI-agent JSON contract |
| FR21 | Clap metadata for `--keepalive-interval` and `--no-keepalive`: `about`, doc comments, `value_parser` / numeric validation, `conflicts_with`, and `after_long_help` examples (including a `--json` example where applicable) per `steering/tech.md` clap-help rules | Must | Added by #185 — non-negotiable per tech steering |
| FR22 | `agentchrome capabilities` manifest and generated man pages include the new flags with descriptions and defaults | Must | Added by #185 — downstream surfaces are clap-driven, so FR21 must propagate |
| FR23 | Update `README.md` with a session-resilience section covering auto-reconnect semantics, keep-alive flag/env/config, disable mechanism, and error-kind scripting guidance, including at least one worked example | Must | Added by #185 — user-facing discoverability |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Health check < 100ms for local connections; session file read < 5ms |
| **Security** | Session file readable only by the current user (mode 0600); no secrets stored |
| **Reliability** | Graceful degradation when Chrome dies; stale session detection; no panics on invalid session files |
| **Platforms** | macOS, Linux, Windows (session file path adapts per platform) |
| **Error Messages** | Actionable messages: suggest `agentchrome connect` for stale sessions, `tabs list` for invalid tab IDs |

---

## UI/UX Requirements

Reference `structure.md` and `product.md` for project-specific design standards.

| Element | Requirement |
|---------|-------------|
| **Interaction** | [Touch targets, gesture requirements] |
| **Typography** | [Minimum text sizes, font requirements] |
| **Contrast** | [Accessibility contrast requirements] |
| **Loading States** | [How loading should be displayed] |
| **Error States** | [How errors should be displayed] |
| **Empty States** | [How empty data should be displayed] |

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

| Field | Type | Description | Required | Added |
|-------|------|-------------|----------|-------|
| `ws_url` | String | WebSocket debugger URL | Yes | #6 |
| `port` | u16 | CDP debugging port | Yes | #6 |
| `pid` | Option<u32> | Chrome process ID (only for launched instances) | No | #6 |
| `timestamp` | String (ISO 8601) | When the session was created | Yes | #6 |
| `last_reconnect_at` | Option<String (ISO 8601)> | When auto-reconnect last rewrote this file; `null` if never | No | #185 |
| `reconnect_count` | u32 | Cumulative successful reconnects for this session; starts at 0 | Yes (default 0) | #185 |

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
- [x] Issue #87 — Fix connect auto-discover overwriting session pid (complete) — #185 must not regress pid preservation
- [x] Issue #94 — Fix connect auto-discover reconnect (complete) — #185 extends reconnect behavior to all commands

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
- **Automatic Chrome re-launch on process termination** (#185) — when Chrome is gone, the tool reports an unrecoverable error and asks the user to `connect --launch`; it does not auto-launch Chrome
- **Page state preservation across reconnection** (#185) — navigation history, JS state, open dialogs, and network recording buffers may be lost across a reconnect. Only the CDP transport is restored.
- **Cross-invocation keep-alive** (#185) — keep-alive frames fire only within the lifetime of a single CLI invocation. Keeping the WebSocket alive between invocations would require a daemon, which is explicitly out of scope

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| All acceptance criteria pass | 36/36 BDD scenarios green | `cargo test --test bdd` |
| Health check latency | < 100ms for local connections | Timing test |
| Session file round-trip | Read + parse < 5ms | Benchmark test |
| No panics on invalid session | 0 panics | Tests with corrupted/missing session files |
| Cross-platform session paths | Works on macOS + Linux + Windows | CI tests |
| Auto-reconnect probe latency | Per-attempt < 500ms for local connections (#185) | Timing test |
| Reconnect success rate under transient drop | 100% within configured `max_attempts` when Chrome is reachable (#185) | BDD + unit test |
| Keep-alive overhead | < 1 KB/min of CDP traffic on an idle connection (#185) | Manual observation + unit test |
| No regression on short-lived commands | P95 latency of `tabs list` unchanged vs. pre-#185 baseline | Benchmark test |

---

## Open Questions

- [ ] Should the session file location be configurable (e.g., `AGENTCHROME_SESSION` env var)? — Nice to have, can defer
- [x] Should `connect --disconnect` require confirmation before killing Chrome? — No, CLI tools should be non-interactive; the `--disconnect` flag is explicit intent
- [ ] (#185) Should keep-alive default interval be 30 s or match Chrome's own WebSocket idle timeout (if documented)? — Currently assumed 30 s; design phase may revise
- [ ] (#185) Should `reconnect_count` persist across `--disconnect` / new `connect` pairs, or reset on each new session? — Currently specified to reset when the session file is recreated

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
