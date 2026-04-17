# Requirements: Coordinate Space Helpers for Frame-Aware Coordinate Resolution

**Issues**: #198
**Date**: 2026-04-16
**Status**: Draft
**Author**: Rich Nunley

---

## User Story

**As a** browser automation engineer working with coordinate-based interactions across frames
**I want** coordinate resolution helpers that translate between frame-local and page-level coordinates, plus element-relative and percentage-based coordinate options on coordinate-dispatching commands
**So that** I can reliably target elements without manually recalculating coordinates every time viewport dimensions shift between the main page and iframes

---

## Background

Users automating enterprise applications report that viewport dimensions shift (e.g., 773px vs 908px) as focus moves between the main page and embedded iframes, invalidating coordinate math every few commands. The current CLI only accepts absolute viewport coordinates: `interact click-at X Y` has no awareness of frame-local vs page-global space, no way to anchor coordinates to an element's bounding box, and no percentage-based alternative. `page element` returns a bounding box but forces the user to do the arithmetic themselves, and any recomputation becomes stale the moment the viewport changes.

Frame-to-page coordinate translation already exists internally in `src/interact.rs` via `get_frame_viewport_offset` (using `DOM.getFrameOwner` + `DOM.getBoxModel`), but it is only wired into the `click-at`/`drag-at`/`mousedown-at`/`mouseup-at` command paths as a hidden translation. This feature exposes coordinate resolution as a first-class `page coords` command, and adds `--relative-to` and percentage syntax to the coordinate-dispatching interact commands so users can write stable, readable commands that survive frame focus shifts. This builds on the iframe frame-targeting work from issue #189 — when frame targeting lands, coordinate space helpers let users stop doing coordinate math entirely.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Resolve coordinates for a selector in the main frame

**Given** the page is loaded with a `<button id="submit">` whose bounding client rect is `{x: 100, y: 200, width: 80, height: 32}` in the main frame's viewport
**When** `agentchrome page coords --selector "css:#submit"` is run
**Then** JSON on stdout contains:
- `frame.index: 0` (the frame the selector was resolved in)
- `frame.id`: the main frame's CDP frame ID
- `frameLocal.boundingBox: {x: 100, y: 200, width: 80, height: 32}`
- `frameLocal.center: {x: 140, y: 216}` (bounding box center)
- `page.boundingBox: {x: 100, y: 200, width: 80, height: 32}` (identical to frameLocal for main frame)
- `page.center: {x: 140, y: 216}`
- `frameOffset: {x: 0, y: 0}` (main frame has no offset)
**And** the exit code is 0

### AC2: Resolve coordinates for a selector in a nested iframe

**Given** the page contains an iframe offset at `(50, 100)` in the page, and that iframe contains a `<button id="inner">` whose bounding client rect is `{x: 10, y: 20, width: 80, height: 32}` in the iframe's viewport
**When** `agentchrome page coords --frame 1 --selector "css:#inner"` is run
**Then** JSON on stdout contains:
- `frame.index: 1`
- `frameLocal.boundingBox: {x: 10, y: 20, width: 80, height: 32}` (coordinates relative to the iframe's viewport)
- `page.boundingBox: {x: 60, y: 120, width: 80, height: 32}` (frame-local + frame offset)
- `frameLocal.center: {x: 50, y: 36}`
- `page.center: {x: 100, y: 136}`
- `frameOffset: {x: 50, y: 100}`
**And** the exit code is 0

### AC3: Resolve coordinates for a UID target

**Given** a prior `page snapshot` assigned UID `s7` to an element whose bounding box is `{x: 200, y: 300, width: 120, height: 40}`
**When** `agentchrome page coords --selector "s7"` is run
**Then** `frameLocal.boundingBox` and `page.boundingBox` reflect that element's coordinates
**And** the command accepts both UID (`s7`) and CSS selector (`css:#id`) forms identically to `page element`

### AC4: Click at an offset within an element

**Given** a `<button>` with bounding client rect `{x: 100, y: 200, width: 80, height: 32}`
**When** `agentchrome interact click-at 10 5 --relative-to "css:button"` is run
**Then** the dispatched click lands at page coordinates `(110, 205)` (element's top-left origin plus the given offset)
**And** JSON on stdout contains `clicked_at: {x: 110, y: 205}` — the resolved page coordinates, not the relative input
**And** the exit code is 0

### AC5: Click at a percentage position within an element

**Given** a `<div id="target">` with bounding client rect `{x: 100, y: 200, width: 200, height: 100}`
**When** `agentchrome interact click-at 50% 50% --relative-to "css:#target"` is run
**Then** the dispatched click lands at the element's center, page coordinates `(200, 250)`
**And** JSON on stdout contains `clicked_at: {x: 200, y: 250}`
**And** `0% 0%` resolves to the element's top-left corner, and `100% 100%` resolves to `(width, height) - 1` from the top-left (i.e., the bottom-right-inclusive pixel)

### AC6: Mixed absolute and percentage coordinates

**Given** a `<div>` with bounding client rect `{x: 100, y: 200, width: 200, height: 100}`
**When** `agentchrome interact click-at 50% 10 --relative-to "css:div"` is run
**Then** the x coordinate is interpreted as a percentage of the element's width (50% of 200 = 100), and the y coordinate is interpreted as an absolute offset in pixels (10)
**And** the dispatched click lands at page coordinates `(200, 210)`

### AC7: `--relative-to` applies to drag-at, mousedown-at, and mouseup-at

**Given** a `<div>` with bounding client rect `{x: 100, y: 200, width: 200, height: 100}`
**When** `agentchrome interact drag-at 0 0 100% 100% --relative-to "css:div"` is run
**Then** the drag starts at the element's top-left `(100, 200)` and ends at its bottom-right `(299, 299)`
**And** `agentchrome interact mousedown-at 50% 50% --relative-to "css:div"` dispatches a press at the element's center
**And** `agentchrome interact mouseup-at 50% 50% --relative-to "css:div"` dispatches a release at the element's center
**And** each command's JSON output reports the resolved page coordinates, not the input values

### AC8: `--relative-to` with `--frame` resolves in that frame's space

**Given** an iframe at page offset `(50, 100)` containing a `<button>` with frame-local bounding client rect `{x: 10, y: 20, width: 80, height: 32}`
**When** `agentchrome interact click-at 50% 50% --frame 1 --relative-to "css:button"` is run
**Then** the click dispatches at page-global coordinates `(100, 136)` — the button's center in page space
**And** `clicked_at` in the output reports the resolved page coordinates `(100, 136)`

### AC9: Missing selector produces a structured error

**Given** no element on the page matches the selector
**When** `agentchrome page coords --selector "css:#does-not-exist"` is run
**Then** stderr contains a single JSON error object with fields `error` and `code`
**And** the exit code is `3` (target error, consistent with other element-target-not-found errors)
**And** nothing is written to stdout

### AC10: Invalid percentage value produces a structured error

**Given** a percentage value outside the range `0%`–`100%` (e.g., `150%`), or a malformed value (e.g., `5%%`)
**When** `agentchrome interact click-at 150% 50% --relative-to "css:button"` is run
**Then** stderr contains a single JSON error object (matching the global error contract — exactly one JSON object, not two from clap + app)
**And** the exit code is `1` (validation error)
**And** nothing is dispatched to Chrome

### AC11: `--relative-to` without a matching element produces an error before dispatch

**Given** `--relative-to "css:#missing"` references an element that does not exist
**When** `agentchrome interact click-at 50% 50% --relative-to "css:#missing"` is run
**Then** stderr contains a single JSON error object
**And** the exit code is `3` (target error)
**And** no mouse event is dispatched to Chrome

### AC12: `examples interact` and `examples page` include coordinate helper examples

**Given** the new coordinate helpers are implemented
**When** `agentchrome examples interact` is run
**Then** the output includes at least one example using `--relative-to` on `click-at` with both absolute-offset and percentage syntax
**And** when `agentchrome examples page` is run, the output includes at least one example using `page coords` with `--selector` and with `--frame`

### Generated Gherkin Preview

```gherkin
Feature: Coordinate Space Helpers
  As a browser automation engineer working with coordinate-based interactions across frames
  I want coordinate resolution helpers and element-relative coordinate options
  So that I can target elements without manually recalculating coordinates

  Scenario: Resolve coordinates for a selector in the main frame
    Given a page with a button whose bounding box is (100, 200, 80, 32)
    When I run "page coords --selector css:#submit"
    Then the output contains frame-local and page-level coordinates matching the bounding box

  Scenario: Resolve coordinates for a selector in a nested iframe
    Given a page with an iframe at page offset (50, 100) containing a button at frame-local (10, 20, 80, 32)
    When I run "page coords --frame 1 --selector css:#inner"
    Then page.boundingBox is (60, 120, 80, 32) and frameOffset is (50, 100)

  # ... all ACs become scenarios
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `page coords --selector <target> [--frame <index>]` subcommand returning frame-local and page-level coordinates for the selector's bounding box and center | Must | Selector accepts both UID (`s7`) and CSS (`css:#id`) like `page element` |
| FR2 | `--relative-to <selector>` flag on `interact click-at`, `interact drag-at`, `interact mousedown-at`, `interact mouseup-at` | Must | When present, X/Y arguments are treated as offsets from the element's top-left origin |
| FR3 | Percentage-value syntax (`N%`) on X/Y arguments when `--relative-to` is present, with independent parsing per axis (mixed absolute+percent permitted) | Must | `0%` → top/left edge, `100%` → bottom-right-inclusive edge |
| FR4 | Frame-aware coordinate translation: `--relative-to` must resolve the target element's bounding box **in the targeted frame's viewport**, then apply the existing frame-to-page offset before dispatching | Must | Reuse `get_frame_viewport_offset` |
| FR5 | Output schema: `page coords` returns a fixed JSON object with fields `frame`, `frameLocal`, `page`, `frameOffset`; coordinate commands continue to report resolved page coordinates in `clicked_at` / `mousedown_at` / `mouseup_at` / `dragged_at` | Must | No breaking changes to existing output fields |
| FR6 | Error contract: missing selector → exit code 3 (target error); invalid percentage or malformed value → exit code 1; exactly one JSON error object on stderr per invocation | Must | Matches global contract from tech.md |
| FR7 | Percentages outside `0%`–`100%` are rejected at parse time (no clamping, no silent wrap-around) | Should | Negative or >100% values indicate a likely bug in the caller |
| FR8 | `examples interact` and `examples page` include new coordinate helper examples, and `page coords --help` / `interact click-at --help` include example invocations with `--relative-to` and percentages | Must | Discoverability — users must be able to learn the feature from built-in help |
| FR9 | BDD feature file covering all acceptance criteria | Must | `tests/features/coordinate-space-helpers.feature` |
| FR10 | Implementation must not alter existing semantics when `--relative-to` is absent (pure absolute-coordinate path continues to work unchanged) | Must | Regression guard |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | `page coords` must complete in < 200ms for a single selector against a loaded page (single `DOM.getBoxModel` + optional frame-offset round-trip) |
| **Security** | No new network calls; reuses existing CDP domains (`DOM`, `Runtime`) |
| **Accessibility** | N/A — command-line feature, no user-facing UI |
| **Reliability** | Frame-offset resolution must use the same code path (`get_frame_viewport_offset`) that `click-at --frame` already uses, so page-level coordinates reported by `page coords` agree with coordinates actually dispatched by `click-at --frame` |
| **Platforms** | macOS, Linux, Windows (matches tech.md) |

---

## UI/UX Requirements

N/A — this is a CLI feature. Output format is JSON on stdout (see Data Requirements).

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--selector` (on `page coords`) | string | UID pattern (`s\d+`) or CSS selector (`css:...`) | Yes |
| `--frame` (on `page coords`) | string | Integer index, path like `1/0`, or `auto` — parsed by existing `parse_frame_arg` | No (defaults to main frame) |
| `--relative-to` (on coord-based interact commands) | string | Same validation as `--selector` on `page coords` | No (when absent, X/Y are absolute page coordinates — existing behavior) |
| `X`, `Y`, `from_x`, `from_y`, `to_x`, `to_y` (when `--relative-to` present) | number OR `N%` | Number: valid `f64`. Percentage: `0 <= N <= 100` before the `%` suffix | Yes |

### Output Data — `page coords`

| Field | Type | Description |
|-------|------|-------------|
| `frame.index` | integer | Index of the frame the selector was resolved in (0 for main frame) |
| `frame.id` | string | CDP frame ID |
| `frameLocal.boundingBox` | `{x, y, width, height}` of f64 | Bounding box in the frame's viewport |
| `frameLocal.center` | `{x, y}` of f64 | Bounding box center in the frame's viewport |
| `page.boundingBox` | `{x, y, width, height}` of f64 | Bounding box in page-global coordinates (frame-local + frame offset) |
| `page.center` | `{x, y}` of f64 | Bounding box center in page-global coordinates |
| `frameOffset` | `{x, y}` of f64 | The frame's top-left offset in page coordinates (0,0 for main frame) |

### Output Data — coordinate-based interact commands (unchanged schema)

Existing `clicked_at`, `mousedown_at`, `mouseup_at`, `dragged_at` fields continue to report the **resolved page coordinates** (not the raw input), preserving the contract established by `click-at --frame N` today.

---

## Dependencies

### Internal Dependencies
- [x] `src/frame.rs` — `FrameContext`, `parse_frame_arg`, `list_frames` (already exists from #189)
- [x] `src/interact.rs::get_frame_viewport_offset` — already exists (already exists from #189)
- [x] `src/page/element.rs::resolve_element_target` — UID/CSS resolution logic (reference implementation to mirror)
- [x] `DOM.getBoxModel` CDP method — already used by existing code

### External Dependencies
- None

### Blocked By
- None (issue #189 iframe frame targeting has already landed, providing the frame infrastructure)

### Related Issues
- #189 — Iframe frame targeting support (infrastructure this feature builds on)
- #194 — Coordinate-based drag and decomposed mouse actions (established the `drag-at` / `mousedown-at` / `mouseup-at` commands this feature extends)

---

## Out of Scope

- **Automatic coordinate stabilization** — tracking element position across DOM changes and recomputing. Users must call `page coords` or use `--relative-to` at the moment of interaction; no live tracking.
- **Named anchor points** (e.g., `"top-left"`, `"center"`, `"bottom-right"`) — percentage syntax covers these cases (`0% 0%`, `50% 50%`, `100% 100%`).
- **Coordinate caching or memoization** — each invocation resolves fresh.
- **`--relative-to` on `interact click`, `hover`, `drag`** — these commands already take an element target, so adding `--relative-to` would be redundant. Scope is limited to the coordinate-dispatching variants (`click-at`, `drag-at`, `mousedown-at`, `mouseup-at`).
- **Percentages on absolute (non-`--relative-to`) coordinate commands** — percentages without a reference element are ambiguous (percent of viewport? page? frame?) and out of scope for this release.
- **Viewport-percentage syntax** (e.g., `50vw 50vh`) — not included.

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Coordinate stability across frame focus shifts | Zero coordinate recomputation required by users when frame changes | Measured by feedback: `click-at --relative-to` with the same arguments produces the correct click regardless of intervening frame focus changes |
| Command adoption | Documented in `examples interact` and `examples page`; `page coords --help` shows examples | Verified via `agentchrome examples interact` and `agentchrome page coords --help` output |
| No regressions | All existing `click-at`/`drag-at`/`mousedown-at`/`mouseup-at` absolute-coordinate scenarios pass unchanged | BDD test suite exit code 0 |

---

## Open Questions

- [x] How should `100%` resolve — to `element.x + width` (first pixel outside), or `element.x + width - 1` (last pixel inside)? **Resolved**: Last pixel inside (`width - 1`, `height - 1`) so that `100% 100%` produces a click that actually hits the element's bottom-right corner.
- [x] Should `--relative-to` accept UIDs in addition to CSS selectors? **Resolved**: Yes — same target syntax as `page element` (`s7` or `css:#id`), for consistency across the CLI.
- [x] Should mixed axes be allowed (e.g., `50% 10`)? **Resolved**: Yes (AC6) — each axis parsed independently.
- [ ] Should `page coords` accept `--uid` as a shortcut or require `--selector` accepting both forms? *(Leaning toward single `--selector` that accepts both, matching `page element`'s target syntax. Will confirm during design.)*

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #198 | 2026-04-16 | Initial feature spec — `page coords` command plus `--relative-to` and percentage syntax on coordinate-dispatching interact commands |

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements (design concerns deferred to PLAN phase)
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified (AC9–AC11)
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (one remains for PLAN phase)
