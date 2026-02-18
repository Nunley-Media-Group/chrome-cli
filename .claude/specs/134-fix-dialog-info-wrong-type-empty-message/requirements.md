# Defect Report: dialog info returns wrong type and empty message for open dialogs

**Issue**: #134
**Date**: 2026-02-17
**Status**: Revised
**Author**: Claude
**Severity**: High
**Related Spec**: `.claude/specs/20-browser-dialog-handling/`

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli navigate https://www.google.com`
3. `chrome-cli js exec "setTimeout(()=>alert('test'),100)" --no-await`
4. `sleep 2`
5. `chrome-cli dialog info --pretty`

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (Darwin 25.3.0) |
| **Version / Commit** | 1.0.0 (commit e50f7b3) |
| **Browser / Runtime** | HeadlessChrome/144.0.0.0 via CDP |
| **Configuration** | Default |

### Frequency

Always — 100% reproducible when a JavaScript dialog is open.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `dialog info` returns `{"open": true, "type": "alert", "message": "test"}` |
| **Actual** | `dialog info` returns `{"open": true, "type": "unknown", "message": ""}` |

### Error Output

```json
{"open":true,"type":"unknown","message":""}
```

The dialog is correctly detected as open (the `Runtime.evaluate` probe times out), but the type and message are missing because the `Page.javascriptDialogOpening` event hasn't arrived in the channel by the time `drain_dialog_event()` calls `try_recv()`.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Dialog info returns correct type and message for alert

**Given** a JavaScript `alert('hello')` dialog is open
**When** I run `chrome-cli dialog info`
**Then** the output includes `"open": true`, `"type": "alert"`, and `"message": "hello"`

### AC2: Dialog info reports confirm dialogs correctly

**Given** a JavaScript `confirm('proceed?')` dialog is open
**When** I run `chrome-cli dialog info`
**Then** the output includes `"open": true`, `"type": "confirm"`, and `"message": "proceed?"`

### AC3: Dialog info still works when no dialog is open

**Given** no dialog is open on the page
**When** I run `chrome-cli dialog info`
**Then** the output includes `"open": false`
**And** the exit code is 0

### AC4: Dialog handle returns correct type and message

**Given** a JavaScript `alert('test')` dialog is open
**When** I run `chrome-cli dialog handle accept`
**Then** the output includes `"dialog_type": "alert"` and `"message": "test"` (not `"unknown"` and `""`)

### AC5: Dialog handle dismisses pre-existing dialogs

**Given** a JavaScript `alert('test')` dialog was opened before the current CLI invocation
**When** I run `chrome-cli dialog handle accept`
**Then** the dialog is dismissed (the page may reload as a side effect)
**And** the exit code is 0

### AC6: Dialog handle returns error when no dialog is open

**Given** no dialog is open on the page
**When** I run `chrome-cli dialog handle accept`
**Then** the output includes an error message "No dialog is currently open"
**And** the exit code is non-zero

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Use a cookie-based interceptor mechanism to store dialog metadata (type, message, defaultValue) before calling the native dialog function, since CDP events are ephemeral and cannot be replayed | Must |
| FR2 | Both `execute_info()` and `execute_handle()` read dialog metadata from the `__chrome_cli_dialog` cookie via `Network.getCookies`, which works while a dialog is blocking the renderer | Must |
| FR3 | `dialog handle` uses `Page.handleJavaScriptDialog` as the primary mechanism, with a `Page.navigate` navigation-based fallback for pre-existing dialogs where the Page domain wasn't enabled | Must |
| FR4 | The no-dialog-open path must not be slowed down — the Runtime.evaluate probe timeout (200ms) only fires when no dialog is blocking | Should |
| FR5 | The cookie interceptor's `document.cookie` assignment is wrapped in `try/catch` to handle `data:` URLs and other restricted contexts gracefully | Must |

---

## Out of Scope

- Changes to `--auto-dismiss-dialogs` flag behavior
- Persistent background daemon for CDP connections
- Adding new dialog command features

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC3)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
