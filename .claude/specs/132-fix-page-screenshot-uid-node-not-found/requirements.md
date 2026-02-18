# Defect Report: Page screenshot --uid fails with 'Could not find node' (regression of #115)

**Issue**: #132
**Date**: 2026-02-17
**Status**: Verified
**Author**: Claude
**Severity**: High
**Related Spec**: `.claude/specs/115-fix-page-screenshot-uid-node-not-found/`

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli navigate https://www.google.com`
3. `chrome-cli page snapshot` — assigns UIDs (e.g., s1 = "About" link)
4. `chrome-cli page screenshot --uid s1 --file /tmp/element.png` — **fails**

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 |
| **Version / Commit** | 1.0.0 (commit e50f7b3) |
| **Browser / Runtime** | HeadlessChrome/144.0.0.0 |
| **Configuration** | Default |

### Frequency

Always

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `page screenshot --uid s1` captures a screenshot of the element identified by UID s1 and saves it to the specified file |
| **Actual** | Returns error: `{"error":"Screenshot capture failed: Failed to get element bounding box: CDP protocol error (-32000): Could not find node with given id","code":1}` with exit code 1 |

### Error Output

```
{"error":"Screenshot capture failed: Failed to get element bounding box: CDP protocol error (-32000): Could not find node with given id","code":1}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Screenshot by UID succeeds after snapshot

**Given** a page has been loaded and `page snapshot` has assigned UIDs
**When** I run `page screenshot --uid s1 --file /tmp/element.png`
**Then** a PNG file is written containing the element's rendered pixels with exit code 0

### AC2: js exec --uid still works (no regression)

**Given** a page has been loaded and `page snapshot` has assigned UIDs
**When** I run `js exec --uid s1 "(el) => el.tagName"`
**Then** the element's tag name is returned with exit code 0

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Pass `backendNodeId` directly to `DOM.getBoxModel` in `resolve_uid_clip()`, removing the intermediate `DOM.describeNode` + transient `nodeId` resolution that caused the error | Must |
| FR2 | `js exec --uid` must continue to resolve UIDs correctly (no regression in `DOM.resolveNode` path) | Should |

**Note**: The original FR1 proposed adding `DOM.getDocument` before `DOM.describeNode`. Verification proved this insufficient — the transient `nodeId` from `DOM.describeNode` is not anchored in the document tree regardless. The corrected approach bypasses `DOM.describeNode` entirely.

---

## Out of Scope

- Changes to `resolve_selector_clip()` (already works correctly)
- Changes to `js exec --uid` path in `src/js.rs` (already works via `DOM.resolveNode`)
- Changes to snapshot UID assignment logic

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
