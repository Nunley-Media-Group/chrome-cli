# Requirements: Element-Targeted Scrolling for Inner Containers

**Issues**: #182
**Date**: 2026-04-16
**Status**: Draft
**Author**: Rich Nunley

---

## User Story

**As a** developer or AI agent automating pages with scrollable inner containers
**I want** to scroll specific elements by CSS selector or accessibility UID
**So that** I can navigate content within nested scrollable containers without resorting to `js exec`

---

## Background

`interact scroll` currently scrolls the viewport by default. A `--container` flag exists that accepts a combined target string (UID like `s3` or `css:.scrollable`), but the interface lacks dedicated `--selector` and `--uid` flags that would provide a more ergonomic and discoverable API for targeting scrollable elements. The `css:` prefix convention is inconsistent with how the `page screenshot` command uses separate `--selector` and `--uid` flags. Additionally, no validation exists to detect whether a targeted element is actually scrollable, leading to silent no-ops when the target cannot scroll.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Scroll inner container by CSS selector

**Given** a page with a scrollable container matching CSS selector `.stage`
**When** I run `interact scroll --selector ".stage" --direction down`
**Then** the `.stage` element scrolls down by the default amount within its own scrollable area
**And** the JSON output includes the scroll delta and new scroll position

**Example**:
- Given: A page with `<div class="stage" style="overflow:auto; height:200px">` containing 1000px of content
- When: `agentchrome interact scroll --selector ".stage" --direction down`
- Then: The `.stage` element scrollTop increases by the viewport height equivalent, and JSON output shows `scrolled.y > 0`

### AC2: Scroll inner container by UID

**Given** a page snapshot has been taken and a scrollable container has UID `s42`
**When** I run `interact scroll --uid s42 --direction down --amount 300`
**Then** the element with UID `s42` scrolls down 300 pixels within its scrollable area
**And** the JSON output includes the scroll delta and new scroll position

**Example**:
- Given: `agentchrome page snapshot` has been run, and a scrollable element is assigned UID `s42`
- When: `agentchrome interact scroll --uid s42 --direction down --amount 300`
- Then: The element scrollTop increases by 300, and JSON output shows `scrolled.y` approximately 300

### AC3: Error on non-scrollable target

**Given** a target element that is not scrollable (no overflow content)
**When** I run `interact scroll --selector "#static-div" --direction down`
**Then** I receive a JSON error on stderr indicating the element is not scrollable
**And** the exit code is non-zero

**Example**:
- Given: A page with `<div id="static-div" style="height:100px">Short content</div>`
- When: `agentchrome interact scroll --selector "#static-div" --direction down`
- Then: stderr contains `{"error": "Element is not scrollable", ...}` and exit code is 1

### AC4: Selector and UID flags conflict with each other

**Given** a user provides both `--selector` and `--uid` flags
**When** the command is parsed
**Then** clap rejects the input with a conflict error before execution
**And** the error is formatted as JSON on stderr

### AC5: Smooth scroll works with targeted containers

**Given** a page with a scrollable container matching CSS selector `.panel`
**When** I run `interact scroll --selector ".panel" --direction down --smooth`
**Then** the `.panel` element scrolls down smoothly (animated) within its scrollable area
**And** the command waits for the smooth scroll animation to settle before reporting final position

### AC6: All four scroll directions work with targeted containers

**Given** a page with a scrollable container that has both horizontal and vertical overflow
**When** I run `interact scroll --selector ".panel" --direction <direction>` for each of up, down, left, right
**Then** the container scrolls in the specified direction by the default amount
**And** the JSON output reflects the correct scroll delta for each direction

### Generated Gherkin Preview

```gherkin
Feature: Element-targeted scrolling for inner containers
  As a developer or AI agent automating pages with scrollable inner containers
  I want to scroll specific elements by CSS selector or accessibility UID
  So that I can navigate content within nested scrollable containers without resorting to js exec

  Scenario: Scroll inner container by CSS selector
    Given a page with a scrollable container matching CSS selector ".stage"
    When I run interact scroll with --selector ".stage" --direction down
    Then the ".stage" element scrolls down by the default amount
    And the JSON output includes scroll delta and position

  Scenario: Scroll inner container by UID
    Given a page snapshot assigns UID "s42" to a scrollable container
    When I run interact scroll with --uid s42 --direction down --amount 300
    Then the element scrolls down 300 pixels
    And the JSON output includes scroll delta and position

  Scenario: Error on non-scrollable target
    Given a target element "#static-div" that is not scrollable
    When I run interact scroll with --selector "#static-div" --direction down
    Then stderr contains a JSON error indicating the element is not scrollable
    And the exit code is non-zero

  Scenario: Selector and UID flags conflict
    Given both --selector and --uid are provided
    When the command is parsed
    Then clap rejects the input with a conflict error

  Scenario: Smooth scroll with targeted container
    Given a page with a scrollable container ".panel"
    When I run interact scroll with --selector ".panel" --direction down --smooth
    Then the container scrolls smoothly and the command waits for animation to settle

  Scenario: All directions work with targeted containers
    Given a page with a container that has horizontal and vertical overflow
    When I scroll in each of the four directions
    Then each direction scrolls the container accordingly
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `--selector` flag accepts a CSS selector string to target a scrollable container for directional scrolling | Must | No `css:` prefix required — raw selector string |
| FR2 | `--uid` flag accepts an accessibility UID to target a scrollable container for directional scrolling | Must | Requires prior `page snapshot` |
| FR3 | `--selector` and `--uid` conflict with each other (mutual exclusion enforced by clap) | Must | |
| FR4 | `--selector` and `--uid` conflict with `--to-element`, `--to-top`, `--to-bottom` (positional scroll modes) | Must | Consistent with existing `--container` conflicts |
| FR5 | `--direction` and `--amount` flags work with `--selector` and `--uid` targeted containers | Must | Same behavior as existing `--container` |
| FR6 | `--smooth` flag works with `--selector` and `--uid` targeted containers | Must | Animated scrolling with settle-wait |
| FR7 | When the targeted element is not scrollable, return a descriptive JSON error on stderr and exit non-zero | Must | Check `scrollHeight > clientHeight` or `scrollWidth > clientWidth` |
| FR8 | JSON output format matches existing scroll output: `{ scrolled: {x, y}, position: {x, y} }` | Must | |
| FR9 | Deprecate or maintain backward compatibility with existing `--container` flag | Should | `--container` continues to work; new flags are the preferred interface |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Scrollability check adds < 10ms overhead (single DOM property read) |
| **Platforms** | macOS, Linux, Windows — all platforms where agentchrome runs |
| **Compatibility** | Existing `--container` flag continues to work unchanged |
| **Output format** | JSON on stdout, JSON errors on stderr per project convention |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--selector` | String (CSS selector) | Valid CSS selector; must match exactly one element | No (conflicts with `--uid`) |
| `--uid` | String (UID like `s5`) | Must be a valid UID from a prior snapshot | No (conflicts with `--selector`) |
| `--direction` | Enum (up/down/left/right) | Must be valid direction | Yes (when using `--selector` or `--uid`) |
| `--amount` | u32 (pixels) | Positive integer | No (defaults to viewport dimension) |
| `--smooth` | bool | N/A | No (defaults to false) |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| `scrolled.x` | f64 | Horizontal scroll delta in pixels |
| `scrolled.y` | f64 | Vertical scroll delta in pixels |
| `position.x` | f64 | Final horizontal scroll position |
| `position.y` | f64 | Final vertical scroll position |
| `snapshot` | object or null | Accessibility snapshot if `--include-snapshot` used |

---

## Dependencies

### Internal Dependencies
- [x] Scroll interaction command (`src/interact.rs`) — existing container scroll infrastructure
- [x] Target resolution (`resolve_target_to_backend_node_id`) — resolves UID and CSS selectors to DOM nodes
- [x] CLI argument parsing (`src/cli/mod.rs`, `ScrollArgs`) — clap derive struct

### External Dependencies
- [x] Chrome DevTools Protocol — `Runtime.callFunctionOn`, `DOM.getDocument`, `DOM.querySelector`

### Blocked By
- None

---

## Out of Scope

- Auto-detection of the "primary" scrollable container on a page
- Nested scrollable container chain handling (scrolling through multiple nested scroll contexts)
- Removing or breaking the existing `--container` flag
- Scroll-to-element within a container (combining `--to-element` with `--selector`/`--uid`)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| All ACs pass BDD tests | 6/6 | `cargo test --test bdd` |
| Scrollability check overhead | < 10ms | Manual measurement during smoke test |
| No regressions in existing scroll modes | 0 failures | Existing scroll feature tests pass |

---

## Open Questions

- None

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #182 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
