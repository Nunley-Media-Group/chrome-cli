# Defect Report: perf vitals returns only URL with no performance metrics

**Issue**: #75
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude
**Severity**: High
**Related Spec**: N/A

---

## Reproduction

### Steps to Reproduce

1. Launch Chrome via `chrome-cli` or connect to a running instance.
2. Navigate to `https://www.google.com/`.
3. Wait for the page to fully load.
4. Run `chrome-cli perf vitals --pretty`.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS / Linux / Windows (all platforms) |
| **Version / Commit** | Current `main` branch |
| **Browser / Runtime** | Chrome/Chromium with CDP enabled |
| **Configuration** | Default settings, no special flags |

### Frequency

Always — reproducible on any real website where the page has finished loading before `perf vitals` is invoked.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `perf vitals` returns a JSON object with `url`, `lcp_ms`, `cls`, and `ttfb_ms` fields populated with numeric values. If any metric cannot be collected, it should appear as `null` in the output and the command should exit with a non-zero status code. |
| **Actual** | The command returns a JSON object containing only the `url` field. The `lcp_ms`, `cls`, and `ttfb_ms` fields are completely absent (not even `null`). The exit code is 0, falsely indicating success. |

### Error Output

```json
{
  "url": "https://www.google.com/..."
}
```

No error message is printed to stderr. Exit code is 0.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Vitals metrics are present in output after page reload

**Given** Chrome is connected and navigated to a real website (e.g., `https://www.google.com/`)
**When** `perf vitals` is executed
**Then** the JSON output contains `lcp_ms` and `ttfb_ms` fields with numeric values
**And** the `cls` field is present (numeric value or `null`)

**Example**:
- Given: Chrome is on `https://www.google.com/`
- When: `perf vitals` is run
- Then: Output is `{"url": "https://www.google.com/", "lcp_ms": 450.2, "cls": 0.01, "ttfb_ms": 120.5}`

### AC2: Null metrics are serialized explicitly rather than omitted

**Given** Chrome is connected to a page where a specific metric cannot be collected (e.g., no layout shifts occur, so CLS has no data)
**When** `perf vitals` is executed
**Then** uncollectable metrics appear as `null` in the JSON output instead of being omitted
**And** the output always contains all three metric fields (`lcp_ms`, `cls`, `ttfb_ms`)

### AC3: Non-zero exit code when critical metrics are missing

**Given** Chrome is connected to a page where LCP and TTFB cannot be collected
**When** `perf vitals` is executed
**Then** the command exits with a non-zero status code
**And** a warning message is printed to stderr indicating which metrics could not be collected

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Remove `skip_serializing_if = "Option::is_none"` from `PerfVitalsResult` and `CoreWebVitals` metric fields so `null` values are always serialized in JSON output | Must |
| FR2 | Add a post-load stabilization delay after `Page.loadEventFired` to allow LCP finalization and layout shift settling before stopping the trace | Must |
| FR3 | Implement fallback TTFB extraction using `navigationStart` and `responseStart` timing from `blink.user_timing` or Navigation Timing events when `ResourceSendRequest`/`ResourceReceiveResponse` pairs are not found | Should |
| FR4 | Exit with a non-zero status code and emit a stderr warning when all three vitals metrics are `null` after trace parsing | Should |
| FR5 | Update `format_vitals_plain` to display `null` metrics as "N/A" instead of silently omitting them | Should |

---

## Out of Scope

- Adding new web vitals metrics beyond LCP, CLS, and TTFB (e.g., FID, INP, FCP)
- Injecting `PerformanceObserver` JavaScript into the page (the trace-based approach is the intended architecture)
- Changing the `perf start` / `perf stop` flow (only `perf vitals` is affected)
- Refactoring the trace parsing infrastructure beyond what is needed to fix this bug

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC2 — null serialization)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
