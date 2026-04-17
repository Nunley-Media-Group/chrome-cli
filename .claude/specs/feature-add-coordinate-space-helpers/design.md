# Design: Coordinate Space Helpers for Frame-Aware Coordinate Resolution

**Issues**: #198
**Date**: 2026-04-16
**Status**: Draft
**Author**: Rich Nunley

---

## Overview

This feature adds one new subcommand — `page coords` — and a new `--relative-to <selector>` flag (plus percentage syntax on X/Y arguments) to the four coordinate-dispatching interact commands: `click-at`, `drag-at`, `mousedown-at`, `mouseup-at`. All coordinate translation logic already exists in `src/interact.rs::get_frame_viewport_offset` and is wired into each of the four commands today; this feature exposes that translation as a user-visible command and adds a new upstream step (element-relative resolution) that runs **before** the existing frame-offset translation.

The architectural shape is deliberately minimal: a single new coordinate-resolution helper module (`src/coords.rs`) that owns element bounding-box lookup and percentage parsing, plus a new `src/page/coords.rs` for the `page coords` subcommand that reuses the helper. The four interact commands are edited to call the helper when `--relative-to` is present, preserving their current absolute-coordinate behavior unchanged. No changes to the CDP layer, no new CDP domains, no changes to session or frame infrastructure.

The key design decision is to **resolve element-relative coordinates inside the target frame's viewport**, then apply the existing frame-offset translation — rather than resolving them in the main frame and attempting to translate bounding boxes across frame boundaries. This mirrors how `click-at --frame N` already treats absolute X/Y coordinates as frame-local, and keeps the translation step identical for all coordinate types.

---

## Architecture

### Component Diagram

```
┌───────────────────────────────────────────────────────────────────┐
│                          CLI Layer (clap)                          │
├───────────────────────────────────────────────────────────────────┤
│  PageCommand::Coords(PageCoordsArgs)                               │
│  ClickAtArgs        { ..., relative_to: Option<String>,            │
│                       x: CoordValue, y: CoordValue }               │
│  DragAtArgs         { ..., relative_to: Option<String>, ... }      │
│  MouseDownAtArgs    { ..., relative_to: Option<String>, ... }      │
│  MouseUpAtArgs      { ..., relative_to: Option<String>, ... }      │
└───────────────────────────────────┬───────────────────────────────┘
                                    ▼
┌───────────────────────────────────────────────────────────────────┐
│                     Command Modules (business logic)               │
├───────────────────────────────────────────────────────────────────┤
│  src/page/coords.rs          src/interact.rs                       │
│  ──────────────────          ─────────────                         │
│  execute_coords()            execute_click_at()                    │
│      │                             │                               │
│      ▼                             ▼                               │
│                 src/coords.rs  (new helper module)                 │
│                 ───────────────────────────────────                │
│                 resolve_element_box(frame_ctx, target)             │
│                 resolve_relative_coords(                           │
│                     x_input, y_input, box, frame_offset            │
│                 ) -> (page_x, page_y)                              │
│                 parse_coord_value("50%") -> CoordValue::Percent(50)│
│                 parse_coord_value("10")  -> CoordValue::Pixels(10) │
└───────────────────────────────────┬───────────────────────────────┘
                                    ▼
┌───────────────────────────────────────────────────────────────────┐
│                           CDP Layer (existing)                     │
│  DOM.getBoxModel, DOM.querySelector, DOM.getFrameOwner             │
│  (via src/frame.rs::get_frame_viewport_offset — unchanged)         │
└───────────────────────────────────────────────────────────────────┘
```

### Data Flow

#### `page coords --frame <index> --selector <target>`

```
1. Parse CLI args (clap): frame arg string, selector string
2. Setup session + resolve FrameContext (existing helper: resolve_optional_frame)
3. Resolve selector to backendNodeId in the frame's document
   (reuse src/page/element.rs::resolve_element_target, parameterized by frame context)
4. Fetch bounding box via DOM.getBoxModel { backendNodeId }
5. Compute frame offset via get_frame_viewport_offset(frame_ctx)
   → frameLocal.boundingBox = raw bounding box
   → page.boundingBox      = raw bounding box + frame offset
   → centers computed from box
6. Emit JSON on stdout; exit 0
```

#### `interact click-at X Y --relative-to <sel> [--frame <idx>]`

```
1. Parse CLI args: x and y as CoordValue (Pixels | Percent), relative_to Option<String>
2. Setup session + resolve FrameContext
3. Compute frame_offset = get_frame_viewport_offset(frame_ctx)
4. If relative_to is Some:
     a. Resolve element in frame context → bounding box (frame-local)
     b. Compute dispatch_x, dispatch_y from box + (x, y) per CoordValue rules
     c. Add frame_offset → page-global coords
   Else:
     a. dispatch_x = x_pixels + frame_offset_x   (existing behavior)
     b. dispatch_y = y_pixels + frame_offset_y
5. Dispatch Input.dispatchMouseEvent at (dispatch_x, dispatch_y)
6. Emit JSON: clicked_at reports the input as given (absolute path) OR the resolved
   page-global coords (relative-to path — see "Output Schema" below)
```

---

## API / Interface Changes

### New Subcommand

| Command | Args | Purpose |
|---------|------|---------|
| `page coords --selector <target>` | `--frame <idx>` (optional) | Resolve a selector to frame-local and page-level coordinates |

Note: `--frame` is already a `page`-level flag (on `PageArgs`), so no new flag is added — `page coords` inherits it like all other `page` subcommands.

### New Flags on Existing Commands

| Command | New Flag | Behavior |
|---------|----------|----------|
| `interact click-at` | `--relative-to <selector>` | X/Y treated as offsets/percentages from the element's top-left |
| `interact drag-at` | `--relative-to <selector>` | Both `from_x/y` and `to_x/y` resolved against the same element |
| `interact mousedown-at` | `--relative-to <selector>` | Same resolution as click-at |
| `interact mouseup-at` | `--relative-to <selector>` | Same resolution as click-at |

### Changed Argument Types

Today, `ClickAtArgs.x` is `f64`. To accept `"50%"` or `"10"`, the type becomes a custom `CoordValue` enum parsed via clap's `value_parser`:

```rust
// src/coords.rs
#[derive(Debug, Clone, Copy)]
pub enum CoordValue {
    /// Absolute pixel value (from input like `"10"` or `"10.5"`).
    Pixels(f64),
    /// Percentage of the `--relative-to` element's dimension (from input like `"50%"`).
    /// Stored as the raw number before the `%` sign.
    Percent(f64),
}

impl CoordValue {
    pub fn parse(s: &str) -> Result<Self, CoordValueParseError> { /* ... */ }
}
```

Clap's `value_parser!` wires this type into all four interact commands' X/Y args.

### Request / Response Schemas

#### `page coords`

**Input (CLI):**
```
agentchrome page coords --selector <target> [--frame <index>]
```

**Output (success, stdout):**
```json
{
  "frame": {
    "index": 1,
    "id": "ABCD1234EFGH5678"
  },
  "frameLocal": {
    "boundingBox": { "x": 10.0, "y": 20.0, "width": 80.0, "height": 32.0 },
    "center":      { "x": 50.0, "y": 36.0 }
  },
  "page": {
    "boundingBox": { "x": 60.0, "y": 120.0, "width": 80.0, "height": 32.0 },
    "center":      { "x": 100.0, "y": 136.0 }
  },
  "frameOffset": { "x": 50.0, "y": 100.0 }
}
```

**Errors (stderr):**

| Condition | Exit Code | Error Shape |
|-----------|-----------|-------------|
| Selector not found | 3 (target error) | `{"error": "...", "code": 3}` — matches `AppError::element_target_not_found` / `css_selector_not_found` |
| Frame index out of range | 3 (target error) | Existing frame-resolution error |
| CDP failure | 5 (protocol error) | Existing CDP error shape |

#### `interact click-at --relative-to`

**Input (CLI):**
```
agentchrome interact click-at <x> <y> --relative-to <selector> [other click-at flags]
```

Where `<x>` and `<y>` are each either a number (e.g., `10` or `10.5`) or a percentage (e.g., `50%`).

**Output schema (unchanged field names, clarified semantics):**

When `--relative-to` is present, `clicked_at.x/y` reports the **resolved page-global coordinates actually dispatched to Chrome** (not the raw input, which would be ambiguous — `"50%"` is not a coordinate). This is a deliberate semantic extension: when the input is a coordinate, the output echoes it; when the input is not a coordinate (percentage or element-relative offset), the output reports the resolved coordinate. This gives automation scripts a verifiable pass-through field for every invocation.

For `drag-at`, `dragged_at.from` and `dragged_at.to` follow the same rule.

**Errors (stderr):**

| Condition | Exit Code |
|-----------|-----------|
| `--relative-to` element not found | 3 (target error) |
| Invalid `CoordValue` (percentage out of range, malformed) | 1 (general / validation error) |
| Percentage used without `--relative-to` | 1 (validation error — covered by FR3; percentage without context is meaningless) |

---

## Database / Storage Changes

None. This feature is stateless.

---

## State Management

### Process-Level State

No new state. Each invocation resolves the element and bounding box fresh. `page coords` does not update the snapshot state file, and does not depend on a prior `page snapshot` (except when the `--selector` is a UID, in which case the existing `read_snapshot_state` path is used, same as `page element`).

### CDP Session State

No new CDP domains. Uses:
- `DOM.getDocument` (already enabled via `DOM` domain, used by `page element`)
- `DOM.querySelector` (already used)
- `DOM.getBoxModel` (already used by `get_frame_viewport_offset`)
- `DOM.getFrameOwner` (already used by `get_frame_viewport_offset`)

---

## UI Components

N/A — CLI feature.

---

## Implementation Details

### New Module: `src/coords.rs`

Responsibility:
- Parse `CoordValue` from CLI strings (`"50%"` → `Percent(50.0)`, `"10"` → `Pixels(10.0)`).
- Resolve a target (UID or CSS selector) to a bounding box **within a given frame context** — duplicates a small amount of logic from `src/page/element.rs` because that file's `resolve_element_target` takes only a session, not a frame context. The cleaner approach is to extract a shared function, but we keep the change surface small by introducing a new frame-aware helper here and leaving `page/element.rs` untouched. If future work wants to consolidate, this module becomes the single owner.
- Compute page-global coordinates from `(x_input, y_input, box, frame_offset)`:

```rust
pub fn resolve_relative_coords(
    x: CoordValue,
    y: CoordValue,
    element_box: BoundingBox,   // frame-local
    frame_offset: (f64, f64),   // frame's offset in page space
) -> (f64, f64) {
    let x_frame_local = match x {
        CoordValue::Pixels(px) => element_box.x + px,
        CoordValue::Percent(p) => {
            // 100% maps to (width - 1) so a 100%,100% click hits the bottom-right
            // pixel inside the element.
            let frac = p / 100.0;
            element_box.x + frac * (element_box.width - 1.0).max(0.0)
        }
    };
    let y_frame_local = match y { /* ... mirror of x */ };
    (x_frame_local + frame_offset.0, y_frame_local + frame_offset.1)
}
```

### New Module: `src/page/coords.rs`

Responsibility:
- Implement `execute_coords(global, args, frame)` that:
  1. Calls `setup_session`.
  2. Resolves frame via `crate::output::resolve_optional_frame`.
  3. Calls `src/coords.rs::resolve_element_box(frame_ctx, selector)` → `BoundingBox` in frame-local space.
  4. Calls `get_frame_viewport_offset(frame_ctx)` (currently `pub(crate)` in `interact.rs` — **make it `pub(crate)` at the crate root by moving it to `src/coords.rs`** or exposing it as a module function. See "Refactoring" below.).
  5. Builds the JSON output struct and calls `print_output`.
- Register in `src/page/mod.rs` dispatcher alongside other subcommands.

### CLI Changes (`src/cli/mod.rs`)

- Add `PageCommand::Coords(PageCoordsArgs)` variant with `long_about`, `after_long_help` examples, and `#[command(...)]` attributes matching sibling subcommands.
- Add `pub struct PageCoordsArgs { #[arg(long)] pub selector: String }` — note `--frame` is inherited from the parent `PageArgs`.
- Change `ClickAtArgs.x` and `ClickAtArgs.y` from `f64` to `CoordValue` (with clap `value_parser` using `CoordValue::parse`).
- Same change for `DragAtArgs.{from_x, from_y, to_x, to_y}`, `MouseDownAtArgs.{x, y}`, `MouseUpAtArgs.{x, y}`.
- Add `#[arg(long = "relative-to")] pub relative_to: Option<String>` to all four arg structs.

### Refactoring: `get_frame_viewport_offset`

Currently private in `src/interact.rs` (line 25). Move to `src/coords.rs` as `pub(crate) async fn frame_viewport_offset(...)` so both `src/interact.rs` and `src/page/coords.rs` can call it. `interact.rs` updates its call sites to `use crate::coords::frame_viewport_offset;`.

### Validation of `CoordValue::Percent`

Parse-time (in `CoordValue::parse`): reject values where the number-before-`%` is `< 0.0` or `> 100.0`, or where parsing fails. Return a typed error that surfaces through clap as `AppError` with exit code 1.

Parse-time guard against percentage without `--relative-to`: this check happens in the command executor, not the parser, because clap can't cross-reference argument values with other flags easily. Each of the four executors runs:

```rust
if matches!(args.x, CoordValue::Percent(_)) || matches!(args.y, CoordValue::Percent(_)) {
    if args.relative_to.is_none() {
        return Err(AppError::invalid_argument(
            "percentage coordinates require --relative-to"
        ));
    }
}
```

### Dispatch Path

The dispatch step (`dispatch_click`, `dispatch_drag_interpolated`, `dispatch_mousedown`, `dispatch_mouseup`) is unchanged — it always receives page-global `f64` coordinates. The change is only in how those coordinates are computed before the dispatch call.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: `--anchor <name>` with named positions** (`top-left`, `center`, etc.) | Named anchor flag instead of percentages | Readable, clamps to valid range by construction | Limited vocabulary; users still want "33% from the left" | Rejected — out of scope per requirements; percentage covers all named anchor cases |
| **B: `page coords --uid <uid>` with separate `--selector` flag** | Two flags, mutually exclusive | Matches some existing commands' dual-flag pattern | `page element` already accepts both forms via a single `target` positional; consistency wins | Rejected — use single `--selector` accepting both forms |
| **C: Resolve `--relative-to` element in the main frame, translate box to frame-local** | Cross-frame bounding-box translation | Lets users pass a main-frame selector and have it resolved into an iframe | Ambiguous: which frame owns the selector if the same `#id` exists in both? Harder to reason about; violates the "coordinates are frame-local" invariant `click-at --frame` already established | Rejected — resolve `--relative-to` in the **same frame** as the dispatch target (principle of least surprise) |
| **D: Extract `resolve_element_target` from `page/element.rs` into a shared helper used by both** | Single source of truth for UID/CSS resolution | Removes duplication | Requires parameterizing the helper with a frame context, touching `page element` call sites, increasing blast radius | Rejected for this feature — introduce new frame-aware helper in `src/coords.rs`; consolidate in a follow-up if the duplication becomes painful |
| **E: Report `clicked_at` as the raw input (`{"x": "50%", "y": "50%"}`) when `--relative-to` is present** | Strict "echo input" contract | Preserves existing echo-input behavior literally | Output type would be a union (`f64 \| string`), breaking JSON consumers that assume numeric; doesn't help users verify actual dispatch coords | Rejected — report resolved page-global coords in `clicked_at` (documented semantic extension) |

**Selected: B + D-rejected + E-rejected** — single `--selector` flag on `page coords`, new frame-aware resolver in `src/coords.rs`, `clicked_at` reports resolved page coords when `--relative-to` is present.

---

## Security Considerations

- [x] **Authentication**: N/A — uses existing CDP session.
- [x] **Authorization**: N/A — inherits session permissions.
- [x] **Input Validation**: `CoordValue::parse` rejects malformed input; percentage range enforced at parse time; selector syntax validated identically to `page element`.
- [x] **Data Sanitization**: Selector string is passed to `DOM.querySelector` — CDP handles sanitization; no shell/SQL context to worry about.
- [x] **Sensitive Data**: None — coordinates are non-sensitive.
- [x] **CDP Domains**: No new domains enabled; reuses `DOM` and `Runtime` already enabled by existing commands.

---

## Performance Considerations

- [x] **Latency budget**: `page coords` round-trip target < 200ms. Component breakdown:
  - Session setup: ~10–30ms (existing)
  - Frame resolution: 0–50ms (main frame: 0ms; same-origin iframe: one `Target.getTargets` if not cached; OOPIF: `Target.attachToTarget` if not attached — all amortized by `resolve_optional_frame` cache)
  - `DOM.querySelector`: ~5–20ms
  - `DOM.getBoxModel`: ~5–20ms
  - `get_frame_viewport_offset`: ~5–20ms (one additional `DOM.getBoxModel` for the iframe element; skipped for main frame)
  - **Total**: ~25–140ms typical, < 200ms target.
- [x] **Caching**: None — each invocation resolves fresh (consistent with existing `page element` behavior and the "stateless CLI" principle in `product.md`).
- [x] **Pagination**: N/A (single-element result).
- [x] **Lazy Loading**: N/A.

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| `CoordValue::parse` | Unit (`src/coords.rs`) | Valid pixels (`"10"`, `"10.5"`), valid percentages (`"0%"`, `"50%"`, `"100%"`, `"33.33%"`), invalid (`"50"`+trailing garbage, `"-5%"`, `"150%"`, `"5%%"`, empty) |
| `resolve_relative_coords` | Unit | Pixels+pixels, percent+percent, mixed; main frame offset (0,0) and iframe offset; 0% and 100% edge values |
| `page coords` | BDD (AC1–AC3, AC9) | Main frame, iframe, UID target, missing selector |
| `interact click-at --relative-to` | BDD (AC4–AC6, AC10–AC11) | Absolute offset, percentage, mixed, invalid percentage, missing element |
| `interact drag-at / mousedown-at / mouseup-at --relative-to` | BDD (AC7) | At least one scenario per command |
| `interact click-at --relative-to --frame` | BDD (AC8) | Combined frame + relative-to |
| `examples interact` and `examples page` output | BDD (AC12) | Grep for `--relative-to` and `page coords` in output |
| Regression | BDD (FR10) | Existing `click-at X Y` absolute-coord scenarios pass unchanged |

Smoke test (per `tech.md` verification gates):
- `tests/fixtures/coordinate-space-helpers.html` — a minimal HTML file with:
  - A main-frame element with a known bounding box
  - A nested `<iframe>` at a known page offset
  - An element inside the iframe with a known bounding box
  - Exercise `page coords` against both, and `click-at --relative-to` with `0% 0%` / `50% 50%` / `100% 100%` / absolute offset combinations.

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Changing `ClickAtArgs.x` from `f64` to `CoordValue` breaks existing scripts that pass negative pixel values (`-10`) | Low | Med | Ensure `CoordValue::parse` accepts negative `f64` as `Pixels(-f64)`. Add an explicit BDD regression scenario: `interact click-at -10 -10` parses successfully. |
| Percentage parsing collides with clap's number parsing (clap might try to parse `"50%"` as f64 and reject) | Low | High | Use explicit `value_parser = clap::value_parser!(CoordValue)` via `ValueParserFactory` impl; write a clap parser-unit test confirming `"50%"` reaches our parser. |
| `100%` resolving to `width - 1` surprises users who expect `element.x + width` | Med | Low | Document explicitly in `--help` and in `examples interact`; add an AC (AC5) pinning the behavior. |
| `get_frame_viewport_offset` returns stale offset if the iframe has scrolled or been repositioned between AC1 resolution and dispatch | Low | Med | Same risk already exists in `click-at --frame`; not a new risk introduced by this feature. Document as a known limitation (out-of-scope item: "live coordinate stabilization"). |
| `page coords` reports coordinates that disagree with what `click-at --relative-to` would dispatch (drift in computation) | Low | High | Both paths share the same `resolve_relative_coords` and `frame_viewport_offset` helpers — single source of truth prevents drift. Add a BDD scenario that asserts `page coords` center matches the coordinates a subsequent `click-at 50% 50% --relative-to ...` reports in `clicked_at`. |
| Moving `get_frame_viewport_offset` from `interact.rs` to `coords.rs` introduces a regression in existing `--frame` click behavior | Low | High | Keep the move surgical (identical function body, different path); run full BDD regression suite; ensure `interact.rs` call sites update to the new path. |
| Users pass `-5%` expecting "5% from the right edge" | Low | Low | Reject at parse time per FR7; the error message explains valid range. Document in help that percentages are of the element's own box, 0–100 only. |

---

## Open Questions

- [x] Should `page coords` accept both UID and CSS forms via a single `--selector` flag? **Resolved**: Yes — single `--selector` that accepts both forms, matching `page element`'s target resolution.
- [x] Should `clicked_at` echo the input or report resolved coords when `--relative-to` is used? **Resolved**: Report resolved page-global coords (documented semantic extension, see Alternatives E).
- [x] Does `100% 100%` hit inside or just outside the element? **Resolved**: Inside (`width - 1`, `height - 1`).

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #198 | 2026-04-16 | Initial design — `page coords` command + `--relative-to` / percentage support on coordinate-dispatching interact commands; new `src/coords.rs` helper module |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`): new commands implemented as command modules, CLI changes in `src/cli/mod.rs`, helper extracted to its own module
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes (N/A)
- [x] State management approach is clear (stateless, no new state)
- [x] No UI components (CLI feature)
- [x] Security considerations addressed
- [x] Performance impact analyzed with latency budget
- [x] Testing strategy defined (unit + BDD + smoke)
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
