# Defect Report: http_get read_to_string blocks waiting for EOF that Chrome never sends

**Issue**: #68
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude
**Severity**: Critical

---

## Reproduction

### Steps to Reproduce

1. Ensure Chrome is installed and no existing connection session exists
2. Run `chrome-cli connect --launch`
3. Observe the command hangs for ~30 seconds
4. Command fails with `"Chrome startup timed out on port XXXXX"` (exit code 4)

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (confirmed; likely affects Linux and Windows) |
| **Version / Commit** | Current `main` branch |
| **Root Cause File** | `src/chrome/discovery.rs`, function `http_get()`, line 168 |
| **Affected Chain** | `launch_chrome()` → `query_version()` → `http_get()` |

### Frequency

Always — 100% reproducible on macOS.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | Chrome is launched, the DevTools port is detected, a session is saved, and connection metadata is printed as JSON with exit code 0. |
| **Actual** | Chrome is launched and its DevTools HTTP server responds correctly within ~220ms, but `http_get()` uses `read_to_string()` which blocks waiting for EOF/connection close. Chrome accepts `Connection: close` but does not close the TCP connection promptly, causing `read_to_string()` to block until the 5-second read timeout fires. The resulting `EAGAIN` (os error 35 on macOS) is treated as a fatal error, discarding the complete, valid response. |

### Error Output

```
[DEBUG http_get] connecting to 127.0.0.1:57360
[DEBUG http_get] connect failed: Connection refused (os error 61)  <- Chrome still starting
[DEBUG http_get] connecting to 127.0.0.1:57360
[DEBUG http_get] connected OK, sending request
[DEBUG http_get] read 555 bytes after 219.690084ms: HTTP/1.1 200 OK ...  <- Full response received!
[DEBUG http_get] rest-read EAGAIN, using partial response              <- read_to_string fails on EOF wait
```

The polling loop in `launcher.rs:181-202` retries every 100ms, but each failed `http_get` call takes 5 seconds (the read timeout), so only ~5-6 attempts happen in the 30-second timeout window.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Bug Is Fixed — Successful launch and connect

**Given** Chrome is installed and no existing connection session exists
**When** the user runs `chrome-cli connect --launch`
**Then** Chrome is launched, the DevTools port is detected, a session is saved, and connection metadata is printed as JSON with exit code 0

### AC2: No Regression — Headless mode still works

**Given** Chrome is installed
**When** the user runs `chrome-cli connect --launch --headless`
**Then** Chrome is launched in headless mode and the connection succeeds within the default 30-second timeout

### AC3: http_get handles Chrome's delayed connection close

**Given** Chrome's DevTools HTTP server sends a complete HTTP response with `Content-Length` header but does not immediately close the TCP connection
**When** `http_get` reads the response
**Then** the response body is returned successfully based on `Content-Length` rather than waiting for EOF/connection close

### AC4: Cross-platform HTTP response reading

**Given** Chrome's DevTools HTTP behavior may differ across macOS, Linux, and Windows
**When** `http_get` reads from the DevTools server on any supported platform
**Then** the response is correctly parsed regardless of whether the server closes the connection immediately, delays, or keeps it alive

### AC5: Graceful timeout with actionable error message

**Given** Chrome is launched but its DevTools server never becomes ready (e.g., port conflict, crash)
**When** the startup timeout expires
**Then** an error message is printed that includes the port number and suggests using `--timeout` to increase the wait time or `--headless` as an alternative

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Replace `read_to_string` in `http_get` with Content-Length-aware reading that does not depend on EOF | Must |
| FR2 | Parse HTTP response headers to extract `Content-Length` and read exactly that many body bytes | Must |
| FR3 | Handle chunked transfer encoding if Chrome ever uses it (defensive) | Should |
| FR4 | Improve timeout error message to suggest `--timeout` and `--headless` flags | Could |

---

## Out of Scope

- Switching to a full HTTP client library (e.g., `reqwest`) — the hand-rolled HTTP client is intentional to keep dependencies minimal
- Changing Chrome launch arguments or process management
- Supporting non-Chrome browsers
- Adding integration tests that require a real Chrome instance (separate issue)

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
