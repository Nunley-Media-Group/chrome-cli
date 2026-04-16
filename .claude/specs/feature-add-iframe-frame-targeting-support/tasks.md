# Tasks: Iframe/Frame Targeting Support

**Issues**: #189
**Date**: 2026-04-15
**Status**: Planning
**Author**: Rich Nunley

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 3 (T001–T003) | [ ] |
| Backend | 12 (T004–T015) | [ ] |
| Frontend | N/A (CLI tool) | N/A |
| Integration | 4 (T016–T019) | [ ] |
| Testing | 5 (T020–T024) | [ ] |
| **Total** | **24 tasks** | |

---

## Phase 1: Setup

### T001: Create frame module with core types

**File(s)**: `src/frame.rs`, `src/lib.rs`
**Type**: Create, Modify
**Depends**: None
**Acceptance**:
- [ ] `src/frame.rs` exists with `FrameArg`, `FrameInfo`, and `FrameContext` type definitions
- [ ] `FrameArg` enum has variants: `Index(u32)`, `Path(Vec<u32>)`, `Auto`
- [ ] `FrameInfo` struct has all metadata fields: `index`, `id`, `url`, `name`, `security_origin`, `unreachable`, `width`, `height`, `depth`, `parent_id`, `child_ids`
- [ ] `FrameContext` enum has variants: `MainFrame`, `SameOrigin { frame_id, execution_context_id }`, `OutOfProcess { session, frame_id }`
- [ ] `pub mod frame;` added to `src/lib.rs`
- [ ] `parse_frame_arg()` function parses integers, slash-separated paths, and "auto"
- [ ] `cargo build` succeeds

**Notes**: Types only — no CDP logic yet. `FrameContext` variants will be populated by T005.

### T002: Add CLI arguments and subcommands

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `--frame` argument (type `Option<String>`) added to `PageArgs`, `DomArgs`, `InteractArgs`, `FormArgs`, `JsArgs`, `NetworkListArgs`, `NetworkInterceptArgs`
- [ ] `--worker` argument (type `Option<u32>`) added to `JsExecArgs` with `conflicts_with = "frame"`
- [ ] `--pierce-shadow` boolean flag added to `PageSnapshotArgs` and `DomArgs`
- [ ] `PageSubcommand::Frames` variant added (no args)
- [ ] `PageSubcommand::Workers` variant added (no args)
- [ ] `cargo build` succeeds
- [ ] `cargo clippy --all-targets` passes

**Notes**: Follow existing patterns for argument placement. Use `#[arg(long)]` for all new flags.

### T003: Add error constructors for frame/worker targeting

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `frame_not_found(index: u32)` constructor returns TargetError (code 3) with message referencing `page frames`
- [ ] `frame_path_invalid(path: &str, segment: u32, max_children: u32)` constructor returns TargetError with descriptive message
- [ ] `frame_detached()` constructor returns TargetError with message about frame no longer being available
- [ ] `element_not_in_any_frame()` constructor returns TargetError with message referencing `page frames`
- [ ] `worker_not_found(index: u32)` constructor returns TargetError with message referencing `page workers`
- [ ] `cargo build` succeeds

---

## Phase 2: Backend Implementation

### T004: Implement frame enumeration (Page.getFrameTree)

**File(s)**: `src/frame.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `list_frames()` sends `Page.getFrameTree` via the managed session
- [ ] Response parsed into depth-first document-order `Vec<FrameInfo>`
- [ ] Main frame assigned index 0, depth 0
- [ ] Child frames indexed sequentially in depth-first order
- [ ] `<frame>` elements within `<frameset>` are enumerated identically to `<iframe>`
- [ ] Frame dimensions obtained via `DOM.getBoxModel` on frame owner elements (0 if unavailable)
- [ ] `cargo test --lib` passes for unit tests on tree traversal

**Notes**: `Page.getFrameTree` returns `{ frameTree: { frame: {...}, childFrames: [...] } }`. Recurse depth-first.

### T005: Implement frame resolution (same-origin vs OOPIF)

**File(s)**: `src/frame.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `resolve_frame()` accepts `FrameArg::Index(n)` and returns the correct `FrameContext`
- [ ] For index 0 or absent frame arg → `FrameContext::MainFrame`
- [ ] For same-origin frames → `FrameContext::SameOrigin` with `frame_id` and `execution_context_id` obtained from `Runtime.executionContextCreated` events
- [ ] For OOPIF frames → `FrameContext::OutOfProcess` with new `ManagedSession` via `Target.attachToTarget`
- [ ] OOPIF detection via `Target.getTargets` checking for matching frame target
- [ ] Invalid index returns `AppError::frame_not_found()`
- [ ] `frame_session()` helper returns the appropriate session reference
- [ ] `frame_id()` helper returns `Option<&str>` for CDP method parameters

### T006: Implement nested frame path resolution

**File(s)**: `src/frame.rs`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] `resolve_frame()` handles `FrameArg::Path(segments)` by traversing the frame tree parent→child
- [ ] Each path segment indexes into the current frame's `child_ids`
- [ ] If a segment exceeds available children → `AppError::frame_path_invalid()` with the failing segment
- [ ] Path `[0]` resolves to main frame's first child (not main frame itself — path always starts from main frame)
- [ ] `cargo test --lib` passes for path resolution unit tests

### T007: Implement page frames subcommand

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `execute_frames()` handler calls `list_frames()` and outputs JSON array on stdout
- [ ] Each frame serialized with all `FrameInfo` fields (index, id, url, name, securityOrigin, unreachable, width, height, depth)
- [ ] Output respects `--json`/`--pretty`/`--plain` output format flags
- [ ] Empty page (no iframes) returns single-element array with main frame at index 0
- [ ] `cargo build` succeeds

### T008: Implement page workers subcommand

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `execute_workers()` handler calls `Target.getTargets` and filters for worker types
- [ ] Workers classified as `"service_worker"`, `"shared_worker"`, or `"worker"` based on CDP target type
- [ ] Worker status derived from CDP target info
- [ ] JSON array output with index, id, type, url, status
- [ ] Empty result (no workers) returns empty JSON array `[]`

### T009: Integrate --frame into page commands

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] `page snapshot --frame N` calls `Accessibility.getFullAXTree` with `frameId` parameter (same-origin) or via OOPIF session
- [ ] `page find --frame N` searches within the targeted frame
- [ ] `page screenshot --frame N` captures the frame's content (uses `clip` parameter based on frame bounds for same-origin, or session-scoped capture for OOPIF)
- [ ] `page text --frame N` extracts text from the targeted frame only
- [ ] `page element --frame N` resolves UIDs within the frame
- [ ] `page wait --frame N` waits for conditions within the frame
- [ ] Commands without `--frame` behave identically to current implementation (AC10)

### T010: Extend snapshot state with frame context

**File(s)**: `src/snapshot.rs`
**Type**: Modify
**Depends**: T009
**Acceptance**:
- [ ] `SnapshotState` struct extended with `frame_index: Option<u32>` and `frame_id: Option<String>`
- [ ] `write_snapshot_state()` persists frame fields when present
- [ ] `read_snapshot_state()` deserializes frame fields (backwards compatible — old files without these fields still load)
- [ ] Frame-scoped snapshots persist frame context for cross-invocation UID resolution (AC9)
- [ ] `cargo test --lib` passes for serialization round-trip

### T011: Integrate --frame and --worker into js exec

**File(s)**: `src/js.rs`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] `js exec --frame N` resolves frame, uses `Runtime.evaluate` with frame's execution context ID (same-origin) or via OOPIF session
- [ ] `js exec --worker N` attaches to worker target, uses `Runtime.evaluate` via worker session
- [ ] `--frame` and `--worker` are mutually exclusive (enforced by clap)
- [ ] Without `--frame` or `--worker`, behavior is identical to current (AC10)
- [ ] Frame-scoped `document.title` returns the iframe's title (AC4)

### T012: Integrate --frame and --pierce-shadow into dom commands

**File(s)**: `src/dom.rs`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] All dom subcommands (`select`, `get-text`, `get-html`, `get-attribute`, `set-attribute`, `set-text`, `remove`, `get-style`, `set-style`, `parent`, `children`, `siblings`) accept and use `--frame`
- [ ] Frame-scoped `DOM.querySelector` resolves within the frame's document root
- [ ] `--pierce-shadow` on `dom select` uses JS-based recursive shadow root traversal when standard `DOM.querySelector` misses
- [ ] `--pierce-shadow` on other dom commands resolves the target through shadow DOM if needed
- [ ] `--frame` and `--pierce-shadow` can be combined (AC28)

### T013: Integrate --frame into interact commands with coordinate translation

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] All interact subcommands (`click`, `click-at`, `hover`, `drag`, `type`, `key`, `scroll`) accept and use `--frame`
- [ ] UID-based interactions (`click`, `hover`, `type`) resolve UIDs within the frame context
- [ ] `click-at --frame N x y` translates coordinates: gets iframe's viewport offset via `DOM.getBoxModel`, adds offset to user coordinates, dispatches to main viewport
- [ ] `hover --frame N` and `drag --frame N` apply the same coordinate translation
- [ ] `key --frame N` and `scroll --frame N` work within frame context (key dispatch is session-scoped, scroll targets frame)
- [ ] Coordinates for UID-based commands use `DOM.getBoxModel` which already returns viewport-relative coordinates via the frame session

### T014: Integrate --frame into form commands

**File(s)**: `src/form.rs`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] All form subcommands (`fill`, `fill-many`, `clear`, `submit`) accept and use `--frame`
- [ ] UID resolution uses the frame's snapshot state (matching `frame_index`)
- [ ] Form operations execute within the frame's DOM context
- [ ] `--frame` combined with UID from frame-scoped snapshot resolves correctly (AC9)

### T015: Integrate --frame into network commands

**File(s)**: `src/network.rs`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] `NetworkRequestBuilder` extended with `frame_id: Option<String>` field
- [ ] `frame_id` captured from `Network.requestWillBeSent` event's `frameId` parameter
- [ ] `network list --frame N` filters results to only requests from the specified frame
- [ ] `network intercept --frame N` checks `frameId` in interception handler; non-matching requests pass through via `Fetch.continueRequest`
- [ ] Without `--frame`, all requests are shown/intercepted as before (no regression)

---

## Phase 3: N/A (CLI Tool — No Frontend)

---

## Phase 4: Integration

### T016: Implement --frame auto (automatic frame search)

**File(s)**: `src/frame.rs`
**Type**: Modify
**Depends**: T005, T010
**Acceptance**:
- [ ] `resolve_frame_auto()` iterates frames in document order
- [ ] For each frame, checks if the target UID exists in the frame's UID map (via quick snapshot or persisted state)
- [ ] Returns `(FrameContext, frame_index)` for the first match
- [ ] If no frame contains the target → `AppError::element_not_in_any_frame()`
- [ ] Auto-detected frame index included in command output as `"frame": N` field
- [ ] Capped at 50 frames to bound search time

### T017: Wire new subcommands in main.rs dispatch

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T007, T008
**Acceptance**:
- [ ] `PageSubcommand::Frames` dispatched to `page::execute_frames()`
- [ ] `PageSubcommand::Workers` dispatched to `page::execute_workers()`
- [ ] `cargo build` succeeds
- [ ] Manual test: `agentchrome page frames --help` shows correct usage

### T018: Update examples with frame, worker, and shadow DOM

**File(s)**: `src/examples.rs`
**Type**: Modify
**Depends**: T007, T008, T009, T011, T012
**Acceptance**:
- [ ] `examples page` includes: `page frames`, `page snapshot --frame 1`, `page snapshot --pierce-shadow`, `page workers`
- [ ] `examples interact` includes: `interact click --frame 1 s3`, `interact click-at --frame 1 100 200`
- [ ] `examples dom` includes: `dom select --frame 1 "css:button"`, `dom select --pierce-shadow "css:#shadow-btn"`
- [ ] `examples js` includes: `js exec --frame 1 "document.title"`, `js exec --worker 0 "self.registration.scope"`
- [ ] `examples network` includes: `network list --frame 1`
- [ ] Examples follow existing `ExampleEntry` struct pattern

### T019: Implement --pierce-shadow supplemental pass in snapshot

**File(s)**: `src/snapshot.rs`
**Type**: Modify
**Depends**: T009
**Acceptance**:
- [ ] When `--pierce-shadow` is set and standard `Accessibility.getFullAXTree` misses shadow DOM elements, a supplemental JS pass runs
- [ ] JS pass finds shadow hosts via `document.querySelectorAll('*')` filtered by `el.shadowRoot !== null`
- [ ] For each shadow host, queries `el.shadowRoot` for interactive elements
- [ ] Supplemental nodes merged into the tree under their shadow host parent
- [ ] UIDs assigned to shadow DOM elements follow the same `s{N}` convention
- [ ] `backendDOMNodeId` for shadow elements is captured for cross-command resolution

---

## Phase 5: BDD Testing

### T020: Create test fixture HTML file

**File(s)**: `tests/fixtures/iframe-frame-targeting.html`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] HTML file contains: main page with form and button, same-origin iframe with unique content, nested iframe (grandchild), cross-origin simulation (srcdoc or data: URI for isolated context), `<frameset>` section with `<frame>` elements, web component with open shadow DOM root containing interactive elements
- [ ] HTML comment at top documents which ACs each section covers
- [ ] No external dependencies (self-contained, no CDN)
- [ ] Deterministic content — no JavaScript randomness or timing dependencies

### T021: Create BDD feature file

**File(s)**: `tests/features/iframe-frame-targeting.feature`
**Type**: Create
**Depends**: T020
**Acceptance**:
- [ ] All 28 acceptance criteria from requirements.md have corresponding Gherkin scenarios
- [ ] Feature file uses Given/When/Then format
- [ ] Scenarios are independent (no shared mutable state)
- [ ] Valid Gherkin syntax

### T022: Implement BDD step definitions

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T021
**Acceptance**:
- [ ] Step definitions added for all scenarios in `iframe-frame-targeting.feature`
- [ ] Steps follow existing cucumber-rs World patterns in `bdd.rs`
- [ ] Steps invoke agentchrome CLI commands and validate JSON output
- [ ] `cargo test --test bdd` compiles successfully (Chrome-dependent scenarios may be skipped in CI)

### T023: Smoke test against real headless Chrome

**File(s)**: (no file changes — execution task)
**Type**: Verify
**Depends**: T009, T011, T012, T013, T014, T015, T016, T019, T020
**Acceptance**:
- [ ] `cargo build` succeeds
- [ ] `agentchrome connect --launch --headless` starts Chrome
- [ ] `agentchrome navigate file://<path>/tests/fixtures/iframe-frame-targeting.html` loads fixture
- [ ] `agentchrome page frames` returns JSON array with all expected frames
- [ ] `agentchrome page snapshot --frame 1` returns iframe-scoped accessibility tree
- [ ] `agentchrome js exec --frame 1 "document.title"` returns the iframe's title
- [ ] `agentchrome dom select --frame 1 "css:button"` returns element from iframe
- [ ] `agentchrome interact click --frame 1 <uid>` clicks element in iframe
- [ ] `agentchrome page snapshot --pierce-shadow` includes shadow DOM elements
- [ ] `agentchrome page frames` includes `<frame>` elements from frameset
- [ ] `agentchrome page snapshot --frame 99` returns JSON error with code 3
- [ ] `agentchrome connect disconnect` and Chrome cleanup

### T024: Verify no regressions

**File(s)**: (no file changes — verification task)
**Type**: Verify
**Depends**: T023
**Acceptance**:
- [ ] `cargo test --lib` passes (all unit tests)
- [ ] `cargo clippy --all-targets` passes
- [ ] `cargo fmt --check` passes
- [ ] All existing commands without `--frame` behave identically (spot check: `page snapshot`, `js exec`, `dom select`, `interact click`, `form fill`, `network list`)
- [ ] No orphaned Chrome processes after testing

---

## Dependency Graph

```
T001 ──┬──▶ T002 ──▶ T008
       │
       ├──▶ T004 ──▶ T005 ──┬──▶ T006
       │              │      │
T003 ──┘              │      ├──▶ T007 ──┐
                      │      │           │
                      ├──▶ T009 ──┬──▶ T010   T020 (parallel)
                      │      │    │           │
                      │      │    └──▶ T019   │
                      │      │                │
                      ├──▶ T011               │
                      │      │                │
                      ├──▶ T012               │
                      │      │                │
                      ├──▶ T013               │
                      │      │                │
                      ├──▶ T014               │
                      │      │                │
                      └──▶ T015               │
                             │                │
                      T016 ◀─┤ (depends T005, T010)
                             │                │
                      T017 ◀─┤ (depends T007, T008)
                             │                │
                      T018 ◀─┤ (depends T007+)│
                             │                │
                             ▼                ▼
                      T021 ◀──────────────── T020
                             │
                             ▼
                            T022
                             │
                             ▼
                            T023
                             │
                             ▼
                            T024
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #189 | 2026-04-15 | Initial task breakdown |

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included (T020–T024)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
