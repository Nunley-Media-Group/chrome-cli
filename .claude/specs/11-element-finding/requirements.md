# Requirements: Element Finding

**Issue**: #11
**Date**: 2026-02-12
**Status**: Draft
**Author**: Claude (writing-specs)

---

## User Story

**As a** developer or automation engineer
**I want** to find elements on a page by text, CSS selector, or accessibility attributes
**So that** I can locate specific interactive elements for clicking, filling, or inspecting from scripts

---

## Background

The MCP server has a `find` concept where users can locate elements by their accessible name or role. This is essential for AI agents that need to locate specific interactive elements to click or fill. The `page find` command builds on the snapshot/uid infrastructure from issue #10, searching the accessibility tree rather than raw HTML. For CSS selector searches, CDP DOM methods provide a complementary backend.

---

## Acceptance Criteria

### AC1: Find elements by text query (happy path)

**Given** Chrome is connected with a page loaded
**When** I run `chrome-cli page find "Submit"`
**Then** a JSON array of matching elements is returned
**And** each element includes uid, role, name, and bounding box
**And** elements are in document order

**Example**:
- Given: A page with a "Submit" button and a "Submit Form" heading
- When: `chrome-cli page find "Submit"`
- Then: Both elements are returned, button with uid, heading without (or with uid if interactive)

### AC2: Find elements by CSS selector

**Given** Chrome is connected with a page loaded
**When** I run `chrome-cli page find --selector "button.primary"`
**Then** elements matching the CSS selector are returned as a JSON array
**And** each element includes uid, role, name, and bounding box

### AC3: Filter by accessibility role

**Given** Chrome is connected with a page loaded containing buttons and links
**When** I run `chrome-cli page find "Click" --role button`
**Then** only elements with the "button" role matching "Click" are returned
**And** links containing "Click" are excluded

### AC4: Exact text match

**Given** Chrome is connected with a page containing "Log" and "Login" buttons
**When** I run `chrome-cli page find "Log" --exact`
**Then** only the element with the exact name "Log" is returned
**And** "Login" is excluded

### AC5: Limit results

**Given** Chrome is connected with a page containing 50 links
**When** I run `chrome-cli page find "link" --limit 5`
**Then** at most 5 results are returned
**And** they are in document order (first 5 matches)

### AC6: Default limit

**Given** Chrome is connected with a page containing many matching elements
**When** I run `chrome-cli page find "item"` without `--limit`
**Then** at most 10 results are returned (default limit)

### AC7: Target a specific tab

**Given** Chrome is connected with multiple tabs open
**When** I run `chrome-cli page find "Submit" --tab <ID>`
**Then** the search is performed on the specified tab only

### AC8: No matches found

**Given** Chrome is connected with a page loaded
**When** I run `chrome-cli page find "nonexistent-element-xyz"`
**Then** an empty JSON array `[]` is returned
**And** the exit code is 0 (not an error)

### AC9: Bounding box information

**Given** Chrome is connected with a page containing a visible button
**When** I run `chrome-cli page find "Submit"`
**Then** each result includes a bounding box with x, y, width, and height
**And** bounding box values are numeric (pixels)

### AC10: Snapshot triggered if needed

**Given** Chrome is connected with a page loaded but no prior snapshot
**When** I run `chrome-cli page find "Submit"`
**Then** a snapshot is automatically captured before searching
**And** UIDs are assigned and persisted to snapshot state

### AC11: CSS selector with no text query

**Given** Chrome is connected with a page loaded
**When** I run `chrome-cli page find --selector "input[type=email]"`
**Then** matching elements are returned without requiring a text query argument

### AC12: Combined role and text query

**Given** Chrome is connected with a page containing links and buttons with "Next"
**When** I run `chrome-cli page find "Next" --role link`
**Then** only link elements with "Next" in their name are returned

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Search accessibility tree by text (name, text content, labels) | Must | Substring match by default |
| FR2 | Search by CSS selector via `--selector` flag | Must | Uses CDP DOM methods |
| FR3 | Filter by accessibility role via `--role` flag | Must | Standard ARIA roles |
| FR4 | Exact text matching via `--exact` flag | Must | Case-sensitive exact match |
| FR5 | Limit results via `--limit` flag (default: 10) | Must | |
| FR6 | Return uid, role, name, and bounding box per element | Must | |
| FR7 | Results in document order | Must | |
| FR8 | Auto-trigger snapshot if none exists | Must | Reuses snapshot infrastructure |
| FR9 | Target specific tab via `--tab` flag | Must | Existing global option |
| FR10 | JSON output by default | Must | Consistent with other commands |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Find command completes within 2s for typical pages (< 10,000 nodes) |
| **Reliability** | Gracefully handles pages with no accessibility tree nodes |
| **Platforms** | macOS, Linux, Windows (cross-platform Rust) |
| **Consistency** | Output format matches existing command patterns (JSON to stdout, errors to stderr) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| query | String | Non-empty when no --selector | Conditional |
| --selector | String | Valid CSS selector | No |
| --role | String | Valid accessibility role name | No |
| --exact | Boolean flag | N/A | No |
| --limit | Integer | > 0 | No (default: 10) |
| --tab | String | Valid tab ID | No (default: active tab) |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| uid | String or null | Snapshot UID (e.g., "s1") if element is interactive |
| role | String | Accessibility role (e.g., "button", "link") |
| name | String | Accessible name / text content |
| boundingBox | Object or null | `{x, y, width, height}` in pixels |

---

## Dependencies

### Internal Dependencies
- [x] Issue #10 — Accessibility tree snapshot / UID system (implemented)

### External Dependencies
- Chrome DevTools Protocol: `Accessibility.getFullAXTree`, `DOM.querySelector`, `DOM.querySelectorAll`, `DOM.getBoxModel`

---

## Out of Scope

- XPath selectors
- Fuzzy/phonetic matching
- Regex text matching
- Highlighting or annotating found elements in the browser
- Finding elements across frames/iframes
- Caching or incremental tree updates

---

## Open Questions

- (none — resolved via issue and codebase analysis)

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Edge cases and error states specified
- [x] Dependencies identified
- [x] Out of scope defined
