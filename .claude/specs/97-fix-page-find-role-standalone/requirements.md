# Defect Report: page find --role does not work as standalone search criterion

**Issue**: #97
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude
**Severity**: Medium
**Related Spec**: `.claude/specs/15-element-finding/` *(if exists — covers the original `page find` feature)*

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli navigate https://www.google.com --wait-until load`
3. `chrome-cli page find --role textbox`
4. Error: `{"error":"either a text query or --selector is required","code":1}`

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (Darwin 25.3.0) |
| **Version / Commit** | `c584d2d` (main) |
| **Browser / Runtime** | Chrome via CDP |
| **Configuration** | Default |

### Frequency

Always

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `page find --role textbox` returns a JSON array of matching elements with their UIDs, roles, and names — consistent with `--help` documentation showing `--role` as a standalone search option |
| **Actual** | Returns error `{"error":"either a text query or --selector is required","code":1}` because validation in `execute_find()` requires either `query` or `selector` to be present, ignoring `--role` as a valid standalone criterion |

### Error Output

```json
{"error":"either a text query or --selector is required","code":1}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Role-only search works

**Given** a page is loaded with interactive elements
**When** I run `page find --role textbox`
**Then** the output is a JSON array of matching elements with their UIDs, roles, and names

### AC2: Role filter still works with text query

**Given** a page is loaded
**When** I run `page find "Submit" --role button`
**Then** only elements matching both the text and role are returned

### AC3: Empty role result returns empty array

**Given** a page is loaded
**When** I run `page find --role nonexistent-role`
**Then** the output is an empty JSON array `[]` (not an error)

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `--role` must be accepted as a standalone search criterion (no text or `--selector` required) | Must |
| FR2 | `--role` combined with text query continues to filter by both role and text | Should |
| FR3 | When `--role` is used standalone, all elements matching the role are returned (up to `--limit`) | Must |

---

## Out of Scope

- Adding new roles beyond what the accessibility tree supports
- Role validation against a known list
- Changing `--selector` or text query behavior when `--role` is not involved

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
