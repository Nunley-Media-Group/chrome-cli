# Design: Enhance Form Fill — ARIA Combobox Support

**Issues**: #196
**Date**: 2026-04-16
**Status**: Draft
**Author**: Claude (nmg-sdlc)

---

## Overview

This design adds ARIA combobox detection and automatic click-type-confirm interaction to the existing `form fill` command. The change is localized to three files: `src/form.rs` (core logic), `src/cli/mod.rs` (CLI argument), and `src/examples.rs` (documentation). The key architectural decision is to extend the existing `describe_element` function to also return the `role` attribute, add a combobox-specific fill path (`fill_element_combobox`) alongside the existing text-input and JS-setter paths, and use JavaScript-based click + keyboard character dispatch + listbox polling + confirmation key press for the combobox interaction sequence. The `--confirm-key` flag is added to `FormFillArgs` for combobox elements that confirm with a key other than Enter.

---

## Architecture

### Component Diagram

```
CLI Input: agentchrome form fill s5 "Acme Corp" [--confirm-key Tab]
    ↓
┌─────────────────────────────────────────────────────────────┐
│  CLI Layer (cli/mod.rs)                                      │
│  FormFillArgs { target, value, confirm_key, ... }            │
└────────────────────────┬────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  Command Module (form.rs)                                    │
│                                                              │
│  execute_fill() → fill_element()                             │
│                      ↓                                       │
│              describe_element()                               │
│              returns (node_name, input_type, role)            │
│                      ↓                                       │
│         ┌────────────┼────────────────┐                      │
│         ↓            ↓                ↓                      │
│   role=combobox   is_text_input    select/checkbox            │
│         ↓            ↓                ↓                      │
│  fill_element_   fill_element_   FILL_JS (existing)          │
│  combobox()      keyboard()                                  │
│    ↓                                                         │
│  1. DOM.focus + JS .click()                                  │
│  2. Keyboard char dispatch (type value)                      ��
│  3. Poll for listbox visibility                              │
│  4. Dispatch confirmation key (Enter/custom)                 │
└────────────────────────┬────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  CDP Client (WebSocket JSON-RPC)                             │
│  DOM.describeNode, DOM.focus, Runtime.callFunctionOn,        │
│  Runtime.evaluate, Input.dispatchKeyEvent                    │
└────────────────────────┬────────────────────────────────────┘
                         ↓
                   Chrome Browser
```

### Data Flow

```
1. User runs: agentchrome form fill s5 "Acme Corp"
2. CLI parses FormFillArgs (target="s5", value="Acme Corp", confirm_key=None)
3. execute_fill() sets up CDP session, resolves frame context
4. fill_element() is called with (session, target, value, confirm_key)
5. describe_element() returns (node_name="input", input_type=Some("text"), role=Some("combobox"))
6. role == "combobox" → routes to fill_element_combobox()
7. fill_element_combobox() executes:
   a. DOM.focus on the backend node → element receives focus
   b. Runtime.callFunctionOn: this.click() → triggers click handler, dropdown opens
   c. 50ms delay for dropdown to start rendering
   d. Keyboard char dispatch: types "Acme Corp" character by character
   e. Poll loop (up to 3s): Runtime.evaluate checks aria-expanded="true" or
      visible [role="option"] elements in the associated listbox
   f. Input.dispatchKeyEvent: keyDown+keyUp for Enter (or custom confirm key)
8. execute_fill() formats FillResult JSON and prints to stdout
```

---

## API / Interface Changes

### CLI Changes

| Flag | Type | Default | Purpose |
|------|------|---------|---------|
| `--confirm-key` | `Option<String>` | None (defaults to "Enter") | Override the key used to confirm combobox selection |

Added to `FormFillArgs` in `src/cli/mod.rs`. No collision with global flags (`--port`, `--host`, `--timeout`, `--tab`, `--output`, `--plain`, `--auto-dismiss-dialogs`, `--config`).

### Modified Function Signatures

#### `describe_element` (form.rs)

**Before:**
```rust
async fn describe_element(
    session: &ManagedSession,
    backend_node_id: i64,
) -> Result<(String, Option<String>), AppError>
// Returns (node_name, input_type)
```

**After:**
```rust
async fn describe_element(
    session: &ManagedSession,
    backend_node_id: i64,
) -> Result<(String, Option<String>, Option<String>), AppError>
// Returns (node_name, input_type, role)
```

The function already parses the flat `attributes` array from `DOM.describeNode` to find `type`. The change adds a second scan for `role` in the same loop.

#### `fill_element` (form.rs)

**Before:**
```rust
async fn fill_element(
    session: &ManagedSession,
    target: &str,
    value: &str,
) -> Result<(), AppError>
```

**After:**
```rust
async fn fill_element(
    session: &ManagedSession,
    target: &str,
    value: &str,
    confirm_key: Option<&str>,
) -> Result<(), AppError>
```

The `confirm_key` parameter is passed through to `fill_element_combobox` when the element has `role="combobox"`. For non-combobox elements, it is ignored.

### New Function

#### `fill_element_combobox` (form.rs)

```rust
async fn fill_element_combobox(
    session: &ManagedSession,
    backend_node_id: i64,
    value: &str,
    confirm_key: &str,
) -> Result<(), AppError>
```

Executes the click-type-wait-confirm sequence for ARIA combobox elements.

### Output Schema

No changes to the output schema. `FillResult` remains `{ filled, value, snapshot? }`. The combobox fill produces the same output structure as any other fill.

### Error Cases

| Condition | Error message | Exit code |
|-----------|--------------|-----------|
| No matching option after typing | `"No matching option found in combobox for value: {value}"` | 1 |
| Listbox did not appear within timeout | `"Combobox listbox did not appear within 3000ms after typing"` | 1 |

Both use `AppError::interaction_failed()` which produces structured JSON on stderr.

---

## Database / Storage Changes

None. No schema or migration changes required.

---

## State Management

No new state is introduced. The combobox fill uses the same session management and snapshot state as existing fill operations. The `confirm_key` is a per-invocation parameter, not persisted.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Coordinate-based click** | Resolve element center coordinates and dispatch mousePressed/mouseReleased (same as `interact click`) | Pixel-accurate, matches real user behavior | Requires extracting private helpers from interact.rs or duplicating code; scroll-into-view logic needed | Rejected — over-engineered for dropdown trigger |
| **B: JS `.click()` via callFunctionOn** | Call `this.click()` on the resolved element object | Simple, reuses existing object resolution; triggers click handlers reliably | Not pixel-accurate (no coordinates), but comboboxes respond to DOM click events not mouse position | **Selected** |
| **C: DOM.focus only** | Just focus the element and start typing, assuming focus opens the dropdown | Minimal code change | Many combobox implementations require an actual click to toggle `aria-expanded`; focus alone is insufficient | Rejected — unreliable for major component libraries |
| **D: Separate `form fill-combobox` subcommand** | Create a new subcommand specifically for combobox interactions | Clear separation of concerns | Defeats the purpose of automatic detection; user must know element type in advance | Rejected — violates the "single command" goal |

---

## Security Considerations

- [x] **Input Validation**: `--confirm-key` accepts any string; invalid key names are passed to CDP which returns an error. No injection risk since the value is used only as a CDP key name parameter, not in JS evaluation.
- [x] **Data Sanitization**: The combobox value is typed character-by-character via `Input.dispatchKeyEvent`, not injected into JavaScript strings. No XSS or injection vector.
- [x] **No new permissions**: Uses existing CDP domains (DOM, Runtime, Input) already enabled by form fill.

---

## Performance Considerations

- [x] **Polling overhead**: The listbox visibility poll uses `Runtime.evaluate` every 100ms for up to 3s (max 30 calls). This is negligible for a browser automation tool.
- [x] **Total sequence time**: Click (1 CDP call) + type N chars (N CDP calls) + poll (1-30 calls) + confirm (2 CDP calls) = typically < 2s for short values.
- [x] **No caching**: Each `describe_element` call makes one `DOM.describeNode` CDP call. This is consistent with existing form fill behavior.

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Unit | `#[test]` in form.rs | `is_combobox` classification, `describe_element` role parsing |
| BDD | Gherkin + cucumber-rs | All 7 ACs as scenarios (combobox fill, select preserved, error, confirm-key, examples, fill-many, async delay) |
| Smoke | Manual against Chrome | End-to-end against `tests/fixtures/form-fill-aria-combobox.html` |

---

## Implementation Details

### Combobox Detection via `describe_element`

The existing `describe_element` function parses `DOM.describeNode`'s response, which includes a flat `attributes` array: `["type", "text", "role", "combobox", "aria-expanded", "false", ...]`. Currently it scans for `"type"`. The change adds a second scan for `"role"` in the same `chunks(2)` iterator:

```rust
let (input_type, role) = response["node"]["attributes"].as_array().map_or(
    (None, None),
    |attrs| {
        let mut input_type = None;
        let mut role = None;
        for pair in attrs.chunks(2) {
            match pair.first().and_then(|v| v.as_str()) {
                Some("type") => input_type = pair.get(1).and_then(|v| v.as_str()).map(String::from),
                Some("role") => role = pair.get(1).and_then(|v| v.as_str()).map(String::from),
                _ => {}
            }
        }
        (input_type, role)
    },
);
```

### Click via JavaScript

The combobox click step uses `Runtime.callFunctionOn` with `this.click()`:

```javascript
function() { this.click(); }
```

This is simpler than coordinate-based clicking and reliably triggers the combobox's click event handlers. The element object ID is already resolved by `resolve_to_object_id`.

### Listbox Visibility Polling

After typing the value, the implementation polls for combobox option visibility using `Runtime.evaluate`:

```javascript
(function() {
    // Check 1: aria-expanded on the combobox itself
    var cb = document.querySelector('[role="combobox"][aria-expanded="true"]');
    if (!cb) return false;
    // Check 2: at least one visible option in the associated listbox
    var listboxId = cb.getAttribute('aria-owns') || cb.getAttribute('aria-controls');
    var listbox = listboxId ? document.getElementById(listboxId) : document.querySelector('[role="listbox"]');
    if (!listbox) return false;
    var options = listbox.querySelectorAll('[role="option"]');
    return options.length > 0;
})()
```

The poll runs every 100ms for up to 3000ms. If the result is `true`, the confirmation key is dispatched. If it times out, an error is returned.

### Confirmation Key Dispatch

Uses the same `Input.dispatchKeyEvent` keyDown+keyUp pattern as `fill_element_keyboard` and `interact key`:

```rust
// keyDown
session.send_command("Input.dispatchKeyEvent", Some(json!({
    "type": "keyDown",
    "key": "Enter",  // or custom confirm_key
    "code": "Enter",
}))).await?;

// keyUp
session.send_command("Input.dispatchKeyEvent", Some(json!({
    "type": "keyUp",
    "key": "Enter",
    "code": "Enter",
}))).await?;
```

### Routing in `fill_element`

```rust
async fn fill_element(session, target, value, confirm_key) {
    let backend_node_id = resolve_target_to_backend_node_id(session, target).await?;
    let (node_name, input_type, role) = describe_element(session, backend_node_id).await?;

    if role.as_deref() == Some("combobox") {
        let object_id = resolve_to_object_id(session, target).await?;
        fill_element_combobox(session, backend_node_id, &object_id, value, confirm_key.unwrap_or("Enter")).await
    } else if is_text_input(&node_name, input_type.as_deref()) {
        fill_element_keyboard(session, backend_node_id, value).await
    } else {
        // existing JS setter path for select, checkbox, radio
        let object_id = resolve_to_object_id(session, target).await?;
        // ... existing FILL_JS callFunctionOn
    }
}
```

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Combobox implementations that don't respond to `.click()` | Low | Med | The JS click approach works for all major component libraries (SF Lightning, MUI, Ant Design). Coordinate-based click can be added later if needed. |
| Listbox polling misses non-standard ARIA patterns | Med | Low | The polling checks both `aria-expanded` and `[role="option"]` presence. Libraries that don't set `aria-expanded` are caught by the option presence check. |
| `--confirm-key` value doesn't map to a valid CDP key | Low | Low | CDP returns an error which is surfaced as `AppError::interaction_failed`. No silent failure. |
| `describe_element` tuple return grows unwieldy | Low | Low | Three elements is manageable. If more attributes are needed in the future, refactor to a struct. |

---

## Open Questions

- [ ] Should the polling JS be scoped to the specific combobox (using its `aria-owns`/`aria-controls`) or fall back to any visible `[role="listbox"]`? — Design uses both: tries `aria-owns`/`aria-controls` first, falls back to generic `[role="listbox"]`.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #196 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] Database/storage changes planned with migrations (N/A)
- [x] State management approach is clear (no new state)
- [x] UI components and hierarchy defined (N/A — CLI only)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
