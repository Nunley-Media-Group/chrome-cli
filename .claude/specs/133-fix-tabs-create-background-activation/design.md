# Root Cause Analysis: tabs create --background not preventing tab activation (regression)

**Issue**: #133
**Date**: 2026-02-17
**Status**: Implemented
**Author**: Claude

---

## Root Cause

This bug has survived three previous fix attempts (#82, #95, #121). Each fix increased the polling budget for verifying that `Target.activateTarget` had propagated to Chrome's `/json/list` HTTP endpoint — from 0ms (no verification) to 100ms (10 iterations) to 500ms (50 iterations). Yet the bug persists.

The root cause has **three layers** (the third was discovered during verification):

**Layer 1 — Cross-protocol synchronization gap**: `execute_create` sends `Target.activateTarget` over the CDP WebSocket, then polls Chrome's HTTP `/json/list` endpoint to verify. These are two separate Chrome subsystems (DevTools WebSocket handler vs. HTTP handler). The CDP command updates Chrome's internal `DevToolsManager` state, but the HTTP endpoint reads from the same state via a potentially stale or racy path. The polling loop can exit while the HTTP endpoint still reflects pre-activation ordering.

**Layer 2 — Page-load re-activation**: When `Target.createTarget` creates a tab with a URL, Chrome begins loading the page. Navigation events during page load (e.g., `DidFinishNavigation`) can trigger Chrome to re-activate the new tab *after* our `Target.activateTarget` call has already brought the original tab back. The polling loop might see the correct ordering (original tab at position 0) and exit, but Chrome then reverts the activation as the new tab's page load progresses.

**Layer 3 — `/json/list` ordering does not reflect activation state in headless mode** *(discovered during verification)*: Chrome's HTTP `/json/list` endpoint does **not** reorder targets based on activation in headless mode. Neither `Target.activateTarget` (CDP), `/json/activate/{id}` (HTTP), nor `Page.bringToFront()` cause the ordering to change. The `i == 0` positional heuristic in `execute_list` is fundamentally broken — it measures creation/navigation order, not activation state. However, `document.visibilityState` (queried via CDP `Runtime.evaluate` on page-level sessions) **does** correctly reflect which tab Chrome considers active, even in headless mode.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/tabs.rs` | 145 | `active: i == 0` — positional heuristic in `execute_list`, **broken** in headless mode |
| `src/tabs.rs` | 192-211 | Background re-activation + polling loop in `execute_create` — polling `/json/list` ordering is useless |
| `src/tabs.rs` | 167-175 | `original_active_id` detection uses first page target from `/json/list`, not the actually active tab |

### Triggering Conditions

- The `--background` flag is passed to `tabs create`
- Chrome ignores `background: true` in `Target.createTarget` (consistent behavior)
- Activation methods (`Target.activateTarget`, `/json/activate`, `Page.bringToFront`) **do** change Chrome's internal activation state and `document.visibilityState`
- But `/json/list` ordering does **not** change to reflect activation in headless mode
- The `i == 0` heuristic in `execute_list` reports the wrong tab as active

---

## Fix Strategy

### Approach

Replace the positional heuristic with authoritative CDP visibility queries, and simplify the background activation verification:

1. **In `execute_list`**: Replace `active: i == 0` with a CDP-based `document.visibilityState` query. Connect to the browser WebSocket, create sessions for each page target via `Target.attachToTarget`, evaluate `document.visibilityState`, and mark the `"visible"` tab as active. Fall back to `i == 0` only if all CDP queries fail.

2. **In `execute_create` (background path)**: Use HTTP `/json/activate/{id}` for activation (same as before), but replace the `/json/list` polling loop with CDP visibility verification. After a 100ms settle period, check the original tab's `document.visibilityState` via a CDP session. If not visible (page-load re-activation), retry the HTTP activation once.

3. **In `execute_create` (original_active_id detection)**: Use CDP `document.visibilityState` to find the truly visible tab before creating the new one, rather than assuming the first page target in `/json/list` is active.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/chrome/discovery.rs` | Add `pub async fn activate_target(host, port, id)` function that calls HTTP `GET /json/activate/{id}` | Provides HTTP-level tab activation |
| `src/chrome/mod.rs` | Re-export `activate_target` | Module API consistency |
| `src/tabs.rs` | Add `check_target_visible(client, target_id) -> bool` helper | Reusable CDP visibility check via `Runtime.evaluate("document.visibilityState")` on a session |
| `src/tabs.rs` | Add `query_visible_target_id(ws_url, targets, config) -> Option<String>` | Scans page targets to find the visible one, used by `execute_list` |
| `src/tabs.rs` | In `execute_list`: replace `active: i == 0` with `visible_id` lookup from `query_visible_target_id()` | FR1: authoritative active state from Chrome |
| `src/tabs.rs` | In `execute_create` background path: replace `/json/list` polling with CDP visibility check after HTTP activation + 100ms settle | FR2: reliable verification using the same mechanism as `execute_list` |
| `src/tabs.rs` | In `execute_create` `original_active_id` detection: use CDP visibility instead of first page target | Correct baseline before creating the new tab |

### Blast Radius

- **Direct impact**: `execute_list` (active state detection), `execute_create` (background path + original_active_id detection), new helpers in `tabs.rs`, `activate_target` in `discovery.rs`
- **Indirect impact**: `execute_list` now requires a CDP WebSocket connection (previously only used HTTP `/json/list`). This adds ~10-50ms latency for the visibility query.
- **Non-background path**: The `execute_create` non-background path is unchanged. `original_active_id` is only computed when `background` is true.
- **Risk level**: Medium — `execute_list` behavior changes for all callers (not just `--background`), but the fallback to `i == 0` when CDP fails ensures graceful degradation.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Non-background `tabs create` changes behavior | None | `original_active_id` is only `Some` when `--background` is true |
| `execute_list` latency increase | Low | CDP session queries add ~10-50ms. Acceptable for a CLI tool. Optimistic path checks first target first. |
| CDP session creation fails for chrome:// URLs | Low | `check_target_visible` returns `false` on any error, so chrome:// tabs are simply not marked active (correct behavior — user tabs take precedence) |
| `execute_list` fallback to `i == 0` masks issues | Very Low | Fallback only triggers if CDP WebSocket connection fails entirely. In normal operation, visibility check succeeds. |
| HTTP `/json/activate` endpoint not available | Very Low | Chrome 32+. Error propagates as `ChromeError::HttpError`. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| **A: HTTP activation + `/json/list` polling** | Replace CDP activation with HTTP activation, poll `/json/list` ordering | **Originally selected, failed during verification** — `/json/list` ordering does not change with activation in headless mode |
| **B: `Page.bringToFront()` via page WebSocket** | Connect to original tab's page-level WebSocket, send `Page.bringToFront()` | Tested during verification — does not change `/json/list` ordering either. Does change `visibilityState` but requires per-tab WebSocket. |
| **C: Create at `about:blank`, then navigate** | Create tab at `about:blank`, re-activate original, then navigate new tab | Rejected — requires second WebSocket connection, changes `CreateResult` timing |
| **D: Increase polling budget** | More iterations polling `/json/list` | Rejected — `/json/list` ordering never changes in headless mode. No amount of polling helps. |
| **E: CDP `document.visibilityState` via session multiplexing** | In `execute_list`, attach to each page target via browser WebSocket sessions, evaluate `document.visibilityState` | **Selected** — authoritative, works in headless mode, uses existing `CdpClient::create_session()` infrastructure. Single WebSocket connection. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
- [x] Verified against real headless Chrome during smoke test
