# Root Cause Analysis: `interact key` keyup listeners not observed on target page

**Issues**: #227
**Date**: 2026-04-22
**Status**: Draft
**Author**: Rich Nunley

---

## Root Cause

The bug has two compounding causes, both in `src/interact.rs`:

1. **Missing `windowsVirtualKeyCode` and `text` in the CDP payload.** `dispatch_key_press` (`src/interact.rs:946`), `dispatch_modifier_event` (`src/interact.rs:998`), and the combined `dispatch_key_combination` path all call `Input.dispatchKeyEvent` with only `type`, `key`, `code`, and `modifiers`. Chrome's CDP only populates `KeyboardEvent.keyCode` and `KeyboardEvent.which` from `windowsVirtualKeyCode`; when it is absent (or 0), both DOM fields are `0`. jQuery 1.x/2.x — which the reproduction site (`https://the-internet.herokuapp.com/key_presses`) uses — normalizes `event.which` and its handlers short-circuit when `which === 0`, so `$(document).keyup` never sees a usable event. Additionally, CDP treats the `text` field on `keyDown` as the signal that a `keypress` should be generated and that `input` elements should receive character input; without `text`, printable keys do not feel "real" to page-side listeners that look for paired `keypress` events.

2. **Incorrect `key` value mapping for `Enter`.** `cdp_key_value` (`src/interact.rs:839`) returns `"\r"` for `Enter`. The DOM specification requires `KeyboardEvent.key === "Enter"` for the Enter key; `"\r"` is a text artifact that belongs on the `text` field (keyDown), not on `key`. Listeners that inspect `event.key` will not match `"Enter"` and will ignore the event. `Tab` has the same bug (maps to `"\t"`), and `Space` maps to `" "` — DOM spec actually says `" "` is correct for `key` on Space, but `" "` for Tab and `"\r"` for Enter are wrong.

The issue's hypothesis (1) about cross-process focus loss is not load-bearing for this bug: focus survives across CDP sessions in the live Chrome tab (both `agentchrome interact click` and `agentchrome interact key` attach to the same tab by target ID via the persisted session). The `interact type` control works cross-process for the same reason — focus persists. The defect lives purely in the CDP key-event payload shape.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/interact.rs` | 839–885 (`cdp_key_value`) | Returns wrong DOM `key` value for `Enter` (`"\r"`) and `Tab` (`"\t"`). |
| `src/interact.rs` | 888–939 (`cdp_key_code`) | Returns correct `code` values (`KeyA`, `Enter`, etc.). No change required. |
| `src/interact.rs` | 946–979 (`dispatch_key_press`) | Emits CDP payload missing `windowsVirtualKeyCode` and (for printable keys) `text`. |
| `src/interact.rs` | 998–1022 (`dispatch_modifier_event`) | Same omission for modifier-only events. |
| `src/interact.rs` | ~1024–… (`dispatch_key_combination`) | Same omission on the modifier + key path. |

### Triggering Conditions

- The target page has a listener bound with `addEventListener('keyup'|'keydown', …)` or `$(document).keyup(…)` that reads `event.key`, `event.keyCode`, or `event.which`.
- `agentchrome interact key <KEY>` is invoked for a non-modifier single key (letter, digit, `Enter`, `Tab`, a symbol) or a modifier combination.
- Why this wasn't caught: existing unit tests in `src/interact.rs:2616+` validate `parse_key_combination`, `cdp_key_value`, and `cdp_key_code` in isolation — they never assert the shape of the CDP payload or that the dispatched event actually fires a DOM listener. The BDD layer for `feature-keyboard-input` exercised command-level success, not renderer-observed events.

---

## Fix Strategy

### Approach

Enrich the CDP key-event payload so it matches what real keyboard input produces. This is a minimal, surgical change to three functions in `src/interact.rs`:

1. Add a new helper `windows_virtual_key_code(key: &str) -> i64` that returns the Windows virtual-key code for each supported key (e.g., `A`–`Z` → 65–90, `0`–`9` → 48–57, `Enter` → 13, `Tab` → 9, `Escape` → 27, arrows → 37–40, F-keys → 112–135, etc.). Unknown keys return 0.
2. Add a new helper `key_text(key: &str, modifiers: u8) -> Option<String>` that returns the `text` value for printable keys on `keyDown` (`Some("a")` for `a`, `Some("A")` for `A` with Shift or bare uppercase, `Some("\r")` for `Enter`, `Some("\t")` for `Tab`, `Some(" ")` for `Space`, symbol characters for symbol names, etc.); returns `None` for non-printable keys (Escape, arrows, function keys, modifiers alone).
3. Fix `cdp_key_value` so `Enter` → `"Enter"` and `Tab` → `"Tab"`. `Space` stays `" "` (DOM-correct). The `text` layer (above) carries `"\r"` / `"\t"` / `" "` on keyDown.
4. Update `dispatch_key_press`, `dispatch_modifier_event`, and the combined `dispatch_key_combination` path to include `windowsVirtualKeyCode` on every event and `text` on `keyDown` when `key_text` returns `Some`.

Why minimal: no new commands, no CLI surface changes, no new dependencies, and no change to the char-synthesis (`dispatch_char`) path that `interact type` uses. The two new helpers are private functions colocated with the existing `cdp_key_value` / `cdp_key_code` helpers.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/interact.rs` | Add `windows_virtual_key_code(&str) -> i64` alongside `cdp_key_code`. | CDP needs this to populate `event.keyCode` / `event.which`. Without it, jQuery and any legacy listener that checks `.which` ignore the event. |
| `src/interact.rs` | Add `key_text(&str, u8) -> Option<String>` that yields the character a real keyboard would produce (respecting Shift for letters). | CDP uses `text` on `keyDown` to drive `keypress` and `input` events; omitting it is why letter keys feel "dead" to listeners. |
| `src/interact.rs` | Fix `cdp_key_value`: `Enter` → `"Enter"`, `Tab` → `"Tab"`. Leave `Space`, `Escape`, arrows, etc. as-is. | The DOM spec defines `KeyboardEvent.key` as `"Enter"` / `"Tab"`; the prior `"\r"` / `"\t"` values broke `event.key` comparisons. |
| `src/interact.rs` | Extend `dispatch_key_press`, `dispatch_modifier_event`, and `dispatch_key_combination` to add `windowsVirtualKeyCode` and (for `keyDown` of printable keys) `text` to the JSON payload. | Single-source enrichment so every CDP dispatch path carries the full event shape. |
| `src/interact.rs` (tests) | Add unit tests for `windows_virtual_key_code` and `key_text` covering the same key categories as existing `cdp_key_value` / `cdp_key_code` tests. | Matches the existing test convention for keyboard helpers. |

### Blast Radius

- **Direct impact**: `src/interact.rs` — three dispatch helpers and two new private helpers. No public API changes; `KeyArgs`, `execute_key`, and the `interact key` CLI surface are unchanged.
- **Indirect impact**:
  - `interact type` uses `dispatch_char` (type: `"char"`) which is a different CDP path and is untouched — AC4 guards this.
  - Any consumer of `dispatch_key_press` (currently only `execute_key`) now sends richer events. Richer events are strictly additive from the renderer's perspective — a page that worked without `windowsVirtualKeyCode` will continue to work with it set.
- **Risk level**: Low. The change adds fields to an existing CDP call; it does not alter ordering, retry behavior, or session lifecycle.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Adding `text` on `keyDown` for printable keys causes `interact key A` to also produce character input on focused inputs (double-insert if combined with `interact type`). | Medium | This is actually correct behavior — a real keyboard press of `A` on a focused input inserts `A`. Existing BDD scenarios that call `interact key` on a non-input (e.g., body-level keyup handler) are unaffected. New AC1 verifies the `/key_presses` fixture; the test fixture's input is cleared between scenarios. |
| Changing `Enter` from `"\r"` to `"Enter"` could break a consumer that was relying on the buggy value. | Low | Internal repo search shows no such consumer; the only consumer is `dispatch_key_press`. AC3 (`Ctrl+Enter` / `Shift+Enter`-style combos) and a dedicated Enter regression scenario (AC2) guard this. |
| `windowsVirtualKeyCode` mapping wrong for an unusual key (e.g., `ContextMenu`, `Pause`) could make that key misbehave. | Low | The helper returns 0 for unknown keys (matches today's behavior — no worse than the status quo). Tests cover the common letter / digit / Enter / Tab / Escape / arrows / F-keys set explicitly. |
| Headless Chrome behaves differently than headed Chrome for key events. | Low | The feature exercise gate uses headless Chrome against a local `tests/fixtures/interact-key-keyup-event.html`, mirroring the `/key_presses` behavior (uses `document.addEventListener('keyup', …)` and a non-jQuery assertion) so CI deterministically reproduces the fix. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Route `interact key` through `Input.insertText` for printable keys. | Would reliably produce character input. | Does not fire `keydown` / `keyup` — defeats the purpose of the command. |
| Add a composite `--then-key` to `interact click`. | Avoids cross-process focus concerns. | Orthogonal to the bug; focus already persists across invocations. Out of scope per requirements. |
| Fix only `Enter` (leave `windowsVirtualKeyCode` off). | Smaller diff. | Fails AC1 (letter key) against jQuery-style listeners. The repro site uses one. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (new private helpers colocated with `cdp_key_value` / `cdp_key_code`)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #227 | 2026-04-22 | Initial defect design |
