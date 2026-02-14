# Design: Network Request Monitoring

**Issue**: #19
**Date**: 2026-02-14
**Status**: Approved
**Author**: Claude (spec generation)

---

## Overview

This feature adds a `network` subcommand group to chrome-cli with three subcommands: `list`, `get`, and `follow`. The implementation subscribes to CDP Network domain events, correlates request/response pairs by `requestId`, and provides filtering, pagination, and real-time streaming. The architecture follows the established command module pattern (identical to `console.rs`) with output types, a dispatcher function, and per-subcommand implementations.

The key technical challenge is correlating multiple asynchronous CDP events (`requestWillBeSent`, `responseReceived`, `loadingFinished`, `loadingFailed`) into coherent `NetworkRequest` objects. A `HashMap<String, NetworkRequest>` keyed by CDP `requestId` accumulates partial state as events arrive, with navigation tracking to segment requests by page load.

---

## Architecture

### Component Diagram

```
CLI Layer (src/cli/mod.rs)
  └─ NetworkArgs { command: NetworkCommand }
     ├─ List(NetworkListArgs)    → filters, pagination
     ├─ Get(NetworkGetArgs)      → request ID, save paths
     └─ Follow(NetworkFollowArgs) → filters, timeout, verbose

Command Layer (src/network.rs)
  └─ execute_network(global, args)
     ├─ execute_list()   → drain events, correlate, filter, paginate, output JSON array
     ├─ execute_get()    → drain events, find by ID, fetch bodies, output detail JSON
     └─ execute_follow() → streaming loop, filter, output JSON lines

CDP Layer (src/connection.rs — ManagedSession)
  └─ ensure_domain("Network")
  └─ subscribe("Network.requestWillBeSent")
  └─ subscribe("Network.responseReceived")
  └─ subscribe("Network.loadingFinished")
  └─ subscribe("Network.loadingFailed")
  └─ subscribe("Page.frameNavigated")
  └─ send_command("Network.getResponseBody", ...)
  └─ send_command("Network.getRequestPostData", ...)
```

### Data Flow

#### `network list` / `network get`

```
1. Setup session → enable Network + Page domains
2. Subscribe to Network.requestWillBeSent, responseReceived, loadingFinished, loadingFailed, Page.frameNavigated
3. Drain events with short timeout (100ms idle window, like console read)
4. Correlate events by requestId into HashMap<String, NetworkRequestBuilder>
5. For list: filter → paginate → serialize as JSON array
6. For get: find by assigned numeric ID → fetch bodies via CDP → serialize detail JSON
```

#### `network follow`

```
1. Setup session → enable Network + Page domains
2. Subscribe to the same Network events
3. Enter tokio::select! loop (identical pattern to console follow)
4. As loadingFinished/loadingFailed arrives, build completed request entry
5. Apply filters → print JSON line → flush stdout
6. Exit on Ctrl+C or --timeout
```

---

## API / Interface Changes

### New CLI Commands

| Command | Type | Purpose |
|---------|------|---------|
| `chrome-cli network list` | GET-like | List network request summaries |
| `chrome-cli network get <REQ_ID>` | GET-like | Get detailed request/response info |
| `chrome-cli network follow` | Stream | Real-time network request stream |

### CLI Arguments

#### `network list`

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--tab <ID>` | string | active tab | Target tab (global opt) |
| `--type <TYPES>` | string | all | Comma-separated resource types |
| `--url <PATTERN>` | string | none | URL substring filter |
| `--status <CODE>` | string | none | Status code filter (exact or wildcard like `4xx`) |
| `--method <METHOD>` | string | none | HTTP method filter |
| `--limit <N>` | usize | 50 | Max results per page |
| `--page <N>` | usize | 0 | Pagination page (0-based) |
| `--include-preserved` | bool | false | Include previous navigation requests |

#### `network get <REQ_ID>`

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<REQ_ID>` | u64 | required | Numeric request ID |
| `--save-request <PATH>` | PathBuf | none | Save request body to file |
| `--save-response <PATH>` | PathBuf | none | Save response body to file |

#### `network follow`

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--type <TYPES>` | string | all | Comma-separated resource types |
| `--url <PATTERN>` | string | none | URL substring filter |
| `--method <METHOD>` | string | none | HTTP method filter |
| `--timeout <MS>` | u64 | none | Auto-exit after N ms |
| `--verbose` | bool | false | Include headers in stream |

### Output Schemas

#### `network list` — JSON array

```json
[
  {
    "id": 1,
    "method": "GET",
    "url": "https://example.com/api/data",
    "status": 200,
    "type": "xhr",
    "size": 1234,
    "duration_ms": 45.2,
    "timestamp": "2026-02-14T12:00:00.000Z"
  }
]
```

#### `network get <ID>` — JSON object

```json
{
  "id": 1,
  "request": {
    "method": "POST",
    "url": "https://example.com/api/data",
    "headers": { "Content-Type": "application/json" },
    "body": "{\"key\": \"value\"}"
  },
  "response": {
    "status": 200,
    "status_text": "OK",
    "headers": { "Content-Type": "application/json" },
    "body": "{\"result\": \"ok\"}",
    "binary": false,
    "truncated": false,
    "mime_type": "application/json"
  },
  "timing": {
    "dns_ms": 5.0,
    "connect_ms": 10.0,
    "tls_ms": 15.0,
    "ttfb_ms": 50.0,
    "download_ms": 20.0
  },
  "redirect_chain": [
    {
      "url": "http://example.com/api/data",
      "status": 301
    }
  ],
  "type": "xhr",
  "size": 1234,
  "duration_ms": 100.2,
  "timestamp": "2026-02-14T12:00:00.000Z"
}
```

#### `network follow` — JSON lines (one per completed request)

```json
{"method":"GET","url":"https://example.com/api","status":200,"type":"xhr","size":1234,"duration_ms":45.2,"timestamp":"2026-02-14T12:00:00.000Z"}
```

With `--verbose`:
```json
{"method":"GET","url":"https://example.com/api","status":200,"type":"xhr","size":1234,"duration_ms":45.2,"timestamp":"2026-02-14T12:00:00.000Z","request_headers":{"Accept":"*/*"},"response_headers":{"Content-Type":"application/json"}}
```

### Errors

| Condition | Error Message | Exit Code |
|-----------|---------------|-----------|
| Request ID not found | `"Network request {id} not found"` | GeneralError (1) |
| CDP connection closed | `"CDP connection closed"` | ConnectionError (2) |
| File write failure | `"Failed to write to {path}: {err}"` | GeneralError (1) |
| No network data | Empty array `[]` (not an error) | Success (0) |

---

## State Management

### In-Memory Request Tracking

```rust
/// Builder for accumulating network request data from multiple CDP events.
struct NetworkRequestBuilder {
    cdp_request_id: String,       // CDP-assigned request ID
    assigned_id: usize,           // Sequential numeric ID for CLI
    method: String,               // From requestWillBeSent
    url: String,                  // From requestWillBeSent
    resource_type: String,        // From requestWillBeSent
    timestamp: f64,               // From requestWillBeSent (epoch ms)
    request_headers: Option<Map>, // From requestWillBeSent
    post_data: Option<String>,    // Lazily fetched via getRequestPostData
    status: Option<u16>,          // From responseReceived
    status_text: Option<String>,  // From responseReceived
    response_headers: Option<Map>,// From responseReceived
    mime_type: Option<String>,    // From responseReceived
    encoded_data_length: Option<u64>, // From loadingFinished
    timing: Option<ResourceTiming>, // From responseReceived.response.timing
    redirect_chain: Vec<RedirectEntry>, // Accumulated from redirects
    completed: bool,              // Set by loadingFinished
    failed: bool,                 // Set by loadingFailed
    error_text: Option<String>,   // From loadingFailed
    navigation_id: u32,           // Which navigation context this belongs to
}
```

### Navigation Tracking

Navigation changes are tracked by subscribing to `Page.frameNavigated` and incrementing a navigation counter. Each request is tagged with the navigation ID active when it was initiated. The `--include-preserved` flag controls whether requests from previous navigations are included in results.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Fetch all via CDP command** | Use a single CDP command to get network logs | Simpler | CDP has no such command; Network domain is event-based | Rejected |
| **B: Event-based accumulation** | Subscribe to Network.* events, accumulate in HashMap | Matches CDP architecture, proven pattern in navigate.rs | More complex correlation logic | **Selected** |
| **C: HAR format output** | Output in HTTP Archive format | Standard format | Over-engineered for CLI, adds complexity | Rejected — out of scope |

---

## Security Considerations

- [x] **No authentication needed**: Local CDP only (per product.md)
- [x] **Input validation**: File paths validated before write; request ID validated as numeric
- [x] **Data sanitization**: Response bodies are raw from Chrome; no user-controlled data injected into commands
- [x] **Sensitive data**: Request/response bodies may contain sensitive data; only written to user-specified paths on explicit --save-* flags

---

## Performance Considerations

- [x] **Event draining**: Short timeout (100ms idle) for list/get to collect buffered events, matching console read approach
- [x] **Pagination**: Default limit of 50 prevents large JSON outputs
- [x] **Body fetching**: Bodies are only fetched on `network get` (not list), avoiding unnecessary CDP calls
- [x] **Streaming**: Follow mode flushes stdout after each line for real-time piping
- [x] **Memory**: Request builders are dropped after output; follow mode only keeps in-flight requests

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Output types | Unit | Serialization of NetworkRequest, NetworkRequestDetail |
| Filtering | Unit | Type/URL/status/method filter functions |
| Pagination | Unit | Page boundary calculation |
| Status wildcard | Unit | Parsing `4xx` → range 400-499 |
| Event correlation | Unit | Building request from multiple events |
| CLI arguments | BDD | Argument parsing and validation |
| End-to-end | BDD | Full command execution with Chrome |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Events arrive out of order | Medium | Medium | HashMap builder tolerates any event order; fields are Optional |
| Very high request volume | Low | Medium | Pagination + limit cap; follow mode has no buffer |
| Binary body detection | Low | Low | Check MIME type; CDP returns base64 flag |
| CDP request ID collisions on redirect | Medium | Medium | Track redirect chain; use final request ID |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (console.rs pattern)
- [x] All API/interface changes documented with schemas
- [x] State management approach is clear (HashMap builder pattern)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
