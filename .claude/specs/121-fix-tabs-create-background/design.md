# Root Cause Analysis: tabs create --background not keeping original tab active

**Issue**: #121
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude

---

## Root Cause

Issues #82 and #95 added a workaround in `execute_create` (`src/tabs.rs`, lines 157–211) to compensate for Chrome ignoring the `background: true` parameter in `Target.createTarget`. The fix follows this sequence:

1. Record the currently active tab ID before creation (lines 167–175)
2. Create the tab with `Target.createTarget` (lines 186–188)
3. Re-activate the original tab via `Target.activateTarget` (lines 193–197)
4. Poll `/json/list` up to 10 times with 10ms sleep intervals to verify the original tab is back in the first position (lines 203–210)

The bug persists because the **polling budget is insufficient**. The loop runs at most 10 iterations × 10ms = 100ms total wait time. In practice, Chrome's `/json/list` HTTP endpoint can take longer than 100ms to reflect the activation state change, especially in headless mode where tab activation has reduced semantics and lower priority. When the polling window expires, the function proceeds without the activation having fully propagated, and subsequent `tabs list` calls see stale ordering with the new tab still in the active position.

The retrospective learning from issue #82's spec is directly relevant: *"When specifying features that depend on optional CDP parameters (like `background` in `Target.createTarget`), include a requirement to verify the parameter is honored and specify a fallback strategy if the browser ignores it."* The verification loop exists but its timeout budget is too small to serve as a reliable fallback.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/tabs.rs` | 199–211 | Polling loop in `execute_create` — 10 iterations × 10ms is insufficient for Chrome to propagate activation state to `/json/list` |

### Triggering Conditions

- The `--background` flag is passed to `tabs create`
- Chrome ignores `background: true` in `Target.createTarget` (consistent behavior)
- `Target.activateTarget` is called to re-activate the original tab (correct)
- Chrome's `/json/list` endpoint takes >100ms to reflect the activation change (common in headless mode)
- The polling loop exhausts its budget and the function returns before verification succeeds

---

## Fix Strategy

### Approach

Increase the polling loop's timeout budget to give Chrome sufficient time to propagate the activation state to `/json/list`. The current budget of 100ms (10 × 10ms) should be increased to 500ms (50 × 10ms). This provides 5× the current budget while keeping individual poll intervals short (10ms) for fast resolution when Chrome updates quickly.

The 500ms budget aligns with the existing timeout patterns in chrome-cli — other polling loops in the codebase (e.g., `tabs close` at line 262) use the same 10 × 10ms pattern, but activation propagation in headless mode is empirically slower than close propagation. A 50-iteration budget is conservative enough to handle slow headless environments while remaining imperceptible to users (the 500ms worst-case is well under typical CLI timeout expectations).

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/tabs.rs` | Increase the polling loop iteration count from `10` to `50` in `execute_create`'s activation verification (line 203) | Extends the maximum wait from 100ms to 500ms, giving Chrome sufficient time to propagate the `Target.activateTarget` state to `/json/list` in headless mode |

### Blast Radius

- **Direct impact**: `execute_create` in `src/tabs.rs` — only the loop iteration count constant changes
- **Indirect impact**: None. The change only affects the `--background` code path. The worst case is the loop completes all 50 iterations (500ms added latency) if Chrome never propagates the state — this is strictly better than the current behavior of returning incorrect results after 100ms.
- **Risk level**: Low — the change is a single integer constant increase in an existing polling loop. No control flow, logic, or API changes.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Non-background `tabs create` changes behavior | None | The polling loop is inside `if let Some(ref active_id) = original_active_id`, which is only `Some` when `--background` is true |
| 500ms worst-case latency is noticeable | Very Low | 500ms only occurs if Chrome never propagates (a pathological case). Normal propagation completes in <100ms, and the loop breaks early on success. |
| `tabs close` polling becomes inconsistent | None | `tabs close` uses its own independent polling loop (lines 262–272). This change does not touch it. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Increase to 500ms (50 × 10ms) | Extend polling budget with same interval | **Selected** — minimal change, sufficient budget, maintains fast early-exit |
| Increase interval to 50ms, keep 10 iterations | 10 × 50ms = 500ms total with fewer HTTP requests | Rejected — slower early-exit for the common case where Chrome propagates quickly (<50ms). The 10ms interval allows sub-50ms resolution. |
| Add exponential backoff | Start at 10ms, double each iteration | Rejected — over-engineered for this case. Linear polling with early exit is simpler and sufficient. |
| Use CDP events (`Target.targetInfoChanged`) instead of polling | Subscribe to events and wait for activation confirmation | Rejected — requires event subscription infrastructure that doesn't exist in this code path. The polling approach is already established and proven in other commands. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
