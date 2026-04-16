# Requirements: Page Analyze Command for Page Structure Discovery

**Issues**: #190
**Date**: 2026-04-16
**Status**: Approved
**Author**: Claude (spec-writer)

---

## User Story

**As a** browser automation engineer starting work on an unfamiliar page
**I want** a single command that reveals the page's structure, frameworks, and potential automation challenges
**So that** I can choose the right interaction strategy immediately instead of spending 30+ minutes probing the DOM manually

---

## Background

Users report spending the first 30 minutes of complex automation sessions manually probing pages with `js exec` to understand the DOM architecture — discovering iframes, identifying overlays that intercept clicks, finding accessibility shadow DOM elements, and cataloging media elements that gate navigation. Existing commands like `perf analyze`, `page snapshot`, and `page find` each cover a piece of the puzzle but none provides a holistic structural overview. A `page analyze` command would reduce this discovery phase to a single call, immediately revealing the page structure and suggesting interaction approaches.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Structural summary output

**Given** a loaded page with iframes, media elements, overlays, and a known frontend framework
**When** `page analyze` is run
**Then** JSON output includes:
- `iframes`: array with count, URLs, visibility, and dimensions per frame
- `frameworks`: array of detected frontend frameworks (React, Angular, Vue, Storyline, SCORM)
- `interactiveElements`: count of interactive elements per frame (main frame and each iframe)
- `media`: array of media elements with tag type, source, and playback state
- `overlays`: array of overlay/blocker elements covering interactive areas (with selector, dimensions, z-index)
- `shadowDom`: object with presence flag and host element count
- `summary`: object with total iframe count, total interactive element count, and boolean flags for has_overlays, has_media, has_shadow_dom, has_frameworks

**Example**:
- Given: A page with one iframe pointing to `https://example.com/child`, a React app, one `<video>` element paused, and an overlay div at z-index 9999
- When: `agentchrome page analyze`
- Then: JSON with `iframes` array of length 1, `frameworks` containing `"React"`, `media` array with one entry showing `"paused"` state, `overlays` array with one entry

### AC2: Frame-scoped analysis

**Given** a `--frame <index>` argument
**When** `page analyze` is run
**Then** analysis is scoped to the specified frame context — all counts, element detection, and overlay analysis operate within that frame only
**And** the output JSON `scope` field indicates the frame index used

**Example**:
- Given: A page with iframe at index 1 containing a Storyline course
- When: `agentchrome page analyze --frame 1`
- Then: JSON reflects only the content within frame 1, with `scope` set to `"frame:1"`

### AC3: Simple page handling

**Given** a page with no iframes, overlays, special frameworks, media elements, or shadow DOM
**When** `page analyze` is run
**Then** the output reflects a simple structure:
- `iframes` is an empty array
- `frameworks` is an empty array
- `media` is an empty array
- `overlays` is an empty array
- `shadowDom.present` is `false`
- `summary.iframeCount` is `0`
- `summary.hasOverlays` is `false`
**And** the command exits with code 0 without errors

### AC4: Documentation updated

**Given** the new `page analyze` command
**When** `examples page` is run
**Then** `page analyze` examples are included in the output with at least:
- Basic usage example
- Frame-scoped usage example

### AC5: Cross-origin iframe enumeration

**Given** a page with cross-origin (out-of-process) iframes
**When** `page analyze` is run
**Then** cross-origin iframes are listed in the `iframes` array with their URL and dimensions
**And** interactive element counts for cross-origin iframes are reported (or `null` if inaccessible due to security restrictions)
**And** the `iframes` entry includes a `crossOrigin` boolean field set to `true`

### AC6: Invalid frame index error

**Given** a `--frame <index>` argument with an index that does not exist
**When** `page analyze` is run
**Then** a JSON error is written to stderr with a descriptive message indicating the frame index is out of range
**And** the command exits with a non-zero exit code

### AC7: Undetermined output fields use null

**Given** a page where certain analysis dimensions cannot be determined (e.g., a media element's playback state is not queryable, or an iframe's interactive element count is inaccessible)
**When** `page analyze` is run
**Then** fields that cannot be determined appear as `null` in the JSON output rather than being omitted or set to zero
**And** the command completes successfully with exit code 0

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | New `page analyze` subcommand with structured JSON output | Must | Follows existing `PageCommand` enum pattern |
| FR2 | Iframe enumeration with URLs, visibility, dimensions, and cross-origin flag | Must | Reuse `frame::list_frames()` for frame tree; augment with visibility/dimension info |
| FR3 | Frontend framework detection (React, Angular, Vue, Svelte, Storyline, SCORM) | Should | Use DOM signatures |
| FR4 | Interactive element count per frame (main + each iframe) | Must | Count elements matching interactive role selectors |
| FR5 | Media element cataloging with tag type, source, and playback state | Should | Query video, audio, embed elements |
| FR6 | Overlay/blocker element detection | Must | Identify elements with high z-index covering interactive areas |
| FR7 | Accessibility shadow DOM detection (presence + host count) | Should | Detect shadow root hosts |
| FR8 | Help documentation and built-in examples updated | Must | Add examples to src/examples.rs |
| FR9 | BDD test scenarios covering page analyze | Must | Feature file with scenarios for all ACs |
| FR10 | Frame-scoped analysis via existing --frame argument | Must | Reuse PageArgs.frame and frame::resolve_frame() |
| FR11 | Summary object with aggregate counts and boolean flags | Should | Quick-glance overview |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Analysis completes within 5 seconds on pages with up to 10 iframes and 1000 DOM elements |
| **Reliability** | Graceful degradation when individual analysis dimensions fail (report null for that dimension, continue with others) |
| **Platforms** | macOS, Linux, Windows (consistent with all agentchrome commands) |
| **Output** | JSON on stdout, JSON errors on stderr, meaningful exit codes per tech.md |

---

## UI/UX Requirements

| Element | Requirement |
|---------|-------------|
| **Output format** | Structured JSON on stdout; camelCase field names per project convention |
| **Error format** | JSON error objects on stderr with descriptive messages |
| **Exit codes** | 0 for success, standard agentchrome error codes for failures |
| **Frame argument** | Uses existing --frame global option on page command group |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--frame` | string (index, path, or "auto") | Must resolve to valid frame index | No |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| `scope` | string | `"main"` or `"frame:<index>"` indicating analysis scope |
| `url` | string | URL of the analyzed page/frame |
| `title` | string | Page title (main frame only) |
| `iframes` | array of objects | Each: `{ url, name, index, visible, width, height, crossOrigin }` |
| `frameworks` | array of strings | Detected framework names |
| `interactiveElements` | object | `{ main: number, frames: { "<index>": number or null } }` |
| `media` | array of objects | Each: `{ tag, src, state, width, height }` where state is playing/paused/ended/null |
| `overlays` | array of objects | Each: `{ selector, zIndex, width, height, coversInteractive: bool }` |
| `shadowDom` | object | `{ present: bool, hostCount: number }` |
| `summary` | object | `{ iframeCount, interactiveElementCount, hasOverlays, hasMedia, hasShadowDom, hasFrameworks }` |

---

## Dependencies

### Internal Dependencies
- [x] Frame enumeration (`src/frame.rs` — `list_frames()`, `resolve_frame()`)
- [x] CDP client (`src/cdp/client.rs`)
- [x] Session management (`src/session.rs`)
- [x] Output formatting (`GlobalOpts.output`)

### External Dependencies
- [x] Chrome DevTools Protocol (DOM, Runtime, Page domains)

### Blocked By
- None

---

## Out of Scope

- Automated remediation or strategy execution (analysis only — no auto-fix for overlays)
- Performance metrics (covered by `perf analyze` and `perf vitals`)
- Network analysis (covered by `network` commands)
- Full accessibility audit (focus is on structural discovery, not compliance checking)
- CSS layout analysis beyond overlay detection
- JavaScript framework version detection (only presence/absence)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Discovery time reduction | < 5 seconds vs 30+ minutes manual probing | Time to understand page structure |
| Command completeness | All 6 analysis dimensions in single call | Verify JSON output contains all required fields |
| Error resilience | 0 crashes on malformed pages | Run against pages with missing/broken iframes, no media, etc. |

---

## Open Questions

- None

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #190 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified (AC5, AC6, AC7)
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
