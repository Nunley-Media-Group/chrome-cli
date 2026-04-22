# Tasks: Fix `interact key` keyup listeners not observed

**Issues**: #227
**Date**: 2026-04-22
**Status**: Planning
**Author**: Rich Nunley

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix CDP key-event payload and key-value mappings in `src/interact.rs` | [x] |
| T002 | Add regression BDD feature + test fixture | [x] |
| T003 | Verify no regressions (build, unit, clippy, fmt, feature exercise gate) | [x] |

---

### T001: Fix the Defect

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `cdp_key_value` returns `"Enter"` for `Enter` and `"Tab"` for `Tab` (previously `"\r"` / `"\t"`). Other mappings unchanged.
- [ ] New private helper `windows_virtual_key_code(key: &str) -> i64` returns the Windows virtual-key code for letters (A–Z → 65–90), digits (0–9 → 48–57), `Enter` (13), `Tab` (9), `Escape` (27), `Backspace` (8), `Space` (32), arrow keys (37–40), `Home`/`End`/`PageUp`/`PageDown` (33–36), `Delete`/`Insert` (45–46), function keys F1–F24 (112–135), modifiers (`Shift`→16, `Control`→17, `Alt`→18, `Meta`→91), and 0 for unknown.
- [ ] New private helper `key_text(key: &str, modifiers: u8) -> Option<String>` returns `Some(…)` for printable keys (letters respect Shift, digits, symbols, `Enter`→`"\r"`, `Tab`→`"\t"`, `Space`→`" "`) and `None` for non-printable keys (`Escape`, arrows, F-keys, bare modifiers, navigation keys).
- [ ] `dispatch_key_press` includes `windowsVirtualKeyCode` on both `keyDown` and `keyUp`, and includes `text` on `keyDown` when `key_text` returns `Some`.
- [ ] `dispatch_modifier_event` includes `windowsVirtualKeyCode` on both events (modifiers are non-printable, so no `text`).
- [ ] `dispatch_key_combination` (modifier + key combined path) applies the same enrichment — `windowsVirtualKeyCode` on every event, `text` on the primary key's `keyDown` when applicable.
- [ ] No changes to `dispatch_char` or any `interact type` code path.
- [ ] Bug no longer reproduces: manual replay of the issue's reproduction steps produces `"You entered: A"` and `"You entered: ENTER"`.
- [ ] No unrelated changes included in the diff.

**Notes**: Follow the fix strategy from `design.md`. Keep the new helpers colocated with `cdp_key_value` / `cdp_key_code`. Shift-aware text mapping: when `modifiers & 8` (Shift) is set and key is a single lowercase letter, return `Some(uppercase)`; when key is already uppercase, return `Some(key)` regardless of Shift.

### T002: Add Regression Test

**File(s)**: `tests/features/227-fix-interact-key-keyup-event.feature`, `tests/bdd.rs`, `tests/fixtures/interact-key-keyup-event.html`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] `tests/fixtures/interact-key-keyup-event.html` is a self-contained page with: a focused text input (`id="target"`), a result element (`id="result"`), and a plain `document.addEventListener('keyup', e => { result.innerText = "You entered: " + e.key.toUpperCase(); })` listener (no external jQuery or network dependency).
- [ ] The fixture file-header HTML comment lists which ACs it covers.
- [ ] `tests/features/227-fix-interact-key-keyup-event.feature` contains scenarios for AC1 (letter `A`), AC2 (`Enter`), AC3 (modifier combination `Shift+A`), and AC4 (`interact type` control).
- [ ] Every scenario is tagged `@regression`.
- [ ] Step definitions are added to `tests/bdd.rs` following the existing worlds-in-one-file pattern (see `tests/bdd.rs` and neighbor feature files for the convention).
- [ ] `cargo test --test bdd` passes with the fix applied.
- [ ] Test fails if the fix (T001) is reverted — verified by temporarily reverting `cdp_key_value` and confirming AC2 fails; revert the revert before committing.

### T003: Verify No Regressions

**File(s)**: (no file changes)
**Type**: Verify
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo build` exits 0.
- [ ] `cargo test --lib` exits 0 (new `windows_virtual_key_code` and `key_text` unit tests included).
- [ ] `cargo test --test bdd` exits 0.
- [ ] `cargo clippy --all-targets` exits 0 (clippy `all=deny`, `pedantic=warn`).
- [ ] `cargo fmt --check` exits 0.
- [ ] Feature Exercise Gate: build debug binary, launch headless Chrome, navigate to `tests/fixtures/interact-key-keyup-event.html`, run `interact click target`, then `interact key A`, then read `#result` — confirms `"You entered: A"`. Repeat for `Enter` → `"You entered: ENTER"`. Kill any orphaned Chrome process after.
- [ ] No side effects in the `interact type` (char synthesis) path — AC4 scenario passes.

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #227 | 2026-04-22 | Initial defect tasks |
