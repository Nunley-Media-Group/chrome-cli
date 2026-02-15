# Tasks: Keyboard Input

**Issue**: #15
**Date**: 2026-02-13
**Status**: Planning
**Author**: Claude (writing-specs)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 1 | [ ] |
| Backend | 4 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **8** | |

---

## Phase 1: Setup

### T001: Define CLI argument types for type and key commands

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `TypeArgs` struct with:
  - `text: String` positional arg (`#[arg(required = true)]`)
  - `--delay` u64 with `default_value_t = 0`
  - `--include-snapshot` bool flag
- [ ] `KeyArgs` struct with:
  - `keys: String` positional arg (`#[arg(required = true)]`)
  - `--repeat` u32 with `default_value_t = 1`
  - `--include-snapshot` bool flag
- [ ] `InteractCommand` enum extended with `Type(TypeArgs)` and `Key(KeyArgs)` variants
- [ ] `TypeArgs` and `KeyArgs` imported in `src/interact.rs`
- [ ] `cargo check` passes with no errors

**Notes**: Follow the exact pattern used by `ClickArgs` and other existing interact command args. The `Type` and `Key` variants are added to the existing `InteractCommand` enum alongside `Click`, `ClickAt`, `Hover`, `Drag`.

---

## Phase 2: Backend Implementation

### T002: Implement key validation constants and parsing

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `MODIFIER_KEYS` constant: `&["Alt", "Control", "Meta", "Shift"]`
- [ ] `VALID_KEYS` constant: 85 non-modifier key names (letters a-z/A-Z, digits 0-9, F1-F24, navigation, editing, whitespace, numpad, media, symbols, lock keys, other)
- [ ] `is_modifier(key: &str) -> bool` — checks against `MODIFIER_KEYS`
- [ ] `is_valid_key(key: &str) -> bool` — checks against both `MODIFIER_KEYS` and `VALID_KEYS`
- [ ] `parse_key_combination(input: &str) -> Result<ParsedKey, AppError>`:
  - Splits input on `+`
  - Validates each part via `is_valid_key()`
  - Returns `AppError` with message `"Invalid key: '{part}'"` for unknown keys
  - Detects duplicate modifiers, returns `AppError` with `"Duplicate modifier: '{part}'"`
  - Computes modifier bitmask (1=Alt, 2=Control, 4=Meta, 8=Shift)
  - Extracts primary (non-modifier) key
  - If all parts are modifiers, uses the last one as primary key
- [ ] `ParsedKey` struct with `modifiers: u8` and `key: String`
- [ ] `cargo check` passes with no errors

### T003: Implement CDP key mapping functions

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `cdp_key_value(key: &str) -> &str`:
  - Maps key names to CDP `key` values
  - `Enter` → `"\r"`, `Tab` → `"\t"`, `Space` → `" "`
  - Single characters (a-z, A-Z, 0-9) → the character itself
  - Symbol names → their character (`Minus` → `"-"`, `Comma` → `","`, etc.)
  - Function keys, navigation, lock keys, media → key name as-is
  - Modifiers → key name as-is (`Alt`, `Control`, `Meta`, `Shift`)
- [ ] `cdp_key_code(key: &str) -> String`:
  - Maps key names to CDP `code` values
  - Letters → `"Key{UPPER}"` (e.g., `a` → `"KeyA"`)
  - Digits → `"Digit{N}"` (e.g., `1` → `"Digit1"`)
  - Modifiers → left variant (`Alt` → `"AltLeft"`, `Shift` → `"ShiftLeft"`)
  - All other keys → key name as-is (`Enter` → `"Enter"`, `F1` → `"F1"`)
- [ ] `cargo check` passes with no errors

### T004: Implement keyboard dispatch helpers

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `dispatch_char(session, ch: char) -> Result<()>`:
  - Sends `Input.dispatchKeyEvent` with `{ type: "char", text: ch.to_string() }`
  - Error mapped via `AppError::interaction_failed("char", ...)`
- [ ] `dispatch_key_press(session, key: &str, modifiers: u8) -> Result<()>`:
  - Sends `keyDown` with `{ type: "keyDown", key: cdp_key_value(key), code: cdp_key_code(key), modifiers }`
  - Sends `keyUp` with same `key`/`code` values
  - Errors mapped via `AppError::interaction_failed("key_down"/"key_up", ...)`
- [ ] `dispatch_key_combination(session, parsed: &ParsedKey) -> Result<()>`:
  - Presses modifier keys down in order: Alt (bit 0), Control (bit 1), Meta (bit 2), Shift (bit 3)
  - Dispatches the primary key press via `dispatch_key_press()`
  - Releases modifier keys in reverse order: Shift, Meta, Control, Alt
  - Modifier keyDown events carry the full modifier bitmask; keyUp events carry modifiers=0
- [ ] `cargo check` passes with no errors

### T005: Implement execute_type and execute_key command functions

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `TypeResult` struct with: `typed: String`, `length: usize`, optional `snapshot`
- [ ] `KeyResult` struct with: `pressed: String`, optional `repeat: u32`, optional `snapshot`
- [ ] `execute_type(global, args) -> Result<()>`:
  - Sets up session via `setup_session()`
  - Iterates over each character in `args.text`, calls `dispatch_char()`
  - If `args.delay > 0`: sleeps between each character
  - If `--include-snapshot`: takes snapshot via `take_snapshot()`
  - Builds `TypeResult` with character count
  - Outputs as JSON or plain text (`"Typed N characters"`)
- [ ] `execute_key(global, args) -> Result<()>`:
  - Validates key combination via `parse_key_combination()` **before** connecting
  - Sets up session
  - Repeats `args.repeat` times:
    - If modifiers present: `dispatch_key_combination()`
    - Else: `dispatch_key_press()`
  - If `--include-snapshot`: takes snapshot
  - Builds `KeyResult` (only includes `repeat` field if > 1)
  - Outputs as JSON or plain text (`"Pressed {keys}"`)
- [ ] `print_type_plain()` and `print_key_plain()` formatting functions
- [ ] `execute_interact()` dispatcher updated with `Type` and `Key` arms
- [ ] `cargo check` passes with no errors

---

## Phase 3: Integration

### T006: Wire keyboard commands into the interact dispatcher

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] `execute_interact()` match block includes:
  - `InteractCommand::Type(type_args) => execute_type(global, type_args).await`
  - `InteractCommand::Key(key_args) => execute_key(global, key_args).await`
- [ ] `TypeArgs` and `KeyArgs` imported from `crate::cli`
- [ ] `cargo check` passes with no errors
- [ ] `cargo clippy` passes with no warnings

---

## Phase 4: BDD Testing

### T007: Create BDD feature file for keyboard input

**File(s)**: `tests/features/keyboard.feature` (create)
**Type**: Create
**Depends**: T006
**Acceptance**:
- [ ] All 18 acceptance criteria from requirements.md are scenarios
- [ ] Uses Given/When/Then format
- [ ] Includes happy paths: type text, type with delay, press single key, press key combination, press with multiple modifiers, press with repeat
- [ ] Includes snapshot scenarios: type with include-snapshot, key with include-snapshot
- [ ] Includes error cases: invalid key, duplicate modifier, missing text arg, missing keys arg
- [ ] Includes supported key categories (parameterized)
- [ ] Includes plain text output scenarios
- [ ] Includes tab targeting scenarios
- [ ] Feature file is valid Gherkin syntax

### T008: Implement step definitions and unit tests

**File(s)**: `tests/bdd.rs` (modify), `src/interact.rs` (add unit tests)
**Type**: Modify
**Depends**: T007
**Acceptance**:
- [ ] Step definitions for keyboard scenarios in BDD test harness
- [ ] Unit tests for `parse_key_combination()`:
  - Single key: `"Enter"` → `ParsedKey { modifiers: 0, key: "Enter" }`
  - Modifier + key: `"Control+A"` → `ParsedKey { modifiers: 2, key: "A" }`
  - Multiple modifiers: `"Control+Shift+A"` → `ParsedKey { modifiers: 10, key: "A" }`
  - Invalid key: `"FooBar"` → error
  - Duplicate modifier: `"Control+Control+A"` → error
- [ ] Unit tests for `cdp_key_value()`: Enter→"\r", Tab→"\t", Space→" ", letters, symbols
- [ ] Unit tests for `cdp_key_code()`: letters→"KeyX", digits→"DigitN", modifiers→"XxxLeft"
- [ ] Unit tests for `TypeResult` and `KeyResult` serialization (skip_serializing_if behavior)
- [ ] `cargo test --lib` passes
- [ ] `cargo test` passes (all tests including BDD)

---

## Dependency Graph

```
T001 (CLI args) ──▶ T002 (key validation) ──▶ T003 (key mapping)
                                                       │
                                                       ▼
                                               T004 (dispatch helpers)
                                                       │
                                                       ▼
                                               T005 (command functions)
                                                       │
                                                       ▼
                                               T006 (integration)
                                                       │
                                                       ▼
                                               T007 (feature file)
                                                       │
                                                       ▼
                                               T008 (step defs + unit tests)
```

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
