# Defect Report: Page commands target wrong tab after tabs activate

**Issue**: #137
**Date**: 2026-02-17
**Status**: Draft
**Author**: Claude
**Severity**: High
**Related Spec**: `.claude/specs/122-fix-tabs-activate-state-propagation/`

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli navigate https://www.google.com`
3. `chrome-cli tabs create https://example.com` — creates tab 2
4. `chrome-cli tabs create https://httpbin.org` — creates tab 3
5. `chrome-cli tabs activate <tab2_id>` — reports success with example.com URL
6. `chrome-cli page text` — returns `{"text": "", "url": "about:blank", "title": ""}` instead of example.com content

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 |
| **Version / Commit** | 1.0.0 (commit e50f7b3) |
| **Browser / Runtime** | HeadlessChrome/144.0.0.0 |
| **Configuration** | Default (headless mode) |

### Frequency

Always — each CLI invocation is a separate process; `/json/list` ordering is unreliable across process boundaries.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | After `tabs activate <id>`, `page text` returns content from the activated tab |
| **Actual** | `page text` returns content from whichever tab Chrome's `/json/list` happens to list first, which may be `about:blank` or a different tab entirely |

### Error Output

```
No error — command exits 0 but returns data from the wrong tab.
JSON output example: {"text": "", "url": "about:blank", "title": ""}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Page text reads from activated tab

**Given** multiple tabs are open in a headless Chrome instance
**When** I run `tabs activate <tab_id>` followed by `page text` (as separate CLI invocations)
**Then** the text content is from the activated tab, not from whichever tab `/json/list` lists first

**Example**:
- Given: 3 tabs open — google.com, example.com, httpbin.org
- When: `chrome-cli tabs activate <example_tab_id>`, then `chrome-cli page text`
- Then: output `url` field contains `example.com`

### AC2: Page screenshot captures activated tab

**Given** multiple tabs are open in a headless Chrome instance
**When** I run `tabs activate <tab_id>` followed by `page screenshot --file test.png` (as separate CLI invocations)
**Then** the screenshot shows the activated tab's content

### AC3: Explicit --tab flag still works

**Given** multiple tabs are open and an active tab has been set via `tabs activate`
**When** I run `page text --tab <specific_id>` targeting a different tab
**Then** the text content is from the explicitly specified tab, not from the activated tab

**Example**:
- Given: tabs A (activated) and B are open
- When: `chrome-cli page text --tab <B_id>`
- Then: output `url` field is from tab B

### AC4: Active tab persists across invocations

**Given** I run `tabs activate <tab_id>` to activate a specific tab
**When** the CLI process exits and a new CLI invocation runs `page text`
**Then** the new invocation targets the previously activated tab (cross-invocation state persistence)

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Add an `active_tab_id` field to the session file so that `tabs activate` can persist the activated tab's target ID across CLI invocations | Must |
| FR2 | Modify `resolve_target()` to prefer the persisted `active_tab_id` from the session file when `--tab` is not specified, falling back to the existing first-page heuristic if the persisted target no longer exists | Must |
| FR3 | When `--tab` is explicitly provided, it takes precedence over the persisted `active_tab_id` | Must |

---

## Out of Scope

- Changes to `tabs activate` output format
- Changes to `/json/list` ordering reliability (Chrome behavior)
- Refactoring `select_target()` into a different module
- Persisting active tab for `tabs create` (only `tabs activate` writes the active tab)

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC3)
- [x] Cross-invocation state persistence is tested (AC4 — per retrospective learning)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
