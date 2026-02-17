# Root Cause Analysis: tabs activate not reflected in subsequent tabs list

**Issue**: #122
**Date**: 2026-02-16
**Status**: Draft
**Author**: Claude

---

## Root Cause

The `execute_activate()` function in `src/tabs.rs` (lines 282–311) sends the `Target.activateTarget` CDP command via WebSocket and immediately returns the result without verifying that Chrome's HTTP `/json/list` endpoint has reflected the activation state change. This is the same class of race condition that affected `tabs close` (#120) and `tabs create --background` (#121).

Chrome's DevTools HTTP server maintains its own view of target ordering. The first page-type target in `/json/list` is conventionally the "active" tab. When `Target.activateTarget` is sent via the CDP WebSocket, the WebSocket layer acknowledges the command, but the HTTP endpoint updates asynchronously — typically within a few milliseconds, but sometimes longer (especially in headless mode). If a subsequent `tabs list` command queries `/json/list` before the update has propagated, it sees stale ordering with the previously active tab still in the first position.

Both `execute_create()` (lines 203–210, 50 iterations × 10ms) and `execute_close()` (lines 262–272, 10 iterations × 10ms) already have polling loops that wait for `/json/list` to reflect CDP state changes before returning. `execute_activate()` is the only tab mutation command missing this verification.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/tabs.rs` | 282–311 | `execute_activate()` — sends `Target.activateTarget` and returns immediately without polling |
| `src/tabs.rs` | 295–297 | The `send_command("Target.activateTarget", ...)` call after which polling should be added |

### Triggering Conditions

- Multiple tabs must be open so there is a meaningful activation order change
- The activated tab must be different from the currently active tab
- Chrome's `/json/list` endpoint must be queried (by a subsequent `tabs list`) before it reflects the activation
- The race window is typically 1–50ms, reproducing in the majority of invocations

---

## Fix Strategy

### Approach

Add a polling retry loop to `execute_activate()` after the `Target.activateTarget` command, before constructing the output. The loop will poll `query_targets()` until the activated tab is the first page-type target in `/json/list`, or until 50 retries are exhausted (500ms maximum). This matches the pattern established in `execute_create()` for activation verification.

The verification condition is: the first entry in `/json/list` with `target_type == "page"` has an `id` matching the target that was just activated. This is the same check used in `execute_create()`'s background verification loop.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/tabs.rs` | Add a polling loop after `Target.activateTarget` (line 297) that queries `query_targets()` up to 50 times with 10ms sleep, breaking when the activated target is the first page-type target | Eliminates the race condition by waiting for Chrome's HTTP endpoint to reflect the activation, matching the established pattern in `execute_create()` |

### Blast Radius

- **Direct impact**: `execute_activate()` in `src/tabs.rs` — the only function modified
- **Indirect impact**: None. The `query_targets()` function, `ActivateResult` struct, and output formatting are unchanged. No other callers are affected.
- **Risk level**: Low — adding a bounded polling loop with 10ms sleep intervals is safe; worst case adds ~500ms latency if Chrome is slow to propagate (matching `execute_create()`'s budget)

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Polling loop never converges, adding 500ms latency | Very Low | Graceful degradation — the function proceeds after exhausting retries, same as `execute_create()`. The output is still correct (tab info was captured before the loop). |
| Already-active tab activation slows down | None | The loop checks if the target is already first; if the activated tab was already active, the first poll succeeds immediately (0ms added latency). |
| `execute_create` or `execute_close` behavior changes | None | Those functions have their own independent polling loops. This change does not touch them. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
