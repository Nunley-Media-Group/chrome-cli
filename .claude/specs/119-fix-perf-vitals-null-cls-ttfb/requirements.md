# Defect Report: perf vitals returns null for CLS and TTFB metrics

**Issue**: #119
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude
**Severity**: High
**Related Spec**: `.claude/specs/22-performance-tracing/` and `.claude/specs/75-fix-perf-vitals-missing-metrics/`

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli navigate https://www.google.com`
3. `chrome-cli perf vitals`
4. Observe output: `{"url":"https://www.google.com/","lcp_ms":219.638,"cls":null,"ttfb_ms":null}`

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 |
| **Version / Commit** | 0.1.0 (commit 112e231) |
| **Browser / Runtime** | HeadlessChrome/144.0.0.0 |
| **Configuration** | Default (no config file) |

### Frequency

Always — reproduces 100% on pages with no layout shifts (CLS) and on pages where CDP resource tracing events are absent or cached (TTFB).

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | CLS should be `0.0` when no layout shifts occurred; TTFB should be a positive number for any successfully loaded page |
| **Actual** | CLS is `null` and TTFB is `null`, while LCP is correctly measured |

### Error Output

```json
{"url":"https://www.google.com/","lcp_ms":219.638,"cls":null,"ttfb_ms":null}
```

No error on stderr; exit code is 0. The command "succeeds" but returns incomplete data.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: CLS returns 0.0 when no layout shifts occur

**Given** a page with no layout shifts has been loaded (e.g., google.com)
**When** I run `chrome-cli perf vitals`
**Then** the `cls` field in the JSON output is `0.0` (not `null`)

### AC2: TTFB is measured for loaded pages

**Given** a successfully loaded page
**When** I run `chrome-cli perf vitals`
**Then** the `ttfb_ms` field in the JSON output is a positive number (not `null`)

### AC3: LCP continues to work correctly

**Given** a loaded page
**When** I run `chrome-cli perf vitals`
**Then** the `lcp_ms` field in the JSON output is a positive number
**And** the metric is not affected by the CLS/TTFB fixes

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `extract_cls()` must return `Some(0.0)` when no `LayoutShift` trace events are found (instead of `None`) | Must |
| FR2 | `extract_ttfb()` must have a reliable fallback mechanism that produces a value for any successfully loaded page, even when CDP resource tracing events and `blink.user_timing` events are both absent | Must |
| FR3 | Existing LCP extraction logic must remain unchanged | Must |

---

## Out of Scope

- Adding new Web Vitals metrics beyond LCP, CLS, TTFB
- Changes to `perf record` or `perf analyze` commands
- Refactoring the trace event parsing architecture
- Changing the JSON output schema (fields remain `lcp_ms`, `cls`, `ttfb_ms`)

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC3)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
