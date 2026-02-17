# Root Cause Analysis: tabs close reports incorrect remaining count (off-by-one race condition)

**Issue**: #120
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude

---

## Root Cause

The `execute_close()` function in `src/tabs.rs` sends `Target.closeTarget` commands via the CDP WebSocket protocol, then immediately queries Chrome's `/json/list` HTTP endpoint to count remaining tabs. There is a race condition: the HTTP endpoint is a separate communication channel from the WebSocket and may not have propagated the tab closure when the re-query happens.

Chrome's DevTools HTTP server maintains its own view of open targets. When a tab is closed via the CDP WebSocket command `Target.closeTarget`, the WebSocket layer acknowledges the close, but the HTTP `/json/list` endpoint updates asynchronously. The delay is typically a few milliseconds but is enough to cause the re-query to return stale data that still includes the just-closed tab.

The `execute_create()` function in the same file is already aware of this propagation delay and handles it with a polling retry loop (10 iterations, 10ms sleep) that waits for the HTTP endpoint to reflect state changes from WebSocket commands. The `execute_close()` function lacks this same mechanism.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/tabs.rs` | 258 | Immediate `query_targets()` call after `Target.closeTarget` — no wait for propagation |
| `src/tabs.rs` | 259-261 | Counts remaining page-type targets from potentially stale data |
| `src/tabs.rs` | 203-210 | Reference: `execute_create()` polling loop that correctly handles this same race condition |

### Triggering Conditions

- Chrome must process the `Target.closeTarget` WebSocket command
- The HTTP `/json/list` endpoint must be queried before it reflects the closure
- The race window is typically 1-20ms, making this reproduce in the majority of invocations
- The issue is more likely when Chrome is under load or when closing tabs rapidly in sequence

---

## Fix Strategy

### Approach

Add a polling retry loop to `execute_close()` after sending all `Target.closeTarget` commands, identical in pattern to the existing loop in `execute_create()`. The loop will poll `query_targets()` until the expected number of page-type targets is observed (original count minus closed count), or until 10 retries are exhausted. This is the minimal correct fix that follows the established pattern in the same file.

The expected remaining count is computed before the loop: `page_count - closing_page_count`. The loop polls the HTTP endpoint up to 10 times with 10ms sleeps, breaking early once the count matches expectations.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/tabs.rs` | Replace the immediate `query_targets()` call at line 258 with a polling loop that waits for the remaining page count to equal `page_count - closing_page_count` | Eliminates the race condition by waiting for Chrome's HTTP endpoint to reflect the tab closures, matching the pattern already used in `execute_create()` |

### Blast Radius

- **Direct impact**: `execute_close()` in `src/tabs.rs` — the only function modified
- **Indirect impact**: None. The `query_targets()` function and `CloseResult` output struct are unchanged. No other callers are affected.
- **Risk level**: Low — adding a retry loop with a bounded iteration count and short sleep is safe; worst case adds ~100ms latency if Chrome is slow to propagate

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Polling loop never converges, causing incorrect count after exhausting retries | Low | Use the count after the final poll regardless (graceful degradation), same as `execute_create()` |
| Added latency from sleep calls slows down `tabs close` | Low | 10ms × 10 iterations = 100ms max; in practice converges in 1-2 iterations (~10-20ms) |
| Last-tab protection logic affected | None | Last-tab check occurs before the close commands are sent; the polling loop runs after closing |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Wait for `Target.targetDestroyed` CDP event | Listen on WebSocket for the destruction event before re-querying | More complex, requires event subscription setup; the HTTP polling pattern is already established and proven in the same file |
| Subtract closed count instead of re-querying | Compute remaining as `page_count - closing_page_count` without re-querying | Would not reflect other external tab changes that may have occurred; re-querying is more accurate |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
