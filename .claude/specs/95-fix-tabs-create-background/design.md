# Root Cause Analysis: tabs create --background does not preserve active tab

**Issue**: #95
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude

---

## Root Cause

Issue #82 introduced a workaround for Chrome ignoring the `background: true` parameter in `Target.createTarget`: after creating the tab, the code re-activates the original tab via `Target.activateTarget`. This fix is present in `src/tabs.rs` (lines 166–197) and follows the correct pattern. However, the bug persists.

The root cause is a **timing/ordering issue** between the `Target.activateTarget` CDP command and Chrome's `/json/list` HTTP endpoint. When `Target.activateTarget` is sent, Chrome acknowledges the command (the CDP response returns successfully), but the internal target ordering exposed by `/json/list` may not immediately reflect the activation. This is particularly pronounced in headless mode (`--headless`), where tab "activation" has reduced semantics since there is no visible window.

The current code sends `Target.activateTarget` and immediately returns. When the user then runs `tabs list` in a subsequent CLI invocation, Chrome's `/json/list` may still report the newly created tab as the first (active) target because the ordering hasn't stabilized. The `execute_list` function (line 131–154) determines the active tab by treating the first `page` target in the `/json/list` response as active (`active: i == 0` on line 144), so stale ordering directly causes the wrong tab to appear active.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/tabs.rs` | 166–174 | Records original active tab ID before creation |
| `src/tabs.rs` | 192–197 | Re-activates original tab — command succeeds but effect may not propagate to `/json/list` ordering immediately |
| `src/tabs.rs` | 199–209 | Re-queries targets for output — does not verify activation took effect |

### Triggering Conditions

- The `--background` flag is passed to `tabs create`
- Chrome ignores `background: true` in `Target.createTarget` (always in current Chrome)
- `Target.activateTarget` completes successfully at CDP level but Chrome's `/json/list` endpoint does not immediately reflect the new ordering
- Subsequent `tabs list` queries `/json/list` before the ordering stabilizes

---

## Fix Strategy

### Approach

After sending `Target.activateTarget`, add a **verification loop** that re-queries `/json/list` and confirms the original tab has returned to the first position in the target list. This ensures the activation has fully propagated before the command exits and the user observes the state.

The verification loop polls `query_targets` with a short delay (e.g., 10ms between attempts) and a maximum number of retries (e.g., 10 attempts = 100ms total). If the original tab is confirmed as the first page target, the loop exits early. If the maximum retries are exhausted, the command proceeds anyway — the activation was sent and may take effect before the next user command.

This is a minimal change: a small polling loop after the existing `Target.activateTarget` call, using the same `query_targets` function already used elsewhere in the function.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/tabs.rs` | After `Target.activateTarget` (line 197), add a verification loop that polls `query_targets` until the original tab is the first page target, with a timeout | Ensures activation has propagated to `/json/list` before the command exits |

### Blast Radius

- **Direct impact**: `execute_create` in `src/tabs.rs` — the only function modified
- **Indirect impact**: None. The verification loop uses `query_targets` which is already called elsewhere in the same function. No new dependencies or shared state changes.
- **Risk level**: Low — the change is additive, gated on `if background { ... }`, and bounded by a retry limit. Worst case adds ~100ms to `--background` creates.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `tabs create` without `--background` changes behavior | None | All new logic is gated on the existing `if let Some(ref active_id) = original_active_id` block |
| Verification loop adds noticeable latency | Low | Loop exits early on success; max 10 iterations × 10ms = 100ms ceiling. Acceptable for correctness. |
| Polling `query_targets` causes excess HTTP requests to Chrome | Very Low | At most 10 extra HTTP GETs to `/json/list` — negligible load on Chrome's built-in HTTP server |
| Re-activation verification times out and tab still appears active | Low | This is a best-effort improvement — even if verification times out, the activation command was sent and will likely take effect before the user's next command |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Add a fixed `sleep` after `Target.activateTarget` | Simple `tokio::time::sleep(50ms)` after re-activation | Rejected — arbitrary delay is fragile; too short may still race, too long adds unnecessary latency |
| Use CDP events (`Target.targetInfoChanged`) to confirm activation | Subscribe to target events and wait for confirmation | Rejected — over-engineered for this fix; requires event subscription setup and adds complexity |
| **Poll `/json/list` until ordering is correct (selected)** | Verify activation took effect before returning | **Selected** — deterministic, bounded, uses existing infrastructure |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
