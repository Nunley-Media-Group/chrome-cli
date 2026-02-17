# Defect Report: Page screenshot --uid fails with 'Could not find node with given id'

**Issue**: #115
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude
**Severity**: High
**Related Spec**: `.claude/specs/12-screenshot-capture/`

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli navigate https://www.google.com`
3. `chrome-cli page snapshot` — assigns UIDs (e.g., s9 = search combobox)
4. `chrome-cli page screenshot --uid s9 --file /tmp/test.png` — **fails**
5. `chrome-cli js exec --uid s9 "(el) => el.tagName"` — **succeeds** (returns "TEXTAREA")

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
| **Expected** | `page screenshot --uid s9` captures a screenshot of the element identified by UID s9 |
| **Actual** | Returns error: `{"error":"Screenshot capture failed: Failed to get element bounding box: CDP protocol error (-32000): Could not find node with given id","code":1}` with exit code 1 |

### Error Output

```
{"error":"Screenshot capture failed: Failed to get element bounding box: CDP protocol error (-32000): Could not find node with given id","code":1}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Screenshot by UID works after snapshot

**Given** a page has been loaded and `page snapshot` has assigned UIDs
**When** I run `page screenshot --uid <uid> --file /tmp/test.png`
**Then** a screenshot of the element is saved successfully with exit code 0

### AC2: JS exec by UID continues to work

**Given** a page has been loaded and `page snapshot` has assigned UIDs
**When** I run `js exec --uid <uid> "(el) => el.tagName"`
**Then** the element is resolved and the function executes correctly with exit code 0

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `resolve_uid_clip()` must ensure the DOM domain is active before issuing `DOM.describeNode` and `DOM.getBoxModel` CDP commands | Must |
| FR2 | `js exec --uid` must continue to resolve UIDs correctly (no regression) | Should |

---

## Out of Scope

- Changes to `js exec` UID resolution (already working correctly)
- Changes to snapshot UID assignment logic
- Refactoring `resolve_selector_clip()` (same pattern but out of scope for this fix)
- Changes to the `ManagedSession` API

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
