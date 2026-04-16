# Requirements: Media Control Commands

**Issues**: #193
**Date**: 2026-04-16
**Status**: Draft
**Author**: Claude (spec agent)

---

## User Story

**As a** browser automation engineer working with media-heavy web applications
**I want** built-in commands to list, play, pause, and seek audio/video elements
**So that** I can control media playback without writing repetitive `js exec` boilerplate every time

---

## Background

Users automating SCORM courses and LMS platforms encounter audio narration gates on every slide — the NEXT button stays disabled until audio finishes playing. The workaround is writing the same `js exec` audio fast-forward snippet repeatedly (reported as ~15 times in a single session), each time burning context window tokens on identical boilerplate. Built-in media commands would eliminate this entirely and provide structured output about playback state.

Currently, agentchrome has no media-related commands. No audio/video element discovery or control is available, no CDP Media domain methods are used, and users must rely on `js exec` with custom JavaScript for all media interactions.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: List media elements

**Given** a page with audio and/or video elements
**When** `media list` is run
**Then** JSON output lists all media elements with: tag (audio/video), src/currentSrc, duration, currentTime, playback state (playing/paused/ended), muted status, and volume
**And** each element includes a zero-based index for targeting

**Example**:
- Given: a page with one `<audio>` (paused at 0s, src="narration.mp3", duration=30s) and one `<video>` (playing at 10s, src="intro.mp4", duration=60s)
- When: `agentchrome media list`
- Then: JSON array with two objects, each containing `tag`, `src`, `currentSrc`, `duration`, `currentTime`, `state`, `muted`, `volume`, and `index`

### AC2: Play a media element

**Given** a paused media element identified by index
**When** `media play <index>` is run
**Then** the media element begins playing and the new state is returned as JSON with `state` equal to `"playing"`

**Example**:
- Given: a paused audio element at index 0
- When: `agentchrome media play 0`
- Then: `{"index":0,"tag":"audio","state":"playing","currentTime":0.0,...}`

### AC3: Pause a media element

**Given** a playing media element identified by index
**When** `media pause <index>` is run
**Then** the media element pauses and the new state is returned as JSON with `state` equal to `"paused"`

### AC4: Seek a media element to a specific time

**Given** a media element identified by index
**When** `media seek <index> <time>` is run
**Then** the media element's `currentTime` is set to `<time>` seconds and the new state is returned as JSON

**Example**:
- Given: an audio element at index 0 with duration 30s, currently at 0s
- When: `agentchrome media seek 0 15.5`
- Then: `{"index":0,"tag":"audio","currentTime":15.5,...}`

### AC5: Seek a media element to its end

**Given** a media element identified by index
**When** `media seek-end <index>` is run
**Then** the media element's `currentTime` is set to its `duration` and the `state` becomes `"ended"`

**Example**:
- Given: an audio element at index 0 with duration 30s
- When: `agentchrome media seek-end 0`
- Then: `{"index":0,"tag":"audio","currentTime":30.0,"state":"ended",...}`

### AC6: Bulk media control with --all flag

**Given** a page with multiple media elements and the `--all` flag
**When** `media seek-end --all` is run
**Then** all media elements on the page are seeked to their end and results are returned for each element as a JSON array

**Example**:
- Given: a page with 3 audio elements
- When: `agentchrome media seek-end --all`
- Then: JSON array of 3 result objects, each with `state` equal to `"ended"`

### AC7: Frame-scoped media control

**Given** a page with iframes containing media elements and `--frame <index>` argument
**When** any media command is run with `--frame <index>`
**Then** it targets media elements within the specified frame context only

**Example**:
- Given: a page with an iframe at index 0 containing an audio element
- When: `agentchrome media --frame 0 list`
- Then: JSON array listing only media elements within that iframe

### AC8: Cross-validation of state mutation via list

**Given** a paused media element at index 0
**When** `media play 0` is run followed by `media list`
**Then** the media list output shows the element at index 0 with `state` equal to `"playing"`

### AC9: No media elements on page

**Given** a page with no audio or video elements
**When** `media list` is run
**Then** an empty JSON array `[]` is returned with exit code 0

### AC10: Invalid media index

**Given** a page with 2 media elements (indices 0 and 1)
**When** `media play 5` is run (index out of bounds)
**Then** a JSON error is output on stderr with a descriptive message
**And** the exit code is non-zero

### AC11: Seek beyond duration clamps to duration

**Given** a media element at index 0 with duration 30s
**When** `media seek 0 999` is run
**Then** the media element's `currentTime` is clamped to its duration (30s)
**And** the returned state reflects the clamped value

### AC12: Documentation updated

**Given** the new media commands
**When** `examples media` is run
**Then** media command examples are included in the output with at least 3 example entries

### AC13: Media element with selector targeting

**Given** a media element on the page
**When** `media play css:audio.narration` is run (CSS selector instead of index)
**Then** the matching media element begins playing and the new state is returned as JSON

### Generated Gherkin Preview

```gherkin
Feature: Media Control Commands
  As a browser automation engineer working with media-heavy web applications
  I want built-in commands to list, play, pause, and seek audio/video elements
  So that I can control media playback without writing repetitive js exec boilerplate

  Scenario: List media elements
    Given a page with audio and video elements
    When I run "agentchrome media list"
    Then the output is a JSON array of media element objects
    And each object contains tag, src, currentSrc, duration, currentTime, state, muted, volume, and index

  Scenario: Play a paused media element
    Given a page with a paused audio element at index 0
    When I run "agentchrome media play 0"
    Then the output JSON has "state" equal to "playing"

  # ... all ACs become scenarios
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | New `media list` subcommand enumerating all audio/video elements | Must | Returns JSON array |
| FR2 | New `media play <target>` subcommand | Must | Target by index or CSS selector |
| FR3 | New `media pause <target>` subcommand | Must | |
| FR4 | New `media seek <target> <time>` subcommand | Must | Time in seconds (float) |
| FR5 | New `media seek-end <target>` subcommand (seek to duration) | Must | Primary use case: skip narration gates |
| FR6 | `--all` flag for bulk operations on play/pause/seek/seek-end | Should | Returns JSON array of results |
| FR7 | `--frame` support at the command group level for frame-scoped targeting | Should | Same pattern as `page`, `js`, `interact`, `form`, `dom` |
| FR8 | Target by CSS selector in addition to index | Should | Prefix `css:` for selectors, bare integer for index |
| FR9 | Help documentation and built-in examples updated | Must | Add media group to `examples.rs` |
| FR10 | BDD test scenarios covering media commands | Must | |
| FR11 | Mute/unmute and volume control (`media mute`, `media volume`) | Could | Deferred to future issue |
| FR12 | JSON output on stdout, JSON errors on stderr, standard exit codes | Must | Compliance with global output contract |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Media commands should complete within the global timeout default (30s). Individual media queries via JS evaluation should resolve in < 500ms. |
| **Security** | No new security surface — JS is evaluated in page context via existing CDP Runtime.evaluate path. |
| **Reliability** | Graceful handling of media elements with no source, zero duration, or error state. |
| **Platforms** | macOS, Linux, Windows — same as all agentchrome commands. |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| target | String | Non-negative integer (index) or `css:` prefixed CSS selector | Yes (except `list` and `--all`) |
| time | f64 | Non-negative float, seconds | Yes (for `seek` only) |
| --all | bool | Flag, mutually exclusive with target | No |
| --frame | String | Frame index, path, or 'auto' | No |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| index | u32 | Zero-based index of the media element on the page |
| tag | String | `"audio"` or `"video"` |
| src | String | The `src` attribute value (may be empty if using `<source>` children) |
| currentSrc | String | The resolved media source URL currently in use |
| duration | f64 | Total duration in seconds (`NaN` serialized as `null` for unknown duration) |
| currentTime | f64 | Current playback position in seconds |
| state | String | One of `"playing"`, `"paused"`, `"ended"` |
| muted | bool | Whether the element is muted |
| volume | f64 | Volume level from 0.0 to 1.0 |
| loop | bool | Whether the element loops |
| readyState | u32 | HTMLMediaElement readyState (0-4) |

---

## Dependencies

### Internal Dependencies
- [x] CDP client (`cdp/client.rs`) — WebSocket JSON-RPC for Runtime.evaluate
- [x] Connection management (`connection.rs`) — session setup
- [x] Frame resolution (`frame.rs`) — `--frame` support
- [x] Output helpers (`output.rs`) — `print_output`, `setup_session`
- [x] CLI framework (`cli/mod.rs`) — Command enum, clap derive types
- [x] Examples subsystem (`examples.rs`) — built-in examples

### External Dependencies
- None (uses HTMLMediaElement API via Runtime.evaluate)

### Blocked By
- None

---

## Out of Scope

- Media recording or streaming
- WebRTC media control
- Canvas-based media (e.g., custom video players rendered on canvas)
- Media download or extraction
- CDP Media domain integration (JS approach is simpler and sufficient)
- Mute/unmute and volume control (FR11 — deferred to future issue)
- Media event monitoring/following (e.g., `media follow` to stream playback events)

---

## Open Questions

- None — all requirements are clear from the issue and existing codebase patterns.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #193 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Edge cases and error states specified (AC9, AC10, AC11)
- [x] Cross-validation AC included (AC8, per retrospective learning)
- [x] Dependencies identified
- [x] Out of scope defined
- [x] Open questions documented
