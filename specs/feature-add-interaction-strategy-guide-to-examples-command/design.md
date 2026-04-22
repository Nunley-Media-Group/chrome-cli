# Design: Interaction Strategy Guide in Examples Command

**Issues**: #201, #218
**Date**: 2026-04-21
**Status**: Amended
**Author**: Claude (spec-writer)

---

## Overview

Extend the existing `examples` subcommand (`src/examples.rs`) with a new `strategies` path that renders scenario-based interaction guides. The feature is **pure presentation layer** — no CDP calls, no Chrome connection, no I/O beyond stdout/stderr. Strategy content is compile-time static data, encoded as a `Vec<Strategy>` alongside the existing `Vec<CommandGroupSummary>`, reusing the same `OutputFormat` (plain / `--json` / `--pretty`) and the same `print_output` helper.

The launch set is **ten strategy guides** (per expanded requirements AC8 and FR5–FR5f):

| # | Name | Focus |
|---|------|-------|
| 1 | `iframes` | Frame targeting via `--frame`, cross-frame limits, `js exec` workaround |
| 2 | `overlays` | Detecting and bypassing full-viewport overlays (acc-blocker, modal, cookie consent) |
| 3 | `scorm` | SCORM/LMS player automation: iframe + media-gate + navigation patterns |
| 4 | `drag-and-drop` | `interact drag-at`, decomposed `mousedown-at`/`mouseup-at`, `--steps` |
| 5 | `shadow-dom` | `--pierce-shadow` on `dom`/`page`/`interact` for web-component UIs |
| 6 | `spa-navigation-waits` | `--wait-until networkidle\|selector`, polling with `page find`, `interact click --wait-until` |
| 7 | `react-controlled-inputs` | `form fill` vs `js exec` for controlled inputs; ARIA combobox `--confirm-key` |
| 8 | `debugging-failed-interactions` | Meta-workflow: `diagnose` \u2192 `page hittest` \u2192 `page coords` \u2192 `console read` \u2192 `network list` \u2192 `page snapshot` |
| 9 | `authentication-cookie-reuse` | Persisting session auth via `cookie list`/`set`/`delete`/`clear` across invocations |
| 10 | `multi-tab-workflows` | `tabs list`/`create`/`activate`/`close`; handling SSO-style new-tab opens; `--tab` targeting |

The CLI shape uses the **flat-positional** approach: `ExamplesArgs` gains a second optional positional `name`. The dispatcher special-cases the literal `"strategies"` in the first positional to branch into the strategy path, and uses the second positional to select a specific strategy by kebab-case name. Existing behavior (`agentchrome examples`, `agentchrome examples navigate`, `agentchrome examples --json`, unknown-group error) is preserved byte-for-byte — the branch is additive.

Because `src/examples.rs` already weighs in at ~1000 lines and strategies will add several hundred more, the file is refactored into a submodule (`src/examples/` with `mod.rs`, `commands.rs`, `strategies.rs`). This satisfies the issue's hint at a "modular structure" and scales cleanly when additional strategies are added later (per FR16).

---

## Architecture

### Component Diagram

Per `structure.md`, this feature lives entirely in the **Command Module** layer — it does not touch the CDP or Chrome layers.

```
┌──────────────────────────────────────────────────────────┐
│                    CLI Layer (src/cli/mod.rs)             │
│  ┌──────────────────────────────────────────────────────┐│
│  │ Command::Examples(ExamplesArgs)                       ││
│  │   ExamplesArgs {                                      ││
│  │     command: Option<String>,  // existing             ││
│  │     name:    Option<String>,  // NEW (2nd positional) ││
│  │   }                                                   ││
│  └──────────────────────────────────────────────────────┘│
└───────────────────────────┬──────────────────────────────┘
                            ▼
┌──────────────────────────────────────────────────────────┐
│           Dispatcher (src/main.rs → src/examples/mod.rs)  │
│  execute_examples(global, args) {                         │
│    match args.command {                                   │
│      None                    => list_all_groups_plus_strategies │
│      Some("strategies")      => dispatch_strategies(args.name)  │
│      Some(other_group_name)  => lookup_existing_group           │
│    }                                                      │
│  }                                                        │
└───────────────────────────┬──────────────────────────────┘
                            ▼
┌──────────────────────────────────────────────────────────┐
│            Data Modules                                   │
│  ┌─────────────────────────┐   ┌─────────────────────┐  │
│  │ src/examples/commands.rs│   │src/examples/         │  │
│  │ all_examples() -> Vec<  │   │  strategies.rs      │  │
│  │   CommandGroupSummary>  │   │ all_strategies()    │  │
│  │ (existing, unchanged)   │   │   -> Vec<Strategy>  │  │
│  └─────────────────────────┘   └─────────────────────┘  │
└───────────────────────────┬──────────────────────────────┘
                            ▼
┌──────────────────────────────────────────────────────────┐
│           Output Layer (src/output.rs, unchanged)         │
│  print_output(value, &global.output) → JSON/pretty        │
│  format_plain_* → plain text                              │
└──────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. User runs:  agentchrome examples strategies [<name>] [--json|--pretty]
2. clap parses into Command::Examples(ExamplesArgs { command: Some("strategies"), name: … })
3. main.rs dispatches to examples::execute_examples(&global, &args)
4. Dispatcher sees command == Some("strategies") → enters strategy branch
5. If args.name is None:    (listing path \u2014 lightweight per tech.md Progressive Disclosure)
     - Plain mode: format_plain_strategy_list(&strategy_summaries()) \u2192 stdout  (one line per strategy: `<name> \u2014 <summary>`)
     - JSON mode:  print_output(&strategy_summaries(), &global.output)         (array of {name, title, summary} only)
6. If args.name is Some(name):    (detail path \u2014 full body)
     - find_strategy(name) \u2192 Option<Strategy>
     - Found:    Plain \u2192 format_plain_strategy_detail | JSON \u2192 print_output(&strategy, ...)
     - NotFound: AppError { message: "Unknown strategy: '\u2026'. Available: \u2026",
                            code: ExitCode::GeneralError } \u2192 JSON error to stderr, exit 1
```

For the bare `agentchrome examples` path (no positional), the listing includes strategies as a synthetic command-group entry so the top-level JSON shape (`Vec<CommandGroupSummary>`) is preserved (AC3, AC11). The dispatcher appends one entry:

```rust
CommandGroupSummary {
    command: "strategies".into(),
    description: "Scenario-based interaction strategy guides (iframes, overlays, SCORM, drag-and-drop)".into(),
    examples: vec![
        ExampleEntry { cmd: "agentchrome examples strategies", description: "List all strategy guides", flags: None },
        ExampleEntry { cmd: "agentchrome examples strategies iframes", description: "Show the iframe strategy guide", flags: None },
        ExampleEntry { cmd: "agentchrome examples strategies --json", description: "Machine-readable strategy listing", flags: None },
    ],
}
```

This satisfies the existing tests `each_group_has_at_least_3_examples` and `no_empty_fields` automatically.

---

## API / Interface Changes

### CLI Surface

| Invocation                                              | Behavior                                              |
|---------------------------------------------------------|-------------------------------------------------------|
| `agentchrome examples`                                  | Top-level listing; includes synthetic `strategies` entry (unchanged existing groups) |
| `agentchrome examples strategies`                       | **Listing** (progressive disclosure): plain-text lines `<name> \u2014 <summary>` for all 10 strategies \u2014 no sectioned detail |
| `agentchrome examples strategies <name>`                | **Detail** (full body): sectioned plain-text guide for one strategy |
| `agentchrome examples strategies --json` / `--pretty`   | JSON array of `StrategySummary` objects (`{name, title, summary}`) \u2014 summary-only per tech.md Progressive Disclosure rule |
| `agentchrome examples strategies <name> --json` / `--pretty` | JSON object with full `Strategy` schema for one strategy |
| `agentchrome examples strategies <unknown>`             | JSON error on stderr (exit 1), listing available names |
| `agentchrome examples <existing-group>`                 | Unchanged (AC11)                                      |
| `agentchrome examples <unknown>`                        | Unchanged error behavior (AC11)                       |

Aligned with the new `tech.md` **Progressive Disclosure for Listings** steering principle (listing \u2192 summary-only; detail \u2192 full body) and the **Clap Help Entries** steering principle (the `Examples` variant gains updated `long_about` + `after_long_help` covering both listing and detail paths, including at least one `--json` example; the new `name` positional carries a doc comment that points to `agentchrome examples strategies` as the source of valid names).

### Clap type changes

```rust
// src/cli/mod.rs (existing ExamplesArgs, modified)

/// Arguments for the `examples` subcommand.
#[derive(Args)]
pub struct ExamplesArgs {
    /// Command group to show examples for (e.g., navigate, tabs, page),
    /// or the literal "strategies" to access scenario-based interaction guides.
    pub command: Option<String>,

    /// When `command` is "strategies", the strategy name to show
    /// (e.g., iframes, overlays, scorm, drag-and-drop).
    pub name: Option<String>,
}
```

The `long_about` / `after_long_help` on the `Examples` variant in the `Command` enum (`src/cli/mod.rs` around line 644–661) is updated to document strategies usage:

```rust
/// Show usage examples for commands
#[command(
    long_about = "Show usage examples for agentchrome commands. Without arguments, lists all \
        command groups with a brief description and one example each. With a command name, \
        shows detailed examples for that specific command group. With \"strategies\" as the \
        first positional, shows scenario-based interaction strategy guides.",
    after_long_help = "\
EXAMPLES:
  # List all command groups with summary examples
  agentchrome examples

  # Show detailed examples for the navigate command
  agentchrome examples navigate

  # List all interaction strategy guides
  agentchrome examples strategies

  # Show the iframe strategy guide
  agentchrome examples strategies iframes

  # Get all strategies as JSON (for programmatic use)
  agentchrome examples strategies --json

  # Pretty-printed JSON output
  agentchrome examples --pretty"
)]
Examples(ExamplesArgs),
```

### New types (`src/examples/strategies.rs`)

Two serializable types \u2014 one lightweight for listings, one full for details. This is the direct implementation of the `tech.md` Progressive Disclosure for Listings principle:

```rust
use serde::Serialize;

/// Lightweight listing shape \u2014 returned by `examples strategies [--json]`.
/// Progressive disclosure: three fields only, ~100\u2013200 bytes per entry.
#[derive(Serialize, Clone)]
pub struct StrategySummary {
    pub name: String,     // kebab-case, e.g. "iframes"
    pub title: String,    // "Working with iframes"
    pub summary: String,  // one-line
}

/// Full strategy shape \u2014 returned only by `examples strategies <name> [--json]`.
#[derive(Serialize, Clone)]
pub struct Strategy {
    pub name: String,
    pub title: String,
    pub summary: String,
    pub scenarios: Vec<String>,
    pub capabilities: Vec<String>,
    pub limitations: Vec<String>,
    pub workarounds: Vec<Workaround>,
    pub recommended_sequence: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct Workaround {
    pub description: String,
    pub commands: Vec<String>,
}

pub fn all_strategies() -> Vec<Strategy> {
    // 10 guides at launch (see Overview table):
    //   iframes, overlays, scorm, drag-and-drop, shadow-dom,
    //   spa-navigation-waits, react-controlled-inputs,
    //   debugging-failed-interactions, authentication-cookie-reuse,
    //   multi-tab-workflows
}

/// Cheap listing: map `all_strategies()` to summary form.
/// Does NOT allocate full bodies in the output \u2014 just `.iter().map(|s| StrategySummary { \u2026 })`.
pub fn strategy_summaries() -> Vec<StrategySummary> { /* ... */ }

/// Detail lookup.
pub fn find_strategy(name: &str) -> Option<Strategy> { /* linear scan */ }
```

All fields serialize with default Serde naming (snake_case \u2014 matches existing `CommandGroupSummary` field naming).

**Memory note**: `all_strategies()` builds the full `Vec<Strategy>` in memory; `strategy_summaries()` projects it to summaries. Because data is compile-time static and tiny (~20\u201330 KB total), holding both shapes transiently costs nothing measurable. A future optimization could memoize via `OnceLock`, but is not required for the 50ms budget.

### Request / Response Schemas

#### `agentchrome examples strategies --json` (listing \u2014 progressive disclosure)

**Output (success):** 10-element array of `StrategySummary`. Each entry contains **exactly three fields**:
```json
[
  {"name": "iframes",                         "title": "Working with iframes",              "summary": "Target and interact with elements inside iframes and frames"},
  {"name": "overlays",                        "title": "Handling overlays",                 "summary": "Detect, dismiss, and bypass full-viewport overlays and acc-blockers"},
  {"name": "scorm",                           "title": "Automating SCORM / LMS players",    "summary": "Drive SCORM courses: iframes, media gates, navigation buttons"},
  {"name": "drag-and-drop",                   "title": "Drag-and-drop interactions",        "summary": "Coordinate drags, decomposed mousedown/mouseup, step interpolation"},
  {"name": "shadow-dom",                      "title": "Piercing shadow DOM",               "summary": "Target elements inside shadow roots with --pierce-shadow"},
  {"name": "spa-navigation-waits",            "title": "SPA navigation waits",              "summary": "Wait for SPA/async rendering via --wait-until and polling"},
  {"name": "react-controlled-inputs",         "title": "Filling React / controlled inputs", "summary": "When form fill works vs needing js exec for controlled fields"},
  {"name": "debugging-failed-interactions",   "title": "Debugging failed interactions",     "summary": "Meta-workflow: diagnose \u2192 hittest \u2192 coords \u2192 console \u2192 network"},
  {"name": "authentication-cookie-reuse",     "title": "Reusing authentication via cookies","summary": "Persist and replay session cookies across agentchrome invocations"},
  {"name": "multi-tab-workflows",             "title": "Multi-tab workflows",               "summary": "Handle SSO-style new-tab flows and coordinate across tabs"}
]
```

Total payload: ~1.5 KB (well under the 4 KB listing budget).

#### `agentchrome examples strategies iframes --json` (detail \u2014 full body)

**Output (success):** Single full `Strategy` object.
```json
{
    "name": "iframes",
    "title": "Working with iframes",
    "summary": "Target and interact with elements inside iframes and frames",
    "scenarios": [
      "A SCORM course is embedded in an iframe",
      "A cross-origin payment widget is rendered as an iframe",
      "Content is lazy-loaded into a frame after navigation"
    ],
    "capabilities": [
      "agentchrome page frames — enumerate frames by index",
      "agentchrome page snapshot --frame N — accessibility tree of a specific frame",
      "agentchrome interact --frame N click <uid> — click inside a frame",
      "agentchrome dom --frame N select <selector> — query DOM inside a frame"
    ],
    "limitations": [
      "Cross-origin frames expose only URL and dimensions; interactive element counts are null",
      "Directly piercing into nested frames requires a separate --frame call per level"
    ],
    "workarounds": [
      {
        "description": "Read text from a cross-origin frame via js exec against the frame",
        "commands": [
          "agentchrome js --frame 1 exec \"document.title\""
        ]
      }
    ],
    "recommended_sequence": [
      "agentchrome page frames",
      "agentchrome page snapshot --frame 1",
      "agentchrome interact --frame 1 click s3"
    ]
}
```

Other strategies are looked up the same way: `agentchrome examples strategies overlays --json`, `\u2026 shadow-dom --json`, etc.

**Errors:**

| Code / Type                       | Condition                                       |
|-----------------------------------|-------------------------------------------------|
| exit 1, JSON on stderr            | Unknown strategy name; message includes the invalid name and lists available names |

Example error payload:
```json
{"error": "Unknown strategy: 'nonexistent-strategy'. Available: iframes, overlays, scorm, drag-and-drop", "code": 1}
```

---

## Database / Storage Changes

**None.** Strategy content is compile-time static Rust data. No persistence, no migration, no schema changes.

---

## State Management

**None.** The command is stateless — no session file reads, no CDP connection, no global mutation. Per invocation, the dispatcher builds the static data, formats it, writes to stdout, and returns.

---

## UI Components

### Plain-text detail format (`agentchrome examples strategies iframes`)

```
iframes \u{2014} Working with iframes

SCENARIOS
  - A SCORM course is embedded in an iframe
  - A cross-origin payment widget is rendered as an iframe
  - Content is lazy-loaded into a frame after navigation

CURRENT CAPABILITIES
  agentchrome page frames
    Enumerate frames by index
  agentchrome page snapshot --frame N
    Accessibility tree of a specific frame
  agentchrome interact --frame N click <uid>
    Click inside a frame

LIMITATIONS
  - Cross-origin frames expose only URL and dimensions; interactive element counts are null
  - Directly piercing into nested frames requires a separate --frame call per level

WORKAROUNDS
  # Read text from a cross-origin frame via js exec against the frame
  agentchrome js --frame 1 exec "document.title"

RECOMMENDED SEQUENCE
  1. agentchrome page frames
  2. agentchrome page snapshot --frame 1
  3. agentchrome interact --frame 1 click s3
```

Uses `std::fmt::Write` via `write!`/`writeln!` into a `String`, matching existing `format_plain_summary` and `format_plain_detail` style in `examples.rs`.

### New code files

| File                          | Role                                                                 |
|-------------------------------|----------------------------------------------------------------------|
| `src/examples/mod.rs`         | Re-exports + `execute_examples` dispatcher + shared types + tests    |
| `src/examples/commands.rs`    | Existing `all_examples()` + `format_plain_summary`/`_detail` (moved) |
| `src/examples/strategies.rs`  | `Strategy`, `Workaround`, `all_strategies()`, formatters             |
| `tests/features/examples-strategies.feature` | BDD scenarios for AC1–AC12                              |

Deleted: `src/examples.rs` (content moved into submodule).

### Component hierarchy (module re-export)

```
src/examples/
├── mod.rs              pub use commands::*; pub use strategies::{Strategy, ...};
│                       pub fn execute_examples(...)                        ← dispatcher
│                       fn format_plain_summary_with_strategies(...)        ← adds synthetic entry
├── commands.rs          pub struct CommandGroupSummary { ... }
│                       pub struct ExampleEntry { ... }
│                       pub fn all_examples() -> Vec<CommandGroupSummary>
│                       pub(super) fn format_plain_summary(groups) -> String
│                       pub(super) fn format_plain_detail(group) -> String
└── strategies.rs        pub struct Strategy { ... }
                        pub struct Workaround { ... }
                        pub fn all_strategies() -> Vec<Strategy>
                        pub(super) fn format_plain_strategy_list(&[Strategy]) -> String
                        pub(super) fn format_plain_strategy_detail(&Strategy) -> String
```

Only `main.rs`'s current `use crate::examples::execute_examples;` entry point and the `mod examples;` declaration change; every existing unit test moves with its code into `commands.rs` unchanged.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Flat second positional (`examples strategies [<name>]`)** | Add `name: Option<String>` to `ExamplesArgs`; dispatcher branches on `command == "strategies"`. | Minimal CLI surface change; zero risk of breaking existing `examples <group>` ergonomics; matches issue description literally. | Slight "magic string" in dispatcher for `"strategies"`. | **Selected** |
| B: clap subcommand enum inside `ExamplesArgs` | Replace `command: Option<String>` with a `#[command(subcommand)] sub: Option<ExamplesSub>` enum with variants `Group(GroupArgs)` and `Strategies(StrategyArgs)`. | Cleaner domain modeling; per-variant `--help`. | Clap's derive mode requires the subcommand keyword on the CLI, breaking the current bare-positional pattern `examples navigate`. Would force `examples group navigate` or complex `disable_help_subcommand` hacks. | Rejected — breaks AC11 (no regression in existing invocations) |
| C: Separate top-level `strategies` subcommand | Add `agentchrome strategies ...` as a sibling of `examples`. | Unambiguous CLI. | Issue explicitly asks for it under `examples`; user discoverability from `examples` listing is lost; two near-duplicate subcommand implementations. | Rejected — diverges from issue and loses grouping |
| D: External data file (TOML/JSON) shipped with binary | Keep strategy content in a checked-in data file; parse at runtime. | Non-Rust contributors can edit. | Adds runtime I/O and a failure mode; violates the 50ms perf target risk; current `examples` content is already in-Rust data. | Rejected — inconsistent with existing pattern |
| E: Keep everything in `src/examples.rs` (no submodule split) | Append strategy types and data to the existing file. | Smallest diff. | File already at ~1000 lines; strategy content adds several hundred; limits future strategy additions (FR16). | Rejected — poor scalability |

---

## Security Considerations

- [x] **Authentication**: N/A — no network or CDP access.
- [x] **Authorization**: N/A.
- [x] **Input Validation**: Strategy name lookup is an exact match against a static allow-list; no path traversal, SQL, or shell interpolation.
- [x] **Data Sanitization**: N/A — all data is compile-time static; no user-controlled content reaches output.
- [x] **Sensitive Data**: None — strategy text is documentation.
- [x] **Command injection**: Strategy `recommended_sequence` and `workarounds[].commands` are strings printed verbatim for copy-paste; they are never `exec`'d by agentchrome itself. User remains responsible for running them.

---

## Performance Considerations

- [x] **Caching**: N/A — everything is static data, compiled in.
- [x] **Pagination**: Not needed — strategy count is small (4 at launch; well under 100).
- [x] **Lazy Loading**: Strategy `Vec` is built per-invocation (same pattern as `all_examples()`); impact is negligible (< 1ms) relative to the 50ms startup budget.
- [x] **Indexing**: Strategy lookup is linear scan on a Vec of ~10 entries; no index needed.
- [x] **Binary size**: Each strategy guide is ~1–3 KB of string data. Ten guides add ~20–30 KB, well under the 100 KB informal target in the requirements NFRs and the 10 MB binary-size target in `tech.md`.

---

## Testing Strategy

| Layer             | Type              | Coverage |
|-------------------|-------------------|----------|
| `strategies.rs`   | Unit              | `all_strategies()` returns the ten required guides (AC8); all fields non-empty; no duplicate `name`; every `recommended_sequence` command starts with `agentchrome`; `name` is kebab-case and serializes to snake_case JSON |
| `strategies.rs`   | Unit              | `format_plain_strategy_list` contains all ten strategy names and summaries; output does not start with `[` or `{` (AC1); total length under 1 KB (progressive disclosure budget) |
| `strategies.rs`   | Unit              | `format_plain_strategy_detail` contains all required sections (Scenarios, Current Capabilities, Limitations, Workarounds, Recommended Sequence) for each of the ten launch strategies |
| `strategies.rs`   | Unit (progressive disclosure guard) | Serializing `strategy_summaries()` produces JSON that does NOT contain the detail field names (`scenarios`, `capabilities`, `limitations`, `workarounds`, `recommended_sequence`) \u2014 guards against accidental leak of full bodies into the listing path |
| `strategies.rs`   | Unit              | Serializing `find_strategy("iframes").unwrap()` produces JSON that DOES contain all detail field names |
| `cli/mod.rs`      | Unit              | `agentchrome examples strategies` and `agentchrome examples strategies iframes` both parse successfully; `ExamplesArgs { command: Some("strategies"), name: None }` and `{ command: Some("strategies"), name: Some("iframes") }` respectively |
| `cli/mod.rs`      | Unit (clap help steering) | `Command::Examples` variant has non-empty `long_about` and `after_long_help`; `after_long_help` contains the literal substring `examples strategies` and at least one `--json` example (enforces tech.md Clap Help Entries principle for this surface) |
| `examples/mod.rs` | Unit              | Dispatcher routing: `("strategies", None)` → list, `("strategies", Some("iframes"))` → detail, `("strategies", Some("bogus"))` → error, `(None, _)` → top-level listing includes a "strategies" entry, `(Some("navigate"), _)` → existing behavior (ignores `name`), `(Some("bogus"), _)` → existing error |
| `examples/mod.rs` | Unit              | JSON serialization: `all_strategies()` serializes to valid JSON with the expected field shape and snake_case keys (AC4); flags field is absent when None in existing `ExampleEntry` (pre-existing, preserved) |
| CLI integration   | Unit (in `cli/mod.rs` tests) | `agentchrome examples strategies` and `agentchrome examples strategies iframes` both parse successfully into `ExamplesArgs { command: Some("strategies"), name: … }` |
| Feature           | Integration (BDD) | `tests/features/examples-strategies.feature` covers AC1–AC12; the existing `tests/features/examples.feature` remains passing (AC11) |
| Cross-platform    | CI                | Existing CI matrix (macOS + Linux + Windows); no platform-specific code introduced |
| Smoke             | Manual            | Not applicable — no Chrome dependency. A `cargo run -- examples strategies iframes` sanity check suffices and is added as the final task before "Verify No Regressions" |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Adding a second positional to `ExamplesArgs` changes clap's parsing for `agentchrome examples <unknown>` (now `<unknown>` gets parsed as `command` and nothing as `name`, same as before) | Low | Low | Existing tests (`execute_examples_unknown_command_returns_error`, `error_message_lists_all_available_groups`) are re-run; add explicit parse tests in `cli/mod.rs` for the new shape |
| Strategy guide content becomes stale when new features ship (e.g., iframe `--frame` arrives and the iframes guide still calls it a workaround) | Medium | Medium | FR10 requires guide updates when related features ship; add `strategies.rs` to the list of files touched in the iframe-frame-targeting spec's "Documentation and examples updated" AC. Not a blocker for this feature's initial merge. |
| "strategies" becomes a future user-chosen command group name, creating a collision | Low | Medium | FR14 asserts no current collision. A unit test in `examples/mod.rs` asserts that the static string `"strategies"` does not appear in `all_examples()` as a `command` value. |
| Submodule split triggers large rename diff that obscures content review | Low | Low | Split is mechanical (pure move of existing code); done as a dedicated task (T001) in tasks.md so reviewers can separate "move" from "add" |
| Top-level `examples` listing gains an entry whose description is lengthy and breaks visual alignment in the plain-text table | Low | Low | Keep the `strategies` entry's `description` under 80 chars; verified by a unit test |
| BDD scenarios for strategies create flaky tests because they depend on exact text substrings | Low | Medium | Use `stdout should contain` checks for stable tokens (strategy names, key section headers) rather than full-line matches; keep scenario wording aligned with existing `examples.feature` idioms |

---

## Open Questions

- [ ] Should the plain-text detail format use ANSI color (when stdout is a TTY and `NO_COLOR` is unset) to visually separate section headers? **Proposed answer**: No for the initial implementation — the existing `format_plain_detail` in `commands.rs` uses plain text with `#` comment markers and no color, and matching that style avoids introducing a color layer that does not exist elsewhere in `examples`. Color can be added as a follow-on enhancement if user feedback warrants it.
- [x] ~~Should additional strategies ship at launch?~~ **Resolved 2026-04-16**: launch set expanded to ten guides (see Overview table and requirements FR5–FR5f / AC8). The data structure still supports future additions without refactoring (FR16).

---

## Change History

| Issue | Date       | Summary                                                                                       |
|-------|------------|-----------------------------------------------------------------------------------------------|
| #201  | 2026-04-16 | Initial feature design                                                                        |
| #201  | 2026-04-16 | Expanded launch strategy set from 4 to 10 guides (sync with requirements update)              |
| #218  | 2026-04-21 | Progressive Disclosure retrofit for `examples` and `capabilities` listings (see § "Progressive Disclosure Retrofit — Added by #218") |

---

## Progressive Disclosure Retrofit — Added by #218

### Summary

Issue #218 retrofits the two remaining non-compliant listing commands to conform to the `tech.md` **Progressive Disclosure for Listings** principle. The rule's grandfather exemption was removed during #201 review, so the originally out-of-scope retrofit is now the scope of this follow-on issue. Behavior for every other listing command in the CLI is unchanged.

Affected surfaces:

| Command | Before | After |
|---------|--------|-------|
| `agentchrome examples --json` | `Vec<CommandGroupSummary>` where each entry carries nested `examples: Vec<ExampleEntry>` (~3–5 KB) | Array of `{command, description}` only (~1 KB); `examples` array removed from listing entries |
| `agentchrome examples <group> --json` | Single `CommandGroupSummary` with nested `examples` array | Unchanged (detail path already exists; reused) |
| `agentchrome capabilities --json` | Monolithic `CapabilitiesManifest` with `commands: Vec<CommandDescriptor>` carrying nested `subcommands`/`args`/`flags` (~8–15 KB) | `CapabilitiesManifest` where each entry in `commands` is a `{name, description}` summary (~1 KB total) |
| `agentchrome capabilities <command> --json` | Does not exist | New positional returning full `CommandDescriptor` with `subcommands`, `args`, `flags` |
| `agentchrome capabilities <unknown>` | N/A | JSON error on stderr; exit 1 |

### API / Type Changes

#### New listing types (serialization split — mirrors the `StrategySummary` / `Strategy` pattern from #201)

```rust
// src/examples/commands.rs — added by #218
#[derive(Serialize, Clone)]
pub struct CommandGroupListing {
    pub command: String,
    pub description: String,
}

impl From<&CommandGroupSummary> for CommandGroupListing { /* project `command` + `description` */ }

// src/capabilities.rs — added by #218
#[derive(Serialize, Clone)]
pub struct CommandListing {
    pub name: String,
    pub description: String,
}

#[derive(Serialize, Clone)]
pub struct CapabilitiesManifestListing {
    pub name: String,
    pub version: String,
    pub commands: Vec<CommandListing>,
    pub global_flags: Vec<FlagDescriptor>,
    pub exit_codes: Vec<ExitCodeDescriptor>,
}

impl From<&CapabilitiesManifest> for CapabilitiesManifestListing { /* summary projection */ }
```

Rationale for two-type split rather than `#[serde(skip_serializing_if)]` on the existing types: a guard unit test (FR26) asserts the listing JSON does not contain detail field names at all. Skipping fields at runtime leaves the type surface capable of serializing detail data into the listing path, which is exactly the regression the guard test prevents. Two-type split gives a compile-time guarantee that listing-shaped output cannot carry detail fields.

#### Clap change — new `Capabilities` positional

```rust
// src/cli/mod.rs — CapabilitiesArgs, modified by #218
pub struct CapabilitiesArgs {
    /// When present, return the full descriptor for the named command;
    /// when absent, return the summaries-only listing.
    /// Valid names: run `agentchrome capabilities` to list.
    pub command: Option<String>,
    // existing fields (compact, etc.) unchanged
}
```

The `Capabilities` variant gains an updated `long_about` + `after_long_help` covering both listing and detail paths with at least one `--json` example (FR22).

#### Dispatcher changes

```rust
// src/examples/mod.rs — execute_examples bare-listing branch, modified by #218
None => {
    let groups = all_examples_with_synthetic_strategies();
    let listing: Vec<CommandGroupListing> = groups.iter().map(Into::into).collect();
    // Plain path unchanged (summary lines already use command + description only).
    // JSON path now prints `&listing` instead of `&groups`.
}

// src/capabilities.rs — execute_capabilities, modified by #218
match args.command {
    None => {
        let manifest = build_manifest(&root_cmd, args.compact);
        let listing = CapabilitiesManifestListing::from(&manifest);
        print_output(&listing, &global.output)
    }
    Some(ref name) => {
        let manifest = build_manifest(&root_cmd, args.compact);
        match manifest.commands.iter().find(|c| c.name == *name) {
            Some(descriptor) => print_output(descriptor, &global.output),
            None => Err(AppError::general(format!(
                "Unknown command: '{name}'. Available: {}",
                manifest.commands.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(", ")
            ))),
        }
    }
}
```

The existing `--command <name>` flag path (already in `capabilities.rs` around lines 436 and 584, where it filters to matching commands) is either reused by or removed in favor of the new positional — tasks.md T020 decides which based on whether any existing caller relies on the flag form. Default assumption: keep the flag as a hidden alias for one release, log a deprecation warning, remove in the next major.

### Plain-text output for `examples` listing

The plain-text top-level `examples` listing already prints one line per group (`command — description`) without the `examples` array, so the human-readable path is already summary-shaped. The retrofit is a JSON-only change for that surface. For `capabilities`, the plain-text path currently shows the full manifest; the retrofit trims it to one-line-per-command, matching the existing `examples` summary style.

### Output sizes

| Surface | Current | Target |
|---------|---------|--------|
| `examples --json` | 3–5 KB | < 1.5 KB |
| `capabilities --json` | 8–15 KB | < 2 KB |
| `capabilities <command> --json` | N/A (new) | 1–3 KB per command (detail) |

All comfortably under the 4 KB listing budget from the steering rule.

### Breaking change & migration

The listing-shape change is a CLI contract break. Per the issue, the user chose the breaking path over an opt-in `--full` flag. Mitigation:

- `CHANGELOG.md` entry under the next release heading explicitly labels the change as breaking and names the new detail path for callers that need full bodies (FR25).
- The existing `examples.feature` BDD scenario asserting `examples` array presence on the listing (AC11d) is retired in favor of a new scenario asserting the summary shape (AC19).
- No in-code deprecation shim — v1 pre-GA, no promised backward compatibility.

### Alternatives Considered (added by #218)

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **F: Two-type split (listing type vs. detail type)** | `CommandGroupListing` / `CommandListing` separate from detail types; dispatcher serializes the listing type. | Compile-time guarantee the listing path cannot leak detail fields; mirrors #201's `StrategySummary` / `Strategy` pattern; unit test (FR26) becomes a compile-time property. | Two types per surface. | **Selected** |
| G: `#[serde(skip_serializing_if)]` on nested fields | Keep one type; skip `examples`/`subcommands` when serializing via a listing flag. | Fewer types. | Serialization context leaks into the type's invariants; the guard test becomes a runtime check rather than a structural one; regression-prone. | Rejected |
| H: `--full` opt-in flag | Listing stays as-is; `--full` flag returns detail bodies. | Zero breaking change. | User explicitly rejected this path; also continues to default into non-compliance. | Rejected (issue directive) |

### Security Considerations (added by #218)

- Command-name lookup in `capabilities <name>` is an exact match against a static allow-list derived from `build_manifest` — no path traversal, shell interpolation, or dynamic dispatch.
- No new authentication / authorization surface.

### Performance Considerations (added by #218)

- Summary projection is O(n) over a ~17-command manifest; negligible relative to the existing 50ms budget.
- `build_manifest` is still called once per invocation (same as today); summary projection is a cheap `.iter().map(Into::into).collect()` pass.

### Testing Strategy (added by #218)

| Layer | Type | Coverage |
|-------|------|----------|
| `examples/commands.rs` | Unit | Serializing `CommandGroupListing` produces JSON whose keys are exactly `{command, description}` — no `examples` key (FR26) |
| `capabilities.rs` | Unit | Serializing `CommandListing` produces JSON whose keys are exactly `{name, description}` — no `subcommands`/`args`/`flags` (FR26) |
| `capabilities.rs` | Unit | `find_command` returns `None` for unknown, `Some(CommandDescriptor)` for known |
| `capabilities.rs` | Unit (clap help steering) | `Command::Capabilities` variant carries `long_about` + `after_long_help` mentioning `capabilities <command>` and at least one `--json` example (mirrors T015 from #201) |
| Feature | Integration (BDD) | `tests/features/examples.feature` updated: existing "each entry has examples array" scenario is replaced by summary-shape scenario (AC13); `tests/features/capabilities.feature` extended with scenarios for AC15–AC17 |
| Manual smoke | Verify | Sizes measured: `examples --json | wc -c`, `capabilities --json | wc -c` both under 4096 |

### Risks & Mitigations (added by #218)

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| New `capabilities <command>` positional collides with an existing flag / arg on `Capabilities` (per retrospective on arg-name collisions) | Medium | Medium | Task T020 greps existing `CapabilitiesArgs` for field-name collisions before adding; an existing `--command <name>` flag likely exists and must be reconciled (either removed or reshaped into the positional) |
| Shell completions / man pages stale after clap change | Medium | Low | FR22 / AC18 require man page + completion check; verification task runs `cargo xtask man capabilities` |
| Downstream agents break when listing shape changes | Medium | Medium | Explicitly chosen breaking change; `CHANGELOG.md` breaking-change entry (FR25); announced via release notes; new `capabilities <command>` detail path documented in the same entry |
| `--command` flag form already exists and some consumer uses it | Low | Medium | Keep the flag as a hidden alias in one release, print a stderr deprecation warning, remove in the next major |
| Listing-size measurement varies with binary-name or command metadata drift | Low | Low | Under-4 KB assertion has comfortable headroom (target < 2 KB for capabilities, < 1.5 KB for examples) |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Architecture follows existing project patterns (per `structure.md` — command module layer, JSON output via `print_output`, plain formatting via `std::fmt::Write`)
- [x] All API/interface changes documented with schemas (`Strategy`, `Workaround`, CLI positional addition)
- [x] Database/storage changes planned with migrations (N/A — documented)
- [x] State management approach is clear (stateless, documented)
- [x] UI components and hierarchy defined (module layout + plain-text format mockup)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented (five options evaluated)
- [x] Risks identified with mitigations
