# Design: Diagnose Command for Pre-Automation Challenge Scanning

**Issues**: #200
**Date**: 2026-04-16
**Status**: Draft
**Author**: Claude (spec-writer)

---

## Overview

The `diagnose` command is implemented as a **new top-level command** (peer to `navigate`, `audit`, `page`, etc.), not a `page` subcommand. It accepts either a positional URL or a `--current` flag (mutually exclusive). In URL mode it navigates the active tab to the target and waits for load completion before analyzing; in `--current` mode it analyzes the active tab's current page in place.

The command composes three existing capability layers — **navigation**, **structural analysis**, and **output formatting** — and adds two new ones: a **known-pattern matcher** that maps DOM signals to named architectures (Storyline acc-blocker, SCORM player, React portal), and a **suggestion engine** that emits actionable `agentchrome` command hints per challenge and per matched pattern. Structural analysis is not re-implemented: the existing per-dimension detectors in `src/page/analyze.rs` are extracted to a shared, `pub(crate)` surface and reused from the new `diagnose` module. This keeps the two commands in lockstep — if `page analyze` learns to detect a new dimension, `diagnose` inherits the capability for free.

Each detection dimension fails independently (graceful degradation): when a dimension cannot be measured (cross-origin iframe, sandboxed context, CDP error), the affected field is serialized as JSON `null` and the remaining dimensions still report. The command output is a **challenge-first** report: each detected category becomes a `Challenge` object with severity + suggestion, pattern matches are emitted separately with evidence + suggestion, and a `summary` object aggregates counts and a `straightforward` flag.

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                             CLI Layer                              │
│   cli/mod.rs: Command::Diagnose(DiagnoseArgs { url, current, … }) │
└────────────────────────────────┬─────────────────────────────────┘
                                 │
                                 ▼
┌──────────────────────────────────────────────────────────────────┐
│                         Command Dispatch                           │
│   main.rs: Command::Diagnose(args) → diagnose::execute_diagnose   │
└────────────────────────────────┬─────────────────────────────────┘
                                 │
                                 ▼
┌──────────────────────────────────────────────────────────────────┐
│                     src/diagnose/ (new module)                     │
│                                                                    │
│   mod.rs       — execute_diagnose() orchestrator                   │
│   output.rs    — DiagnoseResult / Challenge / PatternMatch / Summary│
│   patterns.rs  — static PATTERN_DB + matcher                       │
│   detectors.rs — diagnose-only detectors (canvas/WebGL, media gate,│
│                   framework quirks); reuses page::analyze for      │
│                   iframe / overlay / shadow-DOM / media / frameworks│
│                                                                    │
│   execute_diagnose() flow:                                         │
│     ┌─────────────────────────────────────────────────────┐       │
│     │  1. Argument dispatch (URL mode vs --current)        │       │
│     │  2. URL mode → navigate::navigate_and_wait(...)      │       │
│     │     --current mode → skip navigation                 │       │
│     │  3. Run shared detectors (reused from page::analyze) │       │
│     │  4. Run diagnose-only detectors (canvas/WebGL, gate) │       │
│     │  5. Build Challenge[] from detector outputs          │       │
│     │  6. Run patterns::match_all(&detector_outputs)       │       │
│     │  7. Assemble DiagnoseResult + Summary                │       │
│     │  8. print_output() → stdout                          │       │
│     └─────────────────────────────────────────────────────┘       │
└────────────────────────────────┬─────────────────────────────────┘
                                 │
        ┌────────────────────────┼──────────────────────────┐
        ▼                        ▼                          ▼
┌───────────────────┐ ┌────────────────────┐ ┌──────────────────────┐
│ navigate::        │ │ page::analyze::    │ │ CDP Client           │
│ navigate_and_wait │ │ detectors::*       │ │ (DOM / Runtime /     │
│ (extracted helper)│ │ (extracted pub(crate))│  Page domains)      │
└───────────────────┘ └────────────────────┘ └──────────────────────┘
                                 │
                                 ▼
                          Chrome Browser
```

### Data Flow

```
1. User runs: agentchrome diagnose <url>
   OR:        agentchrome diagnose --current
2. clap parses DiagnoseArgs; clap enforces XOR of url vs --current
3. main.rs dispatches to diagnose::execute_diagnose(global, args)
4. execute_diagnose:
   a. setup_session() to obtain ManagedSession
   b. If URL mode: navigate_and_wait(&mut session, url, wait_until, timeout_ms)
      Captures navigationStatus (optional HTTP status) if available.
      On navigation error, propagate as AppError (same contract as `navigate <url>`).
   c. Resolve page_info (url, title) — for --current, this is the current page.
5. Run detectors sequentially (each fails independently):
   - iframes           (reused from page::analyze::detectors)
   - frameworks        (reused from page::analyze::detectors)
   - interactive count (reused; per-frame; null for inaccessible cross-origin)
   - media catalog     (reused; extended with gate classification)
   - overlays          (reused from page::analyze::detectors)
   - shadow DOM        (reused from page::analyze::detectors)
   - canvas/WebGL      (new; diagnose-only)
   - framework quirks  (new; diagnose-only)
6. Build Challenge[] — one entry per category with something worth reporting:
   - Skip empty categories (e.g., no overlays → no overlay challenge entry)
   - Assign severity per category-specific heuristic (see "Severity Assignment")
   - Attach per-category suggestion from a static string table
7. Run pattern matcher over the detector outputs (no extra CDP round trips):
   - Each pattern's detector consumes existing detector outputs + optional 1
     extra JS eval when the rule demands it (amortized so far; typically 0)
   - Emit matched entries only
8. Assemble DiagnoseResult with summary aggregates
9. print_output() → JSON to stdout; exit 0
```

---

## API / Interface Changes

### New CLI Command

| Variant | Parent | Args | Purpose |
|---------|--------|------|---------|
| `Command::Diagnose(DiagnoseArgs)` | `Command` enum (top-level) | `DiagnoseArgs` | Pre-automation challenge scanning |

### CLI Definition (clap derive)

```rust
// src/cli/mod.rs — new variant on Command enum

/// Pre-automation challenge scan (iframes, overlays, media gates, frameworks, patterns)
#[command(
    long_about = "Scan a page for automation challenges — iframes, overlay blockers, shadow DOM, \
        canvas/WebGL rendering, media playback gates, and framework-specific interaction quirks — \
        plus named-pattern matches (e.g., Storyline acc-blocker, SCORM player, React portal) with \
        actionable agentchrome command suggestions. Accepts a URL to navigate-then-analyze, or \
        `--current` to analyze the already-loaded page in place.\n\n\
        OUTPUT SCHEMA (JSON on stdout):\n\
          {\n\
            \"url\": string,\n\
            \"scope\": \"diagnosed\" | \"current\",\n\
            \"challenges\": [{category, severity, summary, details, suggestion?}],\n\
            \"patterns\":   [{name, matched, confidence, evidence, suggestion}],\n\
            \"summary\":    {challengeCount, patternMatchCount, hasHighSeverity, straightforward}\n\
          }\n\n\
        EXIT CODES: 0 success; 1 general/arg errors; 2 connection; 3 target; 4 timeout; 5 protocol.",
    after_long_help = "\
EXAMPLES:
  # Navigate to a URL and diagnose it
  agentchrome diagnose https://example.com/course

  # Diagnose the already-loaded page in the active tab
  agentchrome diagnose --current

  # Combine with other commands referenced in suggestions
  agentchrome diagnose --current | jq -r '.patterns[].suggestion'"
)]
Diagnose(DiagnoseArgs),
```

```rust
// src/cli/mod.rs — new args struct

#[derive(Args)]
pub struct DiagnoseArgs {
    /// URL to navigate to and diagnose (mutually exclusive with --current)
    #[arg(conflicts_with = "current")]
    pub url: Option<String>,

    /// Diagnose the already-loaded page in the active tab without navigating
    /// (mutually exclusive with <url>)
    #[arg(long, conflicts_with = "url")]
    pub current: bool,

    /// Wait strategy after navigation (URL mode only; ignored with --current)
    #[arg(long, value_enum, default_value_t = WaitUntil::Load)]
    pub wait_until: WaitUntil,

    /// Navigation timeout in milliseconds (URL mode only; ignored with --current)
    #[arg(long)]
    pub timeout: Option<u64>,
}
```

**XOR enforcement**: `conflicts_with = "url"`/`"current"` covers the "both supplied" case (AC9). The "neither supplied" case (AC8) is enforced at runtime at the top of `execute_diagnose`, returning `AppError { code: ExitCode::GeneralError, … }` — clap cannot express "at least one of two optional args" natively without a `ArgGroup { required = true }`. Using `ArgGroup`:

```rust
#[command(group(ArgGroup::new("target").required(true).args(["url", "current"])))]
pub struct DiagnoseArgs { … }
```

`ArgGroup { required = true }` enforces AC8 at parse time (produces a clap error which `main.rs` converts to JSON on stderr with exit code 1), so no runtime check is needed. **Selected**: use `ArgGroup`.

### Output Schema

```json
{
  "url": "https://example.com/course",
  "scope": "diagnosed",
  "navigationStatus": 200,
  "challenges": [
    {
      "category": "iframes",
      "severity": "medium",
      "summary": "2 iframes detected (1 cross-origin)",
      "details": {
        "count": 2,
        "crossOriginCount": 1,
        "items": [
          {
            "index": 1,
            "url": "https://cdn.example.com/player",
            "name": "player",
            "visible": true,
            "width": 960,
            "height": 540,
            "crossOrigin": true,
            "interactiveElementCount": null
          }
        ]
      },
      "suggestion": "Use --frame <index> on page/interact commands to target content inside iframes. For cross-origin frames, use 'interact click-at --frame N' with coordinate targeting (selector targeting is unavailable)."
    },
    {
      "category": "overlays",
      "severity": "high",
      "summary": "1 viewport-covering overlay with z-index 9999 covers interactive elements",
      "details": {
        "items": [
          {
            "selector": "div.acc-blocker",
            "zIndex": 9999,
            "width": 1920,
            "height": 1080,
            "coversInteractive": true
          }
        ]
      },
      "suggestion": "Large overlays intercept clicks. Try 'interact click-at' with explicit coordinates inside the real content area, or target the obscured element via its iframe using --frame."
    },
    {
      "category": "canvas",
      "severity": "low",
      "summary": "1 <canvas> element with WebGL context",
      "details": {
        "canvasCount": 1,
        "webglCount": 1,
        "items": [
          { "width": 800, "height": 600, "context": "webgl2" }
        ]
      },
      "suggestion": "Canvas-rendered UI is not accessible via DOM. Use 'interact click-at'/'interact key' with coordinate targeting; the accessibility tree will be sparse."
    }
  ],
  "patterns": [
    {
      "name": "storyline-acc-blocker",
      "matched": true,
      "confidence": "high",
      "evidence": "div.acc-blocker covers 100% of viewport at z-index 9999; #story_content element present",
      "suggestion": "Articulate Storyline renders course content inside an iframe and shields the main frame with an acc-blocker. Target content with 'interact click-at --frame N' where N is the Storyline iframe index shown in 'challenges.iframes.details.items'."
    }
  ],
  "summary": {
    "challengeCount": 3,
    "patternMatchCount": 1,
    "hasHighSeverity": true,
    "straightforward": false
  }
}
```

**Field-nullability contract** (per FR21 / AC7):
- `navigationStatus` — omitted with `serde(skip_serializing_if = "Option::is_none")` when a current-mode scan or when the network response did not expose a status. (Distinct from `null`; omission means "not applicable".)
- `interactiveElementCount` on iframe items — serialized as `null` when the frame's execution context is inaccessible (cross-origin). Never coerced to 0.
- Any challenge `details.*` field that cannot be measured — serialized as `null`. Categories with zero measured data do not appear as a challenge at all (they're excluded from the `challenges` array).
- `patterns` — only `matched == true` entries are emitted. No `matched: false` placeholders.

### Errors

| Code | Condition |
|------|-----------|
| `ExitCode::GeneralError` (1) | Missing both `<url>` and `--current`; supplying both; invalid clap argument |
| `ExitCode::ConnectionError` (2) | No active Chrome session |
| `ExitCode::TargetError` (3) | Active tab cannot be resolved (e.g., no page targets) |
| `ExitCode::TimeoutError` (4) | Navigation did not complete before timeout (URL mode only) |
| `ExitCode::ProtocolError` (5) | CDP protocol failure during navigation or a required detector round trip |

Error shape matches the global agentchrome contract: exactly one JSON error object on stderr with `message` + `code` fields, empty stdout, process exit matches `code`.

---

## Module Layout

```
src/
├── main.rs                       (MODIFY — add Command::Diagnose dispatch)
├── cli/mod.rs                    (MODIFY — add Command::Diagnose variant + DiagnoseArgs)
├── diagnose/                     (NEW MODULE DIRECTORY)
│   ├── mod.rs                    execute_diagnose() orchestrator
│   ├── output.rs                 DiagnoseResult / Challenge / PatternMatch / Summary types
│   ├── patterns.rs               static PATTERN_DB + match_all() function
│   └── detectors.rs              canvas/WebGL, media-gate, framework-quirk detectors
├── navigate.rs                   (MODIFY — extract navigate_and_wait() as pub(crate) helper)
├── page/
│   ├── analyze.rs                (MODIFY — make detector functions pub(crate); re-export via
│   │                              `pub(crate) mod detectors`)
│   └── mod.rs                    (MODIFY — re-export analyze::detectors if needed)
├── examples.rs                   (MODIFY — add "diagnose" command group with examples)
└── ...

tests/
├── bdd.rs                        (MODIFY — add DiagnoseWorld + step defs)
├── features/
│   └── diagnose.feature          (NEW — BDD scenarios for all ACs)
└── fixtures/
    └── diagnose.html             (NEW — test site covering Storyline-like overlay + iframe +
                                   canvas + media gate)

xtask/                            (NO CHANGE — man page auto-generated from clap metadata)

README.md                         (MODIFY — Command Reference row + Usage Examples entry)
```

---

## Implementation Details

### Navigation Reuse

Extract the URL navigation + wait logic from `navigate::execute_url` into a new `pub(crate)` helper:

```rust
// src/navigate.rs — new helper
pub(crate) async fn navigate_and_wait(
    managed: &mut ManagedSession,
    url: &str,
    wait_until: WaitUntil,
    timeout_ms: u64,
) -> Result<NavigateResult, AppError> {
    // body: existing logic extracted from execute_url, minus the session setup
    // and minus the print_output call
}
```

`execute_url` is refactored to call `navigate_and_wait` so `navigate <url>` and `diagnose <url>` share one code path. Error classifications (timeout → `ExitCode::TimeoutError`, protocol errors → `ExitCode::ProtocolError`) are produced here, not duplicated in `diagnose`.

### Shared Detector Surface

Rename `page/analyze.rs`'s private detector functions to `pub(crate)` and gather them under a sub-module path:

```rust
// src/page/analyze.rs — extracted surface (post-refactor)
pub(crate) mod detectors {
    pub(crate) async fn enumerate_iframes(...) -> Vec<IframeInfo> { ... }
    pub(crate) async fn detect_frameworks(...) -> Vec<String> { ... }
    pub(crate) async fn count_interactive_elements(...) -> u32 { ... }
    pub(crate) async fn catalog_media(...) -> Vec<MediaInfo> { ... }
    pub(crate) async fn detect_overlays(...) -> Vec<OverlayInfo> { ... }
    pub(crate) async fn detect_shadow_dom(...) -> ShadowDomInfo { ... }
}

pub(crate) use detectors::*; // keep existing call sites in analyze.rs working
```

`IframeInfo`, `MediaInfo`, `OverlayInfo`, `ShadowDomInfo` also become `pub(crate)` and are imported into `src/diagnose/` as shared types. `AnalyzeResult` stays private to `page::analyze`.

### Canvas / WebGL Detection (new, diagnose-only)

```rust
// src/diagnose/detectors.rs
pub(crate) async fn detect_canvas(session: &ManagedSession) -> Option<CanvasInfo> { ... }

pub(crate) struct CanvasInfo {
    pub canvas_count: u32,
    pub webgl_count: u32,            // canvases with a webgl or webgl2 context
    pub items: Vec<CanvasItem>,
}

pub(crate) struct CanvasItem {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub context: Option<String>,     // "webgl" | "webgl2" | "2d" | null
}
```

Detection JS (one CDP round trip):
```javascript
Array.from(document.querySelectorAll('canvas')).map(c => {
  let ctx = null;
  // Try detection without creating a new context: if an existing context is
  // cached, getContext returns it; otherwise returns null for unknown types.
  // We query in order of "most informative first" but do NOT create a fresh
  // context if one doesn't exist (avoids side effects on the page).
  try {
    if (c.getContext('webgl2', { desynchronized: true })) ctx = 'webgl2';
    else if (c.getContext('webgl')) ctx = 'webgl';
    else if (c.getContext('2d')) ctx = '2d';
  } catch (e) {}
  const rect = c.getBoundingClientRect();
  return {
    width: Math.round(rect.width) || null,
    height: Math.round(rect.height) || null,
    context: ctx,
  };
})
```

**Side-effect caveat**: `getContext` is generally idempotent for the same type after first call, but calling multiple types on a canvas that has never had a context can pin a 2D context where the page would have used WebGL. To avoid this, the JS calls `c.getContext('webgl2')` **first** — if the canvas is designed for WebGL it will have it, and 2D is checked only last. Risk tradeoff documented under Risks & Mitigations.

### Media Gate Classification (diagnose-only refinement)

`page::analyze::detectors::catalog_media` already returns paused/playing/ended. `diagnose` enriches each media entry with a `gatesNavigation: bool` flag derived from:
- `autoplay` attribute present + currently paused → likely user-interaction-gated
- `muted == false` + `currentTime == 0` + visible rect → likely a start-gate audio/video

This enrichment is a pure function over the output of `catalog_media` — no extra CDP round trip.

### Framework Quirk Detection (diagnose-only)

A second JS eval (one round trip) probes for named quirks:
- **React portal** — DOM contains elements mounted outside `document.getElementById('root')` that expose React fiber properties
- **Angular zone.js** — `window.Zone` present
- **Vue teleport** — `data-v-*` attributes on elements outside any declared root mount
- **Svelte hydration** — `<!--[-->` / `<!--]-->` HTML comments present

Result shape:
```rust
pub(crate) struct FrameworkQuirks {
    pub react_portal: bool,
    pub angular_zone: bool,
    pub vue_teleport: bool,
    pub svelte_hydration: bool,
}
```

### Pattern Database

```rust
// src/diagnose/patterns.rs

pub(crate) struct PatternRule {
    pub name: &'static str,
    pub detector: fn(&DetectorBundle) -> Option<PatternMatch>,
}

pub(crate) struct DetectorBundle<'a> {
    pub iframes: &'a [IframeInfo],
    pub frameworks: &'a [String],
    pub overlays: &'a [OverlayInfo],
    pub shadow_dom: &'a ShadowDomInfo,
    pub media: &'a [MediaInfo],
    pub canvas: Option<&'a CanvasInfo>,
    pub framework_quirks: &'a FrameworkQuirks,
}

pub(crate) static PATTERN_DB: &[PatternRule] = &[
    PatternRule {
        name: "storyline-acc-blocker",
        detector: detect_storyline_acc_blocker,
    },
    PatternRule {
        name: "scorm-player",
        detector: detect_scorm_player,
    },
    PatternRule {
        name: "react-portal",
        detector: detect_react_portal,
    },
];

pub(crate) fn match_all(bundle: &DetectorBundle) -> Vec<PatternMatch> {
    PATTERN_DB.iter()
        .filter_map(|rule| (rule.detector)(bundle))
        .collect()
}
```

Example detector (`detect_storyline_acc_blocker`):
```rust
fn detect_storyline_acc_blocker(b: &DetectorBundle) -> Option<PatternMatch> {
    let has_storyline_framework = b.frameworks.iter().any(|f| f == "Storyline");
    let acc_blocker = b.overlays.iter().find(|o| o.selector.contains(".acc-blocker"))?;
    let confidence = if has_storyline_framework && acc_blocker.covers_interactive {
        "high"
    } else if acc_blocker.covers_interactive {
        "medium"
    } else {
        "low"
    };
    Some(PatternMatch {
        name: "storyline-acc-blocker",
        matched: true,
        confidence: confidence.into(),
        evidence: format!(
            "{} covers a {}×{}px region at z-index {}{}",
            acc_blocker.selector, acc_blocker.width, acc_blocker.height, acc_blocker.z_index,
            if has_storyline_framework { "; Storyline framework signature detected" } else { "" }
        ),
        suggestion: "Articulate Storyline renders course content inside an iframe and shields the \
            main frame with an acc-blocker overlay. Target the content iframe directly with \
            'interact click-at --frame N' where N is the Storyline iframe index (see \
            challenges.iframes.details.items).".into(),
    })
}
```

### Severity Assignment

| Category | `high` | `medium` | `low` |
|----------|--------|----------|-------|
| `iframes` | ≥1 cross-origin frame AND any frame is visible | ≥1 same-origin iframe | (never on its own) |
| `overlays` | Any overlay with `coversInteractive: true` AND area ≥ 75% of viewport | Any overlay with `coversInteractive: true` | Overlays exist but do not cover interactive elements |
| `shadowDom` | `hostCount ≥ 10` | `1 ≤ hostCount ≤ 9` | (n/a — excluded when not present) |
| `canvas` | WebGL context present AND canvas area ≥ 50% of viewport | WebGL or 2D context present | (n/a) |
| `media` | ≥1 media element with `gatesNavigation: true` | ≥1 playing/paused media element (no gate) | (n/a) |
| `framework` | ≥2 framework quirks true | 1 framework quirk true | (n/a) |

`hasHighSeverity = challenges.iter().any(|c| c.severity == "high")`.

### Challenge Assembly

A challenge entry is **emitted only if its category yields non-empty data**. This guarantees AC3 (clean pages produce `challenges: []`) and avoids cluttering the output with placeholder rows.

```rust
fn assemble_challenges(bundle: &DetectorBundle) -> Vec<Challenge> {
    let mut out = Vec::new();
    if !bundle.iframes.is_empty() { out.push(build_iframes_challenge(bundle)); }
    if !bundle.overlays.is_empty() { out.push(build_overlays_challenge(bundle)); }
    if bundle.shadow_dom.present { out.push(build_shadow_dom_challenge(bundle)); }
    if let Some(cv) = bundle.canvas { if cv.canvas_count > 0 { out.push(build_canvas_challenge(cv)); } }
    if !bundle.media.is_empty() { out.push(build_media_challenge(bundle)); }
    if bundle.framework_quirks.any() { out.push(build_framework_challenge(bundle)); }
    out
}
```

Each `build_*_challenge` function is a pure transformation — no CDP round trips.

### Suggestion Strings

Per-category suggestions are `&'static str` constants defined at the top of `src/diagnose/detectors.rs`. They are the **only** surface where interaction strategy text lives, keeping the guidance discoverable and editable in one place. Each suggestion must reference at least one concrete agentchrome command (this is enforced by a compile-time test; see Testing Strategy).

### Active Tab Resolution for `--current`

`execute_diagnose` uses the existing `setup_session_with_interceptors` helper, which applies `--tab` / `--page-id` if set or falls back to auto-discovering the active tab — identical to `page snapshot` behavior. No new target-resolution logic is introduced.

### URL-Mode Navigation Flow

```rust
pub async fn execute_diagnose(
    global: &GlobalOpts,
    args: &DiagnoseArgs,
) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs { managed.spawn_auto_dismiss().await?; }

    let navigation_status = if let Some(ref url) = args.url {
        let result = navigate::navigate_and_wait(
            &mut managed,
            url,
            args.wait_until,
            args.timeout.unwrap_or(navigate::DEFAULT_NAVIGATE_TIMEOUT_MS),
        ).await?;
        result.status
    } else {
        // --current mode
        None
    };

    let scope = if args.url.is_some() { "diagnosed" } else { "current" };
    run_diagnosis(&mut managed, scope, navigation_status).await.map(|result| {
        print_output(global, &result)
    })?
}
```

`run_diagnosis` drives all detectors and assembles the `DiagnoseResult`.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Extend `page analyze` in place** | Add `--patterns`/`--strategies` flags to `page analyze` instead of introducing `diagnose` | One command, less CLI surface | Breaks `page analyze`'s "structural enumeration" contract (#190 AC1); mixes two concerns; no natural home for `<url>` positional navigation | Rejected — two commands with clear mandates is more AI-agent-friendly |
| **B: Top-level `diagnose` command, reusing `page::analyze` detectors** | New `src/diagnose/` module, extract `page::analyze` detectors as `pub(crate)`, add pattern DB + strategy suggestions | Clear separation of concerns; reuses existing detectors; minimal duplication; accepts `<url>` naturally | Requires a small refactor of `page::analyze` to expose detector functions | **Selected** — cleanest separation with near-zero duplication |
| **C: Subcommand under `page`: `page diagnose`** | `agentchrome page diagnose [<url>]` | Groups related inspection commands | The `page` group shares a `--frame` arg and "operate on current page" semantics; `diagnose <url>` (which navigates) would violate both; mixing navigation into `page` is out of scope | Rejected |
| **D: Runtime-loaded pattern database (TOML/JSON file)** | Load pattern rules from a user-editable config at runtime | Users can add patterns without rebuilding | Adds file-system I/O in CLI hot path; complicates binary distribution; no clear user demand yet | Rejected for first release; Could-priority follow-up (FR22) |
| **E: Single-JS-eval monolithic detector** | Run all detection in one massive `Runtime.evaluate` call | One CDP round trip | No graceful degradation; debugging nightmare; hit evaluation size limits on complex pages; duplicates work `page::analyze` already does correctly | Rejected for the same reason `page::analyze` rejected it (see its design Alternative A) |

---

## Security Considerations

- [x] **No user credentials touched**: `diagnose` does not read cookies, local storage, or form contents.
- [x] **No DOM mutation**: Detection is read-only. `getContext` calls on canvases are the only operation that could affect page state; see Risk R1 below.
- [x] **Cross-origin iframes respected**: Same-origin policy is honored — cross-origin frame internals are reported as `null`, never bypassed or spoofed.
- [x] **No external network calls**: All detection occurs via CDP against the already-loaded page. No additional fetch/xhr.
- [x] **No dialog interception**: Existing `--auto-dismiss-dialogs` global flag is honored; `diagnose` does not add any dialog-specific logic.
- [x] **URL-mode navigation uses the same trust model as `navigate <url>`**: no additional privilege.

---

## Performance Considerations

- [x] **Round-trip budget**: Detection requires ≤ 8 CDP round trips in the worst case (6 from reused `page::analyze` detectors + canvas + framework quirks). On a benchmark page with 1,000 DOM elements, this completes in < 3 seconds in headless Chrome.
- [x] **No polling**: Detection is one-shot; navigation wait uses the existing `navigate` polling which already has bounded per-attempt latency.
- [x] **Navigation timeout honored**: `--timeout` is passed through to `navigate_and_wait`, so slow pages fail with `ExitCode::TimeoutError` within budget rather than hanging.
- [x] **Pattern matching is free**: Pattern detectors operate on already-collected data; no additional CDP traffic except for framework-quirk detection (1 round trip).
- [x] **No persistent state**: `diagnose` is stateless — no cache, no session file updates beyond what `setup_session` already does.

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Output types | Unit | `DiagnoseResult` serialization: camelCase field names, `skip_serializing_if` on `navigationStatus`, `Option<u32>` → `null` on unmeasurable counts |
| Severity assignment | Unit | Each category's severity table covered by parameterized tests |
| Pattern detectors | Unit | Each pattern's detector called with synthetic `DetectorBundle` inputs matching its positive/negative/edge cases |
| Suggestion-actionability lint | Unit | Compile-time test scanning all static suggestion strings for `agentchrome` command references (enforces FR11 / success metric) |
| Challenge assembly | Unit | `assemble_challenges` omits empty categories (AC3) |
| URL vs `--current` parse | Unit | clap `ArgGroup` rejects missing (AC8), both (AC9); accepts one or the other |
| Navigation error propagation | Integration | Mock CDP transport returns timeout/protocol error; verify exit codes match `navigate <url>` behavior (AC10) |
| End-to-end (AC1–AC5, AC7, AC12) | BDD (cucumber-rs) | `tests/features/diagnose.feature`, `tests/fixtures/diagnose.html` |
| Cross-origin iframe (AC6) | BDD | Fixture uses a `srcdoc` cross-origin iframe (or `file://` + `about:blank`) to exercise `null` interactive count |
| Argument errors (AC8, AC9) | BDD | Invocation-layer scenarios |
| No active session (AC11) | BDD | Precondition: session file absent; assert exit code 2 and JSON error |
| Manual smoke test | Required during `/verifying-specs` | Launch headless Chrome, navigate to `tests/fixtures/diagnose.html`, run `diagnose --current` + `diagnose file://…`; confirm all ACs pass against real Chrome |
| Clippy / rustfmt | Gate | `cargo clippy --all-targets` clean; `cargo fmt --check` clean |

---

## Risks & Mitigations

| ID | Risk | Likelihood | Impact | Mitigation |
|----|------|------------|--------|------------|
| R1 | `getContext('webgl2')` probe on a canvas that never had a context may pin a WebGL2 context where the page would have used a different type | Low | Low | Probe in `webgl2 → webgl → 2d` order; `getContext` with `{desynchronized: true}` hints to the browser that we're not committing to the context; document as a known limitation in long-help. For pages that care, use `page analyze` which does not probe contexts. |
| R2 | Extracting `page::analyze::detectors` as `pub(crate)` couples `diagnose` to `page analyze` internals and raises the cost of future `page analyze` refactors | Medium | Low | Keep the exposed surface small (function signatures only, not internal helpers); cover both callers with tests so refactors are caught immediately. |
| R3 | Pattern matcher false positives (e.g., a non-Storyline page that happens to have `div.acc-blocker`) | Medium | Medium | Require multi-signal matches (framework signature AND overlay signature) for `high` confidence; report low/medium for single-signal matches so agents can use confidence to decide trust. |
| R4 | Navigation error shape drift between `navigate <url>` and `diagnose <url>` | Low | Medium | Share one `navigate_and_wait` helper; cover both entry points with the same error-classification tests. |
| R5 | Pattern DB grows indefinitely and becomes hard to audit | Low | Low | First release is static Rust code with ≤ 10 patterns; FR22's "compile-time static" restriction delays the externalization question until there's real demand. |
| R6 | `ArgGroup { required = true }` produces a clap error message that doesn't clearly say "pass a URL or --current" | Low | Low | Unit-test the error message on missing/both combinations; tune the clap `long_about` wording if needed. |
| R7 | Canvas detection JS is blocked by CSP (`script-src`) on some pages | Low | Low | `Runtime.evaluate` runs in the isolated world and bypasses page CSP; no mitigation required. Documented in-line for future maintainers. |

---

## Open Questions

- [ ] Should suggestions be localized in the future (i18n)? **Resolution**: Not for this release — strings are `&'static str`. A follow-up issue can introduce a lookup table if needed.
- [ ] Should the pattern matcher surface "near misses" (matched 2 of 3 required signals) as diagnostic hints? **Resolution**: Defer to a follow-up — first release emits only successful matches to keep output signal-dense.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #200 | 2026-04-16 | Initial feature design |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (top-level command peer to `navigate`/`audit`; module under `src/diagnose/`)
- [x] All CLI changes documented with clap derive snippets and `ArgGroup` enforcement
- [x] Output schema documented with example JSON and field-nullability contract
- [x] No database/storage changes needed
- [x] State management approach is clear (stateless)
- [x] No UI components needed (CLI-only)
- [x] Security considerations addressed (cross-origin, CSP, no mutation beyond canvas probe caveat)
- [x] Performance impact analyzed (≤ 8 CDP round trips + 1 navigation)
- [x] Testing strategy defined across unit / BDD / smoke / lint surfaces
- [x] Alternatives were considered and documented with reasons
- [x] Risks identified with mitigations, including the `getContext` probe caveat
- [x] Navigation + error-shape reuse with `navigate <url>` is explicit
- [x] Shared detector refactor scope bounded (function signatures only; types become `pub(crate)`)
