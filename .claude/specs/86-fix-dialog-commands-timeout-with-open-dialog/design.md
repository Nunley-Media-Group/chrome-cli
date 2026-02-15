# Root Cause Analysis: Dialog commands timeout when a dialog is actually open

**Issue**: #86
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude

---

## Root Cause

When a JavaScript dialog (alert, confirm, prompt) is open in Chrome, the browser blocks certain CDP domain-enabling commands — most notably `Page.enable` — until the dialog is dismissed. This creates a chicken-and-egg problem: the dialog commands need a CDP session with the Page domain enabled to handle dialogs, but enabling the Page domain requires the dialog to already be dismissed.

The `execute_handle()` function (`src/dialog.rs:144`) calls `managed.ensure_domain("Page")` at line 148 before sending `Page.handleJavaScriptDialog`. The `ensure_domain` method (`src/connection.rs:184`) sends `Page.enable` via the CDP session, which hangs indefinitely when a dialog is open, eventually timing out after the configured `command_timeout` (default 30 seconds).

The same issue affects `execute_info()` (`src/dialog.rs:209`), which calls `ensure_domain("Page")` at line 213 and `ensure_domain("Runtime")` at line 214. Additionally, `apply_emulate_state()` called during `setup_session()` at line 119 may itself send `Runtime.evaluate` commands that block when a dialog is open, causing an earlier timeout before the dialog-specific code even runs.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/dialog.rs` | 111-122 | `setup_session()` — creates CDP session and calls `apply_emulate_state()` |
| `src/dialog.rs` | 144-200 | `execute_handle()` — calls `ensure_domain("Page")` at line 148 before handling dialog |
| `src/dialog.rs` | 209-260 | `execute_info()` — calls `ensure_domain("Page")` and `ensure_domain("Runtime")` at lines 213-214 |
| `src/connection.rs` | 184-192 | `ensure_domain()` — sends `{domain}.enable` CDP command that blocks with open dialog |
| `src/emulate.rs` | 227-287 | `apply_emulate_state()` — may call `Runtime.evaluate` which also blocks with open dialog |

### Triggering Conditions

- A JavaScript dialog (alert, confirm, or prompt) is currently open in the target tab
- The user runs `dialog info` or `dialog handle accept/dismiss`
- `Page.enable` (or `Runtime.enable`/`Runtime.evaluate`) is sent via CDP before handling the dialog
- Chrome blocks the domain enable command until the dialog is dismissed, which never happens because the command that would dismiss it can't execute

---

## Fix Strategy

### Approach

The dialog commands should skip all domain enablement and emulation state that can block when a dialog is open. The fix involves two changes:

1. **Skip `apply_emulate_state()` for dialog commands**: Create a dialog-specific session setup function (or parameterize `setup_session`) that does not call `apply_emulate_state()`. Emulation state (user agent, viewport, device scale) is irrelevant for dialog handling — the dialog is already open and these settings don't affect dialog interaction.

2. **Skip `ensure_domain("Page")` and `ensure_domain("Runtime")` calls**: The `Page.handleJavaScriptDialog` CDP command does not actually require `Page.enable` to have been called first — it works directly on the session. Similarly, for `dialog info`, instead of using a `Runtime.evaluate` probe (which requires `Runtime.enable` and itself blocks), detect the dialog state by attempting `Page.handleJavaScriptDialog` with a cancel/no-op, or by subscribing to events at the target level without enabling the full domain.

The key insight is that `Page.handleJavaScriptDialog` works without `Page.enable` — it's a direct CDP method that operates on the session regardless of domain state. Event subscriptions for `Page.javascriptDialogOpening` also arrive without `Page.enable` because Chrome sends dialog events as soon as a session is attached.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/dialog.rs` | Add `setup_dialog_session()` that skips `apply_emulate_state()` | Emulation state application can block (calls `Runtime.evaluate`) and is irrelevant for dialog handling |
| `src/dialog.rs` | In `execute_handle()`, remove `ensure_domain("Page")` call | `Page.handleJavaScriptDialog` works without `Page.enable` |
| `src/dialog.rs` | In `execute_info()`, remove `ensure_domain("Page")` and `ensure_domain("Runtime")` calls; replace the `Runtime.evaluate` probe with a non-blocking detection approach | Both domain enables and the evaluate probe block when a dialog is open |
| `src/dialog.rs` | Update `execute_info()` probe logic to use a method that doesn't block with open dialogs | Need a dialog detection mechanism that works regardless of dialog state |

### Blast Radius

- **Direct impact**: `src/dialog.rs` — only `execute_handle()` and `execute_info()` are modified
- **Indirect impact**: None — dialog commands have their own `setup_session()` function and don't share execution paths with other commands. The `ManagedSession` and `ensure_domain` code in `connection.rs` is unchanged.
- **Risk level**: Low — changes are isolated to the dialog module. No other commands are affected because each command group has its own session setup.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Dialog handle fails when no dialog is open (existing error path) | Low | AC6 regression test verifies the "no dialog open" error is still returned |
| Dialog info returns incorrect state when no dialog is open | Low | AC5 regression test verifies `"open": false` is still returned correctly |
| Event subscription for `Page.javascriptDialogOpening` stops working without `Page.enable` | Medium | Verify during implementation that CDP sends dialog events without `Page.enable`; the CDP spec indicates events fire on session attachment, not domain enablement |
| `apply_emulate_state()` skip causes unexpected behavior in dialog commands | Low | Emulation overrides (user agent, viewport, network) are irrelevant to dialog interaction — dialogs are browser-level UI, not page content |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| **A: Browser-level CDP target** | Connect to the browser target instead of the page target to handle dialogs | Requires significant refactoring of the connection model; `Target.attachToTarget` already provides session-level access which is sufficient |
| **B: Try/catch with fallback** | Attempt `Page.enable` with a short timeout, fall back to direct command if it fails | Adds unnecessary latency (timeout wait); the root cause is well-understood — `Page.enable` simply isn't needed |
| **C: Skip only `Page.enable`** | Keep `apply_emulate_state()` but skip `Page.enable` | Insufficient — `apply_emulate_state()` calls `Runtime.evaluate` which also blocks with open dialogs |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
