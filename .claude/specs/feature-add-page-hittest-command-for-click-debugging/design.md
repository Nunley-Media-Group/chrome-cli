# Design: Page Hit Test Command for Click Debugging

**Issues**: #191
**Date**: 2026-04-16
**Status**: Draft
**Author**: Rich Nunley

---

## Overview

This feature adds a `page hittest X Y` subcommand to the existing `page` command group. It combines CDP `DOM.getNodeForLocation` for hit target identification with `document.elementsFromPoint()` for z-index stack enumeration, then cross-references results to detect overlay interception. The command follows the same architecture as existing `page` subcommands (`text`, `snapshot`, `find`, `element`, `wait`) — a new `hittest.rs` module under `src/page/` with a corresponding `HitTest` variant in `PageCommand`.

Frame targeting reuses the existing `--frame` argument on `PageArgs`, which is already propagated to all `page` subcommands. The implementation leverages the frame resolution infrastructure in `src/frame.rs` (`resolve_frame`, `FrameContext`). Viewport bounds checking uses the existing `get_viewport_dimensions` helper in `src/page/mod.rs`.

---

## Architecture

### Component Diagram

```
CLI Layer (cli/mod.rs)
├── PageArgs { --frame }
│   └── PageCommand::HitTest(PageHitTestArgs { x, y })
│
Command Layer (page/mod.rs → page/hittest.rs)
├── execute_hittest(global, args, frame)
│   ├── setup_session()          → ManagedSession
│   ├── get_viewport_dimensions()→ (u32, u32)        [bounds check]
│   ├── resolve_frame()          → FrameContext       [if --frame]
│   ├── DOM.getNodeForLocation   → backendNodeId      [hit target]
│   ├── DOM.describeNode         → tag, id, class     [target details]
│   ├── Runtime.evaluate         → elementsFromPoint() [z-index stack]
│   ├── Accessibility.getFullAXTree or snapshot state  [UID lookup]
│   ├── detect_overlay()         → interceptedBy       [compare stack[0] vs deeper]
│   ├── generate_suggestion()    → String | null       [if overlay]
│   └── print_output()           → JSON stdout
│
CDP Layer (cdp/client.rs)
└── send_command() for: DOM.getNodeForLocation, DOM.describeNode,
    DOM.getDocument, Runtime.evaluate
```

### Data Flow

```
1. User runs `agentchrome page hittest 100 200`
2. CLI parses X=100, Y=200 as positional u32 args
3. Dispatcher routes to page/hittest.rs::execute_hittest()
4. Session is established via setup_session()
5. Viewport dimensions fetched — bounds check against (X, Y)
6. If --frame provided, resolve_frame() gets FrameContext
7. DOM.getNodeForLocation(x, y) → returns backendNodeId of top hit
8. DOM.describeNode(backendNodeId) → tag, id, class, nodeId
9. Runtime.evaluate("document.elementsFromPoint(100, 200)") → ordered element list
10. For each element in stack: extract tag, id, class, computed z-index
11. Accessibility tree lookup: try to resolve UID for hit target & stack elements
12. Overlay detection: compare DOM.getNodeForLocation result vs stack analysis
13. If overlay detected: generate workaround suggestion
14. Serialize HitTestResult and print_output() to stdout
```

---

## API / Interface Changes

### New Subcommand

| Subcommand | Type | Purpose |
|------------|------|---------|
| `page hittest <X> <Y>` | CLI subcommand | Hit test at viewport coordinates |

### CLI Arguments (PageHitTestArgs)

| Argument | Type | Position | Required | Notes |
|----------|------|----------|----------|-------|
| `x` | u32 | positional (1) | Yes | X viewport coordinate |
| `y` | u32 | positional (2) | Yes | Y viewport coordinate |

Note: `--frame` is inherited from the parent `PageArgs` struct, not duplicated on the subcommand.

### Output Schema

**Success (stdout, exit 0):**

```json
{
  "frame": "main",
  "hitTarget": {
    "tag": "div",
    "id": "acc-blocker",
    "class": "overlay transparent",
    "uid": null
  },
  "interceptedBy": {
    "tag": "div",
    "id": "acc-blocker",
    "class": "overlay transparent",
    "uid": null
  },
  "stack": [
    {
      "tag": "div",
      "id": "acc-blocker",
      "class": "overlay transparent",
      "uid": null,
      "zIndex": "9999"
    },
    {
      "tag": "button",
      "id": "submit",
      "class": "primary",
      "uid": "s5",
      "zIndex": "auto"
    },
    {
      "tag": "body",
      "id": null,
      "class": null,
      "uid": null,
      "zIndex": "auto"
    },
    {
      "tag": "html",
      "id": null,
      "class": null,
      "uid": null,
      "zIndex": "auto"
    }
  ],
  "suggestion": "Element intercepted by div#acc-blocker — try targeting the underlying button#submit (uid: s5) directly"
}
```

**Error — out of bounds (stderr, exit 3):**

```json
{
  "error": "Coordinates (5000, 5000) are outside the viewport bounds (1280x720)",
  "code": 3
}
```

**Error — no connection (stderr, exit 2):**

```json
{
  "error": "No active session. Run 'agentchrome connect' first.",
  "code": 2
}
```

---

## Database / Storage Changes

None. This is a stateless query command.

---

## State Management

No persistent state. The command is a single-shot query:
1. Reads the current accessibility tree snapshot state (if available) for UID resolution
2. Makes CDP calls and returns results
3. No state is written or mutated

UID resolution is best-effort: if a snapshot exists (from a prior `page snapshot` call), UIDs are resolved from the cached `uid_map`. If no snapshot exists, UIDs are `null`. This avoids forcing a full accessibility tree capture on every hit test.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: DOM.getNodeForLocation only** | Use only CDP's built-in hit test | Simple, one CDP call | No z-index stack, no overlay detection | Rejected — insufficient for the core use case |
| **B: elementsFromPoint() only** | Use only JavaScript API | Full stack, z-index info | No frame-scoped resolution via CDP, slower due to JS round-trip, does not reflect CDP's actual hit testing | Rejected — misses CDP-level targeting |
| **C: Combined CDP + JS** | DOM.getNodeForLocation for hit target + elementsFromPoint() for stack | Best of both: accurate hit target from CDP + full stack from JS | Two round-trips | **Selected** — necessary for both accuracy and completeness |
| **D: Force snapshot on every hittest** | Always capture accessibility tree before hit test | Guaranteed UIDs | 200-500ms overhead per invocation | Rejected — UIDs are best-effort, cached if available |

---

## Security Considerations

- [x] **Authentication**: N/A — local CDP connection only
- [x] **Authorization**: N/A — no elevation of privilege
- [x] **Input Validation**: X, Y validated as u32 by clap; bounds-checked against viewport dimensions before CDP calls
- [x] **Data Sanitization**: Element attributes (id, class) are passed through from CDP/JS — no user-generated content risk
- [x] **Sensitive Data**: N/A — no secrets or PII in hit test results

---

## Performance Considerations

- [x] **CDP round-trips**: 3-4 calls (viewport dims, getNodeForLocation, describeNode, Runtime.evaluate) — estimated 50-150ms total
- [x] **JS evaluation**: `elementsFromPoint()` + `getComputedStyle()` per element in stack — estimated 20-50ms depending on DOM complexity
- [x] **Accessibility lookup**: O(1) hashmap lookup from cached snapshot state — negligible
- [x] **Total budget**: Well within 500ms target even on complex pages

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Output serialization | Unit | HitTestResult, StackElement serialize with correct null/camelCase |
| Overlay detection | Unit | detect_overlay logic: overlay present, no overlay, same element |
| Suggestion generation | Unit | generate_suggestion: with overlay, without overlay, with/without UIDs |
| CLI argument parsing | Unit (BDD) | Positional args, missing args, non-numeric args |
| Full command | Integration (BDD) | AC1-AC8 as Gherkin scenarios |
| Examples output | Integration (BDD) | `examples page` includes hittest entries |

---

## File Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/cli/mod.rs` | Add `HitTest(PageHitTestArgs)` to `PageCommand` enum; add `PageHitTestArgs` struct | New subcommand registration |
| `src/page/mod.rs` | Add `mod hittest;` and `PageCommand::HitTest` match arm in dispatcher | Module wiring |
| `src/page/hittest.rs` | **New file** — `execute_hittest()`, output types, overlay detection, suggestion generation | Core implementation |
| `src/examples.rs` | Add `page hittest` examples to the `page` command group | AC4: documentation |
| `tests/features/page-hittest.feature` | **New file** — Gherkin scenarios for AC1-AC8 | BDD tests |
| `tests/bdd.rs` | Add step definitions for page hittest scenarios | BDD step wiring |
| `tests/fixtures/page-hittest.html` | **New file** — test fixture with overlays, stacked elements, iframes | Feature exercise gate |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `DOM.getNodeForLocation` not available in older Chrome versions | Low | Medium | Minimum Chrome version already enforced by CDP client; this API exists since Chrome 72 |
| `elementsFromPoint()` returns different order in iframes vs main frame | Medium | Low | Frame-scoped execution via FrameContext ensures JS runs in correct context |
| Accessibility snapshot state stale or absent | Medium | Low | UIDs are best-effort with explicit `null`; no forced snapshot capture |
| Overlay detection false positives (legitimate overlays vs transparent wrappers) | Medium | Medium | Compare only topmost element vs stack — if they differ, flag as interception. User decides via suggestion text |

---

## Open Questions

- None

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #191 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] Database/storage changes planned with migrations (N/A)
- [x] State management approach is clear (stateless query)
- [x] UI components and hierarchy defined (CLI subcommand)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
