# Requirements: CLI Skeleton with Clap Derive Macros

**Issue**: #3
**Date**: 2026-02-10
**Status**: Draft
**Author**: Claude (spec-driven)

---

## User Story

**As a** developer or AI agent using chrome-cli
**I want** a well-structured CLI with comprehensive help text, global flags, and subcommand stubs
**So that** I can discover all capabilities, configure connection settings, and understand available commands before they are fully implemented

---

## Background

chrome-cli is a Rust CLI tool for browser automation via the Chrome DevTools Protocol. It is designed to be consumed primarily by AI agents (Claude Code), which means every subcommand, flag, and argument must have rich, descriptive help text. This issue establishes the foundational CLI structure — the argument parsing, subcommand hierarchy, global options, output format flags, and exit code conventions — that all future commands will build upon.

The current `main.rs` only prints the package name and version. This feature replaces that with a full clap-based CLI skeleton.

---

## Acceptance Criteria

### AC1: Top-level help displays comprehensive tool description

**Given** chrome-cli is installed
**When** I run `chrome-cli --help`
**Then** the output includes a description of what the tool does
**And** lists all available subcommands with descriptions
**And** lists all global flags and options

### AC2: Version flag displays version information

**Given** chrome-cli is installed
**When** I run `chrome-cli --version`
**Then** the output contains the package name and version string

### AC3: Output format flags are mutually exclusive

**Given** chrome-cli is installed
**When** I run `chrome-cli --json --plain <subcommand>`
**Then** the CLI rejects the conflicting flags with an error
**And** the error message explains the conflict

### AC4: Global connection options accept custom values

**Given** chrome-cli is installed
**When** I run `chrome-cli --port 9333 --host 192.168.1.100 tabs`
**Then** the port is parsed as 9333
**And** the host is parsed as "192.168.1.100"

### AC5: Default connection values are applied

**Given** chrome-cli is installed
**When** I run `chrome-cli tabs` without specifying --port or --host
**Then** the port defaults to 9222
**And** the host defaults to "127.0.0.1"

### AC6: Subcommand stubs return "not yet implemented"

**Given** chrome-cli is installed
**When** I run `chrome-cli connect`
**Then** the stderr output contains a JSON error: `{"error": "not yet implemented", "code": 1}`
**And** the exit code is 1

### AC7: Each subcommand group has help text

**Given** chrome-cli is installed
**When** I run `chrome-cli tabs --help`
**Then** the output includes a description of what the tabs command group does
**And** the description is detailed enough for an AI agent to understand

### AC8: Exit codes are correctly returned

**Given** chrome-cli is installed
**When** a stub subcommand is executed
**Then** the exit code is 1 (general error for "not yet implemented")

### AC9: Error output is structured JSON on stderr

**Given** chrome-cli is installed
**When** an error occurs (e.g., stub command)
**Then** the error is written to stderr (not stdout)
**And** the error is formatted as JSON: `{"error": "<message>", "code": <N>}`

### AC10: All 12 subcommand groups are registered

**Given** chrome-cli is installed
**When** I run `chrome-cli --help`
**Then** the output lists all 12 subcommand groups: connect, tabs, navigate, page, dom, js, console, network, interact, form, emulate, perf

### AC11: WebSocket URL option is accepted

**Given** chrome-cli is installed
**When** I run `chrome-cli --ws-url ws://localhost:9222/devtools/browser/abc tabs`
**Then** the WebSocket URL is parsed correctly

### AC12: Timeout option is accepted

**Given** chrome-cli is installed
**When** I run `chrome-cli --timeout 5000 tabs`
**Then** the timeout is parsed as 5000 milliseconds

### AC13: Tab ID option is accepted

**Given** chrome-cli is installed
**When** I run `chrome-cli --tab abc123 js`
**Then** the tab ID is parsed as "abc123"

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Parse CLI arguments using clap derive macros | Must | Use `#[derive(Parser)]` |
| FR2 | Register 13 subcommand groups with stubs | Must | Each returns "not yet implemented" |
| FR3 | Global output format flags (--json, --pretty, --plain) | Must | Mutually exclusive via clap group |
| FR4 | Global connection options (--port, --host, --ws-url) | Must | With sensible defaults |
| FR5 | Global --timeout option | Must | Milliseconds, optional |
| FR6 | Global --tab option | Must | Target tab ID, optional |
| FR7 | Structured JSON error output to stderr | Must | `{"error": "...", "code": N}` |
| FR8 | Exit code conventions (0-5) defined as enum | Must | With Display impl |
| FR9 | Comprehensive help text on all commands and flags | Must | AI-agent-friendly descriptions |
| FR10 | clap dependency with derive and env features | Must | Add to Cargo.toml |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | CLI parsing adds < 1ms overhead; binary stays < 10MB |
| **Reliability** | All argument combinations that should work, do work; conflicting flags are rejected cleanly |
| **Platforms** | Must compile on macOS, Linux, and Windows |
| **Maintainability** | Each subcommand is a separate module stub for easy future implementation |

---

## Data Requirements

### Input Data (CLI Arguments)

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| --port | u16 | Valid port number (1-65535) | No (default: 9222) |
| --host | String | Non-empty string | No (default: 127.0.0.1) |
| --ws-url | String | Valid WebSocket URL | No |
| --timeout | u64 | Positive integer (milliseconds) | No |
| --tab | String | Non-empty string | No |
| --json | bool | Flag | No (default: true) |
| --pretty | bool | Flag | No |
| --plain | bool | Flag | No |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| error | String | Error message (stderr, JSON format) |
| code | u8 | Exit code (0-5) |

---

## Dependencies

### Internal Dependencies
- [x] Issue #1 — Cargo workspace setup (completed)

### External Dependencies
- [ ] `clap` crate with `derive` and `env` features
- [ ] `serde` and `serde_json` for JSON error output

---

## Out of Scope

- Actual implementation of any subcommand (all stubs)
- CDP WebSocket connection logic
- Chrome process discovery/launch
- Output formatting layer beyond error JSON
- Any async runtime setup

---

## Open Questions

None — all requirements are clear from the issue.

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Edge cases and error states are specified
- [x] Dependencies are identified
- [x] Out of scope is defined
