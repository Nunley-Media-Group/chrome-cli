# Defect Report: page snapshot returns empty accessibility tree on real-world websites

**Issue**: #73
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude
**Severity**: Critical

---

## Reproduction

### Steps to Reproduce

1. Launch Chrome and navigate to a real-world website (e.g., google.com)
2. Run `page snapshot` (text mode)
3. Run `page snapshot --pretty` (JSON mode)
4. Observe the output contains only the root node with no children

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (all platforms affected) |
| **Version / Commit** | Current `main` branch |
| **Browser / Runtime** | Chrome/Chromium via CDP |
| **Configuration** | Default; `Accessibility.getFullAXTree` CDP method |

### Frequency

Always — reproducible on complex, dynamically-rendered pages (google.com, etc.). Simple test pages with static HTML work correctly.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `page snapshot` returns a full accessibility tree with all interactive elements annotated with snapshot UIDs (e.g., `[s1]`, `[s2]`), enabling downstream commands like `interact click`, `form fill`, etc. |
| **Actual** | Text mode returns only `- RootWebArea "Google"`. JSON mode returns `{"children":[],"name":"Google","role":"RootWebArea"}`. No UIDs are generated, so all UID-dependent commands fail. |

### Error Output

```
# Text mode output:
- RootWebArea "Google"

# JSON mode output:
{"children":[],"name":"Google","role":"RootWebArea"}
```

Note: `page text` works correctly and returns visible page content, confirming the page has loaded and rendered.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Bug Is Fixed — snapshot returns populated tree on real-world pages

**Given** Chrome is running and navigated to a real-world website (e.g., google.com)
**When** the user runs `page snapshot`
**Then** the accessibility tree contains more than just the root node
**And** interactive elements are annotated with snapshot UIDs (e.g., `[s1]`, `[s2]`)

### AC2: No Regression — snapshot still works on simple pages

**Given** Chrome is running and navigated to a simple HTML page with known interactive elements
**When** the user runs `page snapshot`
**Then** the accessibility tree contains the expected hierarchy
**And** interactive elements receive UIDs matching their roles

### AC3: UID-dependent commands work after snapshot on real-world pages

**Given** Chrome is running and navigated to a real-world website
**And** the user has run `page snapshot` successfully with UIDs assigned
**When** the user runs a UID-dependent command (e.g., `interact click [uid]`)
**Then** the command resolves the UID and interacts with the correct element

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `Accessibility.getFullAXTree` response must be processed into a tree with children, even on complex real-world pages | Must |
| FR2 | When `getFullAXTree` returns nodes with empty/missing `childIds`, the tree builder must still reconstruct a populated tree using alternative parent-child resolution | Must |
| FR3 | Interactive elements in the tree must receive sequential UIDs (`s1`, `s2`, ...) for use by downstream commands | Must |
| FR4 | Existing behavior for simple/static pages must not regress | Must |

---

## Out of Scope

- Full DOM-based tree traversal fallback (separate enhancement)
- Performance optimization of tree building for very large pages
- Support for non-Chromium browsers
- Changes to the `page text` command (already working correctly)

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
