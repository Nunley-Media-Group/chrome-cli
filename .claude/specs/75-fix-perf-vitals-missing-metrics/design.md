# Root Cause Analysis: perf vitals returns only URL with no performance metrics

**Issue**: #75
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude

---

## Root Cause

The bug has two independent contributing causes that combine to produce the observed behavior:

**1. Timing — insufficient post-load delay before stopping the trace.** The `execute_vitals` function (line 757) waits only for `Page.loadEventFired` before immediately stopping the trace. However, Chrome's Largest Contentful Paint (LCP) is not finalized until the page is "settled" — typically a few seconds after the load event. LCP candidate events may still be emitted after `loadEventFired`, and CLS layout shifts can continue after load. By stopping the trace immediately, the trace file may not contain the `largestContentfulPaint::Candidate` or `LayoutShift` events needed by the extraction functions.

**2. Null field omission — `skip_serializing_if` hides missing metrics.** Both `CoreWebVitals` (line 49–57) and `PerfVitalsResult` (line 59–68) use `#[serde(skip_serializing_if = "Option::is_none")]` on all metric fields. When the extraction functions return `None` (because events weren't found in the trace), the serializer omits these fields entirely from the JSON output. The result is a JSON object containing only `"url"`, with no indication that metrics were attempted but unavailable. Additionally, `parse_trace_vitals` failures are silently swallowed by `.unwrap_or(CoreWebVitals { ... all None })` on line 766, hiding any trace parsing errors.

Together: the trace ends too early → extraction functions find no events → all metrics are `None` → serializer omits all `None` fields → user sees only `{"url": "..."}` with exit code 0.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/perf.rs` | 49–68 | `CoreWebVitals` and `PerfVitalsResult` struct definitions with `skip_serializing_if` |
| `src/perf.rs` | 757 | `wait_for_event` with immediate trace stop — no post-load delay |
| `src/perf.rs` | 766 | `parse_trace_vitals` error silently swallowed by `unwrap_or` |
| `src/perf.rs` | 425–460 | `extract_lcp` — requires `largestContentfulPaint::Candidate` events |
| `src/perf.rs` | 462–482 | `extract_cls` — requires `LayoutShift` events |
| `src/perf.rs` | 496–533 | `extract_ttfb` — requires `ResourceSendRequest` + `ResourceReceiveResponse` pair |
| `src/perf.rs` | 844–860 | `format_vitals_plain` — silently omits None metrics from plain output |

### Triggering Conditions

- The page has already loaded before `perf vitals` starts (common in interactive use)
- The trace is stopped immediately after `Page.loadEventFired` with no settling delay
- Chrome does not emit LCP/CLS/TTFB trace events within the narrow capture window
- The `skip_serializing_if` attribute masks the absence of metrics from the user

---

## Fix Strategy

### Approach

The fix addresses both root causes with minimal, targeted changes:

1. **Add a post-load stabilization delay.** After receiving `Page.loadEventFired`, wait an additional ~3 seconds before stopping the trace. This gives Chrome time to finalize LCP candidates, flush layout shift scores, and complete network timing events. The delay value should be a constant (e.g., `POST_LOAD_SETTLE_MS = 3000`) to make it tunable later if needed.

2. **Remove `skip_serializing_if` from metric fields.** Both `CoreWebVitals` and `PerfVitalsResult` should always serialize all metric fields. When a metric is `None`, it will appear as `null` in JSON output rather than being omitted. This gives consumers a clear signal that the metric was attempted but not collected.

3. **Add fallback TTFB extraction.** If `ResourceSendRequest`/`ResourceReceiveResponse` pairs are not found, fall back to computing TTFB from `navigationStart` and `responseStart` events in `blink.user_timing` or the Navigation Timing API entries that Chrome traces emit.

4. **Warn and exit non-zero when all metrics are null.** After parsing, if all three metrics remain `None`, print a diagnostic warning to stderr and exit with a non-zero exit code.

5. **Update plain text formatting.** `format_vitals_plain` should show "N/A" for `None` metrics instead of silently omitting them, maintaining consistent output structure.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/perf.rs` (lines 49–68) | Remove `#[serde(skip_serializing_if = "Option::is_none")]` from `lcp_ms`, `cls`, `ttfb_ms` on both `CoreWebVitals` and `PerfVitalsResult` | Ensures null metrics are always visible in JSON output |
| `src/perf.rs` (line 757) | Add `tokio::time::sleep(Duration::from_millis(POST_LOAD_SETTLE_MS))` after `wait_for_event` and before stopping the trace | Gives Chrome time to finalize LCP/CLS/TTFB events |
| `src/perf.rs` (new constant) | Add `const POST_LOAD_SETTLE_MS: u64 = 3000;` | Configurable settling delay |
| `src/perf.rs` (lines 496–533) | Add fallback TTFB extraction using `navigationStart`/`responseStart` timing events when ResourceSendRequest/ResourceReceiveResponse are absent | Improves TTFB reliability on pages with cached responses |
| `src/perf.rs` (lines 766–780) | After parsing vitals, check if all metrics are `None`; if so, eprintln a warning and return a non-zero exit code | Prevents silent false-success when no metrics are collected |
| `src/perf.rs` (lines 844–860) | Update `format_vitals_plain` to print "LCP: N/A", "CLS: N/A", "TTFB: N/A" for None values instead of omitting them | Consistent plain text output regardless of metric availability |

### Blast Radius

- **Direct impact**: `execute_vitals`, `CoreWebVitals`, `PerfVitalsResult`, `format_vitals_plain`, `extract_ttfb`
- **Indirect impact**: `perf stop` also uses `CoreWebVitals` in its `PerfStopResult` output — removing `skip_serializing_if` there will change its JSON schema (null fields now appear). This is a positive change for consistency but should be noted.
- **Risk level**: Low — changes are confined to the `perf` command module. No CDP protocol changes, no CLI argument changes. The JSON schema change (null fields appearing) is additive and non-breaking for consumers that handle optional fields.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Post-load delay makes `perf vitals` slower for all pages | Medium | 3 seconds is acceptable for a measurement command; document the delay. The alternative (no delay) produces incorrect results. |
| Removing `skip_serializing_if` changes JSON output for `perf stop` as well | Low | Both commands benefit from consistent null-visible output. Consumers should already handle optional fields. |
| Fallback TTFB extraction produces less accurate values than request/response pairs | Low | Fallback is only used when primary extraction fails. Values from `navigationStart`/`responseStart` are the standard Web Performance API definitions. |
| Non-zero exit code on missing metrics breaks scripts expecting exit 0 | Low | Only triggers when ALL metrics are null, which previously returned misleading data anyway. Scripts already needed fixing. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Inject PerformanceObserver JavaScript before reload | Register JS observers to capture LCP/CLS/FID in-page, extract via `Runtime.evaluate` | More invasive change; conflicts with the trace-based architecture used by `perf start`/`perf stop`; introduces JS injection dependency |
| Use CDP Performance domain (`Performance.getMetrics`) | Query Chrome's built-in performance metrics API | Does not provide LCP, CLS, or TTFB — only low-level counters like JSHeapUsedSize |
| Make the delay user-configurable via `--settle-delay` flag | Allow users to tune the post-load wait | Over-engineering for a bug fix; a sensible default is sufficient. Can be added later as a feature if needed. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
