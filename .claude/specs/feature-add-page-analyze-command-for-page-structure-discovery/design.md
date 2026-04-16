# Design: Page Analyze Command for Page Structure Discovery

**Issues**: #190
**Date**: 2026-04-16
**Status**: Approved
**Author**: Claude (spec-writer)

---

## Overview

The `page analyze` command will be implemented as a new subcommand in the existing `page` command group, following the same architectural pattern as `page hittest`. It introduces a new module `src/page/analyze.rs` that orchestrates six analysis dimensions — iframe enumeration, framework detection, interactive element counting, media cataloging, overlay detection, and shadow DOM presence — into a single structured JSON output.

The command leverages existing infrastructure heavily: `frame::list_frames()` for iframe enumeration, the `--frame` argument on `PageArgs` for frame-scoped analysis, and the CDP DOM/Runtime/Page domains that other page commands already use. The primary new logic is the framework detection heuristics (JS-based DOM signature checks), the viewport-wide overlay scanner (extending hittest's point-based approach to a spatial scan), and the media element cataloging.

Each analysis dimension runs independently and uses graceful degradation — if one dimension fails (e.g., framework detection on a sandboxed iframe), it reports `null` for that dimension and continues with the rest, ensuring the command never crashes on unexpected page configurations.

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────┐
│                      CLI Layer                            │
│  cli/mod.rs: PageCommand::Analyze variant (no extra args) │
└───────────────────────────┬──────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│                   Command Dispatch                        │
│  page/mod.rs: execute_page() → analyze::execute_analyze() │
└───────────────────────────┬──────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│                  page/analyze.rs                          │
│  ┌─────────────┐ ┌─────────────┐ ┌──────────────┐       │
│  │  Iframe Enum │ │  Framework  │ │  Interactive  │       │
│  │ (frame.rs)  │ │  Detection  │ │  Elem Count  │       │
│  └─────────────┘ └─────────────┘ └──────────────┘       │
│  ┌─────────────┐ ┌─────────────┐ ┌──────────────┐       │
│  │ Media Catalog│ │  Overlay    │ │  Shadow DOM  │       │
│  │             │ │  Scanner    │ │  Detection   │       │
│  └─────────────┘ └─────────────┘ └──────────────┘       │
│                         │                                │
│                    AnalyzeResult                          │
└───────────────────────────┬──────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│               CDP Client (DOM, Runtime, Page)             │
└───────────────────────────┬──────────────────────────────┘
                            │
                            ▼
                    Chrome Browser
```

### Data Flow

```
1. User runs: agentchrome page analyze [--frame N]
2. CLI parses args → PageCommand::Analyze dispatched
3. execute_analyze() sets up session, resolves optional frame context
4. Enable CDP domains: DOM, Runtime, Page
5. Run analysis dimensions sequentially:
   a. Iframe enumeration via frame::list_frames() + visibility/dimension augmentation
   b. Framework detection via Runtime.evaluate (JS signature checks)
   c. Interactive element counting via Runtime.evaluate (querySelectorAll)
   d. Media element cataloging via Runtime.evaluate (video/audio/embed query)
   e. Overlay detection via Runtime.evaluate (position/z-index scan)
   f. Shadow DOM detection via Runtime.evaluate (shadowRoot host scan)
6. Assemble AnalyzeResult with summary aggregates
7. print_output() writes JSON to stdout
8. Exit code 0 on success
```

---

## API / Interface Changes

### New CLI Variant

| Variant | Parent | Args | Purpose |
|---------|--------|------|---------|
| `PageCommand::Analyze` | `PageCommand` enum | None (uses shared `--frame` from `PageArgs`) | Structural analysis of page |

### CLI Definition (clap derive)

```rust
/// Analyze page structure: iframes, frameworks, overlays, media, shadow DOM
#[command(
    long_about = "Analyze the structural composition of the current page. Returns a JSON \
        report covering iframe hierarchy, detected frontend frameworks, interactive element \
        counts, media elements, overlay blockers, and shadow DOM presence. Useful for \
        understanding an unfamiliar page before choosing an automation strategy.",
    after_long_help = "\
EXAMPLES:
  # Analyze current page
  agentchrome page analyze

  # Analyze within a specific iframe
  agentchrome page analyze --frame 1"
)]
Analyze,
```

### Output Schema

```json
{
  "scope": "main",
  "url": "https://example.com",
  "title": "Example Page",
  "iframes": [
    {
      "index": 1,
      "url": "https://example.com/child",
      "name": "child-frame",
      "visible": true,
      "width": 800,
      "height": 600,
      "crossOrigin": false
    }
  ],
  "frameworks": ["React"],
  "interactiveElements": {
    "main": 15,
    "frames": {
      "1": 8
    }
  },
  "media": [
    {
      "tag": "video",
      "src": "video.mp4",
      "state": "paused",
      "width": 640,
      "height": 480
    }
  ],
  "overlays": [
    {
      "selector": "div#blocker",
      "zIndex": 9999,
      "width": 1280,
      "height": 720,
      "coversInteractive": true
    }
  ],
  "shadowDom": {
    "present": true,
    "hostCount": 3
  },
  "summary": {
    "iframeCount": 1,
    "interactiveElementCount": 23,
    "hasOverlays": true,
    "hasMedia": true,
    "hasShadowDom": true,
    "hasFrameworks": true
  }
}
```

### Errors

| Code | Condition |
|------|-----------|
| `ExitCode::ConnectionError` (2) | No active Chrome session |
| `ExitCode::TargetError` (3) | Invalid --frame index |
| `ExitCode::ProtocolError` (5) | CDP domain failures |

---

## State Management

No new persistent state. The command is stateless — each invocation performs a fresh analysis against the current page DOM. It reuses existing session management via `setup_session()`.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Single JS evaluation** | Run all analysis in one large `Runtime.evaluate` call | Fewer CDP round trips; faster | Hard to debug; no graceful degradation per dimension; JS payload too large | Rejected — individual failures would crash entire analysis |
| **B: Separate JS per dimension** | Run one `Runtime.evaluate` per analysis dimension | Graceful degradation; each dimension isolated; debuggable | More CDP round trips (6 total) | **Selected** — reliability outweighs the minor latency cost |
| **C: Parallel CDP calls** | Use `tokio::join!` to run all dimensions concurrently | Fastest possible; all dimensions in parallel | CDP session is inherently sequential (single WebSocket); risk of interleaved responses | Rejected — CDP protocol constraint makes true parallelism unsafe |

---

## Implementation Details

### Framework Detection

Each framework is detected via a specific DOM/window signature evaluated through `Runtime.evaluate`:

| Framework | Detection Expression |
|-----------|---------------------|
| React | `typeof window.__REACT_DEVTOOLS_GLOBAL_HOOK__ !== 'undefined' \|\| document.querySelector('[data-reactroot]') !== null` |
| Angular | `document.querySelector('[ng-version]') !== null \|\| typeof window.ng !== 'undefined'` |
| Vue | `typeof window.__VUE__ !== 'undefined' \|\| document.querySelector('[data-v-]') !== null` |
| Svelte | `document.querySelector('[class*="svelte-"]') !== null` |
| Storyline | `document.getElementById('story_content') !== null` |
| SCORM | `typeof window.API !== 'undefined' \|\| typeof window.API_1484_11 !== 'undefined'` |

All checks run in a single `Runtime.evaluate` call that returns an array of detected framework names.

### Overlay Detection

Unlike `hittest` (which checks a single point), `analyze` needs to detect overlays across the viewport. The approach:

1. Query all elements with `position: fixed` or `position: absolute` and `z-index > 0` via `Runtime.evaluate`
2. For each candidate, check if its bounding rect covers a significant portion of the viewport (> 50% area)
3. Check if interactive elements exist beneath it (using the same interactive tag list from hittest)
4. Report candidates that both cover significant viewport area AND have interactive elements beneath them

This avoids the cost of probing every coordinate while catching the common overlay patterns (full-page modals, cookie banners, acc-blocker divs).

### Interactive Element Counting

Uses `querySelectorAll` with the standard interactive element selectors:
```javascript
document.querySelectorAll('a[href], button, input, select, textarea, [role="button"], [role="link"], [role="checkbox"], [role="radio"], [role="tab"], [tabindex]:not([tabindex="-1"])').length
```

For same-origin iframes, the same query runs in each frame's execution context. For cross-origin iframes, the count is reported as `null` if the frame's execution context is inaccessible.

### Media Element Cataloging

Queries `<video>`, `<audio>`, and `<embed>` elements, extracting:
- Tag name
- `src` or `currentSrc` attribute
- Playback state: `paused` → "paused", `ended` → "ended", else "playing" (for video/audio); `null` for embed
- Dimensions from `getBoundingClientRect()`

### Shadow DOM Detection

Iterates all elements in the document and checks for `.shadowRoot` presence:
```javascript
Array.from(document.querySelectorAll('*')).filter(el => el.shadowRoot).length
```

Returns `{ present: count > 0, hostCount: count }`.

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Output types | Unit | `AnalyzeResult` serialization, field naming (camelCase) |
| Framework detection | Unit | Each framework's JS expression returns correct results |
| Overlay detection | Unit | Coverage calculation, interactive-beneath check |
| Command execution | BDD (cucumber-rs) | All 7 acceptance criteria as Gherkin scenarios |
| Smoke test | Manual | Real Chrome against test fixture HTML |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Framework detection false positives (e.g., page has `window.API` for non-SCORM reasons) | Medium | Low | Detection checks use multiple signals where possible; document known limitations |
| Overlay detection misses fixed-position overlays outside viewport | Low | Low | Only overlays covering the current viewport are relevant for automation |
| Cross-origin iframe analysis limited by same-origin policy | Medium | Medium | Report `null` for inaccessible dimensions; `crossOrigin: true` flag alerts the user |
| Large DOM pages slow down analysis | Low | Medium | Individual JS evaluations have internal timeouts; overall command timeout via `--timeout` global flag |

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #190 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] State management approach is clear (stateless)
- [x] No UI components needed (CLI-only)
- [x] Security considerations addressed (cross-origin iframe handling)
- [x] Performance impact analyzed (sequential CDP calls, graceful degradation)
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
