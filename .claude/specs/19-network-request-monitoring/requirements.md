# Requirements: Network Request Monitoring

**Issue**: #19
**Date**: 2026-02-14
**Status**: Approved
**Author**: Claude (spec generation)

---

## User Story

**As a** developer / automation engineer
**I want** to monitor and inspect HTTP network requests from the command line
**So that** I can debug network issues, audit API calls, and automate network analysis in scripts and CI pipelines

---

## Background

The chrome-cli tool exposes Chrome DevTools Protocol functionality via a CLI. Network monitoring is a core debugging capability — developers frequently need to see what requests a page makes, inspect headers and bodies, and filter by resource type or status code. The MCP server already provides `list_network_requests` and `get_network_request` tools; this feature brings equivalent (and extended) functionality to the CLI with an additional real-time streaming mode (`network follow`).

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: List network requests (happy path)

**Given** Chrome is running with a page loaded that has made network requests
**When** I run `chrome-cli network list`
**Then** I receive a JSON array of network request summaries
**And** each entry contains `id`, `method`, `url`, `status`, `type`, `size`, `duration_ms`, and `timestamp`

### AC2: List network requests targeting a specific tab

**Given** Chrome is running with multiple tabs open
**When** I run `chrome-cli network list --tab <ID>`
**Then** network requests are listed only from the specified tab

### AC3: Filter by resource type

**Given** Chrome is running with a page that has loaded various resource types
**When** I run `chrome-cli network list --type xhr,fetch`
**Then** only requests of type `xhr` or `fetch` are returned

### AC4: Filter by URL pattern

**Given** Chrome is running with a page that has made requests to various URLs
**When** I run `chrome-cli network list --url "api.example.com"`
**Then** only requests whose URL contains the substring `api.example.com` are returned

### AC5: Filter by HTTP status code (exact)

**Given** Chrome is running with a page that has responses with various status codes
**When** I run `chrome-cli network list --status 404`
**Then** only requests with HTTP status 404 are returned

### AC6: Filter by HTTP status code (wildcard)

**Given** Chrome is running with a page that has responses with various status codes
**When** I run `chrome-cli network list --status 4xx`
**Then** only requests with HTTP status codes 400-499 are returned

### AC7: Filter by HTTP method

**Given** Chrome is running with a page that has made GET and POST requests
**When** I run `chrome-cli network list --method POST`
**Then** only POST requests are returned

### AC8: Pagination

**Given** Chrome is running with a page that has made more than 50 requests
**When** I run `chrome-cli network list --limit 20 --page 1`
**Then** requests 20-39 are returned (second page of 20)

### AC9: Include preserved requests from previous navigations

**Given** Chrome is running and a navigation has occurred (clearing current requests)
**When** I run `chrome-cli network list --include-preserved`
**Then** requests from both before and after the navigation are included

### AC10: Get detailed network request info

**Given** Chrome is running with a completed network request with ID 5
**When** I run `chrome-cli network get 5`
**Then** I receive JSON with full request details (method, URL, headers, body)
**And** full response details (status, headers, body)
**And** timing information (DNS, connect, TLS, TTFB, download)

### AC11: Get request with redirect chain

**Given** Chrome is running with a network request that was redirected
**When** I run `chrome-cli network get <ID>`
**Then** the response includes a `redirect_chain` array with each hop

### AC12: Save request body to file

**Given** Chrome is running with a POST request that has a body
**When** I run `chrome-cli network get <ID> --save-request /tmp/req.txt`
**Then** the request body is saved to `/tmp/req.txt`

### AC13: Save response body to file

**Given** Chrome is running with a completed request that has a response body
**When** I run `chrome-cli network get <ID> --save-response /tmp/resp.json`
**Then** the response body is saved to `/tmp/resp.json`

### AC14: Large body truncation in inline output

**Given** Chrome is running with a response body larger than 10,000 characters
**When** I run `chrome-cli network get <ID>`
**Then** the inline body is truncated to 10,000 characters
**And** a `truncated` field is set to `true`

### AC15: Binary response handling

**Given** Chrome is running with a network request for a binary resource (e.g., image)
**When** I run `chrome-cli network get <ID>`
**Then** the body is not inlined
**And** the response indicates `binary: true`

### AC16: Stream network requests in real-time (follow)

**Given** Chrome is running with a page open
**When** I run `chrome-cli network follow`
**And** the page makes new network requests
**Then** each request is printed as a JSON line as it completes
**And** each line contains `method`, `url`, `status`, `size`, `duration_ms`

### AC17: Follow with filters

**Given** Chrome is running with a page open
**When** I run `chrome-cli network follow --type xhr --method POST`
**And** the page makes XHR POST and GET requests
**Then** only the XHR POST requests appear in the stream

### AC18: Follow with URL filter

**Given** Chrome is running with a page open
**When** I run `chrome-cli network follow --url "api/"`
**And** the page makes requests to various URLs
**Then** only requests with URLs containing `api/` appear in the stream

### AC19: Follow with timeout

**Given** Chrome is running with a page open
**When** I run `chrome-cli network follow --timeout 5000`
**Then** the stream exits automatically after 5 seconds

### AC20: Follow exits on Ctrl+C

**Given** Chrome is running and `chrome-cli network follow` is active
**When** I send SIGINT (Ctrl+C)
**Then** the command exits cleanly with exit code 0

### AC21: Follow verbose mode

**Given** Chrome is running with a page open
**When** I run `chrome-cli network follow --verbose`
**And** the page makes network requests
**Then** each streamed event includes request and response headers

### AC22: No requests available

**Given** Chrome is running with a freshly loaded blank page
**When** I run `chrome-cli network list`
**Then** an empty JSON array `[]` is returned

### AC23: Request not found

**Given** Chrome is running
**When** I run `chrome-cli network get 99999`
**Then** an error is returned indicating the request was not found
**And** the exit code is non-zero

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `network list` returns paginated, filtered request summaries | Must | Core listing functionality |
| FR2 | `network get <ID>` returns full request/response details | Must | Detailed inspection |
| FR3 | `network follow` streams requests in real-time | Must | Streaming/tail mode |
| FR4 | Filtering by resource type (--type) | Must | Comma-separated, matches CDP resource types |
| FR5 | Filtering by URL pattern (--url) | Must | Substring match |
| FR6 | Filtering by HTTP status code (--status) | Must | Exact or wildcard (4xx) |
| FR7 | Filtering by HTTP method (--method) | Must | GET, POST, PUT, etc. |
| FR8 | Pagination (--limit, --page) | Must | Default limit 50, 0-based pages |
| FR9 | Save request/response bodies to file | Should | --save-request, --save-response |
| FR10 | Body truncation at 10,000 chars inline | Must | Matches MCP server limit |
| FR11 | Binary response detection | Must | Mark as binary, don't inline |
| FR12 | Redirect chain in detailed view | Should | Array of redirect hops |
| FR13 | Timing breakdown in detailed view | Should | DNS, connect, TLS, TTFB, download |
| FR14 | Navigation-aware request segmentation | Must | Track which navigation context a request belongs to |
| FR15 | --include-preserved flag for previous navigations | Must | Default: only current navigation |
| FR16 | --verbose flag for follow mode | Could | Include headers in stream |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | `network list` should respond within 200ms for up to 1000 buffered requests |
| **Memory** | Request buffer should not grow unbounded; cap at configurable max (default 1000) |
| **Security** | No data leaves localhost; all CDP connections local only |
| **Reliability** | Graceful handling of disconnected tabs, missing request data |
| **Platforms** | macOS, Linux, Windows (per product.md) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--tab` | string | Valid tab target ID | No |
| `--type` | string | Comma-separated list from valid resource types | No |
| `--url` | string | Non-empty substring pattern | No |
| `--status` | string | Integer (200) or wildcard (4xx) | No |
| `--method` | string | Valid HTTP method | No |
| `--limit` | integer | Positive integer | No (default 50) |
| `--page` | integer | Non-negative integer | No (default 0) |
| `--timeout` | integer | Positive integer (milliseconds) | No |
| `--save-request` | path | Valid writable file path | No |
| `--save-response` | path | Valid writable file path | No |
| `<REQ_ID>` | integer | Non-negative integer | Yes (for `get`) |

### Output Data — `network list`

| Field | Type | Description |
|-------|------|-------------|
| `id` | integer | Unique request identifier within session |
| `method` | string | HTTP method (GET, POST, etc.) |
| `url` | string | Full request URL |
| `status` | integer | HTTP response status code |
| `type` | string | Resource type (document, xhr, fetch, etc.) |
| `size` | integer | Response body size in bytes |
| `duration_ms` | float | Total request duration in milliseconds |
| `timestamp` | string | ISO 8601 timestamp of request initiation |

### Output Data — `network get`

| Field | Type | Description |
|-------|------|-------------|
| `id` | integer | Request identifier |
| `request.method` | string | HTTP method |
| `request.url` | string | Full URL |
| `request.headers` | object | Request headers |
| `request.body` | string/null | Request body (POST/PUT) |
| `response.status` | integer | HTTP status code |
| `response.status_text` | string | Status text |
| `response.headers` | object | Response headers |
| `response.body` | string/null | Response body (truncated or null if binary) |
| `response.binary` | boolean | Whether response is binary |
| `response.truncated` | boolean | Whether body was truncated |
| `timing.dns_ms` | float | DNS resolution time |
| `timing.connect_ms` | float | TCP connection time |
| `timing.tls_ms` | float | TLS handshake time |
| `timing.ttfb_ms` | float | Time to first byte |
| `timing.download_ms` | float | Download time |
| `redirect_chain` | array | Array of redirect entries |

---

## Dependencies

### Internal Dependencies
- [x] #4 — CDP client (merged)
- [x] #6 — Session management (merged)

### External Dependencies
- Chrome/Chromium with CDP enabled (Network domain)

### Blocked By
- None (all dependencies resolved)

---

## Out of Scope

- Network request interception / modification (future feature)
- WebSocket frame inspection (frames within WebSocket connections)
- Certificate/TLS details beyond timing
- HAR export format
- Request replay/resend functionality
- Bandwidth throttling (covered by `emulate` command)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| All BDD scenarios pass | 100% | `cargo test --test bdd` |
| Response time for list | < 200ms | Benchmark with 100 buffered requests |
| Clippy clean | 0 warnings | `cargo clippy` |

---

## Open Questions

- None; requirements are well-defined from the GitHub issue and MCP reference.

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Edge cases and error states specified
- [x] Dependencies identified
- [x] Out of scope defined
