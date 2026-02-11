# Requirements: Chrome Instance Discovery and Launch

**Issue**: #5
**Date**: 2026-02-10
**Status**: Draft
**Author**: Claude (spec-driven)

---

## User Story

**As a** developer or automation engineer
**I want** chrome-cli to discover running Chrome instances and launch new ones with remote debugging enabled
**So that** I can seamlessly connect to Chrome for browser automation without manual setup

---

## Background

The chrome-cli tool needs to connect to a Chrome browser via the Chrome DevTools Protocol (CDP) before any commands can be executed. Currently, the CDP WebSocket client exists (issue #4) but there is no way to discover a running Chrome instance or launch a new one. This feature bridges the gap between the CLI interface and the CDP client by implementing Chrome process discovery, launch, and the `connect` subcommand that orchestrates both.

The MCP server reference implementation uses Puppeteer's built-in browser detection. We need equivalent Rust-native functionality that works across macOS, Linux, and Windows.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Discover Chrome via HTTP debug endpoint

**Given** a Chrome instance is running with `--remote-debugging-port=9222`
**When** I run `chrome-cli connect --port 9222`
**Then** the tool queries `http://127.0.0.1:9222/json/version` and extracts the WebSocket debugger URL
**And** outputs connection info as JSON: `{"ws_url": "ws://...", "port": 9222, "pid": null}`
**And** the exit code is 0

### AC2: Discover Chrome via DevToolsActivePort file

**Given** Chrome is running and has written a `DevToolsActivePort` file in its user data directory
**When** I run `chrome-cli connect` with no flags
**Then** the tool reads the `DevToolsActivePort` file from the platform-default Chrome user data directory
**And** extracts the port number and WebSocket path from the file
**And** connects to the discovered instance

### AC3: List available targets from running Chrome

**Given** a Chrome instance is running with multiple tabs open
**When** the tool connects to Chrome
**Then** it queries `http://{host}:{port}/json/list` to enumerate available targets
**And** target information is available for tab management commands

### AC4: Launch Chrome with remote debugging

**Given** Chrome is installed on the system
**When** I run `chrome-cli connect --launch`
**Then** the tool finds the Chrome executable on the current platform
**And** launches Chrome with `--remote-debugging-port=<port>` using an available port
**And** creates a temporary user data directory for isolation
**And** waits for Chrome to be ready by polling `/json/version`
**And** outputs connection info as JSON with the assigned port and Chrome PID

### AC5: Launch Chrome in headless mode

**Given** Chrome is installed on the system
**When** I run `chrome-cli connect --launch --headless`
**Then** Chrome is launched with both `--remote-debugging-port` and `--headless=new` flags
**And** the tool connects successfully

### AC6: Launch specific Chrome channel

**Given** Chrome Canary is installed on the system
**When** I run `chrome-cli connect --launch --channel canary`
**Then** the tool finds and launches Chrome Canary's executable
**And** connects to it successfully

### AC7: Connect via direct WebSocket URL

**Given** I know the WebSocket debugger URL of a running Chrome instance
**When** I run `chrome-cli connect --ws-url ws://127.0.0.1:9222/devtools/browser/abc123`
**Then** the tool connects directly to the provided WebSocket URL
**And** outputs connection info as JSON

### AC8: Auto-discover or launch (default behavior)

**Given** no Chrome instance is running with debugging enabled
**When** I run `chrome-cli connect` with no flags
**Then** the tool first attempts to discover a running instance (DevToolsActivePort, then common ports)
**And** if no instance is found, launches a new Chrome instance
**And** connects to it and outputs connection info

### AC9: Custom Chrome executable path

**Given** Chrome is installed in a non-standard location
**When** I run `chrome-cli connect --launch --chrome-path /custom/path/to/chrome`
**Then** the tool launches Chrome from the specified path
**And** connects to it successfully

### AC10: Pass additional Chrome flags

**Given** Chrome is installed on the system
**When** I run `chrome-cli connect --launch --chrome-arg --disable-gpu --chrome-arg --no-sandbox`
**Then** Chrome is launched with the additional flags passed through
**And** the tool connects successfully

### AC11: Chrome not installed error

**Given** Chrome is not installed on the system (or not found in any known location)
**When** I run `chrome-cli connect --launch`
**Then** the tool outputs a clear error message indicating Chrome was not found
**And** suggests installing Chrome or using `--chrome-path` to specify the location
**And** the exit code is non-zero

### AC12: Connection timeout

**Given** Chrome is being launched but takes too long to start
**When** the tool polls `/json/version` and the timeout is exceeded
**Then** the tool outputs an error indicating the connection timed out
**And** the launched Chrome process is cleaned up
**And** the exit code is non-zero

### AC13: Port already in use

**Given** port 9222 is already in use by another process (not Chrome with CDP)
**When** I run `chrome-cli connect --port 9222`
**Then** the tool reports that it could not connect to Chrome on that port
**And** the exit code is non-zero

### AC14: Cross-platform Chrome executable discovery

**Given** the tool is running on macOS, Linux, or Windows
**When** the tool needs to find Chrome
**Then** it searches platform-specific paths:
  - macOS: `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`, Chromium, Canary, Beta, Dev
  - Linux: `google-chrome`, `google-chrome-stable`, `chromium-browser`, `chromium` in PATH
  - Windows: Standard install paths in `Program Files` and `Program Files (x86)`

### AC15: Temporary user data directory cleanup

**Given** Chrome was launched with a temporary user data directory
**When** chrome-cli exits or the connection is closed
**Then** the temporary user data directory is cleaned up

### Generated Gherkin Preview

```gherkin
Feature: Chrome instance discovery and launch
  As a developer or automation engineer
  I want chrome-cli to discover running Chrome instances and launch new ones
  So that I can seamlessly connect to Chrome for browser automation

  Scenario: Discover Chrome via HTTP debug endpoint
    Given a Chrome instance is running with remote debugging on port 9222
    When I run "chrome-cli connect --port 9222"
    Then the output should contain a JSON object with "ws_url"
    And the output should contain "port": 9222
    And the exit code should be 0

  Scenario: Connect via direct WebSocket URL
    Given a Chrome instance is running with a known WebSocket URL
    When I run "chrome-cli connect --ws-url ws://127.0.0.1:9222/devtools/browser/abc"
    Then the output should contain a JSON object with "ws_url"
    And the exit code should be 0

  Scenario: Chrome not installed error
    Given Chrome is not installed on the system
    When I run "chrome-cli connect --launch"
    Then stderr should contain "Chrome not found"
    And the exit code should be non-zero
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Query `/json/version` endpoint to discover running Chrome | Must | Returns WebSocket debugger URL |
| FR2 | Query `/json/list` endpoint to enumerate targets | Must | Needed for tab management |
| FR3 | Read `DevToolsActivePort` file for auto-discovery | Must | Platform-specific paths |
| FR4 | Launch Chrome with `--remote-debugging-port` | Must | Core launch functionality |
| FR5 | Find Chrome executable on macOS, Linux, Windows | Must | Platform-specific discovery |
| FR6 | Support `--headless=new` flag for headless mode | Must | Common automation use case |
| FR7 | Create and clean up temporary user data directories | Must | Isolation for launched instances |
| FR8 | Poll `/json/version` until Chrome is ready | Must | Chrome takes time to start |
| FR9 | Support `--chrome-path` for custom executable | Should | Override auto-detection |
| FR10 | Support Chrome channels (stable, canary, beta, dev) | Should | Channel selection |
| FR11 | Support `--chrome-arg` for additional flags | Should | Flexibility for power users |
| FR12 | Scan common debug ports (9222, 9223, etc.) | Could | Convenience for discovery |
| FR13 | Support `--timeout` for Chrome startup wait | Should | Already in global opts |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Chrome discovery via HTTP should complete in < 2s; Chrome launch polling timeout default 30s |
| **Security** | CDP connections only to localhost by default (per tech.md) |
| **Reliability** | Graceful cleanup of launched Chrome process on error/exit; temp directory cleanup |
| **Platforms** | macOS (Intel + Apple Silicon), Linux (x64 + ARM), Windows (x64) |
| **Error Messages** | Clear, actionable messages with suggestions (per product.md brand voice) |

---

## CLI Requirements

| Element | Requirement |
|---------|-------------|
| **Output** | Connection info as JSON to stdout: `{"ws_url": "...", "port": N, "pid": N}` |
| **Error Output** | Error messages to stderr with structured JSON in `--json` mode |
| **Exit Codes** | 0 = success, 2 = connection error, 4 = timeout error |
| **Flags** | `--port`, `--ws-url`, `--launch`, `--headless`, `--channel`, `--chrome-path`, `--chrome-arg` |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--port` | u16 | Valid port number (1-65535) | No (default: scan) |
| `--ws-url` | String | Valid WebSocket URL (ws:// or wss://) | No |
| `--chrome-path` | PathBuf | File exists and is executable | No |
| `--channel` | Enum | One of: stable, canary, beta, dev | No (default: stable) |
| `--chrome-arg` | Vec<String> | Any string (passed through to Chrome) | No |
| `--headless` | bool | Flag | No (default: false) |
| `--launch` | bool | Flag | No (default: false) |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| `ws_url` | String | WebSocket debugger URL |
| `port` | u16 | CDP debugging port number |
| `pid` | Option<u32> | Chrome process ID (only when launched) |

---

## Dependencies

### Internal Dependencies
- [x] Issue #1 — Cargo workspace setup (complete)
- [x] Issue #3 — CLI skeleton with `connect` subcommand stub (complete)
- [x] Issue #4 — CDP WebSocket client (complete)

### External Dependencies
- Chrome/Chromium browser installed on the system (for launch functionality)
- HTTP client for querying `/json/version` and `/json/list` endpoints

### Blocked By
- None (all dependencies are resolved)

---

## Out of Scope

- Firefox or Safari browser support (CDP-only)
- Remote Chrome connections (non-localhost) — future feature
- Chrome profile management beyond temporary directories
- Chrome extension installation or management
- Persistent Chrome session management (daemon mode)
- Chrome download/installation

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Discovery latency | < 2s for running instance | Time from command start to connection info output |
| Launch + connect latency | < 10s (headless), < 15s (headed) | Time from command start to ready state |
| Platform coverage | macOS + Linux + Windows | CI tests pass on all platforms |
| Error clarity | Users can self-resolve | Error messages include actionable suggestions |

---

## Open Questions

- [x] Should `connect` be the default subcommand? — No, explicit is better per CLI conventions
- [ ] Should we support connecting to remote (non-localhost) Chrome instances? — Deferred to future issue
- [ ] Should we add a `--no-cleanup` flag to preserve temp user data directories for debugging? — Nice to have

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified (AC11, AC12, AC13)
- [x] Dependencies are identified (all resolved)
- [x] Out of scope is defined
- [x] Open questions are documented
