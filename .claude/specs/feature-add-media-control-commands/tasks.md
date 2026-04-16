# Tasks: Media Control Commands

**Issues**: #193
**Date**: 2026-04-16
**Status**: Planning
**Author**: Claude (spec agent)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 3 | [ ] |
| Integration | 2 | [ ] |
| Testing | 3 | [ ] |
| **Total** | **10** | |

---

## Phase 1: Setup

### T001: Define CLI types for media command group

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `MediaArgs` struct with `--frame` option and `command: MediaCommand` subcommand
- [ ] `MediaCommand` enum with variants: `List`, `Play(MediaTargetArgs)`, `Pause(MediaTargetArgs)`, `Seek(MediaSeekArgs)`, `SeekEnd(MediaTargetArgs)`
- [ ] `MediaTargetArgs` struct with positional `target: Option<String>` and `--all` flag
- [ ] `MediaSeekArgs` struct with positional `target: Option<String>`, positional `time: f64`, and `--all` flag (with separate `--time` for --all mode)
- [ ] `Command::Media(MediaArgs)` variant added to Command enum
- [ ] Help text and examples for each subcommand
- [ ] `cargo build` succeeds with new types

**Notes**: Follow the pattern used by `DialogArgs`/`DialogCommand` and `CookieArgs`/`CookieCommand`. The `--frame` flag goes on `MediaArgs` (group level), matching `PageArgs`, `JsArgs`, `InteractArgs`, `FormArgs`, `DomArgs`.

### T002: Define output types for media commands

**File(s)**: `src/media.rs` (new file, output types section)
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `MediaInfo` struct with fields: `index` (u32), `tag` (String), `src` (String), `current_src` (String, serde rename to `currentSrc`), `duration` (Option<f64>), `current_time` (f64, serde rename to `currentTime`), `state` (String), `muted` (bool), `volume` (f64), `loop_` (bool, serde rename to `loop`), `ready_state` (u32, serde rename to `readyState`)
- [ ] All structs derive `Serialize`
- [ ] `duration` uses `Option<f64>` to handle NaN to null serialization
- [ ] Unit tests verify JSON serialization field names and null handling

---

## Phase 2: Backend Implementation

### T003: Implement media list command

**File(s)**: `src/media.rs`
**Type**: Modify (append to file created in T002)
**Depends**: T001, T002
**Acceptance**:
- [ ] `execute_media()` dispatcher function matching the pattern in `dialog.rs` and `cookie.rs`
- [ ] `execute_list()` evaluates JS that queries `document.querySelectorAll('audio, video')` and maps each element to a `MediaInfo` JSON object
- [ ] JS evaluation uses `returnByValue: true` for inline results
- [ ] Frame support: calls `resolve_optional_frame()` when `--frame` is provided, using the frame's execution context for the JS evaluation
- [ ] Plain text output for `--plain` flag
- [ ] Returns empty array `[]` when no media elements exist (AC9)
- [ ] Unit tests for JS result parsing and plain text formatting

### T004: Implement media play, pause, seek, seek-end commands

**File(s)**: `src/media.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_play()` evaluates JS: selects target element, calls `el.play()`, returns updated state
- [ ] `execute_pause()` evaluates JS: selects target element, calls `el.pause()`, returns updated state
- [ ] `execute_seek()` evaluates JS: selects target element, sets `el.currentTime = time`, returns updated state
- [ ] `execute_seek_end()` evaluates JS: selects target element, sets `el.currentTime = el.duration`, returns updated state
- [ ] Target resolution: bare integer maps to index-based selection; `css:` prefix maps to CSS selector-based selection (AC13)
- [ ] Invalid index returns descriptive error (AC10)
- [ ] Seek beyond duration clamps to duration (AC11, browser native behavior)
- [ ] seek-end with NaN duration returns error
- [ ] `play()` uses `awaitPromise: true` to handle autoplay restrictions
- [ ] CSS selectors are escaped to prevent JS injection
- [ ] Plain text output for `--plain` flag
- [ ] Unit tests for target parsing, error messages, and serialization

### T005: Implement --all bulk operations

**File(s)**: `src/media.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] When `--all` flag is set, JS operates on all media elements via `querySelectorAll('audio, video')`
- [ ] Returns JSON array of `MediaInfo` results, one per element (AC6)
- [ ] `--all` and `target` are mutually exclusive (clap validation or runtime check)
- [ ] Empty array returned if no media elements on page
- [ ] For `seek --all`, time applies to all elements
- [ ] Unit test for bulk result serialization

---

## Phase 3: Integration

### T006: Wire media command into main.rs dispatch

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `mod media;` added to module declarations
- [ ] `Command::Media(args) => media::execute_media(&global, args).await` added to match arm
- [ ] Alphabetical ordering maintained in module list and match arms
- [ ] `cargo build` succeeds

### T007: Add media examples to examples.rs

**File(s)**: `src/examples.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] New `CommandGroupSummary` for `"media"` with description and at least 5 examples
- [ ] Examples include: `media list`, `media play 0`, `media pause 0`, `media seek-end --all`, `media --frame 0 list`
- [ ] Existing unit test `all_examples_returns_expected_groups` updated to assert `"media"` is present
- [ ] `each_group_has_at_least_3_examples` test passes
- [ ] `error_message_lists_all_available_groups` test passes (AC12)

---

## Phase 4: Testing

### T008: Create BDD feature file

**File(s)**: `tests/features/media-control.feature`
**Type**: Create
**Depends**: T005, T006
**Acceptance**:
- [ ] All 13 acceptance criteria from requirements.md are scenarios
- [ ] Uses Given/When/Then format matching existing feature file conventions
- [ ] Includes Background with `Given agentchrome is built`
- [ ] Feature file is valid Gherkin syntax
- [ ] Scenarios for: list, play, pause, seek, seek-end, bulk --all, frame-scoped, cross-validation, empty page, invalid index, seek clamp, examples, CSS selector

### T009: Implement BDD step definitions

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T008
**Acceptance**:
- [ ] Step definitions for media-specific Given/When/Then steps
- [ ] Steps follow existing patterns in bdd.rs (World struct, process spawning)
- [ ] `cargo test --test bdd` passes for media scenarios

### T010: Manual smoke test against headless Chrome

**File(s)**: `tests/fixtures/media-control.html`
**Type**: Create
**Depends**: T005, T006
**Acceptance**:
- [ ] HTML fixture includes: 2 audio elements (one with src, one with source child), 1 video element, and an iframe with 1 audio element inside
- [ ] Fixture is self-contained with no external dependencies (uses data URIs or silent audio generated inline)
- [ ] HTML comment at top documents which ACs it covers
- [ ] Smoke test procedure:
  1. `cargo build`
  2. `./target/debug/agentchrome connect --launch --headless`
  3. `./target/debug/agentchrome navigate file://path/tests/fixtures/media-control.html`
  4. `./target/debug/agentchrome media list` verifies 3 elements with correct metadata (AC1)
  5. `./target/debug/agentchrome media play 0` verifies state playing (AC2)
  6. `./target/debug/agentchrome media pause 0` verifies state paused (AC3)
  7. `./target/debug/agentchrome media seek 0 5.0` verifies currentTime near 5.0 (AC4)
  8. `./target/debug/agentchrome media seek-end 0` verifies state ended (AC5)
  9. `./target/debug/agentchrome media seek-end --all` verifies all elements ended (AC6)
  10. `./target/debug/agentchrome media --frame 0 list` verifies 1 element from iframe (AC7)
  11. `./target/debug/agentchrome media play 99` verifies error on stderr (AC10)
  12. `./target/debug/agentchrome examples media` verifies examples output (AC12)
  13. `./target/debug/agentchrome connect --disconnect`
- [ ] All ACs verified against real browser

---

## Dependency Graph

```
T001 ---+---> T003 ---> T004 ---> T005 ---+---> T008 ---> T009
        |          |                       |
T002 ---+          +---> T006              +---> T010
        |
        +---> T007
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #193 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included (T008, T009, T010)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
