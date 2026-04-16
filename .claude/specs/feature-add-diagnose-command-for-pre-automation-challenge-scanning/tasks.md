# Tasks: Diagnose Command for Pre-Automation Challenge Scanning

**Issues**: #200
**Date**: 2026-04-16
**Status**: Planning
**Author**: Claude (spec-writer)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 4 | [ ] |
| Backend | 6 | [ ] |
| Integration | 4 | [ ] |
| Testing | 4 | [ ] |
| **Total** | **18 tasks** | |

---

## Task Format

Each task follows this structure:

```
### T[NNN]: [Task Title]

**File(s)**: `path/to/file`
**Type**: Create | Modify | Delete
**Depends**: T[NNN] (or None)
**Acceptance**:
- [ ] Verifiable criterion
```

---

## Phase 1: Setup (Shared types and refactors)

### T001: Extract `navigate_and_wait` helper in `navigate.rs`

**File(s)**: `src/navigate.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `pub(crate) async fn navigate_and_wait(managed: &mut ManagedSession, url: &str, wait_until: WaitUntil, timeout_ms: u64) -> Result<NavigateResult, AppError>` is added
- [ ] The body is the wait-for-navigation logic previously inside `execute_url` (everything after `setup_session` and before `print_output`)
- [ ] `execute_url` is refactored to call `navigate_and_wait` — no behavior change to `agentchrome navigate <url>`
- [ ] Error classification is unchanged (timeout → `ExitCode::TimeoutError`, protocol → `ExitCode::ProtocolError`)
- [ ] `cargo build` succeeds
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy --all-targets` produces no warnings
- [ ] `cargo fmt --check` passes

**Notes**: Keep the function signature `pub(crate)` — it is not a public API. Existing BDD tests for `navigate <url>` must continue to pass unmodified.

---

### T002: Expose `page::analyze` detectors and shared types as `pub(crate)`

**File(s)**: `src/page/analyze.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] Detector functions (`enumerate_iframes`, `detect_frameworks`, `count_interactive_elements`, `catalog_media`, `detect_overlays`, `detect_shadow_dom`) are marked `pub(crate)` and reachable via `crate::page::analyze::detectors::*` (either by moving into a `pub(crate) mod detectors` submodule or by adding a `pub(crate) use` re-export)
- [ ] Shared output types (`IframeInfo`, `MediaInfo`, `OverlayInfo`, `ShadowDomInfo`) are `pub(crate)` and importable from `src/diagnose/`
- [ ] `AnalyzeResult` itself remains private to `page::analyze` (not exposed)
- [ ] `cargo build` succeeds
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy --all-targets` produces no warnings
- [ ] `cargo fmt --check` passes
- [ ] Existing `page analyze` BDD scenarios continue to pass unmodified

**Notes**: Aim for minimal churn — keep the original call sites in `analyze.rs` working with `use detectors::*` (or equivalent re-export) so `execute_analyze` doesn't need edits.

---

### T003: Define `DiagnoseArgs` in `cli/mod.rs`

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] New `pub struct DiagnoseArgs` contains `url: Option<String>` (positional), `current: bool` (`#[arg(long)]`), `wait_until: WaitUntil` (`default_value_t = WaitUntil::Load`), `timeout: Option<u64>`
- [ ] The struct has `#[command(group(ArgGroup::new("target").required(true).args(["url", "current"])))]` so clap rejects "neither supplied" at parse time
- [ ] `#[arg(conflicts_with = "current")]` on `url` and `#[arg(conflicts_with = "url")]` on `current` reject "both supplied"
- [ ] A new `Diagnose(DiagnoseArgs)` variant is added to the top-level `Command` enum with `long_about` documenting the command, argument shape, JSON output schema, and exit codes, and `after_long_help` containing at least two worked examples (one URL, one `--current`)
- [ ] `cargo build` succeeds
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy --all-targets` produces no warnings
- [ ] `cargo fmt --check` passes

**Notes**: `ArgGroup` with `required = true` is the standard clap idiom for "exactly one of". Verify via a unit test (later T017) that the missing-args error message is clear.

---

### T004: Create `src/diagnose/output.rs` with output types

**File(s)**: `src/diagnose/output.rs`, `src/diagnose/mod.rs`
**Type**: Create
**Depends**: T002
**Acceptance**:
- [ ] `DiagnoseResult` struct with `url`, `scope`, `navigationStatus` (skip-if-none), `challenges`, `patterns`, `summary` fields, all `#[serde(rename_all = "camelCase")]`
- [ ] `Challenge` struct with `category`, `severity`, `summary`, `details` (typed per category via an enum `ChallengeDetails` with variants `Iframes / Overlays / ShadowDom / Canvas / Media / Framework`), `suggestion: Option<String>`
- [ ] `PatternMatch` struct with `name`, `matched`, `confidence`, `evidence`, `suggestion`
- [ ] `DiagnoseSummary` struct with `challenge_count`, `pattern_match_count`, `has_high_severity`, `straightforward` (all camelCase when serialized)
- [ ] All unmeasurable fields use `Option<T>` so they serialize as `null` (per FR21 / AC7), **not** skipped with `skip_serializing_if` unless semantically "omitted" (currently only `navigationStatus` uses skip)
- [ ] `cargo build` succeeds
- [ ] `cargo test --lib` passes (serialization round-trip tests pending T016)
- [ ] `cargo clippy --all-targets` produces no warnings
- [ ] `cargo fmt --check` passes

**Notes**: `ChallengeDetails` as a tagged enum serializes cleanly to per-category JSON. Attribute with `#[serde(untagged)]` or keep as an inline object in `Challenge` with flattened category-specific sub-fields — prefer untagged enum to keep `details` a single nested object.

---

## Phase 2: Backend Implementation (Detectors, patterns, orchestrator)

### T005: Implement canvas/WebGL detector

**File(s)**: `src/diagnose/detectors.rs`
**Type**: Create
**Depends**: T004
**Acceptance**:
- [ ] `pub(crate) async fn detect_canvas(session: &ManagedSession) -> Option<CanvasInfo>` returns `Some(CanvasInfo { canvas_count, webgl_count, items })` when `<canvas>` elements are present, `None` when no `<canvas>` elements are on the page
- [ ] Detection JS probes `webgl2 → webgl → 2d` via `getContext` in that order and never creates a new context when one doesn't already exist (use the probe order specified in design.md R1 mitigation)
- [ ] `CanvasItem.width`, `height`, and `context` are `Option<u32>` / `Option<String>` so unmeasurable values serialize as `null` (AC7)
- [ ] CDP-layer failures are caught: the function returns `None` on JS eval error (graceful degradation)
- [ ] `cargo build` succeeds
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy --all-targets` produces no warnings
- [ ] `cargo fmt --check` passes

---

### T006: Implement media-gate refinement and framework-quirk detectors

**File(s)**: `src/diagnose/detectors.rs`
**Type**: Modify (extend the file created in T005)
**Depends**: T005
**Acceptance**:
- [ ] `pub(crate) fn classify_media_gate(media: &[MediaInfo]) -> Vec<MediaGateInfo>` is a pure function over the existing `MediaInfo` output, adding `gates_navigation: bool` per entry (heuristic from design.md)
- [ ] `pub(crate) async fn detect_framework_quirks(session: &ManagedSession) -> FrameworkQuirks` runs ONE `Runtime.evaluate` call that probes React portal, Angular zone.js, Vue teleport, and Svelte hydration signals and returns a struct of four booleans
- [ ] On eval failure, `detect_framework_quirks` returns `FrameworkQuirks { react_portal: false, angular_zone: false, vue_teleport: false, svelte_hydration: false }` (graceful degradation — absence of signal is reported as `false`, not error)
- [ ] `FrameworkQuirks::any() -> bool` helper returns whether any field is true (used by challenge assembler)
- [ ] `cargo build` succeeds
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy --all-targets` produces no warnings
- [ ] `cargo fmt --check` passes

---

### T007: Implement pattern database and matcher

**File(s)**: `src/diagnose/patterns.rs`
**Type**: Create
**Depends**: T004, T005, T006
**Acceptance**:
- [ ] `pub(crate) struct DetectorBundle<'a>` holds references to all detector outputs (iframes, frameworks, overlays, shadow_dom, media, canvas, framework_quirks)
- [ ] `pub(crate) struct PatternRule { name: &'static str, detector: fn(&DetectorBundle) -> Option<PatternMatch> }`
- [ ] `pub(crate) static PATTERN_DB: &[PatternRule]` contains at least three entries: `storyline-acc-blocker`, `scorm-player`, `react-portal`
- [ ] Each detector function implements the multi-signal rules described in design.md, returning `Some(PatternMatch)` only when the rule matches
- [ ] Each `PatternMatch.suggestion` string contains at least one `agentchrome` command token (e.g., `interact click-at`, `--frame`, `page find`)
- [ ] `pub(crate) fn match_all(bundle: &DetectorBundle) -> Vec<PatternMatch>` returns only matched entries
- [ ] Unit tests cover each pattern's positive case, negative case, and confidence-downgrade case (single-signal → `low`/`medium`, multi-signal → `high`)
- [ ] `cargo build` succeeds
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy --all-targets` produces no warnings
- [ ] `cargo fmt --check` passes

**Notes**: Detectors are pure functions over already-collected signals — they must NOT make CDP calls.

---

### T008: Implement severity assignment + challenge assembly

**File(s)**: `src/diagnose/mod.rs` (or a new `src/diagnose/assembly.rs` if file length demands)
**Type**: Modify (extend `mod.rs`)
**Depends**: T004, T005, T006, T007
**Acceptance**:
- [ ] `fn assemble_challenges(bundle: &DetectorBundle) -> Vec<Challenge>` emits challenge entries only for categories with non-empty data (per AC3 / design.md "Challenge Assembly" section)
- [ ] Severity assignment follows the table in design.md (Severity Assignment): `iframes`, `overlays`, `shadowDom`, `canvas`, `media`, `framework` each use the documented thresholds
- [ ] Per-category `suggestion` strings are `&'static str` constants defined once in `src/diagnose/detectors.rs` (or a dedicated `suggestions.rs` file) — each contains at least one `agentchrome` command token
- [ ] `DiagnoseSummary::has_high_severity` is computed from the assembled challenges
- [ ] `DiagnoseSummary::straightforward` == `challenge_count == 0 && pattern_match_count == 0`
- [ ] Unit tests cover the severity thresholds for each category (parameterized tests or one per category)
- [ ] `cargo build` succeeds
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy --all-targets` produces no warnings
- [ ] `cargo fmt --check` passes

---

### T009: Implement `execute_diagnose` orchestrator

**File(s)**: `src/diagnose/mod.rs`
**Type**: Modify
**Depends**: T001, T002, T003, T004, T005, T006, T007, T008
**Acceptance**:
- [ ] `pub async fn execute_diagnose(global: &GlobalOpts, args: &DiagnoseArgs) -> Result<(), AppError>` is the module entry point
- [ ] Session is set up via `setup_session_with_interceptors`
- [ ] If `args.url.is_some()`, `navigate_and_wait` is called; the resulting `status` (`Option<u16>`) is recorded into `navigationStatus`; on error the `AppError` is propagated as-is
- [ ] If `args.current == true`, NO `Page.navigate` call is issued (verified by BDD later); the active tab's current URL/title is read via `get_page_info`
- [ ] All reused detectors from `page::analyze::detectors` are called; plus canvas, framework quirks, and media-gate classification
- [ ] Each detector failure degrades to `null`/empty in the corresponding field — no detector error aborts the command
- [ ] `scope` is `"diagnosed"` when URL mode, `"current"` when `--current`
- [ ] Final `DiagnoseResult` is written via `print_output`
- [ ] Exit code 0 on full success
- [ ] `cargo build` succeeds
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy --all-targets` produces no warnings
- [ ] `cargo fmt --check` passes

---

### T010: Unit test: suggestion-string actionability lint

**File(s)**: `src/diagnose/detectors.rs` (or a dedicated test module under `src/diagnose/`)
**Type**: Create
**Depends**: T007, T008
**Acceptance**:
- [ ] A `#[test]` function iterates every suggestion string (both per-category suggestions and per-pattern suggestions in `PATTERN_DB`)
- [ ] The test asserts each string contains at least one of the token patterns: `agentchrome`, `interact click-at`, `--frame`, `page find`, `page snapshot`, `form fill`, or `js exec`
- [ ] The test fails with a helpful message identifying which suggestion lacks an `agentchrome` command reference
- [ ] The test runs in `cargo test --lib`

**Notes**: This enforces success metric "100% of suggestion strings reference at least one concrete `agentchrome` command" from requirements.md.

---

## Phase 3: Integration (CLI wiring, help, examples, docs)

### T011: Wire `Command::Diagnose` dispatch in `main.rs`

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T003, T009
**Acceptance**:
- [ ] `mod diagnose;` is added to the module list
- [ ] The `run()` match adds `Command::Diagnose(args) => diagnose::execute_diagnose(&global, args).await`
- [ ] `cargo build` succeeds
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy --all-targets` produces no warnings
- [ ] `cargo fmt --check` passes
- [ ] Smoke check: `./target/debug/agentchrome diagnose --help` prints the long-help block

---

### T012: Add `diagnose` entry to `examples.rs`

**File(s)**: `src/examples.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] A new `CommandGroupSummary` for `"diagnose"` is inserted into `all_examples()` in the same alphabetical/existing position pattern used by sibling entries
- [ ] Includes at least three `ExampleEntry` items: basic URL invocation, `--current` invocation, and one example piped through `jq` (or similar) to extract a suggestion from the output — mirroring the style of existing `examples` entries
- [ ] Each `ExampleEntry.description` is concise (one line) and each `cmd` is a valid, copy-pasteable invocation
- [ ] Running `./target/debug/agentchrome examples diagnose` emits the new entries on stdout
- [ ] `cargo build` / `cargo test --lib` / `cargo clippy --all-targets` / `cargo fmt --check` all pass

---

### T013: Update `README.md` Command Reference + Usage Examples

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] A new row is added to the Command Reference table: `| `diagnose` | Pre-automation challenge scan (iframes, overlays, media gates, frameworks, patterns) |`
- [ ] The row is placed in the same ordering pattern as neighboring rows (reasonable adjacent placement, e.g., near `audit` or `page`)
- [ ] The "Usage Examples" section gains at least one `diagnose` example block (URL mode and `--current` mode, or a combined jq pipeline) consistent with the style of existing examples in that section
- [ ] Markdown table syntax is valid; README renders correctly (spot-check by reading the file)

---

### T014: Verify capabilities manifest + man page include `diagnose`

**File(s)**: No file changes — verification only
**Type**: Verify
**Depends**: T003, T011
**Acceptance**:
- [ ] `./target/debug/agentchrome capabilities | jq '.commands[] | select(.name == "diagnose")'` returns a non-empty object describing `diagnose` with its arguments (`url`, `--current`, `--wait-until`, `--timeout`)
- [ ] `cargo xtask man` regenerates man pages without error and the output includes a `diagnose` section (check via inspecting generated manpage source under the xtask output path)
- [ ] `./target/debug/agentchrome man diagnose` prints a man page section for the new command
- [ ] No code changes should be required here — the capabilities manifest and man page pipeline read clap metadata directly. If any change IS required, file a follow-up note in the spec `Change History` with a brief description.

---

## Phase 4: Testing (BDD + unit + smoke)

### T015: Create BDD feature file for `diagnose`

**File(s)**: `tests/features/diagnose.feature`
**Type**: Create
**Depends**: T009
**Acceptance**:
- [ ] Feature file is named exactly `diagnose.feature` under `tests/features/`
- [ ] Contains one Gherkin `Scenario` for each of AC1–AC12 from requirements.md (12 scenarios total)
- [ ] Gherkin syntax is valid — `cargo test --test bdd` does not reject the file at parse time
- [ ] Scenario names are short and descriptive; each scenario's Given/When/Then mirrors the AC text
- [ ] `@wip` or skip tags are used only for scenarios requiring features not yet implemented (there should be none)

---

### T016: Implement BDD step definitions + create test fixture HTML

**File(s)**: `tests/bdd.rs`, `tests/fixtures/diagnose.html`
**Type**: Modify (`tests/bdd.rs`) / Create (fixture HTML)
**Depends**: T015
**Acceptance**:
- [ ] `tests/fixtures/diagnose.html` is a self-contained HTML file covering the ACs — at minimum: at least one iframe (same-origin), at least one viewport-covering overlay with `z-index > 0`, a `<canvas>` element, a `<video>` with `autoplay` attribute, and a `div.acc-blocker` + `#story_content` pair (for Storyline pattern) and a React-detection signal (`[data-reactroot]` or `window.__REACT_DEVTOOLS_GLOBAL_HOOK__` stub). Includes an HTML comment at the top listing which ACs each structural element covers.
- [ ] A second fixture or a fragment in the same file provides a "clean page" scenario covering AC3 (or use a second file `tests/fixtures/diagnose-clean.html`)
- [ ] `tests/bdd.rs` contains a `DiagnoseWorld` struct and step implementations for every Given/When/Then phrase used in `diagnose.feature`
- [ ] Step definitions use the same patterns as sibling BDD worlds in the file (look at `NavigateWorld`/`PageAnalyzeWorld`/etc.)
- [ ] Scenarios that require a live Chrome instance are skipped automatically in environments without Chrome (consistent with existing conventions)
- [ ] `cargo test --test bdd` completes without regression in other feature files
- [ ] `cargo fmt --check` passes

**Notes**: For AC6 (cross-origin iframes), use an `iframe src="about:blank"` with `srcdoc` pointing at a different origin, OR use `data:` URLs. Pick whichever matches existing agentchrome BDD fixture conventions.

---

### T017: Add unit tests for argument parsing + error shapes

**File(s)**: `src/cli/mod.rs` (inline `#[cfg(test)]` module) or a dedicated `tests/cli_diagnose.rs` integration test
**Type**: Create
**Depends**: T003
**Acceptance**:
- [ ] Test: `agentchrome diagnose` (no args) → clap error containing both `url` and `current` arg names; exit code when driven through `main.rs` error handling path is 1
- [ ] Test: `agentchrome diagnose <url> --current` → clap error about the conflict; exit code 1
- [ ] Test: `agentchrome diagnose https://example.com` → parses successfully with `url = Some(...), current = false`
- [ ] Test: `agentchrome diagnose --current` → parses successfully with `url = None, current = true`
- [ ] Test: `agentchrome diagnose --current https://example.com` → clap error (positional+flag still conflict even in that order)
- [ ] `cargo test --lib` passes; `cargo clippy --all-targets` clean; `cargo fmt --check` passes

---

### T018: Manual smoke test against real headless Chrome

**File(s)**: No file changes — verification only; results recorded in the `/verifying-specs` output during verification phase
**Type**: Verify
**Depends**: T009, T011, T012, T013, T014, T015, T016, T017
**Acceptance**:
- [ ] `cargo build` (debug) succeeds
- [ ] `./target/debug/agentchrome connect --launch --headless` starts a clean headless Chrome
- [ ] `./target/debug/agentchrome navigate file://<absolute>/tests/fixtures/diagnose.html` loads the fixture
- [ ] `./target/debug/agentchrome diagnose --current` outputs a valid JSON `DiagnoseResult`; `challenges` contains entries for each structural category the fixture exercises; at least one `PatternMatch` (`storyline-acc-blocker`) is emitted with `confidence: "high"` and a `suggestion` referencing `interact click-at --frame`; `summary.straightforward` is `false`; exit code is 0
- [ ] `./target/debug/agentchrome diagnose file://<absolute>/tests/fixtures/diagnose-clean.html` (or equivalent clean page) outputs `challenges: []`, `patterns: []`, `summary.straightforward: true`, exit code 0
- [ ] `./target/debug/agentchrome diagnose` (no args) writes a JSON error to stderr, exit code 1, stdout empty
- [ ] `./target/debug/agentchrome diagnose --current https://example.com` writes a JSON error to stderr, exit code 1, stdout empty
- [ ] Orphaned Chrome processes are cleaned up: `./target/debug/agentchrome connect disconnect` then `pkill -f 'chrome.*--remote-debugging' || true`
- [ ] Results recorded in the verification report during `/verifying-specs`

**Notes**: This task MUST be included in `/verifying-specs` execution (per tech.md requirement). It is the only end-to-end verification against real Chrome.

---

## Dependency Graph

```
T001 (navigate extract) ─────────────────────────┐
T002 (analyze pub(crate)) ────────┐              │
                                  │              │
T003 (DiagnoseArgs) ────────┐     │              │
                            │     │              │
T004 (output types) ◀───────┴─────┘              │
   │                                              │
   ▼                                              │
T005 (canvas) ──▶ T006 (media gate + quirks) ──▶ T007 (pattern DB)
                                                  │
T004 + T005 + T006 + T007 ──▶ T008 (assembly) ───┤
                                                  ▼
                              T001 + T002 + T003 + T008 ──▶ T009 (orchestrator)
                                                              │
                        ┌─────────────────────────────────────┘
                        ▼
T007, T008 ──▶ T010 (suggestion-lint test)

T009 ──▶ T011 (main.rs dispatch) ──┐
T003 ──▶ T012 (examples.rs) ───────┤
T003 ──▶ T013 (README) ────────────┤
T003, T011 ──▶ T014 (capabilities + man verify)
                        │
                        ▼
T009 ──▶ T015 (feature file) ──▶ T016 (step defs + fixture)
T003 ──▶ T017 (CLI arg tests)
                        │
All above ──▶ T018 (smoke test — final verification gate)
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #200 | 2026-04-16 | Initial feature tasks |

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included (T015–T018 + unit tests embedded in T005–T010)
- [x] Manual smoke test is explicit as the final gate (T018)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
- [x] Every AC from requirements.md maps to a task or is verified in T015/T016/T018
- [x] Rustfmt / clippy / build gates appear as acceptance criteria on every Rust-code-touching task
