# Design: Screenshot Capture

**Issue**: #12
**Date**: 2026-02-12
**Status**: Approved
**Author**: Claude

---

## Overview

This feature adds the `page screenshot` subcommand to capture visual screenshots of browser pages. It uses CDP's `Page.captureScreenshot` method directly, supporting viewport, full-page, element-targeted (via CSS selector or accessibility UID), and region-clipped captures in PNG, JPEG, and WebP formats.

The implementation follows the established command patterns from `page.rs`: resolve connection, create CDP session via `setup_session()`, enable required domains, execute CDP commands, and format output. Element screenshots reuse `DOM.getBoxModel` (already used by `page find`) to compute clip regions. Full-page screenshots temporarily resize the viewport to the full document dimensions, capture, then restore. The `--uid` option resolves backend DOM node IDs from the persisted snapshot state (`~/.chrome-cli/snapshot.json`).

No new external dependencies are needed — base64 encoding is handled by CDP itself (`Page.captureScreenshot` returns base64 data). Image dimensions are obtained from the CDP response metadata or computed from the clip region.

---

## Architecture

### Component Diagram

```
CLI Layer (cli/mod.rs)
  └── PageCommand::Screenshot(PageScreenshotArgs)
        ↓
Command Layer (page.rs)
  └── execute_screenshot()
        ↓
┌──────────────────────────────────────────────────────────────────┐
│                    Screenshot Strategy                            │
│                                                                   │
│  ┌───────────────────┐  ┌──────────────────┐  ┌──────────────┐  │
│  │  Viewport (default)│  │   Full-Page      │  │   Element    │  │
│  │                    │  │                   │  │              │  │
│  │  Page.capture-     │  │  get scroll dims │  │  DOM.getBox- │  │
│  │   Screenshot       │  │  resize viewport │  │   Model      │  │
│  │   (no clip)        │  │  capture         │  │  → clip      │  │
│  │                    │  │  restore viewport │  │  → capture   │  │
│  └───────────────────┘  └──────────────────┘  └──────────────┘  │
│                                                                   │
│  ┌───────────────────┐                                           │
│  │   Region (--clip) │                                           │
│  │                    │                                           │
│  │  Parse X,Y,W,H    │                                           │
│  │  → clip parameter  │                                           │
│  └───────────────────┘                                           │
└──────────────────────────────────────────────────────────────────┘
        ↓
Output Layer
  ├── --file: decode base64, write binary to disk
  └── no --file: JSON { format, data, width, height }
```

### Data Flow

```
1. User runs: chrome-cli page screenshot [OPTIONS]
2. CLI parses args into PageScreenshotArgs
3. Validates mutual exclusion (--full-page vs --selector/--uid)
4. Resolves connection and target tab via setup_session()
5. Enables Page domain (and DOM if element targeting)
6. Determines capture strategy:
   a. Element (--selector or --uid):
      - Resolve element bounding box via DOM.getBoxModel
      - Set clip parameter to bounding box
   b. Full-page (--full-page):
      - Get page dimensions via Runtime.evaluate (scrollWidth, scrollHeight)
      - Get current viewport via Emulation.setDeviceMetricsOverride
      - Set viewport to page dimensions
      - Set captureBeyondViewport: true
   c. Region (--clip):
      - Parse "X,Y,WIDTH,HEIGHT" string
      - Set clip parameter
   d. Viewport (default):
      - No clip, no viewport changes
7. Call Page.captureScreenshot with { format, quality, clip }
8. If full-page: restore original viewport
9. Get image dimensions:
   - From clip region if clipped
   - From viewport metrics if viewport capture
   - Via Runtime.evaluate for full-page (scrollWidth, scrollHeight)
10. Output:
    - With --file: base64-decode data, write binary to path, output JSON with file/format/width/height
    - Without --file: output JSON with data/format/width/height
    - Warn to stderr if base64 data > 10MB
```

---

## API / Interface Changes

### New CLI Subcommand

```
chrome-cli page screenshot [OPTIONS]

Options:
  --full-page           Capture the entire scrollable page
  --selector <CSS>      Screenshot a specific element by CSS selector
  --uid <UID>           Screenshot a specific element by accessibility UID
  --format <FORMAT>     Image format: png (default), jpeg, webp
  --quality <N>         JPEG/WebP quality (0-100) [default: 80]
  --file <PATH>         Save screenshot to file (outputs binary image)
  --clip <X,Y,W,H>     Capture a specific viewport region
```

Global flags `--tab`, `--json`, `--pretty`, `--timeout` all apply as usual.

### Output Schema

**Base64 mode (no --file, default):**

```json
{
  "format": "png",
  "data": "iVBORw0KGgoAAAANSUhEUg...",
  "width": 1280,
  "height": 720
}
```

**File mode (--file):**

```json
{
  "format": "png",
  "file": "/tmp/screenshot.png",
  "width": 1280,
  "height": 720
}
```

### Errors

| Condition | Error Message | Exit Code |
|-----------|---------------|-----------|
| `--full-page` with `--selector`/`--uid` | `Cannot combine --full-page with --selector or --uid` | `GeneralError` (1) |
| Selector not found | `Element not found for selector: {selector}` | `GeneralError` (1) |
| UID not found | `UID '{uid}' not found. Run 'chrome-cli page snapshot' first.` | `GeneralError` (1) |
| No snapshot state | `No snapshot state found. Run 'chrome-cli page snapshot' first.` | `GeneralError` (1) |
| Invalid --clip format | `Invalid clip format: expected X,Y,WIDTH,HEIGHT (e.g. 10,20,200,100)` | `GeneralError` (1) |
| File write failure | `Failed to write screenshot to file: {path}: {error}` | `GeneralError` (1) |
| No connection | Existing `no_session` / `no_chrome_found` | `ConnectionError` (2) |
| Tab not found | Existing `target_not_found` | `TargetError` (3) |
| Timeout | Existing `command_timeout` from CDP layer | `TimeoutError` (4) |

---

## New Files and Modifications

### Modified Files

| File | Change |
|------|--------|
| `src/cli/mod.rs` | Add `PageScreenshotArgs` struct with all options; add `Screenshot(PageScreenshotArgs)` variant to `PageCommand`; add `ScreenshotFormat` enum |
| `src/page.rs` | Add `execute_screenshot()` function, output types (`ScreenshotResult`, `ScreenshotFileResult`), and helper functions for each capture strategy |
| `src/error.rs` | Add `screenshot_failed()`, `uid_not_found()`, `invalid_clip()` error constructors |

### No Changes Needed

| Component | Why |
|-----------|-----|
| `src/cdp/*` | `Page.captureScreenshot` is a standard CDP method; no new transport features needed |
| `src/connection.rs` | `setup_session`, `ManagedSession` all reusable as-is |
| `src/snapshot.rs` | `read_snapshot_state` already exists for UID resolution |
| `src/main.rs` | Already dispatches to `page::execute_page()` which handles `PageCommand` variants |

---

## CDP Protocol Details

### Page.captureScreenshot

**Request:**
```json
{
  "method": "Page.captureScreenshot",
  "params": {
    "format": "png",
    "quality": 80,
    "clip": {
      "x": 0,
      "y": 0,
      "width": 1280,
      "height": 720,
      "scale": 1
    },
    "captureBeyondViewport": true
  }
}
```

**Response:**
```json
{
  "data": "iVBORw0KGgoAAAANSUhEUg..."
}
```

- `format`: "png" | "jpeg" | "webp"
- `quality`: 0-100, only for jpeg/webp
- `clip`: viewport region to capture (with `scale: 1`)
- `captureBeyondViewport`: capture content outside viewport (needed for full-page)

### Full-page Strategy

1. Get page dimensions:
```javascript
JSON.stringify({
  width: Math.max(document.documentElement.scrollWidth, document.documentElement.clientWidth),
  height: Math.max(document.documentElement.scrollHeight, document.documentElement.clientHeight)
})
```

2. Get current device metrics (to restore later):
   - Use `Runtime.evaluate` to read `window.innerWidth`, `window.innerHeight`

3. Override viewport via `Emulation.setDeviceMetricsOverride`:
```json
{
  "width": scrollWidth,
  "height": scrollHeight,
  "deviceScaleFactor": 1,
  "mobile": false
}
```

4. Capture with `captureBeyondViewport: true`

5. Restore viewport via `Emulation.clearDeviceMetricsOverride`

### Element Screenshot Strategy

1. Resolve element's DOM node:
   - `--selector`: `DOM.getDocument` → `DOM.querySelector` → `DOM.getBoxModel`
   - `--uid`: read `snapshot.json` → get `backendDOMNodeId` → `DOM.describeNode` → `DOM.getBoxModel`

2. Extract bounding box from `DOM.getBoxModel` content quad:
```json
{
  "clip": {
    "x": content[0],
    "y": content[1],
    "width": content[4] - content[0],
    "height": content[5] - content[1],
    "scale": 1
  }
}
```

3. Capture with clip parameter

---

## Image Dimension Resolution

Getting accurate width/height for the output JSON:

| Strategy | Width/Height Source |
|----------|-------------------|
| Viewport (default) | `Runtime.evaluate` → `window.innerWidth`, `window.innerHeight` |
| Full-page | `Runtime.evaluate` → `scrollWidth`, `scrollHeight` |
| Element | `DOM.getBoxModel` content quad dimensions |
| Region (--clip) | Parsed from `--clip` argument directly |

---

## Base64 Decoding for --file

CDP returns the screenshot as a base64-encoded string. For `--file`, we must decode this to binary:

```rust
use base64::Engine;
let bytes = base64::engine::general_purpose::STANDARD.decode(&data)?;
std::fs::write(&path, &bytes)?;
```

This requires adding the `base64` crate as a dependency. Alternatively, we can use a minimal inline decoder, but the `base64` crate is standard and well-tested.

**Decision**: Add `base64` crate dependency. It's small, widely used, and avoids reimplementing a standard algorithm.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: CDP `Page.captureScreenshot` (direct)** | Use CDP protocol directly | Full control, no extra dependencies, consistent with architecture | Must handle viewport manipulation manually | **Selected** |
| **B: Headless Chrome `--screenshot` flag** | Launch Chrome with screenshot flag | Simpler for one-shot captures | Doesn't work with existing sessions, no element/clip support | Rejected |
| **C: Binary stdout for --file** | Write binary directly to stdout when `--file` is `-` | Unix-friendly piping | Breaks JSON output contract, complicates detection | Rejected — base64 JSON is safer |

---

## Security Considerations

- [x] **File path validation**: `--file` path is user-controlled; we write to it with `std::fs::write` which respects OS permissions
- [x] **No arbitrary JS**: All JavaScript evaluated is from fixed templates (dimension queries)
- [x] **Local only**: All CDP communication is localhost
- [x] **Sensitive content**: Screenshots may capture sensitive page content — expected for a local CLI tool

---

## Performance Considerations

- **Viewport capture**: Single CDP round-trip, fastest path (~50-200ms)
- **Full-page capture**: 3-4 CDP round-trips (get dimensions, set viewport, capture, restore) — slightly slower for large pages
- **Element capture**: 2-3 CDP round-trips (resolve element, get box model, capture)
- **Base64 overhead**: Large pages (e.g., 10MB+ screenshots) produce ~13MB base64; warn user but don't error
- **No caching**: Each invocation captures current page state

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| CLI args | Unit | `PageScreenshotArgs` parsing, format enum, clip parsing |
| Output types | Unit | `ScreenshotResult` and `ScreenshotFileResult` serialization |
| Error constructors | Unit | `screenshot_failed`, `uid_not_found`, `invalid_clip` |
| Clip parsing | Unit | Valid formats, edge cases, invalid input |
| Feature | BDD (Gherkin) | All 16 acceptance criteria as scenarios |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Full-page viewport restore fails | Low | Medium | Use `Emulation.clearDeviceMetricsOverride` which resets to defaults |
| Very large pages produce huge base64 | Low | Medium | Warn to stderr if base64 > 10MB |
| `DOM.getBoxModel` fails for invisible elements | Medium | Low | Return clear error message |
| base64 crate version conflicts | Low | Low | Use latest stable, minimal features |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] N/A — CLI tool, no UI state management
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
