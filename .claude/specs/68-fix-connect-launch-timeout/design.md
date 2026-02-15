# Root Cause Analysis: http_get read_to_string blocks waiting for EOF that Chrome never sends

**Issue**: #68
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude

---

## Root Cause

The `http_get()` function in `src/chrome/discovery.rs` uses `stream.read_to_string(&mut response)` (line 168) to read the entire HTTP response. This call blocks until one of two conditions: the remote end closes the connection (EOF), or the 5-second read timeout fires.

Chrome's DevTools HTTP server sends the `Connection: close` response header but does **not** promptly close the TCP connection after sending the response body. This means `read_to_string()` successfully reads the complete HTTP response (headers + body, typically ~555 bytes in <250ms) into the buffer, then continues blocking on the next `read()` syscall waiting for more data that will never arrive.

When the 5-second read timeout fires, the OS returns `EAGAIN` (error 35 on macOS, `WouldBlock` on other platforms). `read_to_string()` treats this as an `Err`, and `http_get()` maps it to `ChromeError::HttpError`, discarding the complete valid response that was already buffered. The polling loop in `launch_chrome()` (lines 181–202) retries every 100ms, but since each failed `http_get()` call takes 5 seconds, only ~5–6 attempts fit in the 30-second default timeout — all of which fail the same way.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/chrome/discovery.rs` | 146–192 | `http_get()` — the buggy function that reads the HTTP response |
| `src/chrome/discovery.rs` | 57–59 | `query_version()` — calls `http_get()` |
| `src/chrome/discovery.rs` | 69–71 | `query_targets()` — calls `http_get()` |
| `src/chrome/launcher.rs` | 181–202 | `launch_chrome()` polling loop — retries `query_version()` |

### Triggering Conditions

- Chrome's DevTools HTTP server sends a complete response but delays closing the TCP connection
- `read_to_string()` blocks past the first successful read, waiting for EOF
- The read timeout (5 seconds) fires, producing an error that discards the already-read data
- This is the **default behavior** on macOS and likely on all platforms

---

## Fix Strategy

### Approach

Replace the `read_to_string()` call with a chunked read loop that:

1. Reads data into a byte buffer incrementally using `stream.read()`
2. After each read, checks whether the header/body separator (`\r\n\r\n`) has been found
3. Once headers are complete, parses the `Content-Length` header value
4. Continues reading until exactly `Content-Length` body bytes have been received
5. Returns the body immediately — no waiting for EOF or connection close

For defensive robustness, if `Content-Length` is not present in the response headers, the function falls back to returning whatever body data was received after the headers (graceful degradation rather than blocking forever).

A read timeout error (`WouldBlock`/`EAGAIN`) that occurs **after** the complete body has been received is silently ignored, since the data is already in the buffer.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/chrome/discovery.rs` | Rewrite the response-reading portion of `http_get()` to use chunked reads with `Content-Length`-aware termination | Fixes the root cause: no longer depends on EOF to know when reading is complete |
| `src/chrome/error.rs` | Update `StartupTimeout` display to suggest `--timeout` and `--headless` | FR4: actionable error messages (Could priority) |

### Blast Radius

- **Direct impact**: `http_get()` in `src/chrome/discovery.rs` — internal function, not public API
- **Indirect impact**: `query_version()` and `query_targets()` both call `http_get()`. These are called by `discover_chrome()` and `launch_chrome()`. All callers benefit from the fix with no API changes.
- **Risk level**: Low — `http_get()` is a private helper with a simple contract (return the HTTP body as a `String`). The function signature, return type, and error semantics are unchanged. Only the internal reading strategy changes.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Misparse `Content-Length` from headers | Low | Unit test with realistic Chrome response headers; parse only the first `Content-Length` header value |
| Buffer too small for large responses | Low | Use a growable `Vec<u8>` buffer; Chrome DevTools responses are typically <2KB |
| Break on servers that close connection immediately (EOF before `Content-Length` bytes) | Low | The read loop handles both `Ok(0)` (EOF) and reaching `Content-Length` as termination conditions |
| Chunked transfer encoding not handled | Low | Chrome's DevTools HTTP server uses `Content-Length`, not chunked encoding. If encountered, fall back to reading until EOF/timeout (current behavior minus the error on timeout) |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Use `reqwest` or `ureq` HTTP client | Replace hand-rolled HTTP with a proper library | Out of scope per issue — project intentionally minimizes dependencies |
| Reduce read timeout from 5s to 500ms | Would fail faster, allowing more polling retries | Doesn't fix the root cause; would break on slow networks or large responses |
| Use non-blocking I/O with `mio` or `tokio::net::TcpStream` | Async-native TCP reading | Significant refactor of `http_get()`; the `spawn_blocking` approach is fine for this use case |
| Catch `WouldBlock` and return partial data | After timeout, check if buffer already has a complete response | Fragile — still wastes 5 seconds per attempt; doesn't solve the core problem |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
