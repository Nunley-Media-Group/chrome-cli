# Root Cause Analysis: dialog info returns wrong type and empty message for open dialogs

**Issue**: #134
**Date**: 2026-02-17
**Status**: Revised
**Author**: Claude

---

## Root Cause

The original analysis incorrectly assumed Chrome re-emits `Page.javascriptDialogOpening` when `Page.enable` is sent on a new session. **Chrome does not do this.** CDP events are ephemeral — they fire once at the moment the dialog opens and are never replayed.

The actual root cause is an **architectural mismatch**: chrome-cli creates a fresh CDP connection per command invocation, but dialog metadata is only available via the `Page.javascriptDialogOpening` event, which fires exactly once. When `dialog info` runs, it connects to Chrome fresh — by that time, the dialog event has long since fired (during a previous command's lifetime or while no CLI process was connected) and is permanently lost.

The sequence:
1. User runs `js exec "setTimeout(()=>alert('test'),100)" --no-await` — CLI connects, sends JS, disconnects
2. 100ms later, the `alert()` fires — Chrome emits `Page.javascriptDialogOpening` but **no CDP client is connected to receive it**
3. User runs `dialog info` — CLI connects fresh, subscribes to events, sends `Page.enable` (which blocks because dialog is open)
4. Chrome does NOT re-emit the dialog event for the new session
5. `drain_dialog_event()` finds nothing in the channel → returns `("unknown", "", "")`

The `try_recv()` vs `recv()` distinction is irrelevant because the event **never arrives** — not even with a 500ms timeout.

Additionally, there is no CDP query method to retrieve the current dialog state. CDP's dialog handling is purely event-driven: you must be subscribed and connected when the dialog opens to receive its metadata.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/dialog.rs` | 132-161 | `setup_dialog_session()` — subscribes to events and sends `Page.enable`, but Chrome never re-emits the dialog event |
| `src/dialog.rs` | 242-289 | `execute_info()` — correctly detects dialog is open (Runtime.evaluate probe), but cannot get metadata |
| `src/dialog.rs` | 183-233 | `execute_handle()` — can handle the dialog but lacks metadata for the response |
| `src/dialog.rs` | 299-318 | `drain_dialog_event()` — waits for an event that never arrives |

### Triggering Conditions

- A dialog must be open before the CLI invocation (the common case)
- No persistent CDP connection exists between CLI invocations
- Chrome does not provide a retroactive dialog state query API

---

## Fix Strategy

### Approach: Cookie-Based Dialog Interceptors

Since CDP events are ephemeral and cannot be queried retroactively, we use a **proactive instrumentation strategy**: override the native `window.alert()`, `window.confirm()`, and `window.prompt()` functions to store dialog metadata in a cookie before calling the original function.

Key insight: **`Network.getCookies` works while a dialog is blocking the renderer** — it bypasses the renderer entirely. This means a fresh CDP session can read cookies even when a dialog is open.

The approach has two parts:

**Part 1 — Proactive: Install interceptors during common commands**

When commands that interact with pages run (navigate, js exec, etc.), inject a script via `Runtime.evaluate` that:
1. Overrides `window.alert`, `window.confirm`, `window.prompt`
2. Before calling the original function, stores `{type, message, defaultValue}` in a cookie named `__chrome_cli_dialog`
3. The cookie has a short max-age (e.g., 300s) to auto-expire

Also register the script via `Page.addScriptToEvaluateOnNewDocument` so it persists across navigations within the session.

**Part 2 — Reactive: Read cookie in dialog commands**

When `dialog info` or `dialog handle` runs:
1. Detect dialog is open via `Runtime.evaluate` probe timeout (existing approach, works)
2. Read `Network.getCookies` to extract the `__chrome_cli_dialog` cookie
3. Parse the cookie JSON for type, message, and defaultValue
4. Use this metadata for the response

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/dialog.rs` | Add `read_dialog_cookie()` async fn that reads `Network.getCookies` and parses `__chrome_cli_dialog` | Provides dialog metadata from cookie side-channel |
| `src/dialog.rs` | Add `probe_dialog_open()` helper to detect dialog via `Runtime.evaluate` timeout | Shared helper for info and handle commands |
| `src/dialog.rs` | Add `dismiss_via_navigation()` fallback for `dialog handle` | When `Page.handleJavaScriptDialog` fails (Page domain wasn't enabled before dialog), reload page via `Page.navigate` to dismiss dialog |
| `src/dialog.rs` | Simplify `setup_dialog_session()` — no event subscription, just `Page.enable` with timeout | No longer returns event receiver since cookie approach replaces event-based metadata |
| `src/dialog.rs` | Remove `drain_dialog_event()` entirely | Event-based metadata is unreliable; cookie approach is the primary mechanism |
| `src/dialog.rs` | Update `execute_info()` to use `probe_dialog_open()` + `read_dialog_cookie()` | Probe detects dialog; cookie provides metadata |
| `src/dialog.rs` | Update `execute_handle()` with two-tier fallback: CDP → navigation | First tries `Page.handleJavaScriptDialog`; if CDP fails (pre-existing dialog), falls back to `Page.navigate` reload |
| `src/connection.rs` | Add `install_dialog_interceptors()` method to `ManagedSession` | Reusable interceptor with `try/catch` around cookie set (handles `data:` URLs gracefully) |
| Common command modules | Call `install_dialog_interceptors()` after session setup | Ensures interceptors are in place before dialogs fire |

**Navigation fallback details:**

`Page.handleJavaScriptDialog` requires the Page domain to have been enabled before the dialog opened. For pre-existing dialogs (opened before the current CLI invocation), Page.enable blocks and never completes, so CDP says "No dialog is showing." The navigation fallback:
1. Calls `Page.getNavigationHistory` (works while dialog blocks — browser-level, not renderer)
2. Calls `Page.navigate` to the current URL — this dismisses the dialog as a side effect and reloads the page
3. Verifies dismissal via `probe_dialog_open()`

Limitations of the navigation fallback:
- The page is reloaded (state is lost), but the dialog was blocking all interaction anyway
- Accept/dismiss semantics are not preserved — both result in a page reload
- Prompt response text cannot be delivered to the page

### Blast Radius

- **Direct impact**: `src/dialog.rs` (dialog commands), `src/connection.rs` (new method on `ManagedSession`)
- **Indirect impact**: Command modules that call `install_dialog_interceptors()` — minimal, best-effort call that cannot fail the command
- **Risk level**: Medium — adds a cookie side-channel, JS interception, and navigation fallback. Interceptors are installed best-effort and failure falls back to existing behavior. Navigation fallback only triggers when the standard CDP approach fails.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Interceptor script breaks page JavaScript | Very Low — the script wraps native functions and calls the originals | Interceptor is a simple closure that only adds a cookie before delegating |
| Cookie conflicts with page cookies | Very Low — uses `__chrome_cli_` prefix unlikely to collide | Prefix is distinctive and cookie auto-expires in 300s |
| `Network.getCookies` blocked by dialog | Proven not to happen — tested and confirmed it works | Network domain bypasses the renderer |
| Commands become slower due to interceptor install | Negligible — `Runtime.evaluate` for a small script is <5ms | Fire-and-forget with short timeout, non-blocking |
| Interceptors not installed (Chrome launched externally) | Medium — falls back to `"unknown"` type | Acceptable degradation; users can run any command first to install |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| `try_recv()` → `recv()` with timeout | Original fix — wait longer for the event | Event never arrives; the wait is pointless |
| Persistent background daemon | Keep a CDP connection alive between CLI invocations | Major architectural change, out of scope |
| `Page.addScriptToEvaluateOnNewDocument` only | Register interceptor for future navigations | Only fires on next navigation, not current page |
| localStorage instead of cookies | Store dialog info in localStorage | `Network.getCookies` is confirmed to work while blocked; localStorage requires renderer |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — focused on dialog metadata retrieval
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
- [x] Approach validated experimentally (cookie read works while dialog blocks)
