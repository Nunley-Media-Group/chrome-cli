# Design: Form Input and Filling

**Issue**: #16
**Date**: 2026-02-13
**Status**: Approved
**Author**: Claude (nmg-sdlc)

---

## Overview

This feature adds a `form` subcommand to chrome-cli with three operations: `fill` (set a single field), `fill-many` (set multiple fields from JSON), and `clear` (reset a field). The implementation follows the same architecture as the existing `interact` command module — a new `src/form.rs` file with CLI arg types defined in `src/cli/mod.rs` and dispatch wired through `src/main.rs`.

Form values are set using CDP's `Runtime.callFunctionOn` to directly modify DOM element properties and dispatch synthetic events (`input`, `change`) for framework compatibility. Target elements are resolved from UIDs (via the snapshot UID map) or CSS selectors (via `DOM.querySelector`), reusing the same target resolution pattern from `interact.rs`.

---

## Architecture

### Component Diagram

```
CLI Input (chrome-cli form fill s1 "John")
    ↓
┌─────────────────┐
│   CLI Layer      │  ← Parse args: FormArgs → FormCommand::Fill(FormFillArgs)
│   cli/mod.rs     │
└────────┬────────┘
         ↓
┌─────────────────┐
│  Command Layer   │  ← form.rs: resolve target, set value, dispatch events
│   form.rs        │
└────────┬────────┘
         ↓
┌─────────────────┐
│   CDP Layer      │  ← DOM.resolveNode, Runtime.callFunctionOn
│   ManagedSession │
└────────┬────────┘
         ↓
   Chrome Browser
```

### Data Flow

```
1. User runs: chrome-cli form fill s1 "John"
2. CLI layer parses args into FormFillArgs { target: "s1", value: "John", ... }
3. form.rs dispatcher calls execute_fill()
4. Setup CDP session (resolve_connection → resolve_target → CdpClient::connect)
5. Enable DOM and Runtime domains
6. Resolve UID "s1" to backend node ID via snapshot state
7. Use DOM.resolveNode to get Runtime object ID
8. Call Runtime.callFunctionOn with JS that:
   a. Detects element type (input/select/textarea/checkbox)
   b. Sets the value appropriately
   c. Dispatches input + change events
9. Optionally take snapshot if --include-snapshot
10. Return JSON result: {"filled": "s1", "value": "John"}
```

---

## API / Interface Changes

### New CLI Commands

| Command | Args | Purpose |
|---------|------|---------|
| `form fill <TARGET> <VALUE>` | `--include-snapshot` | Fill a single form field |
| `form fill-many <JSON>` | `--file <PATH>`, `--include-snapshot` | Fill multiple fields |
| `form clear <TARGET>` | `--include-snapshot` | Clear a form field |

### CLI Arg Structs (in cli/mod.rs)

```rust
// Top-level form args
pub struct FormArgs {
    pub command: FormCommand,
}

pub enum FormCommand {
    Fill(FormFillArgs),
    FillMany(FormFillManyArgs),
    Clear(FormClearArgs),
}

pub struct FormFillArgs {
    pub target: String,       // UID (s1) or CSS selector (css:#email)
    pub value: String,
    pub include_snapshot: bool,
}

pub struct FormFillManyArgs {
    pub json: Option<String>,  // Inline JSON array
    pub file: Option<PathBuf>, // JSON file path
    pub include_snapshot: bool,
}

pub struct FormClearArgs {
    pub target: String,
    pub include_snapshot: bool,
}
```

### Output Schemas

#### `form fill` output

```json
{
  "filled": "s1",
  "value": "John"
}
```

With `--include-snapshot`:
```json
{
  "filled": "s1",
  "value": "John",
  "snapshot": { ... }
}
```

#### `form fill-many` output

```json
[
  { "filled": "s1", "value": "John" },
  { "filled": "s2", "value": "Doe" }
]
```

With `--include-snapshot`:
```json
{
  "results": [
    { "filled": "s1", "value": "John" },
    { "filled": "s2", "value": "Doe" }
  ],
  "snapshot": { ... }
}
```

#### `form clear` output

```json
{
  "cleared": "s1"
}
```

### Errors

| Condition | Error Message |
|-----------|---------------|
| UID not found in snapshot | `"UID not found: s999. Take a snapshot first with 'page snapshot'."` |
| No snapshot state | `"No snapshot state found. Run 'page snapshot' first."` |
| CSS selector matches no element | `"Element not found for selector: #nonexistent"` |
| Invalid JSON for fill-many | `"Invalid JSON: expected array of {uid, value} objects"` |
| File not found (fill-many --file) | `"File not found: fields.json"` |
| Element is not fillable | `"Element is not a fillable form field"` |

---

## State Management

No new persistent state. The feature reuses:
- **Snapshot state** (`~/.chrome-cli/snapshot.json`) — read UID-to-backendNodeId mappings
- **Session state** (`~/.chrome-cli/session.json`) — resolve CDP connection

When `--include-snapshot` is used, snapshot state is updated (same pattern as `interact` commands).

---

## Implementation Details

### Target Resolution (reuse from interact.rs)

The target resolution logic (`is_uid`, `is_css_selector`, `resolve_target_to_backend_node_id`) is duplicated from `interact.rs`. Both modules need the same pattern:

1. Check if target starts with `s` followed by digits → UID lookup from snapshot state
2. Check if target starts with `css:` → DOM.querySelector via CDP
3. Otherwise → error

### Value Setting via Runtime.callFunctionOn

The core fill logic uses `Runtime.callFunctionOn` to execute a JavaScript function on the resolved element:

```javascript
function(value) {
  const el = this;
  const tag = el.tagName.toLowerCase();

  if (tag === 'select') {
    // Find matching option and set selectedIndex
    const options = Array.from(el.options);
    const idx = options.findIndex(o => o.value === value || o.textContent.trim() === value);
    if (idx >= 0) {
      el.selectedIndex = idx;
      el.value = options[idx].value;
    }
  } else if (el.type === 'checkbox' || el.type === 'radio') {
    el.checked = value === 'true' || value === 'checked';
  } else {
    // text, password, email, number, textarea, etc.
    el.value = value;
  }

  // Dispatch events for framework compatibility
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
}
```

### CDP Calls Sequence

```
1. DOM.enable
2. Runtime.enable
3. DOM.resolveNode({ backendNodeId }) → { object: { objectId } }
4. Runtime.callFunctionOn({
     objectId,
     functionDeclaration: "<fill JS>",
     arguments: [{ value: "<user value>" }]
   })
5. (Optional) Accessibility.getFullAXTree for snapshot
```

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Input.dispatchKeyEvent** | Simulate keystrokes to type into fields | Mimics real user input | Slow for long values; doesn't work for select/checkbox; requires focus management | Rejected — unreliable for form filling |
| **B: Runtime.callFunctionOn** | Execute JS directly on element to set value | Fast, reliable, works for all element types, dispatches proper events | Bypasses some native browser behaviors | **Selected** — matches MCP server approach |
| **C: DOM.setAttributeValue** | Set the `value` attribute via DOM | Simple CDP call | Doesn't update JS `.value` property; doesn't trigger events | Rejected — incomplete |

---

## Security Considerations

- [x] **Input Validation**: Target must be valid UID format or css: prefix
- [x] **Data Sanitization**: Values are passed as CDP function arguments, not string-interpolated into JS
- [x] **File reading**: `--file` reads local files only; path validated by OS
- [x] **No sensitive data storage**: Values are transient, not persisted

---

## Performance Considerations

- [x] **Single CDP round-trip**: Fill uses one `Runtime.callFunctionOn` call per field
- [x] **Batch optimization**: `fill-many` could batch calls but sequential is simpler and sufficient
- [x] **Snapshot is optional**: Only taken when `--include-snapshot` is passed

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| CLI args | Unit | Arg parsing, validation, conflicts |
| Target resolution | Unit | UID format, CSS selector format |
| Output serialization | Unit | JSON output struct serialization |
| Fill logic | Integration (BDD) | All element types, events, errors |
| Fill-many | Integration (BDD) | JSON parsing, file reading, batch results |
| Clear | Integration (BDD) | Clear and event dispatch |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Framework event dispatch insufficient | Medium | Medium | Dispatch both `input` and `change` with `bubbles: true`; test with React |
| Select option matching ambiguous | Low | Low | Match by option `value` first, fall back to `textContent` |
| UID resolution race condition | Low | Low | UIDs are stable within a snapshot; document that re-snapshot is needed after DOM changes |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed (reuses existing snapshot state)
- [x] State management approach is clear (no new state)
- [x] N/A — CLI tool, no UI components
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives considered and documented
- [x] Risks identified with mitigations
