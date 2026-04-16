# Design: Iframe/Frame Targeting Support

**Issues**: #189
**Date**: 2026-04-15
**Status**: Draft
**Author**: Rich Nunley

---

## Overview

This design adds comprehensive frame, worker, and shadow DOM targeting to AgentChrome. The core architectural addition is a `FrameContext` abstraction in a new `src/frame.rs` module that transparently handles both same-origin iframes (which share the page's CDP target) and cross-origin out-of-process iframes (OOPIFs, which are separate targets). Commands receive a `FrameContext` alongside the existing `ManagedSession`, and all CDP operations are routed through the appropriate session and frame scope.

The `--frame` parameter is added as a command-level argument on all page, dom, interact, form, js, and network commands. It accepts three forms: a flat integer index (from `page frames` output), a slash-separated parent-child path for nested iframes (e.g., `1/0`), or the literal `auto` for automatic frame search. A new `page frames` subcommand enumerates the frame tree, and a new `page workers` subcommand enumerates workers. The `--worker` parameter on `js exec` targets worker execution contexts. The `--pierce-shadow` flag on `page snapshot` and `dom` commands enables open shadow DOM traversal.

The design follows the existing project pattern: CLI parsing via clap derive structs, command dispatch through `main.rs`, session setup in `connection.rs`, and CDP communication via `ManagedSession`. The new `frame.rs` module slots between session setup and command execution, adding frame resolution without modifying the core CDP client.

---

## Architecture

### Component Diagram

```
CLI Input (args including --frame, --worker, --pierce-shadow)
    ↓
┌─────────────────────────────────────────────────────┐
│   CLI Layer (cli/mod.rs)                            │
│   Parse --frame, --worker, --pierce-shadow args     │
└────────────────────┬────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│   Command Dispatch (main.rs)                        │
│   Route to command module with GlobalOpts + args    │
└────────────────────┬────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│   Session Setup (connection.rs)                     │
│   resolve_connection → resolve_target → CdpSession  │
└────────────────────┬────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│   Frame Resolution (frame.rs) ◀── NEW              │
│   Page.getFrameTree → resolve index/path/auto       │
│   → FrameContext (SameOrigin | OutOfProcess)        │
└────────────────────┬────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│   Command Modules (page.rs, dom.rs, interact.rs...) │
│   Use FrameContext to scope CDP operations          │
└────────────────────┬────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│   CDP Client (cdp/client.rs)                        │
│   WebSocket JSON-RPC via session_id routing         │
└────────────────────┬────────────────────────────────┘
                     ↓
                Chrome Browser
```

### Data Flow — Frame-Targeted Command

```
1. User runs: agentchrome page snapshot --frame 2
2. CLI layer parses --frame "2" into FrameArg::Index(2)
3. main.rs dispatches to page::execute_page() with GlobalOpts + PageArgs
4. Command calls setup_session(global) → (CdpClient, ManagedSession)
5. Command calls resolve_frame(&managed, frame_arg) → FrameContext
6. FrameContext::resolve() calls Page.getFrameTree, builds ordered frame list
7. Frame at index 2 is identified; its frame ID and origin checked
8. If same-origin: subscribe → Runtime.enable → match default context → FrameContext::SameOrigin { frame_id, execution_context_id }
9. If OOPIF: Target.attachToTarget → new session → FrameContext::OutOfProcess { session }
10. Command calls Accessibility.getFullAXTree with frameId parameter (same-origin)
    OR uses the OOPIF session to call Accessibility.getFullAXTree (cross-origin)
11. Snapshot built, UIDs assigned, state persisted with frame context
12. JSON output on stdout
```

### Data Flow — Auto Frame Detection

```
1. User runs: agentchrome interact click --frame auto s5
2. CLI parses --frame "auto" into FrameArg::Auto
3. Command calls resolve_frame_auto(&managed, "s5") → FrameContext + frame_index
4. resolve_frame_auto() iterates frame tree in document order:
   a. For each frame, reads persisted snapshot state or takes a quick snapshot
   b. Checks if UID "s5" exists in that frame's UID map
   c. Returns first match with its FrameContext and index
5. If no frame contains s5 → TargetError "Element not found in any frame"
6. Command executes click in resolved frame, output includes "frame": N
```

### Data Flow — Shadow DOM Piercing

```
1. User runs: agentchrome page snapshot --pierce-shadow
2. Command calls Accessibility.getFullAXTree (which already includes shadow DOM
   nodes in Chrome's accessibility tree for open shadow roots)
3. Snapshot is built normally — shadow DOM elements get UIDs like any other element
4. For dom commands with --pierce-shadow:
   a. First attempt DOM.querySelector in the main document
   b. If not found, use Runtime.evaluate to find all shadow hosts
   c. For each shadow host, query its shadowRoot for the selector
   d. Return first match
```

---

## API / Interface Changes

### New CLI Arguments

| Argument | Scope | Type | Description |
|----------|-------|------|-------------|
| `--frame <value>` | page, dom, interact, form, js, network commands | `String` | Frame index (integer), path (`1/0`), or `auto` |
| `--worker <index>` | `js exec` only | `u32` | Worker index from `page workers` |
| `--pierce-shadow` | `page snapshot`, all `dom` subcommands | `bool` flag | Enable shadow DOM traversal |

### New Subcommands

| Subcommand | Parent | Purpose |
|------------|--------|---------|
| `page frames` | `page` | List all frames in the page hierarchy |
| `page workers` | `page` | List all workers associated with the page |

### Request / Response Schemas

#### `page frames`

**Input:** No arguments required (uses current tab/page session).

**Output (success):**
```json
[
  {
    "index": 0,
    "id": "MAIN_FRAME_ID",
    "url": "https://example.com",
    "name": "",
    "securityOrigin": "https://example.com",
    "unreachable": false,
    "width": 1280,
    "height": 720,
    "depth": 0
  },
  {
    "index": 1,
    "id": "CHILD_FRAME_ID",
    "url": "https://example.com/embed",
    "name": "content-frame",
    "securityOrigin": "https://example.com",
    "unreachable": false,
    "width": 800,
    "height": 600,
    "depth": 1
  }
]
```

**Errors:**

| Code | Condition |
|------|-----------|
| 2 (ConnectionError) | Not connected to Chrome |
| 5 (ProtocolError) | CDP Page.getFrameTree failed |

#### `page workers`

**Output (success):**
```json
[
  {
    "index": 0,
    "id": "WORKER_TARGET_ID",
    "type": "service_worker",
    "url": "https://example.com/sw.js",
    "status": "activated"
  }
]
```

**Errors:**

| Code | Condition |
|------|-----------|
| 2 (ConnectionError) | Not connected to Chrome |

#### Frame-targeted command output

When `--frame auto` is used, all command outputs include an additional top-level `"frame"` field:

```json
{
  "frame": 2,
  "result": "..."
}
```

For explicit `--frame <index>` or `--frame <path>`, no extra field is added (the user already knows which frame).

#### Error responses for frame/worker targeting

```json
{"error": "Frame index 5 not found. Use 'page frames' to list available frames.", "code": 3}
{"error": "Frame path '1/3' failed at segment 3: parent frame has only 2 children.", "code": 3}
{"error": "Element not found in any frame. Use 'page frames' to list available frames.", "code": 3}
{"error": "Frame is no longer available (detached or navigated away).", "code": 3}
{"error": "Worker index 2 not found. Use 'page workers' to list available workers.", "code": 3}
```

---

## New Module: `src/frame.rs`

This is the core addition. It encapsulates all frame resolution, enumeration, and session management.

### Key Types

```rust
/// Parsed --frame argument
pub enum FrameArg {
    /// Flat integer index (e.g., --frame 2)
    Index(u32),
    /// Parent-child path (e.g., --frame 1/0)
    Path(Vec<u32>),
    /// Automatic search (--frame auto)
    Auto,
}

/// Metadata for a single frame in the tree
pub struct FrameInfo {
    pub index: u32,
    pub id: String,           // CDP frame ID
    pub url: String,
    pub name: String,
    pub security_origin: String,
    pub unreachable: bool,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub parent_id: Option<String>,
    pub child_ids: Vec<String>,
}

/// Resolved frame targeting context — either reuses the page session
/// with frame-scoped parameters, or holds a separate OOPIF session.
pub enum FrameContext {
    /// Main frame — no additional scoping needed
    MainFrame,
    /// Same-origin iframe: reuse page session, pass frameId to CDP methods
    SameOrigin {
        frame_id: String,
        execution_context_id: i64,
    },
    /// Cross-origin OOPIF: separate CDP session attached to the frame target
    OutOfProcess {
        session: ManagedSession,
        frame_id: String,
    },
}
```

### Key Functions

```rust
/// Parse --frame CLI value into FrameArg
pub fn parse_frame_arg(value: &str) -> Result<FrameArg, AppError>

/// Enumerate all frames via Page.getFrameTree, return ordered list
pub async fn list_frames(session: &ManagedSession) -> Result<Vec<FrameInfo>, AppError>

/// Resolve a FrameArg to a FrameContext
pub async fn resolve_frame(
    client: &CdpClient,
    session: &ManagedSession,
    arg: &FrameArg,
) -> Result<FrameContext, AppError>

/// Auto-detect which frame contains a UID (for --frame auto)
pub async fn resolve_frame_auto(
    client: &CdpClient,
    session: &ManagedSession,
    uid: &str,
) -> Result<(FrameContext, u32), AppError>

/// Get the ManagedSession to use for a given FrameContext
/// Returns the OOPIF session or the original page session
pub fn frame_session<'a>(
    ctx: &'a FrameContext,
    page_session: &'a ManagedSession,
) -> &'a ManagedSession

/// Get the frame_id to pass to CDP methods (None for MainFrame)
pub fn frame_id(ctx: &FrameContext) -> Option<&str>
```

### Frame Tree Traversal

`list_frames` calls `Page.getFrameTree` and performs a depth-first traversal of the returned tree, assigning sequential indexes starting at 0 (main frame). The traversal order matches the document order of `<iframe>` elements. `<frame>` elements within `<frameset>` documents are enumerated identically — CDP's `Page.getFrameTree` already includes them.

### Same-Origin vs OOPIF Detection

After getting the frame tree, `resolve_frame` determines the frame type:

1. Call `Target.getTargets` to list all targets
2. Check if any target's `targetId` matches a frame's target (identified by matching URL and frame ID)
3. If a separate target exists → OOPIF → `Target.attachToTarget` with `flatten: true` → `FrameContext::OutOfProcess`
4. If no separate target → same-origin → subscribe to `Runtime.executionContextCreated` events, then enable `Runtime` domain (which replays existing contexts), then match the frame's default context by `frameId` + `isDefault: true` → `FrameContext::SameOrigin`

**Design note**: The subscribe call MUST happen before `Runtime.enable`. Chrome replays execution contexts immediately upon enable, so subscribing after enable causes a race where events arrive before the subscriber is registered. If the default context is not found within 500ms (e.g., because `Runtime` was already enabled earlier in the session), the fallback creates an isolated world via `Page.createIsolatedWorld` with `grantUniversalAccess: true`. The isolated world shares the frame's DOM but has its own JS global scope, so page-script variables are not visible in the fallback path.

### Path Resolution

For `FrameArg::Path(vec![1, 0])`:
1. Start at the main frame (root of frame tree)
2. First segment `1`: select the main frame's child at index 1
3. Second segment `0`: select that frame's child at index 0
4. If any segment index exceeds the number of children, return `TargetError` with the failing segment

---

## Worker Targeting: `src/page.rs` + `src/js.rs`

### Worker Enumeration

`page workers` calls `Target.getTargets` and filters for worker types:
- `type == "service_worker"` → `"service_worker"`
- `type == "shared_worker"` → `"shared_worker"`
- `type == "worker"` → `"worker"` (dedicated web worker)

Workers are indexed in the order returned by CDP.

### Worker JS Execution

`js exec --worker <index>`:
1. Enumerate workers via `Target.getTargets`
2. `Target.attachToTarget` with the worker's `targetId` and `flatten: true`
3. `Runtime.evaluate` through the worker session
4. Workers have no DOM, so only `Runtime.evaluate` is meaningful

`--worker` and `--frame` are mutually exclusive. If both are provided, clap's `conflicts_with` attribute produces an error.

---

## Shadow DOM Piercing

### Accessibility Tree (`page snapshot --pierce-shadow`)

Chrome's `Accessibility.getFullAXTree` already traverses open shadow DOM roots in the accessibility tree. The `--pierce-shadow` flag is a signal to the snapshot builder to include shadow DOM content that may otherwise be filtered. In practice:

1. Call `Accessibility.getFullAXTree` (optionally with `frameId` for frame-scoped)
2. The returned nodes already include shadow DOM content for open shadow roots
3. Shadow DOM elements get UIDs via the existing `build_tree` logic — `backendDOMNodeId` is unique across shadow boundaries
4. UIDs assigned to shadow DOM elements work with subsequent commands because `DOM.resolveNode` with `backendNodeId` resolves across shadow boundaries

If Chrome's accessibility tree doesn't fully expose shadow DOM content in the AX tree (e.g., custom elements without ARIA), the `--pierce-shadow` flag triggers a supplemental pass:
1. `Runtime.evaluate` to find all shadow hosts: `document.querySelectorAll('*')` filtered by `element.shadowRoot !== null`
2. For each shadow host, `Runtime.evaluate` to query `element.shadowRoot.querySelectorAll('*')` for interactive elements
3. Merge supplemental nodes into the tree under their respective shadow host

### Frame-Scoped DOM Queries (same-origin frames)

For OOPIF (cross-origin) frames, `DOM.querySelector` works normally through the OOPIF session because `DOM.getDocument` returns the frame's own document root.

For same-origin frames, `DOM.querySelector` and `DOM.querySelectorAll` **cannot** reliably access the iframe's document through the shared page session. `DOM.getDocument` always returns the main frame's document, and attempts to navigate the DOM tree to the iframe's `contentDocument` via `DOM.getFrameOwner` + `DOM.describeNode` produce node IDs that `DOM.querySelectorAll` cannot use.

The solution uses **JavaScript-based queries** via `Runtime.evaluate` with the frame's `contextId` (from the isolated world):

1. `query_selector_in_context(session, selector, context_id)` — single-element resolution for `resolve_node` (used by `dom get-text`, `dom get-html`, etc.):
   - `Runtime.evaluate("document.querySelector(selector)", contextId)` → `RemoteObject`
   - `DOM.getDocument` (initializes the DOM agent)
   - `DOM.requestNode(objectId)` → session-scoped `nodeId`
   - Standard DOM commands proceed with the resolved `nodeId`

2. `query_selector_all_in_context(session, selector, context_id)` — multi-element query for `dom select`:
   - `Runtime.evaluate("Array.from(document.querySelectorAll(selector))", contextId)` → array `RemoteObject`
   - `Runtime.getProperties(objectId)` → per-element `RemoteObject`s
   - `DOM.requestNode` on each → session-scoped `nodeId`s

3. `page text --frame N` uses `Runtime.evaluate` with `contextId` directly (expression: `document.body?.innerText`), bypassing the DOM agent entirely.

**Design note**: `DOM.requestNode` requires the DOM agent to be initialized. A preceding `DOM.getDocument` call (even without `pierce: true`) is sufficient. Without this, `DOM.requestNode` fails silently or returns `nodeId: 0`.

### DOM Commands (`dom select --pierce-shadow`, etc.)

Standard `DOM.querySelector` does not pierce shadow boundaries. With `--pierce-shadow`:

1. First attempt `DOM.querySelector` on the document root (may find the element if it's not in shadow DOM)
2. If not found, use `Runtime.evaluate` with a JavaScript function that:
   - Recursively traverses shadow roots: `document.querySelectorAll('*')` → filter for `el.shadowRoot` → `el.shadowRoot.querySelectorAll(selector)`
   - Returns the first matching element as a `Runtime.RemoteObject`
3. Use `DOM.requestNode` to convert the `RemoteObject` to a DOM node ID
4. Proceed with the resolved node

For `dom select` (multi-element), the JS function collects all matches across all shadow roots.

### Interact/Form Commands with Shadow DOM UIDs

No `--pierce-shadow` flag needed. UIDs from a `--pierce-shadow` snapshot map to `backendDOMNodeId` values. `DOM.resolveNode` with `backendNodeId` works across shadow boundaries, so `interact click <uid>` resolves the element regardless of shadow DOM nesting. The existing UID resolution path in `interact.rs` and `form.rs` already uses `backendNodeId`.

---

## Frame-Scoped Network Monitoring

### `network list --frame <index>`

1. Resolve `--frame` to a frame ID via `list_frames`
2. Network events (`Network.requestWillBeSent`, `Network.responseReceived`, etc.) include a `frameId` field
3. Modify `NetworkRequestBuilder` in `src/network.rs` to capture `frame_id` from events
4. Add post-collection filter: only include requests where `frame_id == target_frame_id`

### `network intercept --frame <index>`

1. Resolve `--frame` to a frame ID
2. When setting up `Fetch.enable` or `Network.setRequestInterception`, the interception pattern already applies globally
3. In the interception handler, check `event.params.frameId` against the target frame
4. If the request is from the target frame: apply interception (modify/block)
5. If from another frame: `Fetch.continueRequest` to pass through unmodified

---

## State Management

### Snapshot State with Frame Context

Extend `SnapshotState` in `src/snapshot.rs` to include frame information:

```rust
pub struct SnapshotState {
    pub url: String,
    pub timestamp: String,
    pub uid_map: HashMap<String, i64>,
    pub frame_index: Option<u32>,    // NEW: which frame this snapshot is from
    pub frame_id: Option<String>,    // NEW: CDP frame ID for cross-invocation resolution
}
```

When a subsequent command uses `--frame <index>` with a UID, the snapshot state's `frame_index` is checked for consistency. If the UID was captured from frame 1 but the command targets frame 2, a warning is emitted (but execution proceeds, since the user may know what they're doing).

### Frame Argument Parsing State

The `FrameArg` enum is parsed in `frame.rs::parse_frame_arg()`:
- Pure digits → `FrameArg::Index(n)`
- Contains `/` → split on `/`, parse each segment as u32 → `FrameArg::Path(segments)`
- Literal `"auto"` → `FrameArg::Auto`
- Anything else → `AppError` with descriptive message

---

## Coordinate Translation for Frame-Scoped Interactions

When `interact click-at --frame 1 100 200` is used, coordinates must be translated from frame-relative to viewport-relative:

1. Get the `<iframe>` element's box model via `DOM.getBoxModel` on the frame's owner element
2. Extract the content quad points to determine the iframe's position in the viewport
3. Add the iframe's top-left offset to the user's coordinates: `viewport_x = frame_x + user_x`, `viewport_y = frame_y + user_y`
4. For nested iframes (path syntax), accumulate offsets through each nesting level
5. Dispatch `Input.dispatchMouseEvent` with the translated viewport coordinates

For UID-based interactions (`interact click --frame 1 s5`), the element center is already computed via `DOM.getBoxModel` which returns viewport-relative coordinates. No translation is needed since `DOM.getBoxModel` operates within the frame's session for OOPIFs, or includes the frame offset for same-origin frames.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: --frame as global opt** | Add `--frame` to `GlobalOpts` struct (like `--tab`) | Single definition, available everywhere | Pollutes commands that don't use frames (connect, tabs); conflicts with `--worker` become harder to express | Rejected — not all commands support frames |
| **B: --frame per command group** | Add `--frame` to each command group's args struct (PageArgs, DomArgs, etc.) | Precise control, clap validates per-command | Repeated field definition across structs | **Selected** — matches existing pattern for command-specific flags like `--compact`, `--verbose` |
| **C: Separate frame subcommand** | `agentchrome frame 1 page snapshot` prefix syntax | Clear scoping | Breaks existing CLI grammar, complex parsing | Rejected — too disruptive |
| **D: Always create OOPIF sessions** | Use `Target.attachToTarget` for all frames regardless of origin | Simpler code path — one approach for all frames | Same-origin frames may not have separate targets; unnecessary overhead | Rejected — would fail for same-origin frames in some Chrome configurations |
| **E: `FrameContext` dual strategy** | Same-origin: use page session + frameId params. OOPIF: attach to target, new session | Handles all frame types correctly | More complex implementation | **Selected** — correct and robust |
| **F: Enable-then-subscribe context detection** | Enable `Runtime` first, then subscribe to `Runtime.executionContextCreated` | Simple ordering | Race condition: replayed events arrive between `Runtime.enable` and `subscribe()`, causing non-deterministic fallback to isolated world | Rejected during verification — reordered to subscribe-then-enable (selected approach) |
| **G: DOM.querySelector for frame-scoped queries** | Use `DOM.getDocument(pierce: true)` + `DOM.querySelectorAll` on iframe content document | Standard CDP approach | `DOM.getDocument` returns main frame's document for same-origin frames via shared session; content document nodeIds from `DOM.requestNode` are unreliable with `DOM.querySelectorAll` | Rejected during verification — replaced by JS-based `Runtime.evaluate` with `contextId` |

---

## Security Considerations

- [x] **Input Validation**: `--frame` value validated at parse time (integer, path, or "auto"). Invalid values produce structured errors before any CDP communication.
- [x] **No New Attack Surface**: Frame targeting operates within the existing CDP session security boundary. AgentChrome already has full control of the browser via CDP.
- [x] **Cross-Origin Safety**: CDP provides cross-origin iframe access by design (it's a debugging protocol). No additional security bypass is needed or implemented.
- [x] **Shadow DOM**: Only open shadow roots are accessible. Closed shadow roots (mode: "closed") are not pierced, maintaining the web component's encapsulation intent.

---

## Performance Considerations

- [x] **Frame Resolution Overhead**: `Page.getFrameTree` is a single CDP call returning the complete hierarchy. Minimal overhead (< 50ms typical).
- [x] **Lazy Session Creation**: OOPIF sessions are created only when the target frame is cross-origin. Same-origin frames reuse the existing session.
- [x] **Auto-Detection Cost**: `--frame auto` must snapshot or check UIDs in each frame sequentially. Bounded by frame count; typical enterprise pages have < 10 iframes.
- [x] **Shadow DOM Traversal**: The supplemental shadow DOM pass (JS-based) only runs when `--pierce-shadow` is explicitly requested. No overhead for default commands.
- [x] **Network Frame Filtering**: Post-collection filter — no additional CDP subscriptions needed. The `frameId` field is already present in network events.

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Frame parsing | Unit | `parse_frame_arg` for integers, paths, "auto", invalid values |
| Frame tree ordering | Unit | `list_frames` with mock CDP response, verify depth-first document order |
| FrameContext resolution | Unit | Same-origin vs OOPIF detection logic |
| Coordinate translation | Unit | Frame offset calculation for nested iframes |
| Shadow DOM JS query | Unit | Query builder for recursive shadow root traversal |
| UID consistency | Integration | Snapshot in frame → interact in same frame with UID |
| Cross-origin iframe | Integration (BDD) | End-to-end with cross-origin test fixture |
| Worker targeting | Integration (BDD) | Service worker JS execution |
| Network frame filter | Integration (BDD) | Filter requests by frame |
| --frame auto | Integration (BDD) | Auto-detect element across frames |
| Legacy frameset | Integration (BDD) | `<frameset>` / `<frame>` targeting |
| Full feature | BDD (cucumber-rs) | All 28 acceptance criteria as Gherkin scenarios |

---

## Files Modified

| File | Change | Rationale |
|------|--------|-----------|
| `src/frame.rs` | **NEW** — Frame resolution, enumeration, `FrameContext`, `FrameArg` | Core frame targeting abstraction |
| `src/cli/mod.rs` | Add `--frame` to `PageArgs`, `DomArgs`, `InteractArgs`, `FormArgs`, `JsArgs`, `NetworkArgs`. Add `--worker` to `JsExecArgs`. Add `--pierce-shadow` to `PageSnapshotArgs` and `DomArgs`. Add `PageFrames` and `PageWorkers` subcommands. | CLI argument definitions |
| `src/main.rs` | Dispatch `PageFrames` and `PageWorkers` subcommands | Command routing |
| `src/lib.rs` | Add `pub mod frame;` | Module registration |
| `src/connection.rs` | No changes — frame resolution is handled in command modules after `setup_session` | Frame resolution is per-command, not per-session |
| `src/page.rs` | Add `execute_frames()` and `execute_workers()` handlers. Integrate `--frame` into existing page commands. | New subcommands + frame scoping |
| `src/snapshot.rs` | Extend `SnapshotState` with `frame_index` and `frame_id`. Add `--pierce-shadow` supplemental pass. Pass `frameId` to `Accessibility.getFullAXTree`. | Frame-scoped + shadow DOM snapshots |
| `src/js.rs` | Integrate `--frame` for frame-scoped execution. Add `--worker` for worker-scoped execution via separate session. | Frame + worker JS execution |
| `src/dom.rs` | Integrate `--frame` for frame-scoped DOM queries via JS-based `query_selector_in_context`/`query_selector_all_in_context` (see design note below). Add `--pierce-shadow` shadow root traversal. | Frame + shadow DOM for DOM commands |
| `src/output.rs` | Add `resolve_optional_frame()` shared helper for `--frame` resolution across command modules | Frame resolution DRY helper |
| `src/interact.rs` | Integrate `--frame` with coordinate translation for `click-at`, `hover`, `drag`. Route UID-based commands through frame session. | Frame-scoped interactions |
| `src/form.rs` | Integrate `--frame` for frame-scoped form operations. UID resolution through frame session. | Frame-scoped forms |
| `src/network.rs` | Capture `frameId` in `NetworkRequestBuilder`. Add `--frame` filter to `network list`. Add frame-scoped interception logic. | Frame-scoped network monitoring |
| `src/error.rs` | Add `frame_not_found()`, `frame_path_invalid()`, `frame_detached()`, `worker_not_found()`, `element_not_in_any_frame()` error constructors | New error variants |
| `src/examples.rs` | Add frame targeting, worker, and shadow DOM examples to page, interact, dom, network groups | Documentation |
| `tests/features/iframe-frame-targeting.feature` | **NEW** — BDD scenarios for all 28 ACs | Acceptance test coverage |
| `tests/fixtures/iframe-frame-targeting.html` | **NEW** — Test fixture with nested iframes, cross-origin frames, shadow DOM, frameset | Feature exercise fixture |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Chrome version differences in frame tree behavior | Medium | Medium | Test against Chrome stable + Chrome canary. `Page.getFrameTree` is stable CDP. |
| OOPIF detection fails for some frame configurations | Low | High | Fallback: if `Target.attachToTarget` fails, attempt same-origin frame access with `frameId` parameter. Log warning. |
| Shadow DOM accessibility tree inconsistencies across Chrome versions | Medium | Low | The `--pierce-shadow` JS supplemental pass handles cases where AX tree is incomplete. |
| Coordinate translation inaccuracies for deeply nested or transformed iframes | Low | Medium | Use `DOM.getBoxModel` on each nesting level. CSS transforms on iframes are rare in enterprise apps. |
| `--frame auto` performance on pages with many frames | Low | Low | Cap auto-detection at 50 frames. Document the cap in help text. |
| Worker session lifetime — worker may terminate between enumeration and execution | Low | Medium | Catch `Target.detachedFromTarget` event and return descriptive error. |

---

## Open Questions

- None

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #189 | 2026-04-15 | Initial design |
| #189 | 2026-04-15 | Verification sync: fixed context detection to subscribe-before-enable (default context with isolated world fallback); added JS-based DOM query approach for same-origin frames; added `contextId` scoping for `page text`; documented rejected alternatives F and G |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed (session file format extended)
- [x] State management approach is clear (SnapshotState extended)
- [x] No UI components (CLI tool)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
