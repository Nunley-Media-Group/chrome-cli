# Defect Report: tabs create --background does not keep previously active tab focused

**Issue**: #82
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude
**Severity**: Medium
**Related Spec**: `.claude/specs/tab-management/` — AC6

---

## Reproduction

### Steps to Reproduce

1. Launch Chrome: `chrome-cli connect --launch`
2. Create a foreground tab: `chrome-cli tabs create "https://www.google.com"`
3. Create a background tab: `chrome-cli tabs create "https://example.com" --background`
4. List tabs: `chrome-cli tabs list`

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (Darwin 25.2.0) |
| **Version / Commit** | chrome-cli 0.1.0 (commit 01989d5) |
| **Browser / Runtime** | Chrome Stable, launched via `connect --launch` |
| **Configuration** | Default (no custom flags) |

### Frequency

Always — Chrome consistently ignores the `background: true` parameter in `Target.createTarget`.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | The Google tab (TAB_A) remains `active: true`. The example.com tab created with `--background` has `active: false`. Chrome stays visually on TAB_A. |
| **Actual** | The example.com tab (the background tab) shows `active: true`. The previously active Google tab shows `active: false`. Chrome visually switches to the new tab despite `--background`. |

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Bug Is Fixed

**Given** Chrome is running with an active tab (TAB_A)
**When** I run `chrome-cli tabs create --background https://example.com`
**Then** a new tab is created navigating to the URL
**And** TAB_A remains the active tab (verified via `tabs list`)

### AC2: Non-background create still activates

**Given** Chrome is running with an active tab
**When** I run `chrome-cli tabs create https://example.com` (without `--background`)
**Then** the new tab becomes the active tab (existing behavior preserved)

### AC3: No regression on tab create output

**Given** Chrome is running
**When** I run `chrome-cli tabs create --background https://example.com`
**Then** stdout contains a JSON object with `id`, `url`, and `title` fields
**And** the exit code is 0

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | When `--background` is used, record the currently active tab ID before creating the new tab | Must |
| FR2 | After creation with `--background`, re-activate the original tab via `Target.activateTarget` if Chrome did not honor the background parameter | Must |
| FR3 | When `--background` is not used, do not change current activation behavior | Must |

---

## Out of Scope

- Changing the default behavior of `tabs create` without `--background`
- Tab ordering or focus management beyond the `--background` flag
- Investigating why Chrome ignores `Target.createTarget`'s `background` parameter
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
