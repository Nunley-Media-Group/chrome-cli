# Defect Report: network list always returns empty array

**Issue**: #102
**Date**: 2026-02-15
**Status**: Approved
**Author**: Claude (spec generation)
**Severity**: High
**Related Spec**: `.claude/specs/19-network-request-monitoring/`

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli navigate https://www.google.com --wait-until load`
3. `chrome-cli navigate reload` (generates network traffic)
4. `chrome-cli network list` — returns `[]`
5. `chrome-cli network list --type document` — returns `[]`
6. Meanwhile, `chrome-cli network follow --timeout 3000` during a reload DOES capture requests

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (Darwin 25.3.0) |
| **Version / Commit** | `c584d2d` (main) |
| **Browser / Runtime** | Chrome via CDP |
| **Configuration** | Default (headless or headed) |

### Frequency

Always — 100% reproducible on any page that has finished loading.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `network list` returns a JSON array of captured network requests with `id`, `method`, `url`, `status`, and `type` fields after a page has been loaded or navigated. |
| **Actual** | `network list` returns `[]` every time. All filters (`--type`, `--url`, `--limit`) have no effect since the base list is empty. `network get <id>` returns "not found" since there are no captured requests. Only `network follow` (streaming mode) shows network activity. |

### Error Output

```
$ chrome-cli network list
[]
```

No error — the command succeeds with exit code 0 but returns an empty array regardless of network activity.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Network list returns captured requests after page load

**Given** Chrome is running and a page has been loaded (e.g., `https://www.google.com`)
**When** I run `chrome-cli network list`
**Then** the output is a JSON array containing at least one network request
**And** each entry has `id`, `method`, `url`, `status`, and `type` fields

### AC2: Type filter works on captured requests

**Given** Chrome is running and a page has been loaded with network requests of various types
**When** I run `chrome-cli network list --type document`
**Then** only document-type requests are returned
**And** the result is non-empty

### AC3: URL filter works on captured requests

**Given** Chrome is running and a page at `https://www.google.com` has been loaded
**When** I run `chrome-cli network list --url "google"`
**Then** only requests with "google" in the URL are returned
**And** the result is non-empty

### AC4: Network get returns request details for captured requests

**Given** Chrome is running and `network list` has returned request entries
**When** I run `chrome-cli network get <request-id>` with a valid ID from the list
**Then** detailed request, response, and timing information is returned

### AC5: Network follow streaming still works

**Given** Chrome is running with a page open
**When** I run `chrome-cli network follow --timeout 3000` and the page makes requests
**Then** completed requests are streamed as JSON lines
**And** the existing follow behavior is unchanged

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `network list` must return captured requests from the current page — it must not rely solely on a 100ms event drain window after connecting | Must |
| FR2 | Network capture must work without requiring `network follow` to be running in a separate terminal | Must |
| FR3 | Filters (`--type`, `--url`, `--status`, `--method`, `--limit`) must work correctly on the captured request list | Must |
| FR4 | `network get <ID>` must work with IDs returned by `network list` | Must |
| FR5 | The fix must not break `network follow` streaming behavior | Must |

---

## Out of Scope

- Cross-session network request persistence (requests from previous CLI invocations)
- Network request body capture (already out of scope per issue #19)
- Changes to `network follow` streaming behavior
- Long-running background daemon for persistent network monitoring

---

## Validation Checklist

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC5)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
