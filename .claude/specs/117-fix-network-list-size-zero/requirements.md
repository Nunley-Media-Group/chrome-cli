# Defect Report: Network list showing size 0 for most requests

**Issue**: #117
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude
**Severity**: Medium
**Related Spec**: `.claude/specs/19-network-request-monitoring/`

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli navigate https://www.google.com`
3. `chrome-cli network list --limit 5`
4. Observe most requests show `"size": 0`
5. `chrome-cli network get 3` — response headers show `content-length: 377301` but top-level `size: 0`

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 |
| **Version / Commit** | 0.1.0 (commit 112e231) |
| **Browser / Runtime** | HeadlessChrome/144.0.0.0 |
| **Configuration** | Default (no custom config) |

### Frequency

Always — the vast majority of network requests show `size: 0`.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | The `size` field reflects the actual response size. When `encodedDataLength` is 0, the value falls back to the `content-length` response header. |
| **Actual** | `size: 0` for the majority of requests, even when response headers contain a valid `content-length` value. The size field is useless for analysis. |

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Size falls back to content-length when encodedDataLength is 0

**Given** Chrome is connected and a page has been navigated
**When** a network request completes with `encodedDataLength` of 0
**And** the response headers include a `content-length` value
**Then** the `size` field in `network list` uses the `content-length` value

### AC2: Size uses encodedDataLength when it is non-zero

**Given** Chrome is connected and a page has been navigated
**When** a network request completes with a non-zero `encodedDataLength`
**Then** the `size` field in `network list` uses the `encodedDataLength` value (existing behavior preserved)

### AC3: Size fallback applies to network get detail view

**Given** Chrome is connected and a page has been navigated
**When** I run `chrome-cli network get <id>` for a request where `encodedDataLength` was 0
**And** the response headers include a `content-length` value
**Then** the detail view's `size` field uses the `content-length` value

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | When `encodedDataLength` is 0 or absent in `Network.loadingFinished`, fall back to the `content-length` response header value for the `size` field | Must |
| FR2 | The fallback applies consistently across `network list`, `network get`, and `network follow` output modes | Must |
| FR3 | When both `encodedDataLength` is 0 and `content-length` is absent, `size` remains `null`/0 | Should |

---

## Out of Scope

- Changes to network request filtering logic
- Changes to the `network get` detail view beyond the `size` field
- Refactoring the network event correlation pipeline
- Adding new CLI flags or options

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC2)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
