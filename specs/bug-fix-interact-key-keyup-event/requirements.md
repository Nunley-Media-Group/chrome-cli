# Defect Report: `interact key` keyup listeners not observed on target page

**Issues**: #227
**Date**: 2026-04-22
**Status**: Draft
**Author**: Rich Nunley
**Severity**: High
**Related Spec**: `specs/feature-keyboard-input/`

---

## Reproduction

### Steps to Reproduce

1. `agentchrome connect --launch --headless --port <P>`
2. `agentchrome --port <P> navigate https://the-internet.herokuapp.com/key_presses`
3. `agentchrome --port <P> page snapshot` — confirms `s2` is the text input
4. `agentchrome --port <P> interact click s2` — focuses the input (separate process)
5. `agentchrome --port <P> interact key A` — intended to fire keydown + keyup for `A`
6. `agentchrome --port <P> js exec "document.getElementById('result').innerText"` — returns `"You entered:"` with no character suffix

Control: step 4 followed by `agentchrome --port <P> interact type "hello"` correctly writes `hello` to the input (`document.getElementById('target').value === "hello"`), proving keyboard synthesis can reach the renderer.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | Windows 11 |
| **agentchrome version** | 1.33.1 |
| **Browser / Runtime** | Chrome launched via `connect --launch --headless` |
| **Shell** | bash |

### Frequency

Always — reproduces on every invocation of `interact key <KEY>` when the target page relies on `keyup` / `keydown` listeners that read `event.key`, `event.keyCode`, or `event.which`.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `interact key A` on a focused input fires both `keydown` and `keyup` with `event.key === "A"` and a populated `event.keyCode` / `event.which` so jQuery-style `$(document).keyup(...)` handlers observe the event and update `#result` to `"You entered: A"`. `interact key Enter` produces `event.key === "Enter"` and yields `"You entered: ENTER"`. |
| **Actual** | `#result` reads `"You entered:"` with no character suffix for both `A` and `Enter`. The CDP `Input.dispatchKeyEvent` calls are emitted, but page listeners never observe usable key events — `event.which` / `event.keyCode` are `0`, and for `Enter` the `key` field is mis-mapped to `"\r"`. |

### Error Output

```
$ agentchrome --port <P> js exec "document.getElementById('result').innerText"
{"result": "You entered:"}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Bug Is Fixed — letter key

**Given** `/key_presses` is loaded and `s2` (text input) is focused
**When** `agentchrome interact key A` runs (including across separate invocations from the preceding `interact click s2`)
**Then** `document.getElementById('result').innerText === "You entered: A"`
**And** a `keyup` listener attached to `document` observes `event.key === "A"` and non-zero `event.keyCode` / `event.which`

### AC2: Bug Is Fixed — Enter key

**Given** the same setup as AC1
**When** `agentchrome interact key Enter` runs
**Then** `document.getElementById('result').innerText === "You entered: ENTER"`
**And** a `keyup` listener observes `event.key === "Enter"` (not `"\r"`)

### AC3: No regression — modifier combinations

**Given** a page with a `keydown` listener that logs `Ctrl+C` or `Shift+A`
**When** `agentchrome interact key "Ctrl+C"` or `agentchrome interact key "Shift+A"` runs
**Then** existing keydown + keyup + modifier behavior is preserved — the listener observes the correct `event.key`, `event.ctrlKey` / `event.shiftKey`, and `event.code`

### AC4: No regression — `interact type`

**Given** the same focused input
**When** `agentchrome interact type "hello"` runs
**Then** the input `value` is still `hello` (the `char`-synthesis path in `dispatch_char` is unchanged)

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `interact key` must populate CDP `Input.dispatchKeyEvent` with the fields required for standard DOM listeners to observe the event — at minimum: `key` (correct DOM value), `code` (correct physical key), `windowsVirtualKeyCode` (so `event.keyCode` / `event.which` are non-zero), and, for keys that produce text (letters, digits, symbols, Enter), `text` on `keyDown`. | Must |
| FR2 | For `Enter`, CDP `key` must be `"Enter"` (not `"\r"`); `text` on `keyDown` must be `"\r"` so that input elements that convert Enter to submit still behave correctly. | Must |
| FR3 | For single-letter keys, `cdp_key_value` must return the literal letter (already correct) and `windowsVirtualKeyCode` must match the ASCII uppercase code point (e.g., `A` → 65). | Must |
| FR4 | The fix must work across separate `agentchrome` invocations (click in one process, key in another). If CDP focus does not persist across sessions, the key-event path must still work against whichever element is actually focused in the live tab. | Must |

---

## Out of Scope

- Adding a composite `interact click --then-key <K>` convenience form (track separately if needed)
- Synthesizing IME / composition events
- Refactoring the `dispatch_char` / `interact type` path (control case — already works)
- Adding `nativeVirtualKeyCode` support for non-Windows native key codes (only `windowsVirtualKeyCode` is required by CDP for consistent cross-platform behavior)

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC3, AC4)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #227 | 2026-04-22 | Initial defect spec |
