# Tasks: Screenshot Capture

**Issue**: #12
**Date**: 2026-02-12
**Status**: Approved
**Author**: Claude

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 3 | [ ] |
| Backend | 3 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **9** | |

---

## Phase 1: Setup

### T001: Add `base64` dependency to Cargo.toml

**File(s)**: `Cargo.toml`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `base64` crate added to `[dependencies]` section
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy` passes

**Notes**: Only the standard `base64` crate with default features is needed. Used for decoding CDP's base64 screenshot data when writing to `--file`.

### T002: Add error helper constructors for screenshot

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AppError::screenshot_failed(description)` returns message `"Screenshot capture failed: {description}"` with `ExitCode::GeneralError`
- [ ] `AppError::uid_not_found(uid)` returns message `"UID '{uid}' not found. Run 'chrome-cli page snapshot' first."` with `ExitCode::GeneralError`
- [ ] `AppError::invalid_clip(input)` returns message `"Invalid clip format: expected X,Y,WIDTH,HEIGHT (e.g. 10,20,200,100): {input}"` with `ExitCode::GeneralError`
- [ ] Unit tests for all three constructors verify message content and exit code

**Notes**: Follow the existing pattern of `navigation_failed()`, `element_not_found()`, etc.

### T003: Add CLI argument types for `page screenshot`

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `ScreenshotFormat` enum with `Png`, `Jpeg`, `Webp` variants, deriving `ValueEnum`, `Clone`, `Copy`, `Debug`
- [ ] `PageScreenshotArgs` struct with:
  - `--full-page` (`bool`)
  - `--selector` (`Option<String>`)
  - `--uid` (`Option<String>`)
  - `--format` (`ScreenshotFormat`, default: `Png`)
  - `--quality` (`Option<u8>`)
  - `--file` (`Option<PathBuf>`)
  - `--clip` (`Option<String>`)
- [ ] `PageCommand::Screenshot(PageScreenshotArgs)` variant added to `PageCommand` enum
- [ ] `chrome-cli page screenshot --help` displays all options with descriptions
- [ ] Compiles without errors or clippy warnings

**Notes**: Follow the pattern of `PageFindArgs`. Mutual exclusion of `--full-page` with `--selector`/`--uid` is validated at runtime in the executor (not clap), consistent with how `page find` validates query vs selector. The `--clip` is parsed as a string and validated at runtime.

---

## Phase 2: Backend Implementation

### T004: Implement clip region parsing and element resolution helpers

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T002, T003
**Acceptance**:
- [ ] `parse_clip(input: &str) -> Result<ClipRegion, AppError>` that:
  - Parses "X,Y,WIDTH,HEIGHT" string (e.g., "10,20,200,100")
  - Returns `ClipRegion { x: f64, y: f64, width: f64, height: f64 }`
  - Returns `AppError::invalid_clip` for malformed input
  - Accepts integer or decimal values
- [ ] `resolve_element_clip(managed, selector: Option<&str>, uid: Option<&str>) -> Result<ClipRegion, AppError>` that:
  - For `--selector`: enables DOM domain, calls `DOM.getDocument` → `DOM.querySelector` → `DOM.getBoxModel` → extracts clip from content quad
  - For `--uid`: reads snapshot state via `read_snapshot_state()`, looks up `backendDOMNodeId`, calls `DOM.describeNode` → `DOM.getBoxModel` → extracts clip
  - Returns `AppError::element_not_found` if selector matches nothing
  - Returns `AppError::uid_not_found` if UID not in snapshot state
- [ ] `ClipRegion` struct with `x`, `y`, `width`, `height` fields (all `f64`)
- [ ] Unit tests for `parse_clip`: valid "10,20,200,100", decimal "10.5,20.5,200.5,100.5", invalid "abc", too few values, negative values (should parse successfully — CDP will handle)

**Notes**: The `ClipRegion` is an internal type used to build the CDP `clip` parameter. It matches the `BoundingBox` shape but is semantically different (viewport region vs element box). Reuse `resolve_bounding_box` pattern from element finding code.

### T005: Implement full-page screenshot viewport manipulation

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `get_page_dimensions(managed) -> Result<(f64, f64), AppError>` that:
  - Evaluates JS to get `scrollWidth` and `scrollHeight` via `Runtime.evaluate`
  - Returns `(width, height)` as floats
- [ ] `get_viewport_dimensions(managed) -> Result<(u32, u32), AppError>` that:
  - Evaluates JS to get `window.innerWidth` and `window.innerHeight`
  - Returns `(width, height)` as integers
- [ ] `set_viewport(managed, width, height) -> Result<(), AppError>` that:
  - Calls `Emulation.setDeviceMetricsOverride` with `{ width, height, deviceScaleFactor: 1, mobile: false }`
- [ ] `clear_viewport(managed) -> Result<(), AppError>` that:
  - Calls `Emulation.clearDeviceMetricsOverride`

**Notes**: These helpers are used by `execute_screenshot` for the full-page strategy. The `Emulation` domain must be enabled via `ensure_domain`.

### T006: Implement `execute_screenshot()` core function

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T001, T004, T005
**Acceptance**:
- [ ] `ScreenshotResult` struct with `format`, `data`, `width`, `height` fields, derives `Serialize`
- [ ] `ScreenshotFileResult` struct with `format`, `file`, `width`, `height` fields, derives `Serialize`
- [ ] `execute_screenshot(global, args)` function implemented:
  1. Validates mutual exclusion: `--full-page` cannot combine with `--selector`/`--uid` (returns `AppError`)
  2. Sets up CDP session via `setup_session()`
  3. Enables `Page` domain
  4. Determines capture strategy:
     - `--selector`/`--uid`: resolve element clip via T004, also enable `DOM` domain
     - `--full-page`: get page dimensions, set viewport, enable `Emulation` and `Runtime` domains
     - `--clip`: parse clip string via T004
     - Default: no clip (viewport capture)
  5. Builds `Page.captureScreenshot` params: `{ format, quality (if jpeg/webp), clip (if any), captureBeyondViewport (if full-page) }`
  6. Calls `Page.captureScreenshot`
  7. If full-page: restores viewport via `clear_viewport`
  8. Resolves image dimensions (from clip, viewport, or page dimensions)
  9. If `--file`: base64-decode data, write to file, output `ScreenshotFileResult`
  10. If no `--file`: output `ScreenshotResult` with base64 data
  11. Warns to stderr if base64 data length > ~10MB
- [ ] `PageCommand::Screenshot` arm added to `execute_page()` dispatcher
- [ ] `ScreenshotFormat` to CDP format string mapping (`Png` → `"png"`, `Jpeg` → `"jpeg"`, `Webp` → `"webp"`)
- [ ] Quality default of 80 applied when `--quality` not specified and format is jpeg/webp
- [ ] Quality parameter omitted from CDP params when format is png

---

## Phase 3: Integration

### T007: Verify end-to-end with cargo clippy and existing tests

**File(s)**: (all modified files)
**Type**: Verify
**Depends**: T006
**Acceptance**:
- [ ] `cargo clippy --all-targets -- -D warnings` passes with zero warnings
- [ ] `cargo fmt --check` passes
- [ ] `cargo test --lib` passes (all unit tests including new ones)
- [ ] `cargo build` succeeds
- [ ] `chrome-cli page screenshot --help` displays expected usage info with all options

---

## Phase 4: Testing

### T008: Create BDD feature file for screenshot capture

**File(s)**: `tests/features/screenshot-capture.feature`
**Type**: Create
**Depends**: T006
**Acceptance**:
- [ ] All 16 acceptance criteria from `requirements.md` are Gherkin scenarios
- [ ] Uses `Background:` for shared Chrome setup
- [ ] Includes error scenarios (conflicting flags, non-existent elements, invalid clip)
- [ ] Valid Gherkin syntax
- [ ] Scenarios are independent and declarative

### T009: Add unit tests for screenshot types and helpers

**File(s)**: `src/page.rs`, `src/error.rs`
**Type**: Modify
**Depends**: T004, T006
**Acceptance**:
- [ ] `ScreenshotResult` serialization tests (all fields present, correct types)
- [ ] `ScreenshotFileResult` serialization tests (file path, no data field)
- [ ] `parse_clip` tests: valid, decimal, invalid, too few values, empty string
- [ ] Error constructor tests in `error.rs` for `screenshot_failed`, `uid_not_found`, `invalid_clip`
- [ ] All tests pass with `cargo test`

---

## Dependency Graph

```
T001 (base64 dep) ─────────────────┐
                                    │
T002 (error helpers) ──┐            │
                       ├──▶ T004 ──┤
T003 (CLI args)    ────┤           ├──▶ T006 ──▶ T007 (verify)
                       └──▶ T005 ──┘      │
                                          ├──▶ T008 (BDD feature)
                                          └──▶ T009 (unit tests)
```

T001, T002, and T003 can be done in parallel (no interdependency).
T004 and T005 can proceed once T002 and T003 are complete.
T006 depends on T001 (base64), T004 (clip/element helpers), and T005 (viewport helpers).
T007 is a verification gate.
T008 and T009 can proceed once T006 is complete.

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] BDD test tasks included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
