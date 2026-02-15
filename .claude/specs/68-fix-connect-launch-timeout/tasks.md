# Tasks: Fix http_get read_to_string blocking on EOF

**Issue**: #68
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Rewrite `http_get()` with Content-Length-aware reading | [ ] |
| T002 | Improve `StartupTimeout` error message | [ ] |
| T003 | Add unit tests for the new HTTP response parsing | [ ] |
| T004 | Verify no regressions | [ ] |

---

### T001: Rewrite http_get() with Content-Length-aware reading

**File(s)**: `src/chrome/discovery.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `http_get()` reads response incrementally using `stream.read()` into a byte buffer
- [ ] After `\r\n\r\n` is found, `Content-Length` is parsed from response headers
- [ ] Exactly `Content-Length` bytes of body are read, then the function returns immediately
- [ ] If `Content-Length` is absent, falls back to returning whatever body data was received after headers
- [ ] `WouldBlock`/`EAGAIN` after complete body receipt is handled gracefully (not treated as error)
- [ ] EOF (`Ok(0)`) during body read is handled as an early termination condition
- [ ] Function signature and return type are unchanged
- [ ] `cargo clippy` passes with no new warnings

**Notes**: The core fix. Replace lines 166–169 (`read_to_string` block) with a loop that reads into a `Vec<u8>`, detects the header/body boundary, extracts `Content-Length`, and reads the exact body length. Keep the existing `spawn_blocking` wrapper, connect timeout, and write logic unchanged.

### T002: Improve StartupTimeout error message

**File(s)**: `src/chrome/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `StartupTimeout` display includes the port number (already does)
- [ ] Message suggests using `--timeout` to increase the wait time
- [ ] Message suggests `--headless` as an alternative
- [ ] Existing `display_startup_timeout` test is updated to match new message

**Notes**: FR4 (Could priority). Small change to the `Display` impl for `ChromeError::StartupTimeout`.

### T003: Add unit tests for the new HTTP response parsing

**File(s)**: `src/chrome/discovery.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] Unit test: response with `Content-Length` header returns correct body
- [ ] Unit test: response without `Content-Length` returns body after headers
- [ ] Unit test: response with empty body and `Content-Length: 0` returns empty string
- [ ] Unit test: malformed response (no `\r\n\r\n`) returns error
- [ ] All tests pass with `cargo test`

**Notes**: Extract the response-parsing logic into a testable helper function (e.g., `parse_http_response(raw: &[u8]) -> Result<String, ChromeError>`) so it can be unit tested without a real TCP connection. Keep the helper private to the module.

### T004: Verify no regressions

**File(s)**: Existing test files
**Type**: Verify (no file changes expected)
**Depends**: T001, T002, T003
**Acceptance**:
- [ ] `cargo test` — all existing tests pass
- [ ] `cargo clippy` — no new warnings
- [ ] `cargo fmt --check` — no formatting issues
- [ ] No side effects in `query_version()`, `query_targets()`, `discover_chrome()`, or `launch_chrome()`

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T003)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
