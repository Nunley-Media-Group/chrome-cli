# Defect Report: tabs create --background not keeping original tab active

**Issue**: #121
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude
**Severity**: Medium
**Related Spec**: `.claude/specs/82-fix-tabs-create-background/`

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli tabs create https://google.com` — google.com becomes the active tab
3. `chrome-cli tabs create --background https://example.com` — example.com is created
4. `chrome-cli tabs list --plain` — example.com shows as active (marked with `*`), not google.com

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 |
| **Version / Commit** | 0.1.0 (commit 112e231) |
| **Browser / Runtime** | HeadlessChrome/144.0.0.0 |
| **Configuration** | Default (headless mode) |

### Frequency

Always — reproducible on every invocation.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | After `tabs create --background`, the previously active tab (google.com) remains active and appears first in `tabs list` |
| **Actual** | The newly created background tab (example.com) becomes the active tab, appearing first in `tabs list` |

### Error Output

No error output — the command exits with code 0 but the tab ordering in `/json/list` does not reflect the expected activation state.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Background tab does not become active

**Given** Chrome is running with an active tab at google.com
**When** I run `chrome-cli tabs create --background https://example.com`
**Then** `chrome-cli tabs list` shows google.com as the active tab (first in list)

**Example**:
- Given: Tab at `https://google.com` is active (first page target in `/json/list`)
- When: `chrome-cli tabs create --background https://example.com`
- Then: `tabs list` output shows google.com tab with `active: true`

### AC2: Background tab is created successfully

**Given** Chrome is running with an active tab
**When** I run `chrome-cli tabs create --background https://example.com`
**Then** the new tab exists in the tab list with URL containing "example.com"
**And** the new tab's `active` field is `false`

### AC3: Non-background create still activates the new tab

**Given** Chrome is running with an active tab
**When** I run `chrome-cli tabs create https://example.com` (without `--background`)
**Then** the new tab becomes the active tab (first in list)

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | The `tabs create --background` command must reliably keep the previously active tab active, even when Chrome's `/json/list` endpoint is slow to update ordering | Must |
| FR2 | The existing non-background `tabs create` behavior must remain unchanged — the new tab should become active | Should |

---

## Out of Scope

- Changes to tab creation without `--background` flag
- Changes to `tabs activate` command
- Changes to Chrome's CDP behavior (we can only work around it)
- Refactoring the polling mechanism for other commands (e.g., `tabs close`)

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
