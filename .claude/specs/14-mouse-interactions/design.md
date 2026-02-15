# Design: Mouse Interactions

**Issue**: #14
**Date**: 2026-02-13
**Status**: Draft
**Author**: Claude (writing-specs)

---

## Overview

This feature adds an `interact` subcommand group to chrome-cli with four commands: `click`, `click-at`, `hover`, and `drag`. These commands simulate mouse interactions via the Chrome DevTools Protocol's `Input.dispatchMouseEvent` method.

The implementation follows the established command pattern: CLI args defined in `cli/mod.rs`, a new `interact.rs` command module, CDP communication via `ManagedSession`, and JSON/plain output formatting. A shared target resolution module handles the dual UID/CSS-selector targeting system — UIDs are resolved from the persisted snapshot state (`~/.chrome-cli/snapshot.json`), while CSS selectors are resolved via `DOM.querySelector`.

The core flow for all interaction commands is: resolve target → get element coordinates → scroll into view → dispatch mouse event(s) → optionally wait for navigation → format output.

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────────┐
│                     CLI Layer (cli/mod.rs)                     │
│  ┌──────────────┐                                              │
│  │ InteractArgs  │  InteractCommand:                           │
│  │               │    Click(ClickArgs)                         │
│  │               │    ClickAt(ClickAtArgs)                     │
│  │               │    Hover(HoverArgs)                         │
│  │               │    Drag(DragArgs)                           │
│  └──────┬───────┘                                              │
└─────────┼──────────────────────────────────────────────────────┘
          │
          ▼
┌──────────────────────────────────────────────────────────────┐
│               Command Layer (interact.rs)                     │
│  ┌─────────────────┐  ┌─────────────────┐                    │
│  │ execute_click()  │  │ execute_hover() │                    │
│  │ execute_click_at │  │ execute_drag()  │                    │
│  └────────┬────────┘  └────────┬────────┘                    │
│           │                    │                              │
│           ▼                    ▼                              │
│  ┌───────────────────────────────────────────┐               │
│  │ Target Resolution (shared helpers)         │               │
│  │ resolve_target_coords() → (x, y)          │               │
│  │ resolve_uid() → backendDOMNodeId           │               │
│  │ resolve_css_selector() → DOM.querySelector │               │
│  │ get_element_center() → DOM.getBoxModel     │               │
│  │ scroll_into_view() → DOM.scrollIntoView    │               │
│  └───────────────────┬───────────────────────┘               │
│                      │                                        │
│  ┌───────────────────────────────────────────┐               │
│  │ Mouse Dispatch (shared helpers)            │               │
│  │ dispatch_click() → mousePressed+Released   │               │
│  │ dispatch_hover() → mouseMoved              │               │
│  │ dispatch_drag()  → press+move+release      │               │
│  └───────────────────┬───────────────────────┘               │
└──────────────────────┼────────────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────────┐
│               CDP Layer (ManagedSession)                      │
│  DOM.enable → DOM.querySelector, DOM.getBoxModel,             │
│               DOM.scrollIntoViewIfNeeded, DOM.resolveNode,    │
│               DOM.describeNode                                │
│  Input (no enable needed) → Input.dispatchMouseEvent          │
│  Page.enable → Page.frameNavigated (navigation detection)     │
│  Accessibility.getFullAXTree (for --include-snapshot)         │
└──────────────────────────────────────────────────────────────┘
```

### Data Flow — `interact click s1`

```
1. User runs: chrome-cli interact click s1 [--double] [--right] [--include-snapshot]
2. CLI layer parses args → ClickArgs { target: "s1", double: false, right: false, include_snapshot: false }
3. setup_session() → CdpClient + ManagedSession
4. managed.ensure_domain("DOM")
5. managed.ensure_domain("Page")
6. Resolve target "s1":
   a. Detect UID pattern (starts with "s" followed by digits)
   b. Read ~/.chrome-cli/snapshot.json → SnapshotState
   c. Lookup "s1" → backendDOMNodeId (e.g., 42)
   d. DOM.describeNode({ backendNodeId: 42 }) → nodeId
   e. DOM.getBoxModel({ nodeId }) → { content: [[x1,y1], [x2,y2], [x3,y3], [x4,y4]] }
   f. Compute center: x = (x1+x3)/2, y = (y1+y3)/2
7. DOM.scrollIntoViewIfNeeded({ backendNodeId: 42 })
8. Re-compute coordinates after scroll (getBoxModel again)
9. Subscribe to Page.frameNavigated (for navigation detection)
10. Dispatch mouse events via Input.dispatchMouseEvent:
    - mousePressed { x, y, button: "left", clickCount: 1 }
    - mouseReleased { x, y, button: "left", clickCount: 1 }
11. Brief wait (~50ms) for potential navigation event
12. Check if Page.frameNavigated fired → set navigated flag
13. Build ClickResult { clicked: "s1", url, navigated }
14. If --include-snapshot: take new snapshot (Accessibility.getFullAXTree)
15. Output result as JSON/pretty/plain
```

### Data Flow — `interact click-at 100 200`

```
1. User runs: chrome-cli interact click-at 100 200 [--double]
2. CLI layer parses → ClickAtArgs { x: 100.0, y: 200.0, double: false, right: false }
3. setup_session() → CdpClient + ManagedSession
4. Dispatch mouse events at (100, 200) directly (no target resolution needed)
5. Build ClickAtResult { clicked_at: { x: 100, y: 200 } }
6. Output result
```

### Data Flow — `interact hover s3`

```
1. User runs: chrome-cli interact hover s3
2. Resolve target → (x, y) coordinates (same as click steps 4-8)
3. Dispatch single mouseMoved event at (x, y)
4. Build HoverResult { hovered: "s3" }
5. Output result
```

### Data Flow — `interact drag s1 s2`

```
1. User runs: chrome-cli interact drag s1 s2
2. Resolve "from" target s1 → (x1, y1)
3. Resolve "to" target s2 → (x2, y2)
4. Scroll s1 into view
5. Dispatch mouse sequence:
   a. mousePressed at (x1, y1) with button: "left"
   b. mouseMoved from (x1, y1) to (x2, y2) in steps (or single move)
   c. mouseReleased at (x2, y2) with button: "left"
6. Build DragResult { dragged: { from: "s1", to: "s2" } }
7. Output result
```

---

## API / Interface Changes

### New CLI Commands

| Command | Purpose |
|---------|---------|
| `chrome-cli interact click <TARGET>` | Click an element by UID or CSS selector |
| `chrome-cli interact click-at <X> <Y>` | Click at viewport coordinates |
| `chrome-cli interact hover <TARGET>` | Hover over an element |
| `chrome-cli interact drag <FROM> <TO>` | Drag from one element to another |

### New CLI Flags

| Flag | Applies To | Type | Purpose |
|------|-----------|------|---------|
| `--double` | click, click-at | bool | Double-click instead of single click |
| `--right` | click, click-at | bool | Right-click (context menu) instead of left click |
| `--include-snapshot` | click, click-at, hover, drag | bool | Include updated accessibility snapshot in output |

### Request / Response Schemas

#### `interact click <TARGET>`

**Input (CLI args):**
```
chrome-cli interact click <TARGET> [--double] [--right] [--include-snapshot] [--tab ID]
```

**Output (success — JSON):**
```json
{
  "clicked": "s1",
  "url": "https://example.com",
  "navigated": false
}
```

**Output (with --double):**
```json
{
  "clicked": "s1",
  "url": "https://example.com",
  "navigated": false,
  "double_click": true
}
```

**Output (with --right):**
```json
{
  "clicked": "s1",
  "url": "https://example.com",
  "navigated": false,
  "right_click": true
}
```

**Output (with --include-snapshot):**
```json
{
  "clicked": "s1",
  "url": "https://example.com",
  "navigated": false,
  "snapshot": { "role": "document", "name": "...", "children": [...] }
}
```

**Output (plain text):**
```
Clicked s1
```

**Errors:**

| Exit Code | Condition |
|-----------|-----------|
| 1 (GeneralError) | UID not found in snapshot state |
| 1 (GeneralError) | No snapshot state file (for UID targets) |
| 1 (GeneralError) | CSS selector matches no element |
| 1 (GeneralError) | Element has zero-size bounding box |
| 2 (ConnectionError) | Cannot connect to Chrome |
| 3 (TargetError) | Tab not found |
| 4 (TimeoutError) | Command timed out |
| 5 (ProtocolError) | CDP protocol error |

#### `interact click-at <X> <Y>`

**Output (JSON):**
```json
{
  "clicked_at": { "x": 100.0, "y": 200.0 }
}
```

**Output (plain text):**
```
Clicked at (100, 200)
```

#### `interact hover <TARGET>`

**Output (JSON):**
```json
{
  "hovered": "s3"
}
```

**Output (plain text):**
```
Hovered s3
```

#### `interact drag <FROM> <TO>`

**Output (JSON):**
```json
{
  "dragged": { "from": "s1", "to": "s2" }
}
```

**Output (plain text):**
```
Dragged s1 to s2
```

---

## Database / Storage Changes

None. This feature reads from the existing `~/.chrome-cli/snapshot.json` (written by `page snapshot`) but does not write any persistent state itself. When `--include-snapshot` is used, the updated snapshot is included in the output but also written to `~/.chrome-cli/snapshot.json` so subsequent interaction commands can use the updated UIDs.

---

## State Management

### Target Resolution State (in-memory, per-command)

```rust
/// Resolved coordinates for a target element.
struct ResolvedTarget {
    x: f64,
    y: f64,
    backend_node_id: Option<i64>,  // Present for UID/CSS targets, not for click-at
}
```

### Target Resolution Logic

```rust
/// Determine if a target string is a UID or CSS selector.
fn is_uid(target: &str) -> bool {
    // UIDs match pattern: "s" followed by one or more digits
    target.starts_with('s') && target[1..].chars().all(|c| c.is_ascii_digit())
}

fn is_css_selector(target: &str) -> bool {
    target.starts_with("css:")
}
```

### State Transitions

```
Command start → setup_session()
    ↓
DOM.enable + Page.enable
    ↓
[For each target]:
    UID path: read_snapshot_state() → lookup backendDOMNodeId
              → DOM.describeNode → DOM.scrollIntoViewIfNeeded → DOM.getBoxModel
    CSS path: DOM.querySelector → DOM.scrollIntoViewIfNeeded → DOM.getBoxModel
    ↓
Compute center coordinates
    ↓
Input.dispatchMouseEvent(s)
    ↓
[Click only]: check for navigation event
    ↓
[If --include-snapshot]: Accessibility.getFullAXTree → build tree → write snapshot state
    ↓
Build + output result
```

---

## UI Components

N/A — this is a CLI tool, no UI components.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: JavaScript-based clicking** | Use `Runtime.evaluate` to call `element.click()` in page context | Simple, no coordinate math | Doesn't simulate real mouse events (no hover, no coordinates, may not trigger all event listeners) | Rejected — not a real mouse simulation |
| **B: CDP Input.dispatchMouseEvent** | Low-level mouse event dispatch with computed coordinates | Full fidelity, matches real user interaction, works with all event listeners | Requires coordinate computation (getBoxModel), scroll management | **Selected** — matches MCP server approach |
| **C: CDP DOM.focus + synthetic events** | Focus element then fire synthetic click | Simpler than Input dispatch | Missing coordinate info, doesn't trigger pointer events | Rejected — incomplete simulation |

**Design Decision**: Use `Input.dispatchMouseEvent` with coordinates computed from `DOM.getBoxModel`. This matches how the MCP server implements these tools and provides full-fidelity mouse simulation including hover effects, pointer events, and coordinate-dependent handlers. Elements are scrolled into view with `DOM.scrollIntoViewIfNeeded` before coordinate computation.

---

## Security Considerations

- [x] **Input Validation**: Target strings are validated as UID pattern or `css:` prefix; coordinates are validated as positive numbers by clap
- [x] **No sensitive data**: Element coordinates and UIDs contain no secrets
- [x] **Local only**: All CDP communication is localhost
- [x] **Selector injection**: CSS selectors are passed directly to `DOM.querySelector` which handles its own parsing safely

---

## Performance Considerations

- [x] **Minimal CDP round-trips**: Target resolution requires 2-3 CDP calls (describeNode + scrollIntoView + getBoxModel); mouse dispatch is 2 calls (pressed + released)
- [x] **Lazy domain enabling**: DOM domain enabled once per session via `ensure_domain`
- [x] **No polling**: Navigation detection uses event subscription, not polling
- [x] **Snapshot caching**: Snapshot state is read once per command from disk; only re-taken if `--include-snapshot` is specified
- [x] **Coordinate recomputation**: After scroll, coordinates are recomputed once (not polling)

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Target Resolution | Unit | UID pattern detection, CSS prefix parsing |
| Output Types | Unit | Serialization of ClickResult, HoverResult, DragResult, ClickAtResult |
| Plain Text Formatting | Unit | Plain text output for all result types |
| CLI Args | Unit | Clap parsing for interact subcommands |
| Feature | BDD (cucumber-rs) | All 22 acceptance criteria from requirements.md |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Element moves between scroll and click (dynamic page) | Low | Medium | Re-compute coordinates after scroll; brief delay not needed since CDP calls are sequential |
| Zero-size elements (hidden, collapsed) | Medium | Low | Check box model dimensions; return clear error if element has zero area |
| Stale snapshot UIDs (page changed since snapshot) | Medium | Medium | `DOM.describeNode` will fail if backendNodeId is invalid — return clear error suggesting re-snapshot |
| Navigation detection false positives | Low | Low | Only check for `Page.frameNavigated` on main frame, ignore subframe navigations |
| Drag doesn't work on all pages | Medium | Low | Some drag implementations require specific HTML5 drag events; CDP `Input.dispatchMouseEvent` may not trigger all of them. Document this limitation. |

---

## Open Questions

- (None)

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed (reads existing snapshot state)
- [x] State management approach is clear (in-memory target resolution)
- [x] N/A — no UI components
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
