# Tasks: Coordinate Space Helpers for Frame-Aware Coordinate Resolution

**Issues**: #198
**Date**: 2026-04-16
**Status**: Planning
**Author**: Rich Nunley

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 3 | [ ] |
| Backend | 4 | [ ] |
| Frontend | 0 | [ ] |
| Integration | 3 | [ ] |
| Testing | 6 | [ ] |
| **Total** | **16** | |

Note: "Backend" maps to command-module logic (Rust side); "Frontend" is N/A for this CLI-only feature. Integration covers CLI dispatcher wiring and docs; Testing covers unit + BDD + smoke gates.

---

## Phase 1: Setup

### T001: Create `src/coords.rs` module skeleton with `CoordValue` enum

**File(s)**: `src/coords.rs` (new), `src/lib.rs` (modify)
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `src/coords.rs` created with `pub enum CoordValue { Pixels(f64), Percent(f64) }`
- [ ] `BoundingBox { x, y, width, height: f64 }` struct defined
- [ ] Module declared in `src/lib.rs` as `pub mod coords;`
- [ ] `cargo build` succeeds
- [ ] `cargo clippy --all-targets` passes with no new warnings

**Notes**: Module exists as a scaffold before any logic. Later tasks add functions. Keep `CoordValue` derives minimal: `Debug, Clone, Copy`.

### T002: Implement `CoordValue::parse` with percentage range validation

**File(s)**: `src/coords.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `CoordValue::parse(&str) -> Result<CoordValue, CoordValueParseError>` implemented
- [ ] Accepts integer and decimal pixel inputs: `"10"`, `"-10"`, `"10.5"`, `"-10.5"`, `"0"`
- [ ] Accepts percentages in range `0.0`–`100.0`: `"0%"`, `"50%"`, `"100%"`, `"33.33%"`
- [ ] Rejects percentages outside range: `"-5%"`, `"150%"`, `"100.01%"`
- [ ] Rejects malformed: `""`, `"abc"`, `"5%%"`, `"50"`+trailing, `"%"`
- [ ] `CoordValueParseError` implements `std::error::Error + Display` with actionable messages
- [ ] Implement `clap::builder::ValueParserFactory` (or equivalent) so clap routes `"50%"` to this parser without treating it as `f64` first
- [ ] Unit tests cover every valid/invalid case listed above

**Notes**: The clap integration is the trickiest part — test that `#[arg(value_parser = CoordValue::parse)]` (or equivalent via `ValueParserFactory`) actually receives the raw string with the `%` intact. A standalone unit test that parses a minimal `clap::Command` and asserts `"50%"` produces `CoordValue::Percent(50.0)` is required.

### T003: Move `get_frame_viewport_offset` from `src/interact.rs` to `src/coords.rs`

**File(s)**: `src/coords.rs`, `src/interact.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] Function moved verbatim to `src/coords.rs` as `pub(crate) async fn frame_viewport_offset(...)`
- [ ] All four call sites in `src/interact.rs` (`execute_click_at`, `execute_drag_at`, `execute_mousedown_at`, `execute_mouseup_at`) updated to `crate::coords::frame_viewport_offset`
- [ ] `cargo build`, `cargo clippy --all-targets`, and `cargo fmt --check` all pass
- [ ] Existing `cargo test --test bdd` frame-offset scenarios continue to pass (regression guard)

**Notes**: Surgical move — do not change the function body. If the body needs editing, do it in a separate task. Purpose is single ownership of frame-coordinate logic in `coords.rs`.

---

## Phase 2: Backend (Command Logic)

### T004: Implement `resolve_element_box` in `src/coords.rs`

**File(s)**: `src/coords.rs`
**Type**: Modify
**Depends**: T001, T003
**Acceptance**:
- [ ] `pub(crate) async fn resolve_element_box(managed: &mut ManagedSession, frame_ctx: Option<&FrameContext>, selector: &str) -> Result<BoundingBox, AppError>` implemented
- [ ] Accepts UID (`"s7"`) and CSS selector (`"css:#id"`) forms — mirror `src/page/element.rs::resolve_element_target` logic, but frame-aware
- [ ] For CSS selectors, calls `DOM.querySelector` scoped to the frame's document; for UIDs, reads `snapshot_state` (same as `page element`)
- [ ] Calls `DOM.getBoxModel { backendNodeId }` and extracts `model.content[0..2]` as (x, y) and `model.width`/`model.height`
- [ ] Returns a `BoundingBox` in **frame-local** coordinates (no frame offset applied)
- [ ] Returns `AppError::element_target_not_found` / `css_selector_not_found` with exit code 3 when the selector matches nothing
- [ ] Unit-testable: isolate CDP calls via an `async` trait-object or by keeping the function thin and testing the arithmetic separately

**Notes**: Do not consolidate with `page/element.rs::resolve_element_target` in this task — the scope is additive only. A follow-up issue can consolidate.

### T005: Implement `resolve_relative_coords` in `src/coords.rs`

**File(s)**: `src/coords.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `pub(crate) fn resolve_relative_coords(x: CoordValue, y: CoordValue, element_box: BoundingBox, frame_offset: (f64, f64)) -> (f64, f64)` implemented
- [ ] Pixel values add to `element_box.x` / `element_box.y` then add frame offset
- [ ] Percent values compute `element_box.{x,y} + (pct/100.0) * (element_box.{width,height} - 1.0).max(0.0)` then add frame offset — the `- 1.0` ensures `100%` lands on the last pixel inside
- [ ] Mixed axes (Pixels + Percent) computed independently per axis
- [ ] Unit tests for: (a) both pixels, main frame (offset 0,0); (b) both percent 50%, main frame; (c) mixed percent+pixels, main frame; (d) both percent 0% and 100%, iframe with non-zero offset; (e) zero-width element (`width - 1.0` clamps to 0)

**Notes**: Pure function — no I/O, no async. Makes unit testing trivial.

### T006: Implement `page coords` command in `src/page/coords.rs`

**File(s)**: `src/page/coords.rs` (new), `src/page/mod.rs` (modify)
**Type**: Create
**Depends**: T004
**Acceptance**:
- [ ] `src/page/coords.rs` created with `pub(crate) async fn execute_coords(global: &GlobalOpts, args: &PageCoordsArgs, frame: Option<&str>) -> Result<(), AppError>`
- [ ] Output struct matches the schema in `design.md` exactly: `{ frame: {index, id}, frameLocal: {boundingBox, center}, page: {boundingBox, center}, frameOffset }`
- [ ] Uses `setup_session`, `resolve_optional_frame`, `resolve_element_box`, `frame_viewport_offset`
- [ ] For main frame, `frame.index = 0`, `frame.id` = main frame CDP id, `frameOffset = {0, 0}`, `page.*` equals `frameLocal.*`
- [ ] Module declared in `src/page/mod.rs` (`mod coords;`) and invoked from the dispatcher
- [ ] `cargo build` succeeds

### T007: Wire `--relative-to` and `CoordValue` into the four interact executors

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: T003, T004, T005
**Acceptance**:
- [ ] `execute_click_at`, `execute_drag_at`, `execute_mousedown_at`, `execute_mouseup_at` each read `args.relative_to: Option<String>`
- [ ] When `relative_to` is `None`, existing absolute-coordinate path is unchanged (regression: existing BDD scenarios still pass without modification). Each executor extracts `f64` from `CoordValue::Pixels` variant; `CoordValue::Percent` without `--relative-to` returns `AppError` with exit code 1 and message "percentage coordinates require --relative-to"
- [ ] When `relative_to` is `Some(sel)`: call `resolve_element_box(frame_ctx, sel)` → box; call `frame_viewport_offset(frame_ctx)` → offset; call `resolve_relative_coords(x, y, box, offset)` → `(dispatch_x, dispatch_y)`; dispatch at those coords
- [ ] For `drag-at`, `resolve_relative_coords` is called twice (once for `from`, once for `to`) against the same `--relative-to` element
- [ ] Output fields update:
  - When `relative_to` is `Some`: `clicked_at` / `dragged_at.{from,to}` / `mousedown_at` / `mouseup_at` contain the **resolved page-global coords** (not the input)
  - When `relative_to` is `None`: output fields echo the input (existing behavior — unchanged)
- [ ] `cargo build`, `cargo clippy --all-targets`, `cargo fmt --check` all pass

**Notes**: The trickiest regression risk — verify by running the full BDD regression suite, specifically scenarios from `feature-add-coordinate-drag-and-decomposed-mouse-actions` and `feature-add-iframe-frame-targeting-support`.

---

## Phase 3: Frontend

N/A — this is a CLI-only feature. No UI components.

---

## Phase 4: Integration

### T008: Add `PageCommand::Coords` and `PageCoordsArgs` to CLI

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: T006
**Acceptance**:
- [ ] `PageCommand::Coords(PageCoordsArgs)` variant added with `#[command(long_about = ..., after_long_help = ...)]` attributes matching sibling subcommands (e.g., `HitTest`)
- [ ] `pub struct PageCoordsArgs { #[arg(long)] pub selector: String }` defined in the same file near other page arg structs
- [ ] `--frame` inherited from `PageArgs` (no new frame flag on `PageCoordsArgs`)
- [ ] `agentchrome page coords --help` output includes at least two examples in `after_long_help`: one main frame, one with `--frame`
- [ ] `cargo build` succeeds

### T009: Add `CoordValue` type and `--relative-to` flag to interact arg structs

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `ClickAtArgs.x` and `ClickAtArgs.y` changed from `f64` to `CoordValue` with `#[arg(value_parser = ...)]` routing to `CoordValue::parse`
- [ ] `DragAtArgs.{from_x, from_y, to_x, to_y}` similarly changed
- [ ] `MouseDownAtArgs.{x, y}` and `MouseUpAtArgs.{x, y}` similarly changed
- [ ] Each of the four arg structs gains `#[arg(long = "relative-to")] pub relative_to: Option<String>`
- [ ] Existing clap `conflicts_with`/group attributes remain correct
- [ ] `agentchrome interact click-at --help` / `drag-at --help` / `mousedown-at --help` / `mouseup-at --help` show the new `--relative-to` flag and reference percentage syntax in help text
- [ ] `cargo build` succeeds

### T010: Register `PageCommand::Coords` in the page dispatcher

**File(s)**: `src/page/mod.rs`
**Type**: Modify
**Depends**: T006, T008
**Acceptance**:
- [ ] `execute_page` dispatcher adds a `PageCommand::Coords(coords_args) => coords::execute_coords(global, coords_args, frame).await` arm
- [ ] `agentchrome page coords --selector "css:body"` runs end-to-end and returns JSON output against a live headless Chrome (smoke verification deferred to T016)
- [ ] `cargo build` succeeds

### T011: Update `examples interact` and `examples page` with coordinate helper examples

**File(s)**: `src/examples.rs`
**Type**: Modify
**Depends**: T008, T009
**Acceptance**:
- [ ] `examples interact` output includes at least three example invocations: (a) `click-at 10 5 --relative-to "css:button"`, (b) `click-at 50% 50% --relative-to "css:#target"`, (c) `drag-at 0% 50% 100% 50% --relative-to "css:.slider"`
- [ ] `examples page` output includes at least two examples: (a) `page coords --selector "css:#submit"`, (b) `page coords --frame 1 --selector "s7"`
- [ ] Each example includes a brief one-line description above the command (consistent with surrounding examples in the file)
- [ ] `agentchrome examples interact | grep -q -- '--relative-to'` succeeds; `agentchrome examples page | grep -q 'page coords'` succeeds

---

## Phase 5: Testing

### T012: Unit tests for `CoordValue::parse` and `resolve_relative_coords`

**File(s)**: `src/coords.rs` (inline `#[cfg(test)] mod tests`)
**Type**: Modify
**Depends**: T002, T005
**Acceptance**:
- [ ] Every valid/invalid `CoordValue::parse` case from T002's acceptance list has a unit test
- [ ] Every arithmetic case from T005's acceptance list has a unit test
- [ ] `cargo test --lib coords::` passes
- [ ] A clap integration test confirms `#[arg(value_parser = ...)]` routes `"50%"` correctly (small standalone `clap::Command` with `ClickAtArgs` or a minimal reproduction)

### T013: BDD feature file `tests/features/coordinate-space-helpers.feature`

**File(s)**: `tests/features/coordinate-space-helpers.feature` (new)
**Type**: Create
**Depends**: T010, T011
**Acceptance**:
- [ ] One scenario per acceptance criterion (AC1–AC12), 12 scenarios total
- [ ] Uses Given/When/Then format with concrete coordinates
- [ ] Feature file is valid Gherkin syntax (no parse errors under cucumber-rs 0.21)
- [ ] Covers: main-frame and iframe `page coords`, UID and CSS selector targets, absolute offset, percentage, mixed, drag-at/mousedown-at/mouseup-at, `--relative-to` + `--frame`, missing selector error, invalid percentage error, examples output coverage

### T014: Implement BDD step definitions

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T013
**Acceptance**:
- [ ] Every scenario step in `coordinate-space-helpers.feature` resolves to a step definition (no `Skipped` or `Unmatched` steps when `cargo test --test bdd -- --nocapture` runs)
- [ ] A test-fixture HTML file is launched via `navigate file://...` at the start of relevant scenarios
- [ ] Steps that launch headless Chrome follow the cleanup guidance in `tech.md` (kill orphaned processes on completion)
- [ ] `cargo test --test bdd` passes locally against headless Chrome

### T015: Create smoke-test fixture `tests/fixtures/coordinate-space-helpers.html`

**File(s)**: `tests/fixtures/coordinate-space-helpers.html` (new)
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] Self-contained HTML with no external dependencies
- [ ] HTML comment at top lists which ACs it covers (AC1–AC8)
- [ ] Includes a main-frame element with a deterministic bounding box (absolute CSS positioning)
- [ ] Includes a nested iframe at a known page offset (absolute-positioned `<iframe>`), whose `srcdoc` contains an element with a deterministic bounding box
- [ ] Opened in headless Chrome, `page coords` produces the expected coordinates from the ACs

**Notes**: Use `srcdoc` for the iframe to avoid separate-file hassle. Position both iframe and inner element with pixel-exact CSS so tests are deterministic.

### T016: Verification gate — build, clippy, fmt, unit + BDD, feature exercise

**File(s)**: (no file changes — verification-only task)
**Type**: Verify
**Depends**: T007, T011, T012, T014, T015
**Acceptance**:
- [ ] `cargo build` exits 0
- [ ] `cargo clippy --all-targets` exits 0 (no new warnings)
- [ ] `cargo fmt --check` exits 0
- [ ] `cargo test --lib` exits 0
- [ ] `cargo test --test bdd` exits 0
- [ ] Feature exercise against `tests/fixtures/coordinate-space-helpers.html` — manually exercise each AC against headless Chrome per `tech.md` Feature Exercise Gate and record pass/fail
- [ ] Chrome instances killed after verification (`pkill -f 'chrome.*--remote-debugging' || true`)

**Notes**: This task is intentionally the final gate — treat any failure as Critical and fix before declaring the feature complete.

---

## Dependency Graph

```
T001 ── T002 ── T009 ── T011 ──┐                         ┌── T013 ── T014 ──┐
  │       │                     │                         │                  │
  │       └────── T005 ─────────┼──┐                      │                  │
  │                             │  │                      │                  │
  └── T003 ──┬── T004 ── T006 ──┼──┼── T010 ──────────────┘                  ├── T016
             │                  │  │                                         │
             └───────── T007 ───┘  │                                         │
                              (T007 uses T003 + T004 + T005)                 │
                                                                             │
T012 (T002 + T005) ──────────────────────────────────────────────────────────┤
                                                                             │
T015 (independent) ──────────────────────────────────────────────────────────┘

T008 (T006) ─────── T010
```

Linearized critical path: **T001 → T002 → T005 → T007 → T013 → T014 → T016**.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #198 | 2026-04-16 | Initial task breakdown — 16 tasks across Setup / Backend / Integration / Testing phases |

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently given their dependencies
- [x] Acceptance criteria are verifiable (each task's checklist is testable)
- [x] File paths reference actual project structure (verified against `structure.md` and `ls src/`)
- [x] Test tasks are included for each layer (unit T012, BDD T013–T014, smoke T015, final gate T016)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
