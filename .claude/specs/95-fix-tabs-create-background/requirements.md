# Defect Report: tabs create --background does not preserve active tab

**Issue**: #95
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude
**Severity**: Medium
**Related Spec**: `.claude/specs/82-fix-tabs-create-background/` — regression of the same fix; `.claude/specs/7-tab-management/` — AC6

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli tabs create https://www.google.com` — Google tab becomes active
3. `chrome-cli tabs create --background https://www.google.com/search?q=test`
4. `chrome-cli tabs list` — the background tab is now `active: true`

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (Darwin 25.3.0) |
| **Version / Commit** | `c584d2d` (main) |
| **Browser / Runtime** | Chrome, launched via `connect --launch --headless` |
| **Configuration** | Default (no custom flags) |

### Frequency

Always — the `--background` flag consistently fails to preserve the previously active tab.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | The `--background` flag creates the tab without activating it. The previously active tab (Google) remains `active: true`. The new background tab has `active: false`. |
| **Actual** | The new background tab becomes `active: true`. The previously active Google tab becomes `active: false`. |

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Background tab does not become active

**Given** tab A is the active tab
**When** I run `tabs create --background <url>`
**Then** the new tab is created successfully
**And** tab A remains the active tab
**And** the new tab has `active: false`

### AC2: Normal tab creation still activates

**Given** tab A is the active tab
**When** I run `tabs create <url>` (without `--background`)
**Then** the new tab becomes the active tab

### AC3: Background tab appears in tab list

**Given** I created a tab with `--background`
**When** I run `tabs list`
**Then** the background tab appears in the list with its URL and title

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `--background` flag must prevent tab activation after creation | Must |
| FR2 | Normal (non-background) tab creation must still activate the new tab | Must |

---

## Out of Scope

- Tab ordering behavior
- Background tab navigation events
- Refactoring the broader tab management module

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC2, AC3)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
