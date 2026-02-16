# Root Cause Analysis: network list always returns empty array

**Issue**: #102
**Date**: 2026-02-15
**Status**: Approved
**Author**: Claude (spec generation)

---

## Root Cause

The `network list` command always returns an empty array because of a fundamental timing mismatch between CDP session lifecycle and the network event model.

Each `chrome-cli` invocation creates a **new CDP WebSocket connection** and a **new session** (via `setup_session()` at `src/network.rs:230-241`). The `collect_and_correlate()` function (line 466) then enables the `Network` domain and subscribes to events. However, CDP's Network domain is **event-driven only** — it fires events (`requestWillBeSent`, `responseReceived`, `loadingFinished`) as requests happen in real-time. There is no retrospective API such as `Network.getHistoricalRequests()`. By the time the new connection enables the Network domain, all page load requests have already completed and their events were never captured.

The 100ms drain window (`src/network.rs:525`) compounds the issue: even if a few straggling events trickled in, the window is far too short to capture any meaningful data from an already-loaded page. The drain only catches events that happen to fire between `Network.enable` and the deadline — which for a fully loaded page is zero events.

In contrast, `network follow` works because it maintains a **persistent connection** that is already listening when future requests are made. The user triggers new requests (reload, navigation, XHR) after the connection is established, so events are captured live.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/network.rs` | 230-241 | `setup_session()` — creates a new CDP connection per invocation |
| `src/network.rs` | 466-530 | `collect_and_correlate()` — enables Network domain and drains events for 100ms; no events arrive because requests already completed |
| `src/network.rs` | 739-777 | `execute_list()` — calls `collect_and_correlate()` which returns empty builders |
| `src/network.rs` | 784-928 | `execute_get()` — same issue; no events captured, so request ID lookup fails |

### Triggering Conditions

- The page has already finished loading before `network list` is invoked (always true for normal CLI usage)
- CDP's Network domain is event-driven with no retrospective query API
- Each CLI invocation creates a fresh CDP session with no event history

---

## Fix Strategy

### Approach

The fix should trigger a **page reload** after enabling the Network domain and subscribing to events, then collect the resulting network traffic. This is the most reliable approach because:

1. It uses the existing event collection and correlation infrastructure unchanged
2. A reload replays the page's network requests, making them capturable via CDP events
3. It works regardless of page state (fully loaded, partial, SPA)
4. `Page.reload` is a well-supported CDP command already used by `navigate reload`

The reload should be **automatic and implicit** for `network list` and `network get`. After enabling the Network domain and subscribing to events, `collect_and_correlate()` should trigger `Page.reload` and then collect events until the page finishes loading (using `Page.loadEventFired` or `Page.frameStoppedLoading` as the completion signal), with a reasonable timeout fallback.

The drain strategy should change from the current fixed 100ms idle timeout to a **page-load-aware** approach: wait for the page load event after the reload, then apply a short idle window to catch any trailing async requests.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/network.rs` | Modify `collect_and_correlate()` to trigger `Page.reload` after subscribing to events, then wait for page load completion (via `Page.loadEventFired` or `Page.frameStoppedLoading`) plus a short idle window instead of a fixed 100ms drain | Ensures network events are generated and captured by replaying the page's network activity |
| `src/network.rs` | Subscribe to `Page.loadEventFired` in `collect_and_correlate()` as a completion signal | Provides a reliable indicator that the reload has finished and most requests are complete |
| `src/network.rs` | Add a configurable total timeout for the reload+drain cycle (e.g., 5s default, respecting `--timeout` if set) | Prevents indefinite hanging on slow or broken pages |

### Blast Radius

- **Direct impact**: `execute_list()` and `execute_get()` — both call `collect_and_correlate()`, so both benefit from the fix
- **Indirect impact**: None — `execute_follow()` has its own independent event loop and does not call `collect_and_correlate()`
- **Risk level**: Low — the reload is a standard CDP operation; the change is contained within `collect_and_correlate()` which is only called by `list` and `get`

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `network follow` behavior changes | Low | `follow` has its own code path (`execute_follow()`) and does not call `collect_and_correlate()` — no shared code is modified |
| Reload causes visible flicker in headed mode | Low | Reload is a normal browser operation; users of `network list` expect it to inspect the page |
| Reload triggers side effects on stateful pages (POST resubmission) | Low | CDP's `Page.reload` does not resubmit POST data by default; it performs a GET reload. Additionally, this matches the behavior a user would expect — `network list` captures the page's current network profile |
| Timeout too short for slow pages | Medium | Use a generous default (5s) and respect the existing `--timeout` global flag; document the behavior |
| `--include-preserved` flag behavior changes | Low | Navigation tracking (`current_nav_id`) still works since the reload increments the navigation counter; previous requests remain preserved as before |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| **Background daemon** | Run a persistent process that keeps `Network.enable` active and caches requests; `network list` queries the daemon | Over-engineered for a CLI tool; introduces process management complexity, IPC, and daemon lifecycle issues. Violates the "zero config" product principle. |
| **Document the limitation** | Update docs to say `network list` requires `network follow` running in another terminal | Does not fix the bug; contradicts the feature spec (AC1 of issue #19) which says `network list` should return captured requests from a loaded page |
| **Longer drain timeout** | Increase the 100ms drain to several seconds | Does not solve the fundamental problem — for an already-loaded page, no events will arrive regardless of how long we wait. Only wastes time. |
| **Use `Page.getResourceTree` / `Network.getCachedResources`** | Query Chrome's resource cache for previously loaded resources | `Page.getResourceTree` returns resource metadata but not full network request/response details (no timing, headers, status codes). Insufficient for the feature requirements. |

---

## Validation Checklist

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
