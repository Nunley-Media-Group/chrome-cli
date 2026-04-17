# Tasks: Interaction Strategy Guide in Examples Command

**Issues**: #201
**Date**: 2026-04-16
**Status**: Planning
**Author**: Claude (spec-writer)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 3 | [ ] |
| Backend | 4 | [ ] |
| Frontend | N/A (CLI-only — no frontend layer in `structure.md`) | — |
| Integration | 3 | [ ] |
| Testing | 6 | [ ] |
| Verification | 2 | [ ] |
| **Total** | **18 tasks** | |

Note: `structure.md` defines a layered CLI architecture (CLI \u2192 Dispatch \u2192 Command Modules \u2192 CDP); this feature touches only the CLI + Command Module layers. The "Backend" phase maps to the command-module layer, and the "Frontend" phase is not applicable.

---

## Phase 1: Setup

### T001: Split `src/examples.rs` into a submodule (pure move, no content change)

**File(s)**:
- Delete: `src/examples.rs`
- Create: `src/examples/mod.rs` (holds `pub fn execute_examples`, re-exports; pure move of the current dispatcher)
- Create: `src/examples/commands.rs` (holds existing `CommandGroupSummary`, `ExampleEntry`, `all_examples()`, `format_plain_summary`, `format_plain_detail`, and all existing unit tests \u2014 moved verbatim)

**Type**: Create + Delete (pure move)
**Depends**: None
**Acceptance**:
- [ ] `src/examples.rs` no longer exists
- [ ] `src/examples/mod.rs` and `src/examples/commands.rs` exist
- [ ] `use crate::examples::execute_examples;` (the existing import path in `src/main.rs`) still resolves
- [ ] `cargo build` passes
- [ ] `cargo test --lib` passes with **zero** test failures (all existing unit tests pass unmodified)
- [ ] `cargo clippy --all-targets` passes
- [ ] `cargo fmt --check` passes
- [ ] `git diff --stat` shows roughly balanced additions and deletions (the move should be nearly line-for-line)

**Notes**: This is a pure refactor \u2014 no behavior changes. Keep the diff clean so reviewers can easily see the move vs. subsequent additions. Visibility may need to shift: `format_plain_summary`/`format_plain_detail` become `pub(super)` so `mod.rs` can call them.

### T002: Add `Strategy`, `StrategySummary`, `Workaround` types to `strategies.rs`

**File(s)**: `src/examples/strategies.rs` (create)
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] `StrategySummary { name: String, title: String, summary: String }` defined with `#[derive(Serialize, Clone)]`
- [ ] `Strategy { name, title, summary, scenarios: Vec<String>, capabilities: Vec<String>, limitations: Vec<String>, workarounds: Vec<Workaround>, recommended_sequence: Vec<String> }` defined with `#[derive(Serialize, Clone)]`
- [ ] `Workaround { description: String, commands: Vec<String> }` defined with `#[derive(Serialize, Clone)]`
- [ ] Module is declared in `src/examples/mod.rs` via `mod strategies;` and selected types re-exported
- [ ] `cargo build` passes
- [ ] `cargo clippy --all-targets` passes with no new warnings

**Notes**: Fields use snake_case (matches existing `CommandGroupSummary` naming in `commands.rs`). No serde rename directives needed.

### T003: Add `name: Option<String>` second positional to `ExamplesArgs` in `cli/mod.rs`

**File(s)**: `src/cli/mod.rs` (modify \u2014 `ExamplesArgs` around line 3545; `Command::Examples` variant around line 644)
**Type**: Modify
**Depends**: None (independent of T001/T002)
**Acceptance**:
- [ ] `ExamplesArgs` has two positionals: existing `command: Option<String>` and new `name: Option<String>`
- [ ] New `name` field has a doc comment pointing to `agentchrome examples strategies` as the source of valid names (per tech.md Clap Help Entries principle)
- [ ] `Command::Examples` variant's `long_about` now describes the strategies path (listing and detail)
- [ ] `Command::Examples` variant's `after_long_help` EXAMPLES block includes (minimum):
  - `agentchrome examples` (existing)
  - `agentchrome examples navigate` (existing)
  - `agentchrome examples strategies` (new)
  - `agentchrome examples strategies iframes` (new)
  - `agentchrome examples strategies --json` (new)
  - `agentchrome examples --pretty` (existing)
- [ ] `cargo build` passes
- [ ] A new unit test in the existing `#[cfg(test)] mod tests` block asserts:
  - `agentchrome examples strategies` parses to `ExamplesArgs { command: Some("strategies"), name: None }`
  - `agentchrome examples strategies iframes` parses to `ExamplesArgs { command: Some("strategies"), name: Some("iframes") }`
  - `agentchrome examples navigate` still parses to `ExamplesArgs { command: Some("navigate"), name: None }` (regression guard)

**Notes**: Refer to existing variants (e.g., `Connect`, `Page`) for the pattern of `about` + `long_about` + `after_long_help`.

---

## Phase 2: Backend (Command Module)

### T004: Implement strategy data \u2014 4 of 10 launch guides

**File(s)**: `src/examples/strategies.rs` (modify)
**Type**: Create (data)
**Depends**: T002
**Acceptance**:
- [ ] `pub fn all_strategies() -> Vec<Strategy>` defined and returns exactly these four guides with full content: `iframes`, `overlays`, `scorm`, `drag-and-drop`
- [ ] Each guide has non-empty `title`, `summary` (one line, \u2264 90 chars), `scenarios` (\u2265 2), `capabilities` (\u2265 3), `limitations` (\u2265 1), `recommended_sequence` (\u2265 3 commands, each starting with `agentchrome`)
- [ ] Each guide has \u2265 1 `workarounds` entry
- [ ] All command references are **currently shipped** surfaces (verify against `cargo run -- --help` and `cargo run -- <group> --help`) \u2014 per FR15
- [ ] `cargo build` + `cargo clippy --all-targets` pass

**Notes**: Split into two tasks (T004, T005) to keep each code review tractable. Content guidelines:
- **iframes**: reference `page frames`, `page snapshot --frame N`, `interact --frame N`, `dom --frame N`, `js --frame N`; workaround uses `js --frame N exec`; limitations note cross-origin frame field nullability.
- **overlays**: reference `diagnose` (detection), `page analyze`, `page hittest` (verify target), `interact click` + `--wait-until`; workarounds cover acc-blocker dismissal via `js exec` `document.querySelector(...).click()` and CSS `display: none` injection.
- **scorm**: reference iframe + media strategies together; `media list --frame N`, `media seek-end --all`, `page frames`, `interact click-at --frame N`; workarounds cover gated-narration bypass.
- **drag-and-drop**: reference `interact drag-at` (auto \u2192 `--steps` for HTML5 DnD), decomposed `interact mousedown-at` + `interact mouseup-at`, `--relative-to` for percentage coords.

### T005: Implement strategy data \u2014 remaining 6 of 10 launch guides

**File(s)**: `src/examples/strategies.rs` (modify \u2014 extend `all_strategies()`)
**Type**: Modify (data)
**Depends**: T004
**Acceptance**:
- [ ] `all_strategies()` returns exactly ten guides: the four from T004 plus `shadow-dom`, `spa-navigation-waits`, `react-controlled-inputs`, `debugging-failed-interactions`, `authentication-cookie-reuse`, `multi-tab-workflows`
- [ ] Each new guide meets the same quality bar as T004 (non-empty fields, \u2265 2 scenarios, \u2265 3 capabilities, \u2265 1 limitation, \u2265 3-command sequence referencing shipped surfaces)
- [ ] No two guides share the same `name`
- [ ] `cargo build` + `cargo clippy --all-targets` pass

**Notes** (content guidelines):
- **shadow-dom**: `--pierce-shadow` on `dom select`, `page snapshot`, `interact`; workaround uses `js exec` traversal via `.shadowRoot`.
- **spa-navigation-waits**: `navigate --wait-until networkidle|selector`, `interact click --wait-until`, polling with `page find` + `page wait` (if shipped \u2014 verify), `js exec` for framework readiness checks. Cite issues #144/#145/#178 as motivation in a comment.
- **react-controlled-inputs**: `form fill` first; fallback to `js exec` setter via `Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(el, value); el.dispatchEvent(new Event('input', {bubbles: true}))`; ARIA combobox with `form fill --confirm-key Tab|Enter|ArrowDown`.
- **debugging-failed-interactions**: sequence `diagnose --current` \u2192 `page hittest X Y` \u2192 `page coords --selector \u2026` \u2192 `console read --errors-only` \u2192 `network list --type xhr,fetch` \u2192 `page snapshot` as a meta-workflow.
- **authentication-cookie-reuse**: `cookie list --json > session.json`, then `cat session.json | jq -c '.[]' | while read c; do agentchrome cookie set \u2026; done` on subsequent runs; `cookie clear` to reset.
- **multi-tab-workflows**: `tabs list` before click that opens a new tab \u2192 `tabs list` after \u2192 `tabs activate <new-id>` \u2192 operate \u2192 `tabs close`. Cover `--tab` global flag for stateless targeting.

### T006: Implement `strategy_summaries()`, `find_strategy()`, and plain formatters

**File(s)**: `src/examples/strategies.rs` (modify)
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `pub fn strategy_summaries() -> Vec<StrategySummary>` projects `all_strategies()` to summary form (no allocation of full bodies in the output vec)
- [ ] `pub fn find_strategy(name: &str) -> Option<Strategy>` does a linear scan by `name` (exact match)
- [ ] `pub(super) fn format_plain_strategy_list(summaries: &[StrategySummary]) -> String` returns a string where each line is `<name> \u{2014} <summary>` (matching the format style in `commands.rs::format_plain_summary`)
- [ ] `pub(super) fn format_plain_strategy_detail(strategy: &Strategy) -> String` returns a sectioned string with headings `SCENARIOS`, `CURRENT CAPABILITIES`, `LIMITATIONS`, `WORKAROUNDS`, `RECOMMENDED SEQUENCE` (per the design mockup)
- [ ] `cargo build` + `cargo clippy --all-targets` pass

### T007: Wire the strategies path into the dispatcher

**File(s)**: `src/examples/mod.rs` (modify)
**Type**: Modify
**Depends**: T001, T003, T006
**Acceptance**:
- [ ] `execute_examples(global, args)` handles these branches:
  - `args.command == None` \u2192 append synthetic `strategies` entry to `all_examples()` result, then format as today (plain summary or JSON)
  - `args.command == Some("strategies")` && `args.name == None` \u2192 plain mode calls `format_plain_strategy_list(&strategy_summaries())`; JSON mode calls `print_output(&strategy_summaries(), &global.output)`
  - `args.command == Some("strategies")` && `args.name == Some(n)` \u2192 `find_strategy(n)`; Some \u2192 format/print detail; None \u2192 return `AppError { message: "Unknown strategy: '<n>'. Available: <csv of names>", code: ExitCode::GeneralError, custom_json: None }`
  - `args.command == Some(other)` \u2192 existing command-group lookup (unchanged; `args.name` is ignored for non-strategies paths to preserve AC11)
- [ ] The synthetic `strategies` entry in the top-level listing has `command: "strategies"`, a one-line description, and at least three example entries that reference `examples strategies`, `examples strategies iframes`, `examples strategies --json`
- [ ] `cargo build` + `cargo clippy --all-targets` + `cargo fmt --check` pass

---

## Phase 3: Integration

### T008: Update `README.md` Usage Examples section

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T007
**Acceptance**:
- [ ] `README.md` "Usage Examples" (or equivalent discovery) section includes at least one `agentchrome examples strategies` invocation
- [ ] The example is consistent with existing invocation examples in the section (same formatting, same shell prompt style)
- [ ] No other content in `README.md` is modified

### T009: Verify `agentchrome capabilities` output reflects the new `name` positional

**File(s)**: None (verification only; `capabilities.rs` is clap-driven)
**Type**: Verify
**Depends**: T003
**Acceptance**:
- [ ] `cargo run -- capabilities --command examples` output includes the new positional argument metadata for `name`
- [ ] `cargo run -- capabilities` (full) includes the `examples` entry with the updated args

**Notes**: If the capabilities manifest does NOT pick up the new positional automatically, that is a bug in `capabilities.rs`, not in this spec \u2014 log the finding and file a follow-on issue.

### T010: Verify `cargo xtask man examples` renders strategies content

**File(s)**: None (verification only; man pages are clap-driven via `clap_mangen`)
**Type**: Verify
**Depends**: T003
**Acceptance**:
- [ ] `cargo xtask man examples 2>&1 | head -100` (or the equivalent aggregated flow) includes text about the `strategies` path
- [ ] The man page includes the new EXAMPLES entries added in T003
- [ ] If the xtask output does not reflect the new `long_about` / `after_long_help`, investigate whether `xtask/src/main.rs` needs updating \u2014 this is still within scope because the tech.md Clap Help Entries principle requires man pages to cover new surfaces

---

## Phase 4: Testing

### T011: Unit tests for strategy data integrity

**File(s)**: `src/examples/strategies.rs` (append `#[cfg(test)] mod tests { \u2026 }`)
**Type**: Create
**Depends**: T005, T006
**Acceptance**:
- [ ] Test `all_strategies_returns_ten_required_guides`: asserts the ten launch names are all present (AC8)
- [ ] Test `no_duplicate_strategy_names`: asserts `name` is unique across all strategies
- [ ] Test `every_strategy_has_non_empty_fields`: asserts no strategy has empty `title`, `summary`, or empty `scenarios`/`capabilities`/`limitations`/`recommended_sequence`
- [ ] Test `recommended_sequences_start_with_agentchrome`: asserts every command in `recommended_sequence` and every `workarounds[].commands` entry begins with `agentchrome`
- [ ] Test `strategy_names_are_kebab_case`: asserts `name` is kebab-case (regex `^[a-z]+(-[a-z]+)*$`)
- [ ] Test `strategy_name_does_not_collide_with_command_groups`: asserts `"strategies"` is not a `command` value in `all_examples()` (FR14)
- [ ] `cargo test --lib` passes

### T012: Unit tests for progressive disclosure contract

**File(s)**: `src/examples/strategies.rs` (extend test module)
**Type**: Create
**Depends**: T006, T011
**Acceptance**:
- [ ] Test `summary_json_has_only_three_fields`: serialize `strategy_summaries()`; parse back; assert every object has exactly `name`, `title`, `summary` and **none** of `scenarios`, `capabilities`, `limitations`, `workarounds`, `recommended_sequence`
- [ ] Test `detail_json_has_all_fields`: serialize `find_strategy("iframes").unwrap()`; assert all eight detail fields are present
- [ ] Test `summary_listing_under_4kb`: assert `serde_json::to_string(&strategy_summaries()).unwrap().len() < 4096` (AC4)
- [ ] Test `plain_listing_under_1kb`: assert `format_plain_strategy_list(&strategy_summaries()).len() < 1024` (AC1)
- [ ] `cargo test --lib` passes

### T013: Unit tests for plain-text formatting

**File(s)**: `src/examples/strategies.rs` (extend test module)
**Type**: Create
**Depends**: T006
**Acceptance**:
- [ ] Test `plain_list_contains_all_strategy_names`: every strategy name appears on its own line
- [ ] Test `plain_list_does_not_start_with_bracket_or_brace`: asserts plain mode never starts with `[` or `{`
- [ ] Test `plain_detail_contains_required_section_headers`: for each of the ten strategies, `format_plain_strategy_detail` output contains all five required headings (SCENARIOS, CURRENT CAPABILITIES, LIMITATIONS, WORKAROUNDS, RECOMMENDED SEQUENCE)
- [ ] Test `plain_detail_contains_every_recommended_sequence_command`: every command in a strategy's `recommended_sequence` appears verbatim in the detail output
- [ ] `cargo test --lib` passes

### T014: Unit tests for dispatcher routing

**File(s)**: `src/examples/mod.rs` (append/extend test module)
**Type**: Create
**Depends**: T007
**Acceptance**:
- [ ] Test `dispatcher_routes_strategies_listing`: args `{ command: Some("strategies"), name: None }` produces output containing the ten strategy names
- [ ] Test `dispatcher_routes_strategies_detail`: args `{ command: Some("strategies"), name: Some("iframes") }` produces output containing "CURRENT CAPABILITIES"
- [ ] Test `dispatcher_unknown_strategy_returns_error`: args `{ command: Some("strategies"), name: Some("bogus") }` returns `AppError` with message containing `"Unknown strategy"` and all ten valid names
- [ ] Test `dispatcher_top_level_listing_includes_strategies`: args `{ command: None, name: None }` produces output that contains the literal `"strategies"` as a group name
- [ ] Test `dispatcher_existing_group_behavior_preserved`: args `{ command: Some("navigate"), name: None }` produces the same output it did before this feature (regression guard for AC11)
- [ ] Test `dispatcher_existing_unknown_group_error_preserved`: args `{ command: Some("nonexistent"), name: None }` still returns the exact existing `"Unknown command group"` error (regression guard for AC11)
- [ ] `cargo test --lib` passes

### T015: Unit test for clap help metadata (steering compliance)

**File(s)**: `src/cli/mod.rs` (extend test module)
**Type**: Create
**Depends**: T003
**Acceptance**:
- [ ] Test `examples_subcommand_carries_clap_help_metadata`: uses `<Cli as CommandFactory>::command()` to introspect the `examples` subcommand; asserts:
  - `get_long_about()` is non-empty and contains the substring `"strategies"`
  - `get_after_long_help()` contains the substring `"examples strategies"`
  - `get_after_long_help()` contains at least one `--json` example
- [ ] This test codifies the tech.md **Clap Help Entries** steering principle for this surface and prevents future regressions that would strip the long-help text
- [ ] `cargo test --lib` passes

### T016: BDD feature file for strategies

**File(s)**: `tests/features/examples-strategies.feature` (create)
**Type**: Create
**Depends**: T007
**Acceptance**:
- [ ] Gherkin file covers every acceptance criterion AC1\u2013AC12 from `requirements.md` as a separate `Scenario` (one scenario per AC minimum)
- [ ] File is valid Gherkin (parses with `cucumber-rs`)
- [ ] Includes at least one `Scenario Outline` that parameterizes over all ten launch strategy names, asserting each produces a valid detail output when `examples strategies <name> --json` is called
- [ ] Includes a dedicated progressive-disclosure guard scenario: `examples strategies --json` stdout contains `"name"` and `"summary"` but does NOT contain any of `"scenarios"`, `"capabilities"`, `"limitations"`, `"workarounds"`, `"recommended_sequence"` as JSON keys
- [ ] Existing `tests/features/examples.feature` is unchanged (AC11)
- [ ] BDD step definitions in `tests/bdd.rs` are extended as needed; `cargo test --test bdd` passes

**Notes**: Follow the step-definition style used by `tests/features/examples.feature` (e.g., `Given the agentchrome binary is available`, `When I run "..."`, `Then stdout should contain "..."`). Most steps can reuse existing shared step definitions.

---

## Phase 5: Verification

### T017: Manual smoke test (per `tech.md` Manual Smoke Test requirement)

**File(s)**: None (verification only)
**Type**: Verify
**Depends**: T007, T008, T011\u2013T016
**Acceptance**:
- [ ] `cargo build` (debug) succeeds
- [ ] `./target/debug/agentchrome examples strategies` lists all ten strategies (plain)
- [ ] `./target/debug/agentchrome examples strategies iframes` shows full iframe guide sections
- [ ] `./target/debug/agentchrome examples strategies --json | jq 'length'` returns `10`
- [ ] `./target/debug/agentchrome examples strategies --json | jq '.[0] | keys'` returns exactly `["name", "summary", "title"]` (alphabetical, no detail fields)
- [ ] `./target/debug/agentchrome examples strategies iframes --json | jq 'keys'` returns all eight detail keys
- [ ] `./target/debug/agentchrome examples strategies bogus 2>&1 1>/dev/null` writes a JSON error to stderr; exit code is `1`
- [ ] `./target/debug/agentchrome examples` top-level listing includes `strategies` alongside the existing command groups
- [ ] `./target/debug/agentchrome examples --help 2>&1 | grep -E 'strategies'` finds the new help text
- [ ] No Chrome process is launched during any of the above (this feature is Chrome-free)

**Notes**: Since this feature does not touch CDP, the standard Chrome-based Feature Exercise Gate is satisfied by these CLI-only checks. Record outputs in the verification report.

### T018: Verify all verification gates pass (per `tech.md` Verification Gates table)

**File(s)**: None (verification only)
**Type**: Verify
**Depends**: T017
**Acceptance**:
- [ ] `cargo build` \u2192 exit 0
- [ ] `cargo test --lib` \u2192 exit 0
- [ ] `cargo test --test bdd` \u2192 exit 0
- [ ] `cargo clippy --all-targets` \u2192 exit 0 (no new warnings)
- [ ] `cargo fmt --check` \u2192 exit 0
- [ ] Manual smoke test (T017) \u2192 all ACs verified

---

## Dependency Graph

```
T001 (split examples.rs) ──┬──▶ T002 (types) ──▶ T004 (data 1-4) ──┬──▶ T005 (data 5-10) ──▶ T006 (fns/fmt) ──▶ T007 (dispatcher)
                           │                                        │                                               │
                           └──▶ T003 (cli args) ──────────────────────────────────────────────────────────────────▶ T007
                                     │                                                                              │
                                     └──▶ T015 (clap help test)                                                    │
                                                                                                                    │
                                                                                                                    ▼
                                                                                       T008 (README)     T009 (caps)   T010 (man)
                                                                                                                    │
                                                                                             T011, T012, T013, T014, T016 (tests)
                                                                                                                    │
                                                                                                                    ▼
                                                                                                             T017 (smoke) ──▶ T018 (gates)
```

Critical path: **T001 \u2192 T002 \u2192 T004 \u2192 T005 \u2192 T006 \u2192 T007 \u2192 T016 \u2192 T017 \u2192 T018** (9 tasks).

---

## Change History

| Issue | Date       | Summary                  |
|-------|------------|--------------------------|
| #201  | 2026-04-16 | Initial task breakdown   |

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable (each has concrete `cargo` / CLI command or file-inspection step)
- [x] File paths reference actual project structure (per `structure.md`: `src/cli/mod.rs`, `src/examples/`, `tests/features/`, `tests/bdd.rs`, `README.md`)
- [x] Test tasks are included for each layer (unit in `strategies.rs` + `mod.rs` + `cli/mod.rs`, BDD for ACs, manual smoke)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
- [x] Progressive-disclosure contract has dedicated tests (T012, T016)
- [x] Clap-help-steering contract has dedicated test (T015)
