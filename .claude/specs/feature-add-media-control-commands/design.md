# Design: Media Control Commands

**Issues**: #193
**Date**: 2026-04-16
**Status**: Draft
**Author**: Claude (spec agent)

---

## Overview

This feature adds a `media` command group to agentchrome for discovering and controlling HTML5 audio/video elements on a page. The implementation follows the established command module pattern: a new `src/media.rs` file implements the business logic, `cli/mod.rs` gets the clap derive types, `main.rs` gets a dispatch arm, and `examples.rs` gets a new command group.

All media operations are performed via JavaScript evaluation (`Runtime.evaluate`) against the page's `HTMLMediaElement` API. This is the simplest and most reliable approach — it avoids the CDP Media domain (which provides metadata observation but not playback control) and reuses the existing `ManagedSession` infrastructure for frame-scoped execution.

The target resolution model supports both zero-based integer indices (for quick positional targeting after `media list`) and `css:` prefixed CSS selectors (for stable targeting by class/attribute). The `--all` flag enables bulk operations, which is the primary use case (skipping all narration gates on a SCORM slide).

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────────┐
│  CLI Layer (cli/mod.rs)                                       │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ MediaArgs { frame, command: MediaCommand }               │ │
│  │ MediaCommand::List / Play / Pause / Seek / SeekEnd       │ │
│  │ MediaTargetArgs { target, all }                          │ │
│  │ MediaSeekArgs { target, time, all }                      │ │
│  └──────────────────────────────────────────────────────────┘ │
└───────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  Command Dispatch (main.rs)                                   │
│  Command::Media(args) => media::execute_media(&global, args)  │
└───────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  Command Module (media.rs)                                    │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ execute_media() — dispatcher                           │  │
│  │ execute_list() — query all media elements              │  │
│  │ execute_play() — play target element(s)                │  │
│  │ execute_pause() — pause target element(s)              │  │
│  │ execute_seek() — seek target element(s) to time        │  │
│  │ execute_seek_end() — seek target element(s) to end     │  │
│  └──────────────────────┬─────────────────────────────────┘  │
│                         │                                     │
│  Shared helpers:        │                                     │
│  ┌──────────────────────▼─────────────────────────────────┐  │
│  │ build_list_js() — JS to enumerate media elements       │  │
│  │ build_action_js() — JS to perform action + return state│  │
│  │ build_bulk_action_js() — JS for --all operations       │  │
│  │ resolve_target() — parse index vs css: selector        │  │
│  │ parse_media_info() — JSON response → MediaInfo structs │  │
│  └────────────────────────────────────────────────────────┘  │
└───────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  CDP Client (Runtime.evaluate)                                │
│  Executes JavaScript in page context (or frame context)       │
│  Returns serialized JSON via returnByValue: true              │
└──────────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. User runs: agentchrome media seek-end --all --frame 0
2. CLI layer parses args → MediaArgs { frame: Some("0"), command: SeekEnd { all: true, target: None } }
3. main.rs dispatches → media::execute_media()
4. execute_media() calls setup_session() → CdpClient + ManagedSession
5. resolve_optional_frame() switches execution context to frame 0
6. execute_seek_end() builds bulk JS:
   "JSON.stringify(Array.from(document.querySelectorAll('audio,video')).map((el,i) => {
     el.currentTime = el.duration; return { index: i, tag: el.tagName.toLowerCase(), ... };
   }))"
7. ManagedSession.send_command("Runtime.evaluate", { expression, returnByValue: true })
8. Parse JSON response → Vec<MediaInfo>
9. print_output() serializes to stdout as JSON
```

---

## API / Interface Changes

### New CLI Commands

| Command | Args | Purpose |
|---------|------|---------|
| `media list` | (none) | List all audio/video elements with state |
| `media play <target>` | target: index or css:selector | Play a specific element |
| `media pause <target>` | target: index or css:selector | Pause a specific element |
| `media seek <target> <time>` | target + time in seconds | Seek to a specific time |
| `media seek-end <target>` | target: index or css:selector | Seek to end (duration) |

### Shared Flags

| Flag | Type | Scope | Purpose |
|------|------|-------|---------|
| `--all` | bool | play, pause, seek, seek-end | Apply to all media elements |
| `--frame` | String | group level (MediaArgs) | Target a specific iframe |

### Response Schema

#### `media list` Response

```json
[
  {
    "index": 0,
    "tag": "audio",
    "src": "narration.mp3",
    "currentSrc": "https://example.com/narration.mp3",
    "duration": 30.0,
    "currentTime": 0.0,
    "state": "paused",
    "muted": false,
    "volume": 1.0,
    "loop": false,
    "readyState": 4
  }
]
```

#### `media play/pause/seek/seek-end` Response (single target)

```json
{
  "index": 0,
  "tag": "audio",
  "src": "narration.mp3",
  "currentSrc": "https://example.com/narration.mp3",
  "duration": 30.0,
  "currentTime": 30.0,
  "state": "ended",
  "muted": false,
  "volume": 1.0,
  "loop": false,
  "readyState": 4
}
```

#### `--all` Response (bulk)

```json
[
  { "index": 0, "tag": "audio", "state": "ended", "..." : "..." },
  { "index": 1, "tag": "audio", "state": "ended", "..." : "..." }
]
```

### Errors

| Condition | Error Message | Exit Code |
|-----------|---------------|-----------|
| No media element at index | "Media element at index {N} not found. Page has {M} media elements." | 1 (GeneralError) |
| No media element matching selector | "No media element matching selector '{sel}' found." | 1 (GeneralError) |
| No target and no --all flag | Clap validation error (required argument) | 1 |
| NaN duration on seek-end | "Media element at index {N} has no duration (NaN). Cannot seek to end." | 1 (GeneralError) |

---

## Database / Storage Changes

None. Media commands are stateless — they query/modify the live page state via JS evaluation each invocation.

---

## State Management

No persistent state. Each command invocation:
1. Connects to Chrome via existing session
2. Evaluates JavaScript in the target page/frame context
3. Returns the result and exits

The media element state (playing/paused/ended, currentTime, etc.) is owned by the browser page, not by agentchrome. This is consistent with all other agentchrome commands.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: CDP Media domain** | Use Media.enable and listen to Media events for metadata | Richer metadata (codec, pipeline info) | No playback control (can't play/pause/seek via CDP Media domain); event-driven so needs persistent subscription | Rejected — no playback control |
| **B: JS evaluation via Runtime.evaluate** | Query HTMLMediaElement API directly via JS | Full playback control; simple; no domain subscription needed; works in frames | Less metadata than Media domain | **Selected** — sufficient for all use cases |
| **C: Hybrid (CDP Media + JS)** | CDP Media for discovery, JS for control | Best of both | Complexity; Media domain doesn't reliably enumerate elements; would need two code paths | Rejected — unnecessary complexity |

---

## Security Considerations

- [x] **Authentication**: N/A — uses existing CDP session
- [x] **Authorization**: N/A — local Chrome instance
- [x] **Input Validation**: Target index validated as non-negative integer; CSS selectors passed to `document.querySelector()` which safely handles invalid selectors
- [x] **Data Sanitization**: No user data stored; JS evaluation uses parameterized values (index/selector injected into template, not string concatenation with user input) — CSS selectors must be escaped to prevent JS injection
- [x] **Sensitive Data**: N/A — media element metadata only

---

## Performance Considerations

- [x] **Single JS evaluation per command**: Each subcommand executes exactly one `Runtime.evaluate` call (no round-trips)
- [x] **returnByValue**: Results are returned inline in the CDP response, avoiding object handle resolution
- [x] **No domain subscription**: Unlike console/network follow, media commands don't enable CDP domains or subscribe to events — zero overhead when not in use

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Output serialization | Unit | MediaInfo, ActionResult struct serialization; NaN duration → null |
| Target resolution | Unit | Index parsing, css: prefix detection, invalid input handling |
| Plain text formatting | Unit | Plain output for list and action results |
| CLI argument parsing | Unit | Verify clap derives parse correctly, mutual exclusivity of target/--all |
| Feature | Integration (BDD) | End-to-end acceptance criteria (AC1–AC13) via cucumber-rs |
| Feature exercise | Smoke test | Manual verification against real headless Chrome with test fixture |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Media elements with NaN duration (no source loaded) | Medium | Low | Check for NaN and return null; error on seek-end with NaN duration |
| Autoplay restrictions prevent play() from resolving | Low | Low | play() returns a Promise; evaluate with awaitPromise to detect rejection |
| CSS selector injection in JS template | Low | Medium | Escape selector strings before interpolation into JS |
| Frame detachment during execution | Low | Low | Existing frame resolution error handling applies |

---

## Open Questions

- None

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #193 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] State management approach is clear (stateless)
- [x] No UI components (CLI tool)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
