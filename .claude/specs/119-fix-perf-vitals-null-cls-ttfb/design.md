# Root Cause Analysis: perf vitals returns null for CLS and TTFB metrics

**Issue**: #119
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude

---

## Root Cause

There are two independent bugs in `src/perf.rs`, both in the trace event extraction logic:

**CLS Bug**: The `extract_cls()` function (lines 456-475) uses a `found` flag to track whether any `LayoutShift` events with `had_recent_input: false` were encountered. When no layout shifts occur at all (common on simple, well-optimized pages like google.com), the function returns `None`. This is semantically incorrect — zero layout shifts means a CLS score of `0.0`, not an unknown/missing value. The `found` flag conflates "no shifts happened" with "we couldn't measure CLS."

**TTFB Bug**: The `extract_ttfb()` function (lines 490-526) attempts to calculate TTFB by matching `ResourceSendRequest` and `ResourceReceiveResponse` trace events for the main document. It has a fallback in `extract_ttfb_fallback()` (lines 531-550) that looks for `blink.user_timing` events (`navigationStart` and `responseStart`). However, for cached responses, certain page loads, or when Chrome's trace categories don't capture these events, both paths return `None`. There is no third-level fallback using `Navigation Timing` performance entries, which Chrome reliably provides via `Performance.getMetrics` or the performance timeline.

Note: Issue #75 previously addressed the case where metrics were silently omitted from JSON output, ensuring `null` is serialized instead of the field being absent. However, it did not address the underlying extraction logic that produces `None` in the first place.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/perf.rs` | 456-475 | `extract_cls()` — returns `None` instead of `Some(0.0)` when no shifts found |
| `src/perf.rs` | 490-526 | `extract_ttfb()` — primary extraction from resource events fails for cached/missing events |
| `src/perf.rs` | 531-550 | `extract_ttfb_fallback()` — secondary extraction from blink.user_timing also fails |
| `src/perf.rs` | 400-416 | `parse_trace_vitals()` — caller that passes results through unchanged |

### Triggering Conditions

- **CLS**: Any page that loads without triggering a layout shift (no `LayoutShift` events in the trace, or all shifts have `had_recent_input: true`)
- **TTFB**: Pages served from cache, or trace sessions where Chrome doesn't emit `ResourceSendRequest`/`ResourceReceiveResponse` for the main document and also lacks `blink.user_timing` navigation events
- Both conditions are common on fast, well-optimized pages like google.com

---

## Fix Strategy

### Approach

**CLS Fix**: Remove the `found` flag from `extract_cls()` and always return `Some(total_cls)`. When no qualifying `LayoutShift` events exist, `total_cls` remains `0.0`, which is the correct CLS score for a page with no layout shifts. This is a one-line semantic change.

**TTFB Fix**: Add a third-level fallback to `extract_ttfb()` that uses `Performance.getMetrics` CDP call or, within the trace data, looks for `navigationStart` and `firstContentfulPaint` timing marks from the `blink.user_timing` category, or alternatively computes TTFB from the first `network.resourceTiming` entry. The most reliable approach is to use `navigationStart` as baseline and look for the first `ResourceReceiveResponse` event (regardless of request ID matching) as a floor estimate. If even that fails, use the difference between the trace start timestamp and the first paint event as an approximation.

The simplest and most reliable TTFB fallback is: if the primary and secondary extraction both fail, scan for `navigationStart` and compute against the first `ResourceReceiveResponse` for any resource (not just the document). This provides a conservative upper-bound TTFB that is better than `null`.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/perf.rs` (`extract_cls`) | Remove `found` flag; always return `Some(total_cls)` | Zero layout shifts = CLS 0.0, not unknown |
| `src/perf.rs` (`extract_ttfb` / `extract_ttfb_fallback`) | Add additional fallback using `navigationStart` + first `ResourceReceiveResponse` for any resource | Ensures TTFB is available when document-specific resource events are missing |
| `src/perf.rs` (unit tests) | Update existing `extract_cls` test expectations; add test for zero-shift case; add test for new TTFB fallback | Verify fix correctness |

### Blast Radius

- **Direct impact**: `extract_cls()` and `extract_ttfb()` / `extract_ttfb_fallback()` in `src/perf.rs`
- **Indirect impact**: `parse_trace_vitals()` which calls both functions; `execute_vitals()` and `stop_and_collect()` which consume the results. `CoreWebVitals`, `PerfVitalsResult`, and `PerfRecordResult` structs are unchanged.
- **Risk level**: Low — the change only affects the `None` → `Some(value)` transition for these two specific metrics. The `Option<f64>` type remains, so all downstream consumers already handle `Some`. No struct or API changes needed.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| CLS returns 0.0 for pages where CLS genuinely could not be measured (tracing failure) | Low | CLS of 0.0 is semantically correct if tracing ran but no shifts occurred. If tracing itself fails, `parse_trace_vitals` returns an error before reaching `extract_cls`. |
| TTFB fallback returns inaccurate value (e.g., sub-resource timing instead of document timing) | Low | The fallback is conservative. An approximate TTFB is more useful than `null` for the CLI's target users (AI agents, automation scripts). The primary and secondary extraction paths are unchanged. |
| LCP extraction is accidentally modified | Very Low | LCP code is in a separate function (`extract_lcp`) that is not modified. AC3 regression test covers this. |
| Existing unit tests break | Low | Only `extract_cls` tests that assert `None` for no-shift case need updating. All other tests remain valid. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Use `Performance.getMetrics` CDP call for TTFB | Issue a separate CDP command to get navigation timing after trace collection | Adds CDP call complexity and latency; trace-based extraction is preferred for consistency with LCP/CLS |
| Return `0.0` for TTFB when unavailable | Default to zero like CLS | TTFB of 0.0 is semantically wrong (it implies instant response); `null` is better than a false 0.0, but an actual fallback measurement is best |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
