# Root Cause Analysis: console read always returns empty array

**Issue**: #103
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude (spec generation)

---

## Root Cause

The `console read` command always returns an empty array because of a fundamental timing mismatch between CDP session lifecycle and the console event model — the same class of defect as issue #102 (`network list`).

Each `chrome-cli` invocation creates a **new CDP WebSocket connection** and a **new session** (via `setup_session()` at `src/console.rs:146-157`). The `execute_read()` function (line 386) then enables the `Runtime` domain and subscribes to `Runtime.consoleAPICalled` events. However, CDP's Runtime domain is **event-driven only** — it fires `consoleAPICalled` events as console calls happen in real-time. There is no retrospective API such as `Runtime.getHistoricalConsoleMessages()`. By the time the new connection enables the Runtime domain and subscribes, all previous console activity has already occurred and those events were never captured.

The 100ms drain window (`src/console.rs:426`) compounds the issue: for a page that has already generated console output, no `consoleAPICalled` events will arrive during the drain because the calls already happened before the subscription existed. The drain only catches events that fire between `Runtime.enable` and the deadline — which for historical console messages is zero events.

In contrast, `console follow` works because it maintains a **persistent connection** that is already listening when future console calls occur. Similarly, `js exec` captures console output because it subscribes before executing the JavaScript, and the drain window catches events generated during that execution.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/console.rs` | 146-157 | `setup_session()` — creates a new CDP connection per invocation |
| `src/console.rs` | 386-531 | `execute_read()` — enables Runtime domain, subscribes, and drains for 100ms; no events arrive because console calls already completed |
| `src/console.rs` | 424-462 | Drain loop — 100ms idle timeout collects zero events for historical console messages |

### Triggering Conditions

- Console messages were generated before the `console read` CLI invocation (always true for normal usage)
- CDP's `Runtime.consoleAPICalled` is event-driven with no retrospective query API
- Each CLI invocation creates a fresh CDP session with no event history

---

## Fix Strategy

### Approach

Apply the same pattern that fixed issue #102 (`network list`): after enabling the Runtime domain and subscribing to console events, trigger a **page reload** to replay the page's scripts and regenerate console output, then collect the resulting `Runtime.consoleAPICalled` events.

This approach works because:

1. It uses the existing event subscription and parsing infrastructure unchanged
2. A reload re-executes the page's scripts, which regenerate their console calls
3. It works regardless of how console messages were originally generated (inline scripts, external scripts, framework initialization)
4. `Page.reload` is a well-supported CDP command already used elsewhere in the codebase

After enabling the Runtime domain and subscribing to events, `execute_read()` should:
1. Enable the `Page` domain and subscribe to `Page.loadEventFired`
2. Trigger `Page.reload`
3. Wait for `Page.loadEventFired` to signal reload completion
4. Apply a short post-load idle window to catch console messages from deferred scripts
5. Use a total timeout fallback to prevent hanging on slow pages

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/console.rs` | Modify `execute_read()` to enable the `Page` domain, subscribe to `Page.loadEventFired`, trigger `Page.reload`, then wait for page load completion plus a short idle window instead of the fixed 100ms drain | Ensures console events are generated and captured by replaying the page's scripts |
| `src/console.rs` | Add a configurable total timeout for the reload+drain cycle (e.g., 5s default, respecting `--timeout` if set) | Prevents indefinite hanging on slow or broken pages |

### Blast Radius

- **Direct impact**: `execute_read()` — the only function modified; both list mode and detail mode benefit since they operate on the same collected events
- **Indirect impact**: None — `execute_follow()` has its own independent event loop (line 537+) and does not share the drain logic
- **Risk level**: Low — the reload is a standard CDP operation; the change is contained within `execute_read()` which is independent from `execute_follow()`

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `console follow` behavior changes | Low | `follow` has its own code path (`execute_follow()`) and shares no drain logic with `read` |
| Reload causes visible flicker in headed mode | Low | Reload is a normal browser operation; users of `console read` expect it to inspect the current page |
| Reload loses page state (form data, SPA state) | Medium | This is an inherent trade-off of the replay approach. Document that `console read` replays the page. For stateful pages, users should prefer `console follow` to capture messages in real-time. The same trade-off exists and was accepted for `network list` (#102) |
| `--include-preserved` cross-navigation tracking changes | Low | Navigation tracking via `current_nav_id` still works since the reload increments the navigation counter; the existing filtering logic in lines 464-478 is preserved |
| Console messages from dynamically-triggered scripts (e.g., user clicks) not captured | Low | This is expected — replay only re-executes initial page load scripts. Out of scope per requirements. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| **Background daemon** | Run a persistent process that keeps `Runtime.enable` active and caches console messages; `console read` queries the daemon | Over-engineered for a CLI tool; introduces process management complexity. Violates the "zero config" product principle. |
| **Longer drain timeout** | Increase the 100ms drain to several seconds | Does not solve the fundamental problem — for already-occurred console calls, no events will arrive regardless of how long we wait. Only wastes time. |
| **Inject JavaScript to replay `console.log` history** | Use `Runtime.evaluate` to read a hypothetical console history object | No such browser API exists. `console.log` does not store a history buffer accessible to scripts. |
| **Use `Log.entryAdded` domain** | Subscribe to the `Log` domain instead of or in addition to `Runtime.consoleAPICalled` | `Log.entryAdded` has the same event-driven limitation — no retrospective API. Also, `Log` domain captures browser-internal logs, not user `console.*` calls. |

---

## Validation Checklist

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
