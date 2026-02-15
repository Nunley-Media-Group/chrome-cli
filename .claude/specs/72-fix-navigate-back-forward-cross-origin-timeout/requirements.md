# Defect Report: navigate back/forward timeout on cross-origin history navigation

**Issue**: #72
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude
**Severity**: High

---

## Reproduction

### Steps to Reproduce

1. Launch Chrome via chrome-cli
2. Navigate to `https://google.com`
3. Navigate to `https://google.com/about` (which redirects to `https://about.google`, a different origin)
4. Execute `chrome-cli navigate back`
5. Observe timeout error after 30 seconds

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS, Linux, Windows (all platforms) |
| **Version / Commit** | `25cafa4` (current main) |
| **Browser / Runtime** | Chrome/Chromium via CDP |
| **Configuration** | Default (30s navigation timeout) |

### Frequency

Always — 100% reproducible when back/forward navigation crosses origins.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `navigate back` and `navigate forward` successfully navigate across origins and return a JSON success response with the destination URL and title. |
| **Actual** | The command hangs for 30 seconds and then fails with `{"error":"Navigation timed out after 30000ms waiting for load","code":4}`. |

### Error Output

```json
{"error":"Navigation timed out after 30000ms waiting for load","code":4}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Cross-origin navigate back succeeds

**Given** a tab has navigated from `https://example.com` to a page on a different origin (e.g., `https://www.iana.org/domains/reserved`)
**When** I run `chrome-cli navigate back`
**Then** the exit code is 0
**And** the JSON output has key `url` containing the previous origin's URL

### AC2: Cross-origin navigate forward succeeds

**Given** a tab has navigated across origins and then navigated back
**When** I run `chrome-cli navigate forward`
**Then** the exit code is 0
**And** the JSON output has key `url` containing the forward destination URL

### AC3: Same-origin navigate back still works (no regression)

**Given** a tab has navigated between two pages on the same origin
**When** I run `chrome-cli navigate back`
**Then** the exit code is 0
**And** the JSON output has key `url` containing the previous page's URL

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `execute_back` and `execute_forward` must use a CDP event that fires reliably for both same-origin and cross-origin history navigations | Must |
| FR2 | Existing same-origin back/forward behavior must be preserved | Must |

---

## Out of Scope

- Refactoring other navigation commands (e.g., `navigate to`) that are not affected by this bug
- Changing the default timeout value
- Adding new CLI flags or options

---

## Validation Checklist

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
