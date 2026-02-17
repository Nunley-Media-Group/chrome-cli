# Root Cause Analysis: Network list showing size 0 for most requests

**Issue**: #117
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude

---

## Root Cause

The `size` field in network request output is populated exclusively from `encodedDataLength` in the CDP `Network.loadingFinished` event. This value represents bytes transferred over the wire, which Chrome reports as `0` when responses are served from the browser cache, when Chrome's Network domain cannot measure the transfer for certain request types, or when certain transfer encodings are used.

The response headers — including `content-length` — are already captured and stored in the `NetworkRequestBuilder.response_headers` field during the `Network.responseReceived` event handler (line 725 of `src/network.rs`). However, the code never consults this header as a fallback when `encodedDataLength` is 0 or absent.

The same pattern repeats in three output paths: `builder_to_summary` (list mode, line 773), the `NetworkRequestDetail` construction in `execute_get` (detail mode, line 980), and `emit_stream_event` in follow mode (line 1125). All three directly use `encodedDataLength` without fallback.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/network.rs` | 732–737 | `LoadingFinished` handler — captures `encodedDataLength` into `builder.encoded_data_length` |
| `src/network.rs` | 714–730 | `ResponseReceived` handler — captures `response_headers` (contains `content-length`) |
| `src/network.rs` | 762–777 | `builder_to_summary` — maps `encoded_data_length` directly to `size` (line 773) |
| `src/network.rs` | 960–983 | `execute_get` detail construction — maps `encoded_data_length` directly to `size` (line 980) |
| `src/network.rs` | 1119–1133 | Follow mode — reads `encodedDataLength` from event (line 1125), passes to `emit_stream_event` |

### Triggering Conditions

- A network request is served from the browser cache (most common trigger)
- Chrome's Network domain reports `encodedDataLength: 0` in the `loadingFinished` event
- The response included a valid `content-length` header, but it was never consulted

---

## Fix Strategy

### Approach

Add a helper function that resolves the effective size for a network request by checking `encodedDataLength` first, and falling back to parsing the `content-length` response header when the encoded length is 0 or absent. Apply this helper in all three output paths.

For the `NetworkRequestBuilder` (used by list and get modes), the helper takes `encoded_data_length: Option<u64>` and `response_headers: &serde_json::Value` and returns `Option<u64>`. It returns `encoded_data_length` if it is `Some(n)` where `n > 0`, otherwise parses `content-length` from the headers object (which is a JSON object with string keys/values from CDP).

For follow mode, the same logic applies: when the `loadingFinished` event yields `encodedDataLength` of 0, fall back to the `content-length` header stored in the `InFlightRequest.response_headers` field.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/network.rs` | Add `resolve_size(encoded_data_length: Option<u64>, response_headers: &serde_json::Value) -> Option<u64>` helper | Centralizes the fallback logic in one place |
| `src/network.rs` | Update `builder_to_summary` (line 773) to call `resolve_size` | Fixes `network list` output |
| `src/network.rs` | Update `execute_get` detail construction (line 980) to call `resolve_size` | Fixes `network get` output |
| `src/network.rs` | Update follow mode (line 1125–1134) to call `resolve_size` before passing size to `emit_stream_event` | Fixes `network follow` output |

### Blast Radius

- **Direct impact**: `src/network.rs` — three output paths modified, one helper added
- **Indirect impact**: None — the `size` field is a display-only value; no other code depends on it for logic
- **Risk level**: Low

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Requests with valid non-zero `encodedDataLength` show wrong size | Low | The helper returns `encodedDataLength` first when > 0; only falls back when 0/absent |
| `content-length` header parsing fails on malformed values | Low | Use `str::parse::<u64>().ok()` — returns `None` on parse failure, leaving `size` as `None` |
| CDP header keys are case-sensitive and miss `Content-Length` | Medium | CDP normalizes response header names to lowercase in the `headers` object, so `content-length` is correct. Add a case-insensitive search as defense. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Store `content-length` in a dedicated builder field during `ResponseReceived` | Parse header once, store as `Option<u64>` | Adds a struct field for a single fallback; the helper approach is simpler and keeps builder unchanged |
| Always prefer `content-length` over `encodedDataLength` | Simpler logic | `encodedDataLength` is more accurate when non-zero (accounts for compression), so it should remain the primary source |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
