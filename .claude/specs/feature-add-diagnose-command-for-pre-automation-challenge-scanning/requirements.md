# Requirements: Diagnose Command for Pre-Automation Challenge Scanning

**Issues**: #200
**Date**: 2026-04-16
**Status**: Draft
**Author**: Claude (spec-writer)

---

## User Story

**As a** browser automation engineer starting work on a new target page
**I want** an automated diagnostic scan that identifies potential automation challenges before I begin
**So that** I can set expectations and choose the right interaction strategy from the start instead of discovering limitations through trial and error

---

## Background

Users report spending significant time discovering automation limitations through failed interactions — only learning about iframe boundaries, invisible overlays, audio gates, or canvas rendering after many wasted attempts. An automated `diagnose` command that scans a page and reports potential challenges upfront, together with **named pattern matches** (e.g., Storyline acc-blocker, SCORM player, React portal) and **concrete interaction strategy suggestions**, would eliminate this discovery cost.

The `diagnose` command complements the forthcoming `page analyze` (#190). Where `page analyze` produces a structural enumeration (iframes, frameworks, overlays, media, shadow DOM, interactive counts), `diagnose` operates at a higher level — it interprets the same raw structural signals as **challenges** with severity and actionable strategy advice, adds a **named pattern database** for common frontend architectures, and can combine navigation and analysis into a single command so it works as the very first step in an automation session before any other `agentchrome` call.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Diagnose a URL

**Given** a URL is passed as the first positional argument
**When** `agentchrome diagnose <url>` is run
**Then** the browser navigates to that URL, waits for load completion, and then analyzes the page
**And** a structured JSON report is written to stdout containing:
- `url` — the diagnosed URL
- `scope` — `"diagnosed"` when a URL was navigated to, `"current"` when `--current` was used
- `challenges` — array of detected challenges (iframes, overlays, shadow DOM, canvas/WebGL, framework quirks, media gates)
- `patterns` — array of matched known patterns with suggested strategies
- `summary` — aggregate counts and a `straightforward: bool` flag
**And** the command exits with exit code 0

**Example**:
- Given: URL `https://example.com/storyline-course` which hosts a Storyline SCORM course inside an iframe with an acc-blocker overlay, and a background audio element
- When: `agentchrome diagnose https://example.com/storyline-course`
- Then: JSON where `challenges` contains entries for `iframes`, `overlays`, and `mediaGates`; `patterns` contains entries for `storyline-acc-blocker` and `scorm-player`; `summary.straightforward` is `false`

### AC2: Known pattern suggestions

**Given** the page matches one or more patterns in the known-pattern database (e.g., Storyline acc-blocker, SCORM player, React portal)
**When** `diagnose` is run
**Then** each matched pattern appears in the `patterns` array with:
- `name` — canonical pattern identifier (kebab-case), e.g., `"storyline-acc-blocker"`
- `matched` — `true` for matched patterns (matched-false entries are excluded from output)
- `confidence` — `"low"`, `"medium"`, or `"high"`
- `evidence` — a human-readable sentence citing the DOM signal that triggered the match (e.g., selector + coverage percent)
- `suggestion` — actionable advice that references specific `agentchrome` commands (e.g., `"Use 'interact click-at --frame 2' to target elements inside the Storyline iframe"`)

**Example**:
- Given: a page with `div.acc-blocker` covering 90% of the viewport and a Storyline content iframe at index 1
- When: `agentchrome diagnose --current`
- Then: `patterns` contains an entry with `name: "storyline-acc-blocker"`, `confidence: "high"`, `evidence` mentioning the `div.acc-blocker` selector and its viewport coverage, and `suggestion` referencing frame-targeted interaction

### AC3: Clean page report

**Given** a simple static HTML page with no iframes, overlays, shadow DOM, canvas/WebGL, framework quirks, or media gates
**When** `diagnose` is run
**Then** the output reflects a straightforward automation target:
- `challenges` is an empty array
- `patterns` is an empty array
- `summary.challengeCount` is `0`
- `summary.patternMatchCount` is `0`
- `summary.straightforward` is `true`
- `summary.hasHighSeverity` is `false`
**And** the command exits with exit code 0

### AC4: Diagnose the current page

**Given** an already-loaded page in the active tab, with no positional URL argument
**When** `agentchrome diagnose --current` is run
**Then** the page is diagnosed in place without any navigation
**And** the output `scope` field is `"current"`
**And** the output `url` field equals the URL currently displayed in the active tab
**And** no `Page.navigate` CDP call is issued during execution

### AC5: Documentation and examples updated

**Given** the new `diagnose` command is installed
**When** the project's documentation surfaces are consulted
**Then** all of the following are updated:
- `agentchrome examples diagnose` outputs at least a basic URL invocation, a `--current` invocation, and an example demonstrating how strategy suggestions reference other `agentchrome` subcommands
- `agentchrome diagnose --help` (short) describes the command's one-line purpose, its argument shape, and the mutual-exclusion rule between `<url>` and `--current`
- `agentchrome diagnose --help` long-help (via clap `long_about` + `after_long_help`) documents the full JSON output schema (top-level fields and the challenge/pattern/summary sub-objects), all exit codes, and at least two worked invocation examples
- `agentchrome capabilities` output includes `diagnose` in the commands list with its flags and positional argument (the capabilities manifest is generated from clap metadata, so this is satisfied automatically once the clap definition is in place — the AC exists to verify the generator picks it up)
- `agentchrome man diagnose` displays a man page for the new command (auto-generated by `cargo xtask man` from the clap definition; the AC verifies the man-page pipeline emits a section for `diagnose`)
- `README.md` "Command Reference" table includes a new row for `diagnose` with a one-line description, and the README "Usage Examples" section includes at least one `diagnose` invocation example consistent with the other command examples in that section

### AC6: Cross-origin iframe handling

**Given** a diagnosed page containing cross-origin (out-of-process) iframes
**When** `diagnose` is run
**Then** each cross-origin iframe appears in the `iframes` challenge details with `crossOrigin: true` and its accessible fields (`url`, `width`, `height`)
**And** fields that cannot be determined due to same-origin policy restrictions (e.g., interactive element counts inside the cross-origin frame) appear as `null` in the output — they are **not** omitted and **not** coerced to `0`
**And** the command completes with exit code 0 (cross-origin iframes are an expected condition, not an error)

### AC7: Undetermined fields use null, not zero

**Given** a page where one or more diagnostic dimensions cannot be determined (e.g., media element `currentTime` is unreadable, or canvas rendering state cannot be queried from a sandboxed iframe)
**When** `diagnose` is run
**Then** fields that cannot be measured appear as `null` in the JSON output
**And** fields that can be measured and have a true zero value appear as `0` (numeric) or `false` (boolean) — the `null`-vs-`0`/`false` distinction is preserved
**And** the command completes successfully with exit code 0, reporting the remaining dimensions that could be measured

### AC8: Missing URL without `--current` is an argument error

**Given** `agentchrome diagnose` is invoked with neither a URL argument nor the `--current` flag
**When** the command is parsed
**Then** a JSON error object is written to stderr describing that exactly one of `<url>` or `--current` is required
**And** the command exits with exit code 1 (GeneralError)
**And** stdout is empty

### AC9: `<url>` and `--current` are mutually exclusive

**Given** `agentchrome diagnose` is invoked with both a positional URL and the `--current` flag
**When** the command is parsed
**Then** a JSON error object is written to stderr explaining that `<url>` and `--current` cannot be combined
**And** the command exits with exit code 1 (GeneralError)
**And** stdout is empty

### AC10: Navigation failure is reported as a navigation error

**Given** a URL that cannot be reached (unresolvable DNS, refused connection, or navigation timeout)
**When** `agentchrome diagnose <url>` is run
**Then** the resulting error is a navigation-phase failure — a JSON error object is written to stderr with a descriptive message indicating the navigation failed
**And** the command exits with the standard `agentchrome` navigation failure exit code (`4` for timeout or `5` for protocol errors), matching how `agentchrome navigate <url>` would exit for the same URL
**And** stdout is empty
**And** no diagnostic fields are partially written to stdout

### AC11: No active Chrome session is reported as a connection error

**Given** no Chrome instance is currently connected in the session file
**When** `agentchrome diagnose <url>` or `agentchrome diagnose --current` is run
**Then** a JSON error is written to stderr matching the standard agentchrome connection-error shape
**And** the command exits with exit code `2` (ConnectionError)
**And** stdout is empty

### AC12: Unknown pattern or missing data does not cause a failure

**Given** a page that does not match any pattern in the known-pattern database and/or exposes DOM signals the detector has never seen
**When** `diagnose` is run
**Then** the `patterns` array is empty (or contains only genuine matches)
**And** the `challenges` array contains whatever structural categories were detected
**And** the command exits with exit code 0
**And** no error is emitted for "unrecognized" signals — unknown signals are silently ignored by the pattern matcher

### Generated Gherkin Preview

```gherkin
Feature: Diagnose Command for Pre-Automation Challenge Scanning

  Scenario: Diagnose a URL produces a structured JSON report
    Given a URL with known challenges
    When the diagnose command is run with that URL
    Then the JSON report lists challenges, patterns, and a summary
    And exit code is 0

  Scenario: Known patterns include actionable suggestions
    Given a page matching the Storyline acc-blocker pattern
    When diagnose is run
    Then the patterns array contains a storyline-acc-blocker entry with a suggestion referencing frame targeting

  Scenario: Clean page reports straightforward
    Given a simple static HTML page
    When diagnose is run
    Then challenges is empty and summary.straightforward is true

  # ... all ACs become scenarios
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | New top-level `diagnose` subcommand accepting `<url>` (positional) or `--current` (flag), mutually exclusive | Must | Follows top-level `navigate` command pattern |
| FR2 | URL mode navigates to the target and waits for page load before analyzing | Must | Reuse `navigate::execute_url` logic or a shared helper for navigation + wait-until-loaded |
| FR3 | `--current` mode analyzes the active tab's current page without navigating | Must | Must not emit any `Page.navigate` CDP call |
| FR4 | Iframe detection with cross-origin status and per-frame accessibility of interactive counts | Must | Reuse `frame::list_frames()`; interactive counts may be `null` for cross-origin |
| FR5 | Overlay/blocker detection (full-viewport and large-area fixed/absolute-positioned elements with high z-index) | Must | Same approach as `page analyze` overlay scanner |
| FR6 | Shadow DOM detection (presence + host count) | Must | Same approach as `page analyze` |
| FR7 | Canvas and WebGL rendering detection (count of `<canvas>` elements; WebGL context presence) | Should | Use `Runtime.evaluate` to enumerate `<canvas>` and check `getContext('webgl' \|\| 'webgl2')` without retaining contexts |
| FR8 | Media element gate detection — `<video>`/`<audio>` that require user interaction before navigation/autoplay | Must | Detect elements with `autoplay` blocked, paused state, and `media-session` handlers |
| FR9 | Framework-specific interaction quirk detection (React portals rendering outside root, Angular zone.js traps, Vue teleport, Svelte hydration hints) | Should | Heuristic DOM/window signature checks |
| FR10 | Known pattern database with at least: `storyline-acc-blocker`, `scorm-player`, `react-portal` | Must | Each pattern is a declarative rule: DOM selector(s) + coverage/count threshold + confidence + suggestion template |
| FR11 | Each matched pattern produces a `suggestion` field referencing concrete `agentchrome` commands (e.g., `interact click-at --frame N`) | Must | Suggestions must be copy-pasteable command hints |
| FR12 | Challenge entries are categorized (`iframes`, `overlays`, `shadowDom`, `canvas`, `media`, `framework`) with per-category severity (`low`/`medium`/`high`) | Must | Severity derived from signal strength (e.g., full-viewport overlay over interactive elements → `high`) |
| FR13 | Summary object with `challengeCount`, `patternMatchCount`, `hasHighSeverity`, `straightforward` | Must | `straightforward == (challengeCount == 0 && patternMatchCount == 0)` |
| FR14 | Navigation failures are reported using the same JSON stderr contract as `agentchrome navigate <url>`, with the same exit codes | Must | Reuse navigation error handling to avoid divergent error shapes |
| FR15 | Missing-URL / missing-`--current` / both-supplied argument errors are caught at clap parse time and emitted as JSON on stderr | Must | Follows global clap error handling in `main.rs` |
| FR16 | `examples diagnose` output includes diagnose-specific examples (basic, `--current`, strategy-suggestion demo) | Must | Update `src/examples.rs` example registry |
| FR17 | clap `--help` short + long-help for `diagnose` describe the command, its argument shape (`<url>` vs `--current`), the JSON output schema, exit codes, and include worked examples | Must | `long_about` + `after_long_help` on the `Diagnose` subcommand in `src/cli/mod.rs` |
| FR18 | `README.md` "Command Reference" table gains a row for `diagnose`, and the "Usage Examples" section gains at least one `diagnose` invocation consistent with the style of the other command examples | Must | Edit `README.md` directly as part of the implementation tasks |
| FR19 | The capabilities manifest (`agentchrome capabilities`) and man-page pipeline (`cargo xtask man` → `agentchrome man diagnose`) include the new command automatically via clap metadata | Must | Satisfied by clap wiring; verify both surfaces emit the new command during implementation |
| FR20 | BDD feature file covering all acceptance criteria with step definitions in the shared `tests/bdd.rs` | Must | One `.feature` file per `/writing-specs` convention |
| FR21 | Fields that cannot be determined are serialized as JSON `null` (never omitted, never coerced to `0`/`false`) | Must | Explicit `Option<T>` in Rust output types where absence is semantic |
| FR22 | Pattern database is a compile-time static table (no runtime config file loading in the first release) | Could | Keeps binary size down; future releases may externalize the table |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | A diagnose pass (not counting navigation) completes within 5 seconds on pages with up to 10 iframes and 2,000 DOM elements |
| **Navigation timeout** | URL mode honors the global `--timeout` flag and the existing `navigate` default (30,000 ms), with the same propagation and error classification as `navigate <url>` |
| **Reliability** | Each detection dimension degrades gracefully — if one dimension fails (CDP protocol error, sandboxed iframe, JS eval reject), the command reports `null` for that dimension and continues with the others |
| **Output contract** | JSON on stdout only on success; JSON on stderr only on failure; meaningful exit codes per `tech.md`; exactly one error object per invocation |
| **Cross-origin** | Cross-origin iframes never crash the scanner; inaccessible dimensions are reported as `null` with `crossOrigin: true` |
| **Determinism** | The same page scanned twice produces equivalent output (modulo transient media playback state). Pattern confidence values are stable for a given DOM. |
| **Platforms** | macOS, Linux, Windows (consistent with all agentchrome commands) |

---

## UI/UX Requirements

| Element | Requirement |
|---------|-------------|
| **Output format** | Structured JSON on stdout; camelCase field names per project convention |
| **Error format** | JSON error objects on stderr with descriptive messages; exit codes per `tech.md` |
| **Command shape** | `agentchrome diagnose <url>` or `agentchrome diagnose --current`; no subcommands |
| **Global flags** | Honors all existing global flags: `--port`, `--host`, `--ws-url`, `--timeout`, `--tab`, `--page-id`, `--auto-dismiss-dialogs`, `--config`, output-format flags |
| **Help** | Standard clap `--help` and `--help-long`; `examples diagnose` for copy-pasteable usage |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `<url>` (positional) | string | Parsable URL; `--current` must not also be set | Conditional — required unless `--current` is present |
| `--current` | flag (bool) | `<url>` must not also be set | Conditional — required unless `<url>` is present |

**Argument-name collision check**: `current` does not collide with any existing global flag (`port`, `host`, `ws-url`, `timeout`, `tab`, `page-id`, `auto-dismiss-dialogs`, `config`, output format flags) and is not a clap/framework-reserved name.

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| `url` | string | The URL that was diagnosed (navigated-to URL in URL mode, currently-displayed URL in `--current` mode) |
| `scope` | string | `"diagnosed"` (URL mode) or `"current"` (`--current` mode) |
| `challenges` | array of challenge objects | One entry per detected challenge category; see "Challenge Object" below |
| `patterns` | array of pattern objects | One entry per matched known pattern; only matched patterns are emitted |
| `summary` | object | Aggregate summary; see "Summary Object" below |

#### Challenge Object

| Field | Type | Description |
|-------|------|-------------|
| `category` | string | One of `"iframes"`, `"overlays"`, `"shadowDom"`, `"canvas"`, `"media"`, `"framework"` |
| `severity` | string | `"low"`, `"medium"`, or `"high"` |
| `summary` | string | Short human-readable description (e.g., `"2 iframes detected (1 cross-origin)"`) |
| `details` | object | Category-specific detail structure (e.g., for `iframes`: `{ count, crossOriginCount, items: [{ index, url, name, width, height, crossOrigin, visible }] }`) |
| `suggestion` | string \| null | Category-level advice; `null` if no specific advice applies |

#### Pattern Object

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Canonical kebab-case identifier (e.g., `"storyline-acc-blocker"`) |
| `matched` | bool | Always `true` for emitted entries |
| `confidence` | string | `"low"`, `"medium"`, or `"high"` |
| `evidence` | string | Human-readable sentence citing the DOM signal that triggered the match |
| `suggestion` | string | Actionable advice referencing specific `agentchrome` commands |

#### Summary Object

| Field | Type | Description |
|-------|------|-------------|
| `challengeCount` | integer | Number of entries in `challenges` |
| `patternMatchCount` | integer | Number of entries in `patterns` |
| `hasHighSeverity` | bool | `true` if any challenge entry has `severity == "high"` |
| `straightforward` | bool | `true` iff `challengeCount == 0 && patternMatchCount == 0` |

---

## Dependencies

### Internal Dependencies

- [x] Navigation helper (`src/navigate.rs` — URL navigation with wait-until-loaded)
- [x] Frame enumeration (`src/frame.rs` — `list_frames()`)
- [x] CDP client (`src/cdp/client.rs`)
- [x] Session management (`src/session.rs`, `src/connection.rs`)
- [x] Output formatting (`src/output.rs`, `GlobalOpts.output`)
- [x] Examples registry (`src/examples.rs`)
- [x] CLI framework wiring (`src/cli/mod.rs`, `src/main.rs`)
- [ ] Page analyze analyzers (`src/page/analyze.rs`) — **reused** for iframe/overlay/shadow-DOM/media/framework detection logic; `diagnose` layers pattern matching and strategy suggestions on top

### External Dependencies

- [x] Chrome DevTools Protocol (DOM, Runtime, Page domains)

### Blocked By

- [ ] #190 — Page Analyze Command (reused for structural detection). If #190 lands first, `diagnose` reuses its analyzers directly. If `diagnose` lands first or in parallel, the shared analyzers may need to be extracted into a common module accessible to both.

---

## Out of Scope

- Automated remediation or fix application (no auto-dismissing overlays, no auto-unmuting media, no auto-clicking `accept` buttons)
- Accessibility compliance auditing (use `audit lighthouse`)
- Performance diagnosis (use `perf analyze` / `perf vitals`)
- Security scanning
- Full JavaScript framework version detection (only presence/absence and known-quirk detection)
- External pattern database / runtime-loaded pattern rules (compile-time static table only for this release; FR20 scoped as Could)
- Headless-vs-headed behavioral divergence checks (behavior should be identical in both modes)
- Diagnosing a specific iframe scope (`--frame N`) — first release diagnoses the main frame + its direct children; per-frame scoping can follow in a future issue
- Continuous monitoring / re-diagnose on DOM mutation (one-shot only)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Pre-automation discovery time | < 10 s (including navigation) vs 30+ min manual probing | Wall-clock time of a single `diagnose` invocation on representative targets |
| Pattern coverage | ≥ 3 pattern definitions in first release (Storyline, SCORM, React portal) | Count entries in the pattern database |
| Strategy actionability | 100% of suggestion strings reference at least one concrete `agentchrome` command or global flag | Linting test over the static pattern database |
| Error resilience | 0 crashes on malformed, sandboxed, or cross-origin pages across the fixture suite | Fixture test sweep during verification |

---

## Open Questions

- [ ] Should `diagnose --current` require an active tab to already exist, or should it auto-discover the active tab like `page snapshot`? **Proposed answer**: mirror `page snapshot` behavior — auto-discover the active tab via existing session/target resolution.
- [ ] When a URL is passed and navigation returns a non-2xx HTTP status, should `diagnose` still analyze the rendered error page? **Proposed answer**: yes — the error page is a valid diagnostic target; include the HTTP status in the output (field: `navigationStatus`) if available from the response.
- [ ] Should the pattern database support negative rules (explicit "this is NOT a match" overrides)? **Proposed answer**: defer to a follow-up issue; first release supports positive rules only.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #200 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements (reused-module references are in Dependencies, not ACs)
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified (AC6–AC12)
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented with proposed answers
- [x] Cross-origin iframe behavior is explicit (AC6)
- [x] Null-vs-zero output contract is explicit (AC7)
- [x] Exactly one error object per invocation is required (FR14, NFR "Output contract")
- [x] Argument-name collision check performed (Data Requirements > Input Data)
