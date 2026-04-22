# Requirements: Interaction Strategy Guide in Examples Command

**Issues**: #201, #218
**Date**: 2026-04-21
**Status**: Amended
**Author**: Claude (spec-writer)

---

## User Story

**As a** browser automation engineer starting complex automation work
**I want** scenario-based interaction strategy guides in the `examples` command
**So that** I can understand the right approach for common enterprise automation patterns before I start, instead of discovering limitations through trial and error

---

## Background

The current `agentchrome examples` command organizes its content by **command** (`examples navigate`, `examples interact`, `examples page`, …). That shape answers "what can command X do?" but not "how should I approach scenario Y?". Engineers automating enterprise applications repeatedly hit the same classes of problems — iframes, overlays, SCORM/LMS players, drag-and-drop — and need **scenario-first** guidance rather than per-command recipes. One user reported that knowing the iframe limitation upfront would have saved 30+ failed attempts: they would have gone directly to the accessibility shadow DOM workaround instead of trying coordinate-based clicking.

This feature adds a `strategies` section under the existing `examples` subcommand. Each strategy is a named, scenario-based guide that describes the problem, the current capabilities and limitations, recommended command sequences (including which existing agentchrome subcommands to chain), and known workarounds. The content lives alongside the existing command-based examples in `src/examples.rs`, reuses the same output contract (plain by default, JSON via `--json`, pretty JSON via `--pretty`, JSON errors on stderr), and is shipped as part of the same single binary.

Strategy guides are **living documentation**: when new features ship (e.g., the forthcoming iframe `--frame` targeting), the corresponding strategy guide is updated so the recommendation uses the new capability instead of a workaround.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Strategy guide listing (plain text) \u2014 progressive disclosure

**Given** the agentchrome binary is available
**When** `agentchrome examples strategies` is run
**Then** a human-readable listing of all available strategy guides is printed to stdout
**And** the listing contains at minimum the ten launch strategies: `iframes`, `overlays`, `scorm`, `drag-and-drop`, `shadow-dom`, `spa-navigation-waits`, `react-controlled-inputs`, `debugging-failed-interactions`, `authentication-cookie-reuse`, `multi-tab-workflows`
**And** each listed strategy shows only its **name** and a **one-line summary** \u2014 the listing does NOT include scenarios, capabilities, limitations, workarounds, or the recommended command sequence
**And** the total plain-text output is under ~1 KB (progressive disclosure: full content is only loaded when a specific strategy is requested via AC2)
**And** the command exits with exit code 0
**And** stdout does not start with `[` or `{` (plain text, not JSON)

**Example**:
- Given: `agentchrome --version` works
- When: `agentchrome examples strategies`
- Then: stdout contains lines like `iframes \u2014 Target and interact with elements inside iframes and frames` (one line per strategy, 10 lines total); it does NOT contain sections like "CURRENT CAPABILITIES" or "RECOMMENDED SEQUENCE"

### AC2: Individual strategy guide (plain text) \u2014 the "reveal" step

**Given** a strategy named `iframes` exists in the strategy database
**When** `agentchrome examples strategies iframes` is run
**Then** the full iframe strategy guide is printed to stdout in plain text (this is the expensive payload; progressive-disclosure contract: full body loads only on explicit strategy selection)
**And** the guide contains all of the following sections:
- A scenario description (when to use this strategy)
- A "Current capabilities" section referencing concrete agentchrome commands (e.g., `page frames`, `page snapshot --frame N`, `interact --frame N click`)
- A "Limitations" section describing what does NOT currently work
- A "Workarounds" section with at least one runnable command sequence for cross-frame interaction (e.g., using `js exec` to access frame content when direct targeting is unavailable)
- A "Recommended command sequence" section showing a numbered, copy-pasteable sequence of agentchrome invocations
**And** the command exits with exit code 0

### AC3: Strategy guide in top-level examples listing

**Given** the agentchrome binary is available
**When** `agentchrome examples` is run with no arguments
**Then** the top-level listing contains a distinct "strategies" entry alongside the existing command groups (connect, tabs, navigate, page, …)
**And** the "strategies" entry has a one-line description that identifies it as scenario-based guidance
**And** existing command group entries (connect, tabs, navigate, page, dom, js, console, network, interact, form, emulate, perf, dialog, skill, media, config, diagnose) remain present and unchanged in content
**And** the command exits with exit code 0

### AC4: Machine-readable strategy listing — progressive disclosure (`--json`)

**Given** the agentchrome binary is available
**When** `agentchrome examples strategies --json` is run
**Then** stdout is a valid JSON array of **summary** objects (not full guides)
**And** each element has **exactly** these three fields: `name` (kebab-case identifier), `title` (human-readable title), `summary` (one-line description) \u2014 no `scenarios`, `capabilities`, `limitations`, `workarounds`, or `recommended_sequence` fields are present in listing output
**And** the total JSON payload for the listing is under 4 KB (keeps agent context cost bounded: roughly 10 strategies \u00d7 ~150 bytes each)
**And** stderr is empty
**And** the command exits with exit code 0

**Rationale**: Agents call `examples strategies --json` for discovery \u2014 to decide which strategy is relevant. Returning full guide bodies for all strategies in the listing would force every agent to consume ~20\u201330 KB of context just to learn which strategies exist. Progressive disclosure keeps the listing cheap and defers full content to the individual-strategy call.

### AC5: Machine-readable individual strategy \u2014 full detail (`--json`)

**Given** a strategy named `iframes` exists
**When** `agentchrome examples strategies iframes --json` is run
**Then** stdout is a valid JSON object (not an array) with the **full** strategy schema: `name`, `title`, `summary`, `scenarios`, `capabilities`, `limitations`, `workarounds`, `recommended_sequence`
**And** the `name` field equals `"iframes"`
**And** the `recommended_sequence` array is non-empty
**And** the command exits with exit code 0

**Rationale**: The full body loads only when the user or agent has already selected a specific strategy from the listing \u2014 the "reveal" step of progressive disclosure.

### AC6: Pretty-printed JSON (`--pretty`)

**Given** the agentchrome binary is available
**When** `agentchrome examples strategies --pretty` is run
**Then** stdout is valid, multi-line, indented JSON
**And** the command exits with exit code 0

### AC7: Unknown strategy name is an error

**Given** no strategy named `nonexistent-strategy` exists
**When** `agentchrome examples strategies nonexistent-strategy` is run
**Then** a JSON error object is written to stderr containing the message `"Unknown strategy"` and listing the available strategy names
**And** stdout is empty
**And** the command exits with a non-zero exit code (GeneralError, exit code 1)

**Rationale**: This AC enforces the project-wide output contract — JSON errors on stderr, exactly one error object per invocation — for the new `strategies` path, matching the behavior already required for unknown command groups (see `examples.rs` tests `execute_examples_unknown_command_returns_error`).

### AC8: Required strategy guides exist

**Given** the agentchrome binary is available
**When** `agentchrome examples strategies --json` is run
**Then** the output array contains entries with `name` fields equal to **each** of the following ten launch strategies:
- `"iframes"` — working with iframes and cross-frame interaction
- `"overlays"` — detecting and handling full-viewport overlays, modals, acc-blockers
- `"scorm"` — automating SCORM / LMS course players
- `"drag-and-drop"` — coordinate drag interactions and decomposed mousedown/mouseup
- `"shadow-dom"` — piercing shadow roots for web-component UIs
- `"spa-navigation-waits"` — handling SPA async rendering via `--wait-until` and polling
- `"react-controlled-inputs"` — filling React/Vue controlled inputs and ARIA comboboxes
- `"debugging-failed-interactions"` — meta-workflow for diagnosing stuck automations
- `"authentication-cookie-reuse"` — persisting and reusing auth via the `cookie` commands
- `"multi-tab-workflows"` — SSO-style flows that open new tabs; cross-tab coordination

**And** each of those entries has a non-empty `recommended_sequence`
**And** each of those entries references only currently-shipped agentchrome commands in its `recommended_sequence` and `workarounds[].commands` (per FR15)

### AC9: Help documentation references strategies

**Given** the agentchrome binary is available
**When** `agentchrome examples --help` is run
**Then** the help output (short form) mentions that strategy guides are available via `examples strategies`
**And** the long-help (`--help` when the subcommand uses clap `long_about` / `after_long_help`) includes at least one worked example of `agentchrome examples strategies` and one of `agentchrome examples strategies <name>`

### AC10: README and man page coverage

**Given** the new strategy guide feature is installed
**When** the project's documentation surfaces are consulted
**Then** the following are updated:
- `README.md` "Usage Examples" section (or equivalent discovery section) includes at least one `examples strategies` invocation
- `agentchrome man examples` (generated by `cargo xtask man` from clap metadata) documents the strategies sub-usage once the clap definition is in place — the AC verifies the generated man page emits the new text
- `agentchrome capabilities` output reflects any new clap flags or positional arguments introduced by this feature (the capabilities manifest is clap-driven, so this is satisfied automatically once the clap definition is in place)

### AC11: No regression in existing `examples` behavior

**Given** the agentchrome binary is available
**When** any of the existing invocations are run: `agentchrome examples`, `agentchrome examples navigate`, `agentchrome examples navigate --json`, `agentchrome examples <unknown>`, `agentchrome examples --pretty`
**Then** each produces the same behavior it produced before this feature shipped (same exit codes, same top-level JSON shape for command groups, same unknown-group error message)
**And** in particular, the existing command-group names `connect`, `tabs`, `navigate`, `page`, `diagnose`, `dom`, `js`, `console`, `network`, `interact`, `form`, `emulate`, `perf`, `dialog`, `skill`, `media`, `config` are still resolvable via `agentchrome examples <name>`

### AC12: Cross-environment consistency

**Given** the agentchrome binary is available on macOS, Linux, and Windows (per the cross-platform support in `product.md`)
**When** `agentchrome examples strategies --json` is run on each platform
**Then** the output JSON is byte-for-byte identical across platforms (modulo line-ending in plain-text modes) — strategy content is static and does not depend on runtime environment

**Rationale**: Per the retrospective on environment-varying behavior, any feature that might diverge by runtime environment should have explicit cross-environment ACs. The `examples` subcommand does not touch Chrome or CDP, so it SHOULD be fully deterministic — AC12 codifies that expectation.

### AC13: `examples --json` top-level listing returns summaries only (added by #218)

**Given** the agentchrome binary is available
**When** `agentchrome examples --json` is run
**Then** stdout is a valid JSON array
**And** each entry has **exactly** these fields: `command`, `description` — the nested `examples` array is NOT present on listing entries
**And** the total JSON payload is under 4 KB
**And** the command exits with exit code 0

**Supersedes** AC11d's "each JSON entry should have an `examples` array" assertion. AC11d was written under the pre-retrofit grandfather exemption that has since been removed from `tech.md`'s Progressive Disclosure for Listings rule.

### AC14: `examples <group> --json` still returns full detail (added by #218)

**Given** the agentchrome binary is available
**When** `agentchrome examples navigate --json` is run
**Then** stdout is a valid JSON object containing `command`, `description`, and a non-empty `examples` array
**And** each entry in `examples` has `cmd` and `description` fields (optionally `flags`)
**And** the command exits with exit code 0

### AC15: `capabilities --json` listing returns summaries only (added by #218)

**Given** the agentchrome binary is available
**When** `agentchrome capabilities --json` is run
**Then** stdout is a valid JSON object with top-level fields `name`, `version`, `global_flags`, `exit_codes`, and a `commands` array
**And** each entry in `commands` has **exactly** these fields: `name`, `description` — no `subcommands`, `args`, or `flags` arrays on listing entries
**And** the total JSON payload is under 4 KB
**And** the command exits with exit code 0

### AC16: `capabilities <command> --json` returns the full descriptor (added by #218)

**Given** the agentchrome binary is available
**When** `agentchrome capabilities page --json` is run
**Then** stdout is a valid JSON object with the full `CommandDescriptor` shape: `name`, `description`, `subcommands`, `args`, `flags`
**And** the `subcommands` array is populated with each subcommand's full `args` and `flags`
**And** the command exits with exit code 0

### AC17: Unknown command in `capabilities <name>` is an error (added by #218)

**Given** no command named `nonexistent` exists
**When** `agentchrome capabilities nonexistent` is run
**Then** a JSON error object is written to stderr listing available command names
**And** stdout is empty
**And** the command exits with exit code 1

### AC18: Shell completions and man pages reflect the new `capabilities` positional (added by #218)

**Given** the retrofit is in place
**When** `cargo xtask man capabilities` (or the aggregated flow) renders the man page
**Then** the new positional `<command>` is documented
**And** `agentchrome --completions zsh` output includes completion for the new positional

### AC19: BDD tests assert progressive-disclosure compliance for both commands (added by #218)

**Given** the retrofit is in place
**When** `cargo test --test bdd` is run
**Then** `tests/features/examples.feature` contains scenarios asserting the new summary-only shape for `examples --json` and asserting detail-only fields are absent from the listing
**And** `tests/features/capabilities.feature` contains equivalent scenarios covering AC15–AC17
**And** all scenarios pass

### AC20: Breaking-change documentation (added by #218)

**Given** the retrofit is merged
**When** `CHANGELOG.md` is inspected
**Then** there is an entry under the next release heading describing the shape change to `examples --json` and `capabilities --json`, naming the new `capabilities <command>` detail path
**And** the entry explicitly labels the change as a breaking change

### Generated Gherkin Preview

```gherkin
Feature: Interaction Strategy Guide in Examples Command
  As a browser automation engineer starting complex automation work
  I want scenario-based interaction strategy guides in the examples command
  So that I can choose the right approach for common patterns upfront

  Scenario: List all strategy guides (plain text)
    Given the agentchrome binary is available
    When I run "agentchrome examples strategies"
    Then stdout should contain "iframes"
    And stdout should contain "overlays"
    And stdout should contain "scorm"
    And stdout should contain "drag-and-drop"
    And the exit code should be 0

  # ... all ACs become scenarios
```

---

## Functional Requirements

| ID   | Requirement                                                                                                                                     | Priority | Notes |
|------|-------------------------------------------------------------------------------------------------------------------------------------------------|----------|-------|
| FR1  | New `strategies` path under the `examples` subcommand (accessible via `examples strategies` and `examples strategies <name>`)                   | Must     | Positional or nested — design decides |
| FR2  | "iframes" strategy guide content                                                                                                                | Must     | Covers frame targeting (`--frame`), cross-frame limitations, `js exec` workaround |
| FR3  | "overlays" strategy guide content                                                                                                               | Must     | Covers detecting, dismissing, and bypassing full-viewport overlays (acc-blockers, modal backdrops, cookie consent) |
| FR4  | "scorm" strategy guide content                                                                                                                  | Must     | Covers SCORM / LMS player automation: iframe structure, media gates, navigation patterns |
| FR5  | "drag-and-drop" strategy guide content                                                                                                          | Must     | Covers `interact drag-at`, `mousedown-at`/`mouseup-at`, when to use `--steps`, and decomposed mouse actions |
| FR5a | "shadow-dom" strategy guide content                                                                                                             | Must     | Covers `--pierce-shadow` on `dom`/`page`/`interact`; how to discover shadow roots; when CSS selectors fail. Pairs with iframes as a "piercing" strategy |
| FR5b | "spa-navigation-waits" strategy guide content                                                                                                   | Must     | Covers `navigate --wait-until networkidle\|selector`, `interact click --wait-until`, polling with `page find`, and `page wait` (if shipped). Pain-backed by issues #144, #145, #178 |
| FR5c | "react-controlled-inputs" strategy guide content                                                                                                | Must     | Covers when `form fill` (event-based) works vs needing `js exec` to set `.value` directly; ARIA combobox with `--confirm-key`. Pain-backed by bug #161 |
| FR5d | "debugging-failed-interactions" strategy guide content                                                                                          | Must     | Meta-strategy chaining `diagnose` \u2192 `page hittest` \u2192 `page coords` \u2192 `console read` \u2192 `network list` \u2192 `page snapshot` to root-cause stuck automations |
| FR5e | "authentication-cookie-reuse" strategy guide content                                                                                            | Must     | Covers `cookie list`/`cookie set`/`cookie delete`/`cookie clear` for persisting session auth across agentchrome invocations and reusing logged-in state |
| FR5f | "multi-tab-workflows" strategy guide content                                                                                                    | Must     | Covers `tabs list`/`tabs create`/`tabs activate`/`tabs close`, handling SSO-style flows that open a new tab, and using `--tab` to target a specific tab |
| FR6  | Individual strategy access by name (e.g., `examples strategies iframes`)                                                                        | Must     | |
| FR7  | JSON output support for both strategy listing and individual strategies (`--json`, `--pretty`)                                                  | Must     | Reuses existing `OutputFormat` on `GlobalOpts` |
| FR7a | **Progressive disclosure**: the listing path (`examples strategies` and `examples strategies --json`) returns only `name`, `title`, `summary` per strategy \u2014 full guide bodies (`scenarios`, `capabilities`, `limitations`, `workarounds`, `recommended_sequence`) are returned ONLY when a specific strategy is selected via `examples strategies <name>` | Must | Keeps discovery calls cheap for agents (\u2264 4 KB JSON / \u2264 1 KB plain). Informed by the user requirement that agent context should not be flooded with 10 full guide bodies on every discovery call |
| FR8  | "strategies" appears in the top-level `examples` listing alongside existing command groups                                                      | Must     | |
| FR9  | Unknown strategy name returns a JSON error on stderr with exit code 1, listing available strategy names                                         | Must     | Mirrors existing unknown-command-group behavior |
| FR10 | Strategy guides can be updated as new features land (e.g., when iframe `--frame` ships, the iframes strategy's recommendation flips from workaround to direct use) | Should   | Implementation pattern must make this a data change, not a structural refactor |
| FR11 | Help documentation (`examples --help` / `examples --help` long-help) references strategy access, per the `tech.md` **Clap Help Entries** steering principle: short `about`, `long_about` paragraph describing the strategies path, and `after_long_help` EXAMPLES including at least one `examples strategies` and one `examples strategies <name> --json` invocation | Must | Updates `long_about` / `after_long_help` in `src/cli/mod.rs` |
| FR12 | `README.md` "Usage Examples" section includes at least one `examples strategies` invocation                                                     | Must     | |
| FR13 | BDD test scenarios covering strategy listing, individual strategies, JSON output, and error handling                                            | Must     | `tests/features/examples-strategies.feature` |
| FR14 | No new argument name collides with existing global flags or the existing `examples <command>` positional (per retrospective learning)           | Must     | "strategies" must not collide with any command group name — none of: connect, tabs, navigate, page, diagnose, dom, js, console, network, interact, form, emulate, perf, dialog, skill, media, config |
| FR15 | Strategy guide content references only agentchrome commands that actually exist at the time of writing                                          | Must     | Prevents the guide from instructing users to run commands that have not shipped |
| FR16 | Strategy data structure supports future strategies being added without modifying the dispatcher or clap definitions                             | Should   | Dispatch by name from a table in `examples.rs` |
| FR17 | `examples --json` top-level listing emits only `command` and `description` per entry — the nested `examples` array is removed from listing output (added by #218) | Must | Breaking change; supersedes the `examples` array assertion in AC11d |
| FR18 | `examples <group> --json` continues to return the full per-group `examples` array on the detail path (added by #218) | Must | Detail path unchanged |
| FR19 | `capabilities --json` listing emits only `name` and `description` per command — `subcommands`, `args`, `flags` removed from listing entries (added by #218) | Must | Breaking change |
| FR20 | New positional `capabilities <command>` returns the full `CommandDescriptor` on the detail path (added by #218) | Must | New detail path; mirrors the flat-positional pattern chosen for `examples strategies` |
| FR21 | Unknown command in `capabilities <command>` returns a JSON error on stderr with exit code 1, listing available command names (added by #218) | Must | Mirrors existing unknown-strategy / unknown-group behavior |
| FR22 | Clap `long_about` and `after_long_help` on the `Capabilities` variant document the new positional with at least one worked `--json` example (added by #218) | Must | Per tech.md Clap Help Entries steering principle |
| FR23 | The ten already-compliant listing commands (`tabs list`, `network list`, `console read`, `cookie list`, `media list`, `page frames`, `page workers`, `dom select`/`tree`, `skill list`) are NOT modified (added by #218) | Must | Scope guard |
| FR24 | BDD scenarios in `tests/features/examples.feature` and `tests/features/capabilities.feature` cover AC13–AC17 (added by #218) | Must | Update existing feature files, not create new ones |
| FR25 | `CHANGELOG.md` entry under the next release heading labels the shape change as breaking (added by #218) | Must | User explicitly chose the breaking-change path over an opt-in `--full` flag |
| FR26 | Progressive-disclosure guard unit test asserts serialized listing JSON for `examples` and `capabilities` does NOT contain detail field names (`examples`, `subcommands`, `args`, `flags`) (added by #218) | Should | Prevents accidental regression into non-compliance |

---

## Non-Functional Requirements

| Aspect                     | Requirement                                                                                                                          |
|----------------------------|--------------------------------------------------------------------------------------------------------------------------------------|
| **Performance**            | `examples strategies` and `examples strategies <name>` MUST complete in under 50ms on a warm cache (same budget as other `examples` paths); strategy content is compiled in, no I/O |
| **Reliability**            | Strategy content is static data; no CDP or network calls; works offline and without a Chrome connection                              |
| **Accessibility**          | Plain-text output is readable on standard 80-column terminals; JSON output is valid per RFC 8259                                     |
| **Platforms**              | macOS, Linux, Windows (per `product.md`)                                                                                             |
| **Binary size**            | New strategy content contributes well under 100 KB to the compiled binary (~20\u201330 KB expected for 10 guides)                       |
| **Determinism**            | Same inputs produce byte-identical output across platforms and runs (per AC12)                                                       |
| **Progressive disclosure** | Complies with the `tech.md` **Progressive Disclosure for Listings** principle: listing payloads return only `name`/`title`/`summary`; full guide bodies load only via individual strategy selection (see AC4, AC5, FR7a) |
| **Clap help metadata**     | Complies with the `tech.md` **Clap Help Entries** principle: the `Examples` variant in `src/cli/mod.rs` gets an updated `long_about` and `after_long_help` covering `examples strategies` and `examples strategies <name>` with at least one `--json` example; the new positional `name` has a doc comment enumerating valid strategy names or pointing to the listing command (see FR11, AC9, AC10) |

---

## UI/UX Requirements

| Element          | Requirement                                                                                                               |
|------------------|---------------------------------------------------------------------------------------------------------------------------|
| **Plain listing**  | One strategy per line: `<name> \u{2014} <short description>`, matching the existing `format_plain_summary` style in `examples.rs` |
| **Plain detail**   | Sections visually separated by blank lines; section headers in a consistent style; command lines printed verbatim so they can be copy-pasted |
| **JSON shape**     | Field names in `snake_case` (matching existing `examples` JSON output fields `command`, `description`, `examples`); optional fields use `#[serde(skip_serializing_if = "Option::is_none")]` |
| **Error messages** | Unknown strategy error mirrors unknown-command-group: message mentions the invalid name and lists valid names           |
| **Tone**           | Imperative, concrete, and focused on "here is the command to run" — not generic prose                                   |

---

## Data Requirements

### Input Data (CLI arguments)

| Field                            | Type          | Validation                                                                                                         | Required |
|----------------------------------|---------------|--------------------------------------------------------------------------------------------------------------------|----------|
| First positional (currently `command`) | `Option<String>` | If present and value is `"strategies"`, activates strategy mode; otherwise resolves to a command group as today     | No       |
| Second positional (new, for strategy name) | `Option<String>` | Only evaluated when first positional is `"strategies"`; when present, must match a known strategy name         | No       |
| `--json` (on `GlobalOpts`)       | `bool`        | Existing flag                                                                                                      | No       |
| `--pretty` (on `GlobalOpts`)     | `bool`        | Existing flag                                                                                                      | No       |

**Reserved-name check**: The literal `"strategies"` must not collide with any existing command group name. Verified: none of the 17 current groups use that name (see FR14).

### Output Data

#### Strategy listing (`examples strategies [--json]`) \u2014 lightweight

Plain text: one strategy per line, `<name> \u{2014} <summary>`.
JSON: array of `StrategySummary` objects (three fields only).

#### Individual strategy (`examples strategies <name> [--json]`) \u2014 full detail

Plain text: sectioned guide (scenario, capabilities, limitations, workarounds, recommended sequence).
JSON: single full `Strategy` object.

#### `StrategySummary` JSON shape (used for listing \u2014 progressive disclosure)

| Field     | Type   | Description                                      |
|-----------|--------|--------------------------------------------------|
| `name`    | string | Kebab-case identifier (`"iframes"`, `"drag-and-drop"`) |
| `title`   | string | Human-readable title                             |
| `summary` | string | One-line description                             |

No other fields are emitted in the listing. Roughly 100\u2013200 bytes per entry.

#### `Strategy` JSON shape (used for individual lookups \u2014 full detail)

| Field                   | Type                           | Description                                                                                     |
|-------------------------|--------------------------------|-------------------------------------------------------------------------------------------------|
| `name`                  | string                         | Kebab-case identifier (`"iframes"`, `"drag-and-drop"`)                                          |
| `title`                 | string                         | Human-readable title                                                                            |
| `summary`               | string                         | One-line description used in the top-level listing                                              |
| `scenarios`             | array of string                | Short phrases describing when this strategy applies                                             |
| `capabilities`          | array of string                | Currently-shipped agentchrome commands relevant to this strategy                                |
| `limitations`           | array of string                | What does NOT work today                                                                        |
| `workarounds`           | array of `{description, commands: [string]}` | Each workaround has a prose description and a copy-pasteable command sequence   |
| `recommended_sequence`  | array of string                | Ordered, copy-pasteable `agentchrome \u2026` invocations for the canonical happy path             |

---

## Dependencies

### Internal Dependencies

- [x] Existing `examples` subcommand (`src/examples.rs`) — this feature extends it
- [x] `GlobalOpts::output` (`--json`, `--pretty`, `--plain`) — reused as-is
- [x] `print_output` helper in `crate::output` — reused for JSON/pretty paths

### External Dependencies

- None — strategy content is compile-time static data

### Related Specs (not blocking)

- `feature-add-iframe-frame-targeting-support/` — the iframes strategy guide will be updated when that feature ships, per FR10; this spec MUST NOT depend on it shipping first (guide covers current state + workarounds)
- `feature-add-diagnose-command-for-pre-automation-challenge-scanning/` (#200, already merged) — `diagnose` points users toward strategies; strategy guides may reference `diagnose` as a discovery step

### Blocked By

- None

---

## Out of Scope

- Video tutorials or interactive walkthroughs
- Application-specific guides (e.g., "Automating Salesforce") — strategies are generic patterns, not application recipes
- Community-contributed strategy guides (guides are maintained in-tree with the code)
- A plugin mechanism for third-party strategies
- Localization of strategy content — English only for now
- Strategy content that depends on runtime page state (that is the `diagnose` command's job)
- Fuzzy matching or "did you mean \u2026?" suggestions for unknown strategy names (the error lists valid names; exact match required)
- **Retrofitting the existing `agentchrome examples --json` top-level listing** to comply with the new **Progressive Disclosure for Listings** steering principle. That top-level listing currently returns all command groups with all of their examples (~7 KB JSON) \u2014 technically non-compliant under the new tech.md rule now that the grandfather exemption has been removed. Retrofitting it (e.g., summaries-only by default, `--full` opt-in) is a separate change that would touch every existing `examples` BDD scenario and is out of scope for this issue. **Resolved by issue #218** — retrofit of `examples --json` and `capabilities --json` listings is folded into this spec via AC13–AC20 and FR17–FR26.

**Added by #218 (out of scope for the retrofit itself):**

- Retrofitting any of the other ten already-compliant listing commands (`tabs list`, `network list`, `console read`, `cookie list`, `media list`, `page frames`, `page workers`, `dom select`/`tree`, `skill list`) — they already emit lightweight summaries.
- Introducing an opt-in `--full` flag for backward compatibility — the user has explicitly chosen the breaking-change path for v1.
- Changes to BDD step-definition infrastructure beyond what the new scenarios require.
- Further refactoring of `src/examples/` module layout (already done by #201) or `src/capabilities.rs` beyond the new positional + listing-shape change.

---

## Success Metrics

| Metric                                              | Target      | Measurement                                                                                 |
|-----------------------------------------------------|-------------|---------------------------------------------------------------------------------------------|
| Strategy guide coverage at launch                   | 10 guides   | AC8 — all ten named launch strategies present                                                |
| `examples strategies` command completion time       | < 50ms      | `time agentchrome examples strategies --json > /dev/null`                                    |
| BDD scenario count for this feature                 | \u{2265} 12 | Count of scenarios in `tests/features/examples-strategies.feature`                           |
| Regression count in existing `examples` scenarios   | 0           | Existing `tests/features/examples.feature` scenarios remain passing                         |

---

## Open Questions

- [ ] **CLI shape for the strategy sub-path.** Two viable options, to be resolved in PLAN:
    1. **Flat positional approach**: add a second optional positional to `ExamplesArgs` (`name: Option<String>`); dispatcher special-cases `command == "strategies"`. Pros: zero CLI surface change beyond one extra positional, works with current `Option<String>` structure. Cons: ad-hoc special-casing in the dispatcher.
    2. **Enum/subcommand approach**: replace `ExamplesArgs::command: Option<String>` with a clap subcommand enum that has variants `CommandGroup(String)` and `Strategies(StrategyArgs)`. Pros: cleaner domain modeling, `--help` is automatically per-path. Cons: risks breaking `agentchrome examples navigate` ergonomics if clap needs the subcommand keyword explicitly.
  The design phase will evaluate both against existing clap usage and pick one.
- [x] ~~Whether to ship additional strategies beyond the four Must-haves in the initial release, or defer to follow-on issues.~~ **Resolved 2026-04-16**: expanded launch set from 4 to 10 guides — added `shadow-dom`, `spa-navigation-waits`, `react-controlled-inputs`, `debugging-failed-interactions`, `authentication-cookie-reuse`, `multi-tab-workflows` (see FR5a–FR5f and AC8).

---

## Change History

| Issue | Date       | Summary                                                                                          |
|-------|------------|--------------------------------------------------------------------------------------------------|
| #201  | 2026-04-16 | Initial feature spec                                                                             |
| #201  | 2026-04-16 | Expanded launch strategy set from 4 to 10 guides (added shadow-dom, spa-navigation-waits, react-controlled-inputs, debugging-failed-interactions, authentication-cookie-reuse, multi-tab-workflows) |
| #218  | 2026-04-21 | Progressive Disclosure retrofit: `examples --json` and `capabilities --json` listings return summaries only; new `capabilities <command>` detail positional; breaking-change CHANGELOG entry. Supersedes AC11d's `examples` array assertion. |

---

## Validation Checklist

Before moving to PLAN phase:

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements (open question defers CLI shape to design)
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified (AC7, AC11)
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented
