# Defect Report: Page snapshot drops all children under ignored AX nodes

**Issue**: #83
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude (writing-specs)
**Severity**: Critical
**Related Spec**: `.claude/specs/10-accessibility-tree-snapshot/`

---

## Reproduction

### Steps to Reproduce

1. Launch Chrome: `chrome-cli connect --launch`
2. Navigate to any real-world page: `chrome-cli navigate "https://example.com"`
3. Take a snapshot: `chrome-cli page snapshot --pretty`

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (Darwin 25.2.0) |
| **Version / Commit** | chrome-cli 0.1.0 (commit 01989d5) |
| **Browser / Runtime** | Chrome Stable channel |
| **Configuration** | Default settings, no custom flags |

### Frequency

Always — every real-world page where Chrome's CDP response contains intermediate ignored nodes (which is the common case).

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | The snapshot returns a complete tree with headings, paragraphs, links, and other visible elements. Interactive elements have UIDs assigned (e.g., `[s1] link "Learn more"`). |
| **Actual** | Only the root `RootWebArea` node is returned with an empty `children` array. No child elements appear, no UIDs are assigned. The snapshot is completely unusable. |

### Error Output

```json
{
  "children": [],
  "name": "Example Domain",
  "role": "RootWebArea"
}
```

No error is raised — the snapshot silently returns an empty tree.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Bug Is Fixed

**Given** CDP returns an accessibility tree where the root node's only children are ignored nodes that themselves have non-ignored descendants
**When** the snapshot tree is built
**Then** the non-ignored descendant nodes appear as children in the output tree
**And** interactive elements among them have UIDs assigned

**Example**:
- Given: CDP tree `RootWebArea → ignored(id=2) → ignored(id=7) → heading(id=9), paragraph(id=10)`
- When: `build_subtree` processes this tree
- Then: the root's children include heading and paragraph (promoted through ignored ancestors)

### AC2: No Regression on Non-Ignored Trees

**Given** CDP returns an accessibility tree with no ignored intermediate nodes
**When** the snapshot tree is built
**Then** the tree is rendered identically to the current behavior (parent-child hierarchy preserved, UIDs assigned to interactive roles)

### AC3: Deeply Nested Ignored Chains

**Given** CDP returns an accessibility tree with multiple levels of consecutive ignored nodes (e.g., ignored → ignored → ignored → visible)
**When** the snapshot tree is built
**Then** the visible nodes are promoted through all ignored ancestors to the nearest non-ignored ancestor
**And** the order of promoted children is preserved (depth-first traversal order)

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | When `build_subtree` encounters an ignored node, it must recurse into that node's children instead of returning `None` | Must |
| FR2 | Children of ignored nodes must be promoted (flattened) into the nearest non-ignored ancestor's children list | Must |
| FR3 | The promotion must preserve depth-first traversal order of children | Must |

---

## Out of Scope

- Changing how UIDs are assigned to elements
- Modifying the CDP command used (`Accessibility.getFullAXTree`)
- The existing `parentId` fallback mechanism (Issue #73 / PR #78)
- Refactoring the `build_subtree` function beyond the minimal fix
- Changing the existing unit test `build_tree_filters_ignored_nodes` (it tests a sibling-level ignored node which is still correct; this fix addresses ancestor-level ignored nodes)

---

## Validation Checklist

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed (Critical)
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC2)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
