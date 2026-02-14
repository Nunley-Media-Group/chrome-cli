# Design: Keyboard Input

**Issue**: #15
**Date**: 2026-02-13
**Status**: Approved
**Author**: Claude (writing-specs)

---

## Overview

This feature adds two keyboard interaction commands — `type` and `key` — to the existing `interact` subcommand group. The `type` command types text character-by-character using CDP `Input.dispatchKeyEvent` with `type: "char"` events. The `key` command presses individual keys or key combinations (e.g., `Control+A`) using `keyDown`/`keyUp` sequences with proper modifier handling.

The implementation extends `src/interact.rs` (which already contains mouse interaction commands from issue #14) and adds new CLI argument types in `src/cli/mod.rs`. A key validation and mapping system provides correct CDP `key`, `code`, and `modifiers` values for 100+ supported keys. Key combinations are parsed by splitting on `+`, validating each part against a whitelist, and computing a modifier bitmask.

The architecture follows the established interact command pattern: CLI args → session setup → CDP dispatch → optional snapshot → JSON/plain output.

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────────┐
│                     CLI Layer (cli/mod.rs)                     │
│  ┌──────────────┐                                              │
│  │ InteractArgs  │  InteractCommand (extended):                │
│  │               │    ...existing Click/ClickAt/Hover/Drag...  │
│  │               │    Type(TypeArgs)      ← NEW                │
│  │               │    Key(KeyArgs)        ← NEW                │
│  └──────┬───────┘                                              │
└─────────┼──────────────────────────────────────────────────────┘
          │
          ▼
┌──────────────────────────────────────────────────────────────┐
│               Command Layer (interact.rs)                     │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ Key Validation & Mapping                              │    │
│  │ VALID_KEYS: 85 non-modifier key names                 │    │
│  │ MODIFIER_KEYS: Alt, Control, Meta, Shift              │    │
│  │ parse_key_combination() → ParsedKey { modifiers, key }│    │
│  │ cdp_key_value() → CDP "key" field                     │    │
│  │ cdp_key_code() → CDP "code" field                     │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                               │
│  ┌───────────────────────────────────────────┐               │
│  │ Keyboard Dispatch Helpers                  │               │
│  │ dispatch_char(session, ch) → char event    │               │
│  │ dispatch_key_press(session, key, mods)     │               │
│  │   → keyDown + keyUp                        │               │
│  │ dispatch_key_combination(session, parsed)  │               │
│  │   → mod down + key press + mod up          │               │
│  └───────────────────┬───────────────────────┘               │
│                      │                                        │
│  ┌───────────────────────────────────────────┐               │
│  │ Command Functions                          │               │
│  │ execute_type(global, args)                 │               │
│  │ execute_key(global, args)                  │               │
│  └───────────────────┬───────────────────────┘               │
│                      │                                        │
│  Shared with mouse commands:                                  │
│  │ setup_session(), take_snapshot(), print_output()           │
└──────────────────────┼────────────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────────┐
│               CDP Layer (ManagedSession)                      │
│  Input (no enable needed) → Input.dispatchKeyEvent            │
│    - type: "char" (for text input)                            │
│    - type: "keyDown" / "keyUp" (for key presses)              │
│    - key: CDP key value (e.g., "\r" for Enter)                │
│    - code: CDP code value (e.g., "Enter", "KeyA")             │
│    - modifiers: bitmask (1=Alt, 2=Control, 4=Meta, 8=Shift)  │
│  Runtime.evaluate (for getting URL before snapshot)           │
│  Accessibility.getFullAXTree (for --include-snapshot)         │
└──────────────────────────────────────────────────────────────┘
```

### Data Flow — `interact type "Hello World"`

```
1. User runs: chrome-cli interact type "Hello World" [--delay 50] [--include-snapshot]
2. CLI layer parses args → TypeArgs { text: "Hello World", delay: 0, include_snapshot: false }
3. setup_session() → CdpClient + ManagedSession
4. For each character in "Hello World":
   a. dispatch_char(session, 'H') → Input.dispatchKeyEvent({ type: "char", text: "H" })
   b. If --delay > 0: tokio::time::sleep(delay_ms)
   c. dispatch_char(session, 'e') → ...
   d. ... repeat for all 11 characters
5. If --include-snapshot: take_snapshot() → Accessibility.getFullAXTree
6. Build TypeResult { typed: "Hello World", length: 11, snapshot: None }
7. Output result as JSON/pretty/plain
```

### Data Flow — `interact key "Control+A"`

```
1. User runs: chrome-cli interact key "Control+A" [--repeat 3] [--include-snapshot]
2. CLI layer parses args → KeyArgs { keys: "Control+A", repeat: 1, include_snapshot: false }
3. parse_key_combination("Control+A"):
   a. Split by '+' → ["Control", "A"]
   b. Validate: "Control" is valid modifier, "A" is valid key ✓
   c. Return ParsedKey { modifiers: 2 (Control bit), key: "A" }
4. setup_session() → CdpClient + ManagedSession
5. For each repeat (default 1):
   a. dispatch_key_combination(session, parsed):
      i.   keyDown { key: "Control", code: "ControlLeft", modifiers: 2 }
      ii.  keyDown { key: "a", code: "KeyA", modifiers: 2 }
      iii. keyUp   { key: "a", code: "KeyA", modifiers: 2 }
      iv.  keyUp   { key: "Control", code: "ControlLeft", modifiers: 0 }
6. If --include-snapshot: take_snapshot()
7. Build KeyResult { pressed: "Control+A", repeat: None, snapshot: None }
8. Output result as JSON/pretty/plain
```

### Data Flow — `interact key "Enter"` (no modifiers)

```
1. User runs: chrome-cli interact key Enter
2. parse_key_combination("Enter"):
   a. Split by '+' → ["Enter"]
   b. Validate: "Enter" is valid key ✓
   c. Return ParsedKey { modifiers: 0, key: "Enter" }
3. setup_session() → CdpClient + ManagedSession
4. dispatch_key_press(session, "Enter", 0):
   a. keyDown { key: "\r", code: "Enter", modifiers: 0 }
   b. keyUp   { key: "\r", code: "Enter", modifiers: 0 }
5. Build KeyResult { pressed: "Enter", repeat: None, snapshot: None }
6. Output result
```

---

## API / Interface Changes

### New CLI Commands

| Command | Purpose |
|---------|---------|
| `chrome-cli interact type <TEXT>` | Type text character-by-character into the focused element |
| `chrome-cli interact key <KEYS>` | Press a key or key combination |

### New CLI Flags

| Flag | Applies To | Type | Purpose |
|------|-----------|------|---------|
| `--delay <MS>` | type | u64 | Delay between keystrokes in milliseconds (default: 0) |
| `--repeat <N>` | key | u32 | Number of times to press the key (default: 1) |
| `--include-snapshot` | type, key | bool | Include updated accessibility snapshot in output |

### Request / Response Schemas

#### `interact type <TEXT>`

**Input (CLI args):**
```
chrome-cli interact type <TEXT> [--delay MS] [--include-snapshot] [--tab ID]
```

**Output (success — JSON):**
```json
{
  "typed": "Hello World",
  "length": 11
}
```

**Output (with --include-snapshot):**
```json
{
  "typed": "Hello World",
  "length": 11,
  "snapshot": { "role": "document", "name": "...", "children": [...] }
}
```

**Output (plain text):**
```
Typed 11 characters
```

**Errors:**

| Exit Code | Condition |
|-----------|-----------|
| 1 (GeneralError) | Missing required text argument |
| 2 (ConnectionError) | Cannot connect to Chrome |
| 3 (TargetError) | Tab not found |
| 4 (TimeoutError) | Command timed out |
| 5 (ProtocolError) | CDP protocol error during dispatch |

#### `interact key <KEYS>`

**Input (CLI args):**
```
chrome-cli interact key <KEYS> [--repeat N] [--include-snapshot] [--tab ID]
```

**Output (success — JSON, single press):**
```json
{
  "pressed": "Enter"
}
```

**Output (success — JSON, with repeat > 1):**
```json
{
  "pressed": "ArrowDown",
  "repeat": 5
}
```

**Output (with --include-snapshot):**
```json
{
  "pressed": "Control+A",
  "snapshot": { "role": "document", "name": "...", "children": [...] }
}
```

**Output (plain text):**
```
Pressed Enter
```

**Errors:**

| Exit Code | Condition |
|-----------|-----------|
| 1 (GeneralError) | Invalid key name |
| 1 (GeneralError) | Duplicate modifier in combination |
| 1 (GeneralError) | Missing required keys argument |
| 2 (ConnectionError) | Cannot connect to Chrome |
| 3 (TargetError) | Tab not found |
| 4 (TimeoutError) | Command timed out |
| 5 (ProtocolError) | CDP protocol error during dispatch |

---

## Database / Storage Changes

None. These commands do not read or write persistent state. When `--include-snapshot` is used, the updated snapshot is written to `~/.chrome-cli/snapshot.json` (same behavior as mouse interaction commands).

---

## State Management

### Key Validation State (compile-time constants)

```rust
/// Modifier key names.
const MODIFIER_KEYS: &[&str] = &["Alt", "Control", "Meta", "Shift"];

/// All valid key names (non-modifier) — 85 entries.
const VALID_KEYS: &[&str] = &[
    // Letters: a-z, A-Z
    // Digits: 0-9
    // Function keys: F1-F24
    // Navigation, Editing, Whitespace, Numpad, Media, Symbols, Lock keys, Other
];
```

### Parsed Key Combination (in-memory, per-command)

```rust
/// Parsed key combination.
struct ParsedKey {
    /// Modifier bitmask (1=Alt, 2=Control, 4=Meta, 8=Shift).
    modifiers: u8,
    /// The primary (non-modifier) key name.
    key: String,
}
```

### Key Mapping Functions

```rust
/// Get the CDP `key` value for a key name.
/// Maps: "Enter" → "\r", "Tab" → "\t", "Space" → " ",
///        "a" → "a", "Minus" → "-", etc.
fn cdp_key_value(key: &str) -> &str;

/// Get the CDP `code` value for a key name.
/// Maps: "a" → "KeyA", "1" → "Digit1", "Enter" → "Enter",
///        "Alt" → "AltLeft", etc.
fn cdp_key_code(key: &str) -> String;
```

### Modifier Bitmask

| Bit | Value | Modifier |
|-----|-------|----------|
| 0 | 1 | Alt |
| 1 | 2 | Control |
| 2 | 4 | Meta |
| 3 | 8 | Shift |

### State Transitions

```
Command start → validate key (for `key` command only)
    ↓
setup_session()
    ↓
[Type command]:
    For each character → Input.dispatchKeyEvent({ type: "char", text: ch })
        → optional delay
    ↓
[Key command]:
    For each repeat:
        If has modifiers → dispatch_key_combination()
            → modifier keyDown(s) → primary keyDown+keyUp → modifier keyUp(s)
        Else → dispatch_key_press()
            → keyDown + keyUp
    ↓
[If --include-snapshot]: take_snapshot()
    ↓
Build + output result
```

---

## UI Components

N/A — this is a CLI tool, no UI components.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Runtime.evaluate for typing** | Use `element.value = text` or `element.dispatchEvent(new KeyboardEvent(...))` | Simple, no CDP Input domain needed | Doesn't trigger real input events, may not fire change/input handlers, doesn't work with contenteditable | Rejected — not a real keyboard simulation |
| **B: CDP Input.dispatchKeyEvent** | Low-level keyboard event dispatch with proper key codes and modifier handling | Full fidelity, matches real keyboard input, triggers all event listeners, handles combinations | Requires key code mapping tables, modifier sequencing logic | **Selected** — matches MCP server approach |
| **C: CDP Input.insertText** | Use `Input.insertText` for text, `Input.dispatchKeyEvent` only for key presses | Simpler for plain text, single CDP call | Different code path for type vs key, `insertText` doesn't fire keyDown/keyUp events | Rejected — inconsistent event model, may miss handlers expecting key events |

**Design Decision**: Use `Input.dispatchKeyEvent` exclusively. For text typing, use `type: "char"` events (which trigger input events like real typing). For key presses, use `keyDown`/`keyUp` sequences with proper `key`, `code`, and `modifiers` values. This matches the MCP server's approach and provides full-fidelity keyboard simulation.

---

## Security Considerations

- [x] **Input Validation**: Key names validated against a compile-time whitelist before connecting to Chrome
- [x] **No sensitive data**: Key names and typed text contain no inherent secrets (though users may type passwords — this is expected CLI behavior)
- [x] **Local only**: All CDP communication is localhost
- [x] **No code injection**: Key names are validated, not evaluated; text is dispatched as character events, not executed

---

## Performance Considerations

- [x] **Early validation**: Key combination parsing and validation happen before establishing a Chrome connection; invalid keys fail fast
- [x] **Minimal CDP round-trips**: Each character = 1 CDP call; each key press = 2 CDP calls (keyDown+keyUp); each key combination = 2N+2 calls (N modifiers × 2 + key × 2)
- [x] **No domain enabling**: `Input.dispatchKeyEvent` does not require `Input.enable`
- [x] **Configurable delay**: Text typing is instant by default (0ms delay); users opt-in to delays
- [x] **Snapshot is optional**: Accessibility tree capture only occurs when `--include-snapshot` is specified

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Key Validation | Unit | `is_valid_key()`, `is_modifier()`, `parse_key_combination()` |
| Key Mapping | Unit | `cdp_key_value()`, `cdp_key_code()` for all key categories |
| Output Types | Unit | Serialization of `TypeResult`, `KeyResult` |
| Plain Text Formatting | Unit | `print_type_plain()`, `print_key_plain()` |
| CLI Args | BDD | Clap parsing for type and key subcommands, help text |
| Error Handling | BDD | Invalid key, duplicate modifier, missing args |
| Feature | BDD (cucumber-rs) | All 18 acceptance criteria from requirements.md |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Key code mapping errors for obscure keys | Low | Medium | Comprehensive unit tests for all key categories; mapping tables match CDP spec |
| Unicode text dispatch failures | Low | Low | CDP `char` events handle arbitrary Unicode; tested with multi-byte characters |
| Modifier sequencing bugs | Medium | Medium | Unit tests for modifier combinations; follows MCP server's approach |
| Focused element requirement not met | Medium | Low | Document that element must be focused; suggest using `interact click` first |
| Key repeat performance with high counts | Low | Low | No artificial limit on repeat count; each press is fast (~2 CDP calls) |

---

## Open Questions

- (None)

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] State management approach is clear (compile-time key tables, in-memory parsing)
- [x] N/A — no UI components
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
