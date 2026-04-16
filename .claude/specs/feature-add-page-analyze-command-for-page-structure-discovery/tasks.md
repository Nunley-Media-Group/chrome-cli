# Tasks: Page Analyze Command for Page Structure Discovery

**Issues**: #190
**Date**: 2026-04-16
**Status**: Planning
**Author**: Claude (spec-writer)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 5 | [ ] |
| Frontend | 0 | [ ] |
| Integration | 2 | [ ] |
| Testing | 3 | [ ] |
| **Total** | **12** | |

---

## Phase 1: Setup

### T001: Add Analyze variant to PageCommand enum and CLI args

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `PageCommand::Analyze` variant added to the enum with clap `#[command(...)]` attributes
- [ ] Long help text describes the command purpose and includes usage examples
- [ ] `--frame` argument inherited from `PageArgs` (no additional args needed)
- [ ] `cargo build` compiles without errors
- [ ] `agentchrome page analyze --help` displays the new command

**Notes**: Follow the pattern of `PageCommand::HitTest` — no extra args struct needed since the command has no unique arguments beyond the shared `--frame`.

### T002: Define output types for AnalyzeResult

**File(s)**: `src/page/analyze.rs`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] `AnalyzeResult` struct with all output fields: `scope`, `url`, `title`, `iframes`, `frameworks`, `interactive_elements`, `media`, `overlays`, `shadow_dom`, `summary`
- [ ] Supporting structs: `IframeInfo`, `MediaInfo`, `OverlayInfo`, `ShadowDomInfo`, `InteractiveElements`, `AnalyzeSummary`
- [ ] All structs derive `Debug`, `Serialize` with `#[serde(rename_all = "camelCase")]`
- [ ] Optional/nullable fields use `Option<T>` (e.g., `MediaInfo.state`)
- [ ] Types compile without errors

---

## Phase 2: Backend Implementation

### T003: Implement iframe enumeration

**File(s)**: `src/page/analyze.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Calls `agentchrome::frame::list_frames()` to get frame tree
- [ ] For each child frame, produces `IframeInfo` with: `index`, `url`, `name`, `visible`, `width`, `height`, `cross_origin`
- [ ] Cross-origin detection uses `security_origin` comparison against main frame
- [ ] Visibility check via `Runtime.evaluate` for each iframe element
- [ ] Returns empty vec when no iframes exist (AC3)

### T004: Implement framework detection

**File(s)**: `src/page/analyze.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Single `Runtime.evaluate` call checks all framework signatures
- [ ] Detects: React, Angular, Vue, Svelte, Storyline, SCORM
- [ ] Returns `Vec<String>` of detected framework names
- [ ] Returns empty vec when no frameworks detected (AC3)
- [ ] Graceful degradation: returns empty vec on JS evaluation failure (AC7)

### T005: Implement interactive element counting and media cataloging

**File(s)**: `src/page/analyze.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] Counts interactive elements in main frame using `querySelectorAll` with standard interactive selectors
- [ ] For same-origin iframes, counts interactive elements in each frame's execution context
- [ ] For cross-origin iframes, reports `null` for inaccessible counts (AC5, AC7)
- [ ] Media cataloging queries video, audio, embed elements
- [ ] Each media entry includes: `tag`, `src`, `state` (playing/paused/ended/null), `width`, `height`
- [ ] Returns empty vec when no media elements exist (AC3)

### T006: Implement overlay detection and shadow DOM scanning

**File(s)**: `src/page/analyze.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Queries elements with `position: fixed/absolute` and high z-index
- [ ] Checks if candidate overlays cover >50% of viewport area
- [ ] Checks if interactive elements exist beneath overlay candidates
- [ ] Each overlay entry includes: `selector`, `z_index`, `width`, `height`, `covers_interactive`
- [ ] Shadow DOM detection: counts elements with `.shadowRoot` in document
- [ ] Returns `ShadowDomInfo { present: false, host_count: 0 }` when none found (AC3)
- [ ] Returns empty overlays vec when none detected (AC3)

### T007: Implement execute_analyze command executor

**File(s)**: `src/page/analyze.rs`
**Type**: Modify
**Depends**: T003, T004, T005, T006
**Acceptance**:
- [ ] Function signature: `pub async fn execute_analyze(global: &GlobalOpts, frame: Option<&str>) -> Result<(), AppError>`
- [ ] Sets up session via `setup_session(global)`
- [ ] Resolves optional frame context via `frame::resolve_frame()`
- [ ] Enables CDP domains: DOM, Runtime, Page
- [ ] Calls all 6 analysis dimension functions sequentially
- [ ] Assembles `AnalyzeResult` with summary aggregations
- [ ] Outputs via `print_output(&result, &global.output)`
- [ ] Returns `ExitCode::TargetError` for invalid frame index (AC6)
- [ ] Graceful degradation: individual dimension failures produce `null` fields, not command failure (AC7)

---

## Phase 3: Frontend Implementation

*N/A — this is a CLI-only feature with no frontend components.*

---

## Phase 4: Integration

### T008: Wire Analyze into page command dispatcher

**File(s)**: `src/page/mod.rs`
**Type**: Modify
**Depends**: T007
**Acceptance**:
- [ ] `mod analyze;` added to module declarations
- [ ] `PageCommand::Analyze` arm added to `execute_page()` match with `analyze::execute_analyze(global, frame).await`
- [ ] Command dispatches correctly: `agentchrome page analyze` routes to `execute_analyze`

### T009: Add page analyze examples to built-in examples

**File(s)**: `src/examples.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] Two new `ExampleEntry` items added to the `page` command group's examples vec:
  1. Basic: `agentchrome page analyze` — "Analyze page structure: iframes, frameworks, overlays, media"
  2. Frame-scoped: `agentchrome page analyze --frame 1` — "Analyze structure within a specific iframe"
- [ ] `agentchrome examples page` output includes the new entries (AC4)

---

## Phase 5: BDD Testing (Required)

### T010: Create BDD feature file

**File(s)**: `tests/features/page-analyze.feature`
**Type**: Create
**Depends**: T007, T008
**Acceptance**:
- [ ] All 7 acceptance criteria (AC1-AC7) are Gherkin scenarios
- [ ] Uses Given/When/Then format
- [ ] Feature description includes user story
- [ ] Valid Gherkin syntax

### T011: Implement step definitions for page analyze

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T010
**Acceptance**:
- [ ] AnalyzeWorld struct added (or existing World extended) with step definitions for all scenarios
- [ ] Steps use CLI invocation pattern: run agentchrome binary and parse JSON stdout
- [ ] All BDD scenarios pass with `cargo test --test bdd`

### T012: Smoke test against real Chrome

**File(s)**: `tests/fixtures/page-analyze.html`
**Type**: Create
**Depends**: T007, T008
**Acceptance**:
- [ ] Test fixture HTML includes:
  - One same-origin iframe with interactive elements
  - A video element (paused)
  - An overlay div with position fixed and z-index 9999 covering the viewport
  - A custom element with shadow DOM
  - A simulated React root (data-reactroot attribute)
- [ ] Build: `cargo build` succeeds
- [ ] Launch: `./target/debug/agentchrome connect --launch --headless` succeeds
- [ ] Navigate to fixture file succeeds
- [ ] AC1 verified: `page analyze` returns JSON with all 6 dimensions populated
- [ ] AC2 verified: `page analyze --frame 1` scopes to iframe
- [ ] AC3 verified: Navigate to a blank page, `page analyze` returns empty arrays
- [ ] AC4 verified: `examples page` includes analyze entries
- [ ] Cleanup: disconnect and kill Chrome processes

---

## Dependency Graph

```
T001 ──▶ T002 ──┬──▶ T003 ──┐
                │           │
                ├──▶ T004   │
                │           │
                ├──▶ T005 ──┤
                │           │
                └──▶ T006 ──┤
                            │
                            ▼
                          T007 ──▶ T008 ──▶ T010 ──▶ T011
                            │
                            └──▶ T012
T001 ──▶ T009
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #190 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included (T010, T011, T012)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
