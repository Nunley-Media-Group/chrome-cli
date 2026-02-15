# Requirements: Page Text Extraction

**Issue**: #9
**Date**: 2026-02-11
**Status**: Draft
**Author**: Claude (writing-specs)

---

## User Story

**As a** developer / automation engineer
**I want** to extract readable text content from a browser page via the CLI
**So that** I can process page content in scripts and AI pipelines without parsing HTML

---

## Background

Extracting readable text content from a web page is a fundamental capability for browser automation. AI agents and scripts need to understand page content without processing raw HTML. The `page text` command provides this by executing JavaScript in the page context via `Runtime.evaluate` to walk the DOM and return human-readable text. This builds on the existing CDP session infrastructure (Issue #6) and follows the same command patterns established by `tabs` and `navigate`.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Extract all visible text from current page

**Given** Chrome is running with a page loaded at "https://example.com"
**When** I run `chrome-cli page text`
**Then** stdout contains JSON with `text`, `url`, and `title` fields
**And** the `text` field contains the visible text content of the page
**And** the `url` field matches the current page URL
**And** the `title` field matches the page title

### AC2: Target a specific tab with --tab

**Given** Chrome is running with multiple tabs open
**When** I run `chrome-cli page text --tab <ID>`
**Then** the text is extracted from the specified tab
**And** the `url` field matches the targeted tab's URL

### AC3: Plain text output with --plain

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page text --plain`
**Then** stdout contains only the raw text content (no JSON wrapper)
**And** no JSON structure is present in the output

### AC4: Extract text from specific element with --selector

**Given** Chrome is running with a page containing an element matching `#main-content`
**When** I run `chrome-cli page text --selector "#main-content"`
**Then** the `text` field contains only text from that element and its descendants

### AC5: Selector targets non-existent element

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page text --selector "#does-not-exist"`
**Then** stderr contains a JSON error indicating the element was not found
**And** the exit code is non-zero

### AC6: Script and style content excluded

**Given** Chrome is running with a page containing `<script>` and `<style>` elements
**When** I run `chrome-cli page text`
**Then** the `text` field does not contain JavaScript source code
**And** the `text` field does not contain CSS source code

### AC7: Basic structure preserved

**Given** Chrome is running with a page containing headings, paragraphs, and lists
**When** I run `chrome-cli page text`
**Then** paragraphs and headings are separated by newlines in the `text` field
**And** the text maintains a readable structure

### AC8: Page with no content

**Given** Chrome is running with a blank page (about:blank)
**When** I run `chrome-cli page text`
**Then** stdout contains JSON with an empty `text` field
**And** the exit code is 0

### AC9: Pretty JSON output with --pretty

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page text --pretty`
**Then** stdout contains pretty-printed JSON with indentation

### AC10: Iframe content handling (default: skip)

**Given** Chrome is running with a page containing iframes
**When** I run `chrome-cli page text`
**Then** the `text` field contains only text from the main frame
**And** iframe content is not included

### Generated Gherkin Preview

```gherkin
Feature: Page text extraction
  As a developer / automation engineer
  I want to extract readable text content from a browser page via the CLI
  So that I can process page content in scripts and AI pipelines without parsing HTML

  Background:
    Given Chrome is running with CDP enabled

  Scenario: Extract all visible text from current page
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli page text"
    Then stdout contains JSON with keys "text", "url", "title"
    And the "text" field contains visible page text
    And the "url" field is "https://example.com/"

  Scenario: Target a specific tab
    Given multiple tabs are open
    When I run "chrome-cli page text --tab <ID>"
    Then text is extracted from the specified tab

  Scenario: Plain text output
    Given a page is loaded
    When I run "chrome-cli page text --plain"
    Then stdout contains only raw text content

  Scenario: Extract text from specific CSS selector
    Given a page with element "#main-content"
    When I run "chrome-cli page text --selector '#main-content'"
    Then the "text" field contains only that element's text

  Scenario: Selector targets non-existent element
    Given a page is loaded
    When I run "chrome-cli page text --selector '#does-not-exist'"
    Then stderr contains a JSON error
    And the exit code is non-zero

  Scenario: Script and style content excluded
    Given a page with script and style elements
    When I run "chrome-cli page text"
    Then the text does not contain script or style content

  Scenario: Basic structure preserved
    Given a page with headings and paragraphs
    When I run "chrome-cli page text"
    Then paragraphs and headings are separated by newlines

  Scenario: Page with no content
    Given a blank page is loaded
    When I run "chrome-cli page text"
    Then JSON output has an empty "text" field
    And the exit code is 0

  Scenario: Pretty JSON output
    Given a page is loaded
    When I run "chrome-cli page text --pretty"
    Then stdout contains pretty-printed JSON

  Scenario: Iframe content excluded by default
    Given a page with iframes
    When I run "chrome-cli page text"
    Then text contains only main frame content
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `page text` extracts visible text via `Runtime.evaluate` | Must | Core functionality |
| FR2 | JSON output with `text`, `url`, `title` fields | Must | Default output format |
| FR3 | `--plain` flag outputs raw text only | Must | For piping to other tools |
| FR4 | `--selector <CSS>` extracts text from a specific element | Must | Targeted extraction |
| FR5 | `--tab <ID>` targets a specific tab | Must | Consistent with other commands |
| FR6 | Exclude script/style element content | Must | Clean text extraction |
| FR7 | Preserve basic text structure (newlines between blocks) | Must | Readable output |
| FR8 | Handle empty/blank pages gracefully | Must | Return empty text, exit 0 |
| FR9 | `--pretty` flag for pretty-printed JSON | Must | Consistent with other commands |
| FR10 | Skip iframe content by default | Should | Simplifies initial implementation |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Text extraction completes within the global `--timeout` window (default 30s) |
| **Reliability** | Graceful error on disconnected tabs, crashed pages, or pages mid-load |
| **Platforms** | macOS, Linux, Windows (same as project baseline) |
| **Output** | Errors to stderr as JSON, data to stdout; exit codes per `error.rs` conventions |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--tab` | String (ID or index) | Must resolve to a valid target | No (defaults to first page target) |
| `--selector` | String (CSS selector) | Must be a valid CSS selector | No |
| `--plain` | Boolean flag | N/A | No |

### Output Data (JSON mode)

| Field | Type | Description |
|-------|------|-------------|
| `text` | String | Extracted visible text content |
| `url` | String | URL of the page |
| `title` | String | Title of the page |

### Output Data (Plain mode)

Raw text string to stdout, no JSON wrapper.

---

## Dependencies

### Internal Dependencies
- [x] Issue #4 — CDP client (merged)
- [x] Issue #6 — Session/connection management (merged)

### Blocked By
- None (all dependencies resolved)

---

## Out of Scope

- Extracting text from iframes (deferred; main frame only for now)
- Accessibility tree-based extraction (see Issue #10)
- HTML output mode
- Text extraction from PDF content embedded in pages
- Full-text search within extracted text
- Recursive iframe traversal with `--include-iframes` flag (future enhancement)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Text accuracy | `innerText` equivalent output | Manual comparison against browser's `document.body.innerText` |
| No script/style leakage | 0 occurrences | Test with pages containing inline scripts and styles |

---

## Open Questions

- [x] ~~Use `innerText` vs accessibility tree?~~ — Use `innerText` for this issue; accessibility tree is Issue #10
- [x] ~~Include iframe content?~~ — Skip by default for v1

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
