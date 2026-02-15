# Requirements: Screenshot Capture

**Issue**: #12
**Date**: 2026-02-12
**Status**: Approved
**Author**: Claude

---

## User Story

**As a** developer / automation engineer
**I want** to capture screenshots of browser pages (viewport, full-page, or specific elements) via the CLI
**So that** I can visually verify page state, capture evidence for test reports, and debug rendering issues in automated pipelines

---

## Background

Screenshot capture is a core browser automation capability essential for visual debugging, test evidence, and CI/CD artifact generation. The `page screenshot` command captures images via CDP's `Page.captureScreenshot` method, supporting viewport-only, full-page, element-targeted, and region-clipped captures in PNG, JPEG, and WebP formats. Screenshots can be saved to a file or output as base64-encoded JSON for piping into other tools. This builds on the existing CDP session infrastructure (Issue #6) and the accessibility tree UID system (Issue #10) for element targeting.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Capture viewport screenshot (default)

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page screenshot`
**Then** stdout contains JSON with `format`, `data`, `width`, and `height` fields
**And** the `format` field is `"png"`
**And** the `data` field contains a valid base64-encoded PNG image
**And** the `width` and `height` reflect the viewport dimensions

### AC2: Save screenshot to file with --file

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page screenshot --file /tmp/screenshot.png`
**Then** the file `/tmp/screenshot.png` is created with valid PNG image data
**And** stdout contains JSON with `format`, `file`, `width`, and `height` fields
**And** the `file` field contains the path `/tmp/screenshot.png`

### AC3: Target a specific tab with --tab

**Given** Chrome is running with multiple tabs open
**When** I run `chrome-cli page screenshot --tab <ID>`
**Then** the screenshot is captured from the specified tab
**And** the output reflects the targeted tab's viewport

### AC4: Full-page screenshot with --full-page

**Given** Chrome is running with a page that scrolls beyond the viewport
**When** I run `chrome-cli page screenshot --full-page`
**Then** the screenshot captures the entire scrollable page content
**And** the `height` in the output is greater than the viewport height

### AC5: Element screenshot by CSS selector with --selector

**Given** Chrome is running with a page containing an element matching `#logo`
**When** I run `chrome-cli page screenshot --selector "#logo"`
**Then** the screenshot captures only the bounding box of that element
**And** the `width` and `height` reflect the element's dimensions

### AC6: Element screenshot by accessibility UID with --uid

**Given** Chrome has a page loaded and a snapshot has been captured (UIDs assigned)
**When** I run `chrome-cli page screenshot --uid s1`
**Then** the screenshot captures only the bounding box of the element with UID `s1`
**And** the `width` and `height` reflect the element's dimensions

### AC7: JPEG format with --format jpeg

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page screenshot --format jpeg`
**Then** the output `format` field is `"jpeg"`
**And** the `data` contains a valid base64-encoded JPEG image

### AC8: WebP format with --format webp

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page screenshot --format webp`
**Then** the output `format` field is `"webp"`
**And** the `data` contains a valid base64-encoded WebP image

### AC9: Custom quality with --quality

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page screenshot --format jpeg --quality 50`
**Then** the JPEG screenshot is captured with quality 50
**And** the base64 data is smaller than a quality-100 capture of the same page

### AC10: Region clipping with --clip

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page screenshot --clip 10,20,200,100`
**Then** the screenshot captures only the specified region (x=10, y=20, width=200, height=100)
**And** the `width` is 200 and `height` is 100

### AC11: Conflicting --full-page with --selector

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page screenshot --full-page --selector "#logo"`
**Then** stderr contains a JSON error indicating the flags are mutually exclusive
**And** the exit code is non-zero

### AC12: Conflicting --full-page with --uid

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page screenshot --full-page --uid s1`
**Then** stderr contains a JSON error indicating the flags are mutually exclusive
**And** the exit code is non-zero

### AC13: Non-existent CSS selector

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page screenshot --selector "#does-not-exist"`
**Then** stderr contains a JSON error indicating the element was not found
**And** the exit code is non-zero

### AC14: Non-existent UID

**Given** Chrome is running with a page loaded and a snapshot has been captured
**When** I run `chrome-cli page screenshot --uid s999`
**Then** stderr contains a JSON error indicating the UID was not found
**And** the exit code is non-zero

### AC15: Blank page screenshot

**Given** Chrome is running with a blank page (about:blank)
**When** I run `chrome-cli page screenshot`
**Then** a screenshot is captured successfully (blank image, not an error)
**And** the exit code is 0

### AC16: Quality ignored for PNG format

**Given** Chrome is running with a page loaded
**When** I run `chrome-cli page screenshot --format png --quality 50`
**Then** the `--quality` flag is ignored (PNG is lossless)
**And** the screenshot is captured normally

### Generated Gherkin Preview

```gherkin
Feature: Screenshot capture
  As a developer / automation engineer
  I want to capture screenshots of browser pages via the CLI
  So that I can visually verify page state and debug rendering issues

  Background:
    Given Chrome is running with CDP enabled

  Scenario: Capture viewport screenshot (default)
    Given a page is loaded
    When I run "chrome-cli page screenshot"
    Then stdout contains JSON with keys "format", "data", "width", "height"
    And the "format" field is "png"

  Scenario: Save screenshot to file
    Given a page is loaded
    When I run "chrome-cli page screenshot --file /tmp/screenshot.png"
    Then a valid PNG file exists at "/tmp/screenshot.png"
    And stdout contains JSON with keys "format", "file", "width", "height"

  Scenario: Target a specific tab
    Given multiple tabs are open
    When I run "chrome-cli page screenshot --tab <ID>"
    Then the screenshot is captured from the specified tab

  Scenario: Full-page screenshot
    Given a page with content exceeding the viewport
    When I run "chrome-cli page screenshot --full-page"
    Then the screenshot captures the entire scrollable page

  Scenario: Element screenshot by CSS selector
    Given a page with element "#logo"
    When I run "chrome-cli page screenshot --selector '#logo'"
    Then the screenshot captures only that element

  Scenario: Element screenshot by accessibility UID
    Given a page with a snapshot captured
    When I run "chrome-cli page screenshot --uid s1"
    Then the screenshot captures only the element with UID s1

  Scenario: JPEG format
    Given a page is loaded
    When I run "chrome-cli page screenshot --format jpeg"
    Then the output format is "jpeg"

  Scenario: WebP format
    Given a page is loaded
    When I run "chrome-cli page screenshot --format webp"
    Then the output format is "webp"

  Scenario: Custom quality
    Given a page is loaded
    When I run "chrome-cli page screenshot --format jpeg --quality 50"
    Then the screenshot uses quality 50

  Scenario: Region clipping
    Given a page is loaded
    When I run "chrome-cli page screenshot --clip 10,20,200,100"
    Then the screenshot captures region (10,20,200,100)

  Scenario: Conflicting --full-page with --selector
    Given a page is loaded
    When I run "chrome-cli page screenshot --full-page --selector '#logo'"
    Then stderr contains a JSON error about mutually exclusive flags

  Scenario: Conflicting --full-page with --uid
    Given a page is loaded
    When I run "chrome-cli page screenshot --full-page --uid s1"
    Then stderr contains a JSON error about mutually exclusive flags

  Scenario: Non-existent CSS selector
    Given a page is loaded
    When I run "chrome-cli page screenshot --selector '#does-not-exist'"
    Then stderr contains a JSON error about element not found

  Scenario: Non-existent UID
    Given a page with a snapshot captured
    When I run "chrome-cli page screenshot --uid s999"
    Then stderr contains a JSON error about UID not found

  Scenario: Blank page screenshot
    Given a blank page is loaded
    When I run "chrome-cli page screenshot"
    Then a screenshot is captured successfully
    And the exit code is 0

  Scenario: Quality ignored for PNG
    Given a page is loaded
    When I run "chrome-cli page screenshot --format png --quality 50"
    Then the screenshot is captured normally ignoring quality
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `page screenshot` captures viewport via `Page.captureScreenshot` | Must | Core functionality |
| FR2 | JSON output with `format`, `data`, `width`, `height` (base64 mode) | Must | Default when no `--file` |
| FR3 | `--file <PATH>` saves decoded binary image to disk | Must | File output mode |
| FR4 | `--full-page` captures entire scrollable page | Must | Temporarily adjust viewport, restore after |
| FR5 | `--selector <CSS>` captures a specific element by CSS selector | Must | Uses `DOM.getBoxModel` for clip region |
| FR6 | `--uid <UID>` captures a specific element by accessibility UID | Must | Resolves backend node via snapshot state |
| FR7 | `--format <FORMAT>` supports `png`, `jpeg`, `webp` | Must | CDP supports all three |
| FR8 | `--quality <N>` sets JPEG/WebP quality (0-100, default: 80) | Must | Ignored for PNG |
| FR9 | `--clip <X,Y,WIDTH,HEIGHT>` captures a specific viewport region | Must | Direct CDP clip parameter |
| FR10 | `--tab <ID>` targets a specific tab | Must | Consistent with other commands |
| FR11 | Mutual exclusion: `--full-page` cannot combine with `--selector`/`--uid` | Must | Error with clear message |
| FR12 | Blank pages return a screenshot (not an error) | Must | Graceful handling |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Screenshot capture completes within the global `--timeout` window (default 30s) |
| **Reliability** | Graceful error on disconnected tabs, crashed pages, or invisible elements |
| **Platforms** | macOS, Linux, Windows (same as project baseline) |
| **Output** | Errors to stderr as JSON, data to stdout; exit codes per `error.rs` conventions |
| **File size** | Warn to stderr if base64 output exceeds 10MB (large image) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--tab` | String (ID or index) | Must resolve to a valid target | No (defaults to active tab) |
| `--file` | PathBuf | Must be a writable path | No (base64 JSON output if omitted) |
| `--full-page` | Boolean flag | Cannot combine with `--selector`/`--uid` | No |
| `--selector` | String (CSS selector) | Must match at least one element | No |
| `--uid` | String (accessibility UID) | Must exist in snapshot state | No |
| `--format` | Enum: png, jpeg, webp | Must be one of the three | No (default: png) |
| `--quality` | Integer (0-100) | Only meaningful for jpeg/webp | No (default: 80) |
| `--clip` | String "X,Y,WIDTH,HEIGHT" | Must parse to four positive numbers | No |

### Output Data (base64 JSON mode, no --file)

| Field | Type | Description |
|-------|------|-------------|
| `format` | String | Image format: "png", "jpeg", or "webp" |
| `data` | String | Base64-encoded image data |
| `width` | u32 | Image width in pixels |
| `height` | u32 | Image height in pixels |

### Output Data (file mode, --file)

| Field | Type | Description |
|-------|------|-------------|
| `format` | String | Image format |
| `file` | String | Path where the file was saved |
| `width` | u32 | Image width in pixels |
| `height` | u32 | Image height in pixels |

---

## Dependencies

### Internal Dependencies
- [x] Issue #4 — CDP client (merged)
- [x] Issue #6 — Session/connection management (merged)
- [x] Issue #10 — Accessibility tree snapshot / UID system (merged)

### Blocked By
- None (all dependencies resolved)

---

## Out of Scope

- Video capture or screen recording
- PDF export (separate feature)
- Multi-page screenshot stitching
- Automatic retina/device pixel ratio handling (uses viewport pixels)
- Streaming screenshots to stdout in binary mode (base64 JSON only)
- Diffing or comparing screenshots
- Element screenshot via XPath

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Image validity | All output images decode without error | Validate PNG/JPEG/WebP headers in tests |
| Element accuracy | Element screenshots match bounding box | Compare dimensions to `DOM.getBoxModel` output |
| Full-page accuracy | Full-page captures entire scroll height | Compare output height to `document.scrollHeight` |

---

## Open Questions

- [x] ~~Use CDP `Page.captureScreenshot` directly vs. Puppeteer-style wrapper?~~ — Direct CDP, consistent with architecture
- [x] ~~Should `--quality` error for PNG or silently ignore?~~ — Silently ignore (CDP ignores it too)
- [x] ~~Binary stdout vs base64 JSON?~~ — Base64 JSON for scripting; `--file` for binary output

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
