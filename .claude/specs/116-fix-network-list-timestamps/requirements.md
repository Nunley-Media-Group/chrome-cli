# Defect Report: Network list timestamps showing 1970-01-01 instead of real wall-clock time

**Issue**: #116
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude
**Severity**: High
**Related Spec**: `.claude/specs/19-network-request-monitoring/`

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli navigate https://www.google.com`
3. `chrome-cli network list --limit 3`
4. Observe timestamps: all show `1970-01-01T17:XX:XX`

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 |
| **Version / Commit** | 0.1.0 (commit 112e231) |
| **Browser / Runtime** | HeadlessChrome/144.0.0.0 |
| **Configuration** | Default |

### Frequency

Always

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | Timestamps show the actual wall-clock time of each request (e.g., `2026-02-16T21:12:10.044Z`) |
| **Actual** | All timestamps show `1970-01-01T17:18:10.xxx` (epoch + monotonic offset) |

### Error Output

```
$ chrome-cli network list --limit 1
[
  {
    "id": "...",
    "url": "https://www.google.com/",
    "method": "GET",
    "status": 200,
    "type": "Document",
    "timestamp": "1970-01-01T17:14:50.044Z",  <-- should be 2026-02-16T21:12:10.044Z
    ...
  }
]
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Network timestamps reflect real wall-clock time

**Given** Chrome is connected and a page has been navigated to
**When** I run `chrome-cli network list`
**Then** the `timestamp` fields show dates from the current year (not 1970)
**And** the timestamps reflect the approximate time the requests were made

### AC2: Timestamps are valid ISO 8601 in UTC

**Given** Chrome is connected and network requests have been captured
**When** I run `chrome-cli network list`
**Then** all timestamp values match the ISO 8601 format `YYYY-MM-DDTHH:MM:SS.mmmZ`
**And** timestamps end with `Z` indicating UTC

### AC3: Console timestamps still work correctly

**Given** Chrome is connected and a page has been navigated to
**When** I run `chrome-cli console read`
**Then** console timestamps still show correct wall-clock times (no regression)

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Convert CDP monotonic network timestamps to wall-clock time by computing the offset between monotonic clock and system clock | Must |
| FR2 | Preserve existing console timestamp behavior (already uses epoch milliseconds correctly) | Must |

---

## Out of Scope

- Changes to console timestamp handling (already correct — uses epoch milliseconds)
- Changes to network filtering, sorting, or other network fields
- Refactoring the timestamp formatting algorithm shared between console.rs and network.rs

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
