# Defect Report: Dialog commands timeout when a dialog is actually open

**Issue**: #86
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude
**Severity**: Critical
**Related Spec**: `.claude/specs/dialog-handling/`

---

## Reproduction

### Steps to Reproduce

1. Launch Chrome: `chrome-cli connect --launch`
2. Navigate: `chrome-cli navigate "https://www.google.com"`
3. Trigger alert: `chrome-cli js exec "setTimeout(() => alert('test'), 100)"`
4. Wait 2 seconds for the dialog to appear
5. Query dialog: `chrome-cli dialog info --pretty`

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (Darwin 25.2.0) |
| **Version / Commit** | chrome-cli 0.1.0 (commit 01989d5) |
| **Browser / Runtime** | Chrome with CDP enabled |
| **Configuration** | Default (no emulate overrides) |

### Frequency

Always — 100% reproducible when a JavaScript dialog is open.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `dialog info` returns `{"open": true, "type": "alert", "message": "test"}` |
| **Actual** | `{"error":"CDP command timed out: Page.enable","code":4}` with exit code 4 (timeout) |

### Error Output

```json
{"error":"CDP command timed out: Page.enable","code":4}
```

The same timeout occurs with `dialog handle accept`. Both `dialog info` and `dialog handle` are non-functional when a dialog is actually open — the exact scenario they exist for.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: dialog info works with open alert dialog

**Given** a JavaScript alert dialog is open on the page
**When** I run `chrome-cli dialog info --pretty`
**Then** the output shows `"open": true`, the dialog type `"alert"`, and the dialog message

### AC2: dialog handle accept works with open alert dialog

**Given** a JavaScript alert dialog is open on the page
**When** I run `chrome-cli dialog handle accept --pretty`
**Then** the dialog is dismissed and the command returns `"action": "accept"` with exit code 0

### AC3: dialog handle dismiss works with open confirm dialog

**Given** a JavaScript confirm dialog is open on the page
**When** I run `chrome-cli dialog handle dismiss --pretty`
**Then** the dialog is dismissed with `"action": "dismiss"` and exit code 0

### AC4: dialog handle accept with text works for prompt dialog

**Given** a JavaScript prompt dialog is open on the page
**When** I run `chrome-cli dialog handle accept --text "answer" --pretty`
**Then** the dialog is accepted with the provided text and the command returns success

### AC5: dialog commands still work when no dialog is open

**Given** no JavaScript dialog is currently open on the page
**When** I run `chrome-cli dialog info`
**Then** the output shows `"open": false` (no regression from the fix)

### AC6: dialog handle returns error when no dialog is open

**Given** no JavaScript dialog is currently open on the page
**When** I run `chrome-cli dialog handle accept`
**Then** the command returns an error indicating no dialog is open (existing behavior preserved)

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Dialog commands must not call `Page.enable` before handling/querying dialogs — skip or defer domain enablement that blocks when a dialog is open | Must |
| FR2 | `dialog handle` must be able to send `Page.handleJavaScriptDialog` with an open dialog present | Must |
| FR3 | `dialog info` must be able to detect and report open dialogs without first enabling the Page domain | Must |
| FR4 | `apply_emulate_state()` must not block dialog commands when called during session setup | Must |
| FR5 | Existing behavior when no dialog is open must be preserved (no regression) | Should |

---

## Out of Scope

- Handling dialogs from different frames
- Handling beforeunload dialogs (separate behavior)
- Refactoring other commands' session setup
- Adding new dialog command features beyond fixing the timeout

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC5, AC6)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
