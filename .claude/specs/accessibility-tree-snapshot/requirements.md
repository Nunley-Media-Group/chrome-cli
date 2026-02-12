# Requirements: Accessibility Tree Snapshot

**Issue**: #10
**Date**: 2026-02-12
**Status**: Draft
**Author**: Claude (writing-specs)

---

## User Story

**As a** developer / automation engineer
**I want** to capture the accessibility tree of a browser page via the CLI
**So that** AI agents can understand page structure and reference interactive elements for subsequent commands

---

## Background

The accessibility tree snapshot is the primary way AI agents "see" and understand page content. Unlike raw text extraction (`page text`), the snapshot provides a hierarchical, semantic view of the DOM where each interactive element is assigned a unique ID (uid) that can be referenced by future interaction commands (click, fill, hover). This is the foundational capability that all element interaction commands (#14-#17) depend on.

The MCP server's `take_snapshot` tool demonstrates this pattern: it captures the accessibility tree, assigns short unique IDs to each element, and maintains a mapping from those IDs to Chrome's internal backend node IDs. This allows subsequent commands to resolve a uid like `s3` back to the actual DOM element for interaction.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Capture full accessibility tree of current page

**Given** Chrome is running with a page loaded at "https://example.com"
**When** I run `chrome-cli page snapshot`
**Then** stdout contains a text representation of the accessibility tree
**And** the output shows hierarchical indentation reflecting the DOM structure
**And** each node displays its role and accessible name

**Example**:
- Given: a page with a heading, paragraph, and button
- When: `chrome-cli page snapshot`
- Then: output like:
  ```
  - document "Example Domain"
    - heading "Example Domain" [s1]
    - paragraph ""
      - text "This domain is for ..."
    - link "More information..." [s2]
  ```

### AC2: Interactive elements have unique reference IDs

**Given** Chrome is running with a page containing interactive elements (links, buttons, inputs)
**When** I run `chrome-cli page snapshot`
**Then** each interactive element is annotated with a uid in brackets (e.g., `[s1]`)
**And** each uid is unique within the snapshot
**And** uids are short, readable strings (e.g., `s1`, `s2`, `s3`)

### AC3: Target a specific tab with --tab

**Given** Chrome is running with multiple tabs open
**When** I run `chrome-cli page snapshot --tab <ID>`
**Then** the accessibility tree is captured from the specified tab
**And** non-interactive text elements are included in the tree

### AC4: Verbose mode with --verbose

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page snapshot --verbose`
**Then** each element includes additional properties where applicable
**And** properties may include: checked, disabled, expanded, selected, required, pressed, level, value, description, url

### AC5: Save snapshot to file with --file

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page snapshot --file /tmp/snapshot.txt`
**Then** the snapshot is written to `/tmp/snapshot.txt`
**And** stdout is empty (no output to terminal)
**And** the exit code is 0

### AC6: JSON output with --json

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page snapshot --json`
**Then** stdout contains a JSON tree structure
**And** each node has `uid` (if interactive), `role`, `name`, and `children` fields

**Example**:
```json
{
  "uid": null,
  "role": "document",
  "name": "Example Domain",
  "children": [
    {"uid": "s1", "role": "heading", "name": "Example Domain", "children": []},
    {"uid": "s2", "role": "link", "name": "More information...", "children": []}
  ]
}
```

### AC7: UID-to-backend-node mapping stored in session

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page snapshot`
**Then** the uid-to-backend-node-id mapping is persisted to the session state
**And** subsequent interaction commands can resolve uids back to DOM elements

### AC8: UIDs stable across consecutive snapshots of same page

**Given** Chrome is running with a page loaded and unchanged
**When** I run `chrome-cli page snapshot` twice consecutively
**Then** the same elements receive the same uids in both snapshots

### AC9: Large page handling

**Given** Chrome is running with a very large page (e.g., 10,000+ elements)
**When** I run `chrome-cli page snapshot`
**Then** the snapshot completes within the timeout window
**And** if the tree exceeds a reasonable size, it is truncated with a message indicating truncation

### AC10: Page with no accessible content

**Given** Chrome is running with a blank page (about:blank)
**When** I run `chrome-cli page snapshot`
**Then** stdout contains a minimal tree (just the document root)
**And** the exit code is 0

### AC11: Pretty JSON output with --pretty

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page snapshot --pretty`
**Then** stdout contains pretty-printed JSON with indentation

### AC12: Plain text default output format

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page snapshot` (no format flags)
**Then** the default output is the structured text representation (not JSON)
**And** the text uses indentation and `- role "name" [uid]` format

### Generated Gherkin Preview

```gherkin
Feature: Accessibility tree snapshot
  As a developer / automation engineer
  I want to capture the accessibility tree of a browser page via the CLI
  So that AI agents can understand page structure and reference interactive elements

  Background:
    Given Chrome is running with CDP enabled

  Scenario: Capture full accessibility tree
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli page snapshot"
    Then stdout contains a hierarchical text representation of the accessibility tree
    And each node shows its role and accessible name

  Scenario: Interactive elements have unique reference IDs
    Given a page with interactive elements
    When I run "chrome-cli page snapshot"
    Then interactive elements are annotated with unique uids like "[s1]"

  Scenario: Target a specific tab
    Given multiple tabs are open
    When I run "chrome-cli page snapshot --tab <ID>"
    Then the snapshot is from the specified tab

  Scenario: Verbose mode shows additional properties
    Given a page is loaded
    When I run "chrome-cli page snapshot --verbose"
    Then elements include extra properties like checked, disabled, expanded

  Scenario: Save snapshot to file
    Given a page is loaded
    When I run "chrome-cli page snapshot --file /tmp/snapshot.txt"
    Then the snapshot is written to the file
    And stdout is empty

  Scenario: JSON output
    Given a page is loaded
    When I run "chrome-cli page snapshot --json"
    Then stdout is a JSON tree with uid, role, name, children fields

  Scenario: UID mapping persisted to session
    Given a page is loaded
    When I run "chrome-cli page snapshot"
    Then the uid-to-backend-node mapping is stored in session state

  Scenario: UID stability across snapshots
    Given a page is loaded and unchanged
    When I run "chrome-cli page snapshot" twice
    Then the same elements get the same uids

  Scenario: Large page handling
    Given a very large page is loaded
    When I run "chrome-cli page snapshot"
    Then the snapshot completes or truncates with a message

  Scenario: Blank page produces minimal tree
    Given a blank page is loaded
    When I run "chrome-cli page snapshot"
    Then output shows just the document root
    And the exit code is 0

  Scenario: Pretty JSON output
    Given a page is loaded
    When I run "chrome-cli page snapshot --pretty"
    Then stdout is pretty-printed JSON

  Scenario: Default output is structured text
    Given a page is loaded
    When I run "chrome-cli page snapshot"
    Then output uses "- role name [uid]" text format with indentation
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `page snapshot` captures accessibility tree via CDP `Accessibility.getFullAXTree` or `DOMSnapshot.captureSnapshot` | Must | Core functionality |
| FR2 | Default output is hierarchical text with `- role "name" [uid]` format per line | Must | Human-readable default |
| FR3 | Interactive elements (links, buttons, inputs, selects, textareas, checkboxes, radios) assigned short unique UIDs | Must | Foundation for interaction commands |
| FR4 | UID-to-backend-node-id mapping persisted in session state file | Must | Required by interaction commands #14-#17 |
| FR5 | `--json` outputs machine-readable JSON tree with `uid`, `role`, `name`, `children` fields | Must | For programmatic consumption |
| FR6 | `--verbose` includes extra properties (checked, disabled, expanded, selected, required, pressed, level, value, description, url) | Must | Detailed inspection |
| FR7 | `--file <PATH>` saves output to file instead of stdout | Must | For large snapshots and scripting |
| FR8 | `--tab <ID>` targets a specific tab | Must | Consistent with other commands |
| FR9 | UIDs stable across consecutive snapshots of unchanged pages | Should | Predictable for agents |
| FR10 | Large pages truncated gracefully with a truncation message | Should | Prevents terminal flooding |
| FR11 | `--pretty` for pretty-printed JSON output | Must | Consistent with other commands |
| FR12 | Ignored/invisible nodes excluded from output | Should | Clean, relevant output |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Snapshot of typical page (< 5,000 nodes) completes within 5 seconds |
| **Performance** | Snapshot of large page (10,000+ nodes) completes within `--timeout` window or truncates |
| **Reliability** | Graceful error on disconnected tabs, crashed pages, or pages mid-navigation |
| **Platforms** | macOS, Linux, Windows (same as project baseline) |
| **Output** | Errors to stderr as JSON, data to stdout; exit codes per `error.rs` conventions |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--tab` | String (ID or index) | Must resolve to a valid target | No (defaults to first page target) |
| `--verbose` | Boolean flag | N/A | No |
| `--file` | String (file path) | Must be a writable path | No |
| `--json` | Boolean flag | N/A | No (default is text) |
| `--pretty` | Boolean flag | N/A | No |

### Output Data (Text mode - default)

Hierarchical text with indentation:
```
- role "name" [uid]
  - child-role "child-name"
    - grandchild-role "grandchild-name" [uid]
```

### Output Data (JSON mode)

```json
{
  "uid": "s1",
  "role": "button",
  "name": "Submit",
  "properties": {},
  "children": []
}
```

### Session State (UID mapping)

```json
{
  "uid_map": {
    "s1": { "backend_node_id": 42 },
    "s2": { "backend_node_id": 87 }
  },
  "snapshot_url": "https://example.com/"
}
```

---

## Dependencies

### Internal Dependencies
- [x] Issue #4 -- CDP client (merged)
- [x] Issue #6 -- Session/connection management (merged)

### External Dependencies
- Chrome/Chromium with CDP `Accessibility` domain support

### Blocked By
- None (all dependencies resolved)

---

## Out of Scope

- Element interaction commands (click, fill, hover) -- see Issues #14-#17
- Accessibility tree diffing between snapshots
- Live/streaming accessibility tree updates
- ARIA role validation or accessibility auditing
- Cross-frame/iframe accessibility tree traversal (deferred)
- Screenshot overlays showing element references
- Custom UID prefix/format configuration

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Tree accuracy | All interactive elements assigned UIDs | Manual comparison against Chrome DevTools accessibility panel |
| UID stability | Same UIDs for unchanged pages | Consecutive snapshot comparison test |
| Mapping correctness | UIDs resolve to correct backend nodes | Integration test with interaction command |

---

## Open Questions

- [x] ~~CDP API choice: `Accessibility.getFullAXTree` vs `DOMSnapshot.captureSnapshot`?~~ -- Evaluate both in design phase; prefer `Accessibility.getFullAXTree` for direct accessibility semantics
- [x] ~~UID format: sequential (`s1`, `s2`) vs hierarchical (`1.2.3`)?~~ -- Sequential is simpler and consistent with MCP server reference
- [ ] Max tree size before truncation: what's a reasonable default? (Suggest 10,000 nodes)

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Edge cases and error states are specified
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
