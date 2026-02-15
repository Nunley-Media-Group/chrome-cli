# Design: Browser Dialog Handling

**Issue**: #20
**Date**: 2026-02-13
**Status**: Draft
**Author**: Claude (writing-specs)

---

## Overview

This feature adds a `dialog` command group to chrome-cli for handling browser JavaScript dialogs (alert, confirm, prompt, beforeunload). It consists of two subcommands (`dialog handle` and `dialog info`) plus a global `--auto-dismiss-dialogs` flag.

The implementation follows the established command pattern: CLI args defined in `cli/mod.rs`, a new `dialog.rs` command module, CDP communication via `ManagedSession`, and JSON/plain output formatting. The key CDP methods are `Page.javascriptDialogOpening` (event) and `Page.handleJavaScriptDialog` (command).

A critical design consideration is that dialog state must be captured via event subscription *before* the dialog can be queried. The `dialog info` command subscribes to `Page.javascriptDialogOpening` and then triggers a lightweight probe to check for an already-open dialog, while `dialog handle` directly calls the CDP method and uses the event data for the response.

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────────┐
│                     CLI Layer (cli/mod.rs)                     │
│  ┌──────────────┐  ┌─────────────────────────────────────┐   │
│  │ DialogArgs   │  │ GlobalOpts (--auto-dismiss-dialogs)  │   │
│  │ DialogCommand│  └─────────────────────────────────────┘   │
│  └──────┬───────┘                                             │
└─────────┼─────────────────────────────────────────────────────┘
          │
          ▼
┌──────────────────────────────────────────────────────────────┐
│               Command Layer (dialog.rs)                       │
│  ┌──────────────────┐  ┌──────────────────────────┐          │
│  │ execute_handle() │  │ execute_info()            │          │
│  │ - accept/dismiss │  │ - probe for open dialog   │          │
│  └────────┬─────────┘  └────────────┬─────────────┘          │
└───────────┼──────────────────────────┼────────────────────────┘
            │                          │
            ▼                          ▼
┌──────────────────────────────────────────────────────────────┐
│               CDP Layer (ManagedSession)                      │
│  Page.enable → Page.handleJavaScriptDialog                    │
│                Page.javascriptDialogOpening (event)            │
│                Runtime.evaluate (probe for open dialog)        │
└──────────────────────────────────────────────────────────────┘
```

### Data Flow — `dialog handle accept`

```
1. User runs: chrome-cli dialog handle accept [--text "foo"]
2. CLI layer parses args → DialogHandleArgs { action: Accept, text: Some("foo") }
3. setup_session() → CdpClient + ManagedSession
4. managed.ensure_domain("Page")
5. Subscribe to Page.javascriptDialogOpening (to capture dialog metadata)
6. Call Page.handleJavaScriptDialog { accept: true, promptText: "foo" }
7. If CDP returns error (no dialog open) → AppError::no_dialog_open()
8. On success → build HandleResult from the event data + action
9. Output HandleResult as JSON/pretty/plain
```

### Data Flow — `dialog info`

```
1. User runs: chrome-cli dialog info
2. CLI layer parses args → DialogCommand::Info
3. setup_session() → CdpClient + ManagedSession
4. managed.ensure_domain("Page")
5. Subscribe to Page.javascriptDialogOpening
6. Probe for open dialog: Runtime.evaluate("0") — if a dialog is blocking,
   the evaluate will timeout/fail, confirming a dialog exists.
   Alternatively, try Page.handleJavaScriptDialog with a short timeout:
   if it succeeds, a dialog was open (but we don't want to dismiss it).
   → Best approach: use a brief tokio::select! to check if a
     Page.javascriptDialogOpening event arrives within ~100ms.
   → If no event arrives, the dialog was either already opened before
     subscribe, so we attempt Runtime.evaluate to detect blocking.
7. Build DialogInfoResult { open, type, message, default_value }
8. Output as JSON/pretty/plain
```

### Data Flow — `--auto-dismiss-dialogs`

```
1. User runs: chrome-cli navigate https://example.com --auto-dismiss-dialogs
2. Global flag parsed in GlobalOpts
3. Before executing the primary command, subscribe to Page.javascriptDialogOpening
4. Spawn a background task that auto-dismisses any dialog that appears
5. Primary command executes normally
6. On command completion, background task is dropped
```

---

## API / Interface Changes

### New CLI Commands

| Command | Purpose |
|---------|---------|
| `chrome-cli dialog handle <accept\|dismiss>` | Accept or dismiss the current dialog |
| `chrome-cli dialog info` | Check if a dialog is currently open |

### New Global Flag

| Flag | Type | Purpose |
|------|------|---------|
| `--auto-dismiss-dialogs` | bool | Auto-dismiss all dialogs during command execution |

### Request / Response Schemas

#### `dialog handle accept`

**Input (CLI args):**
```
chrome-cli dialog handle accept [--text TEXT] [--tab ID]
```

**Output (success — JSON):**
```json
{
  "action": "accept",
  "dialog_type": "alert",
  "message": "Hello world"
}
```

**Output (success — prompt with text):**
```json
{
  "action": "accept",
  "dialog_type": "prompt",
  "message": "Enter name:",
  "text": "Alice"
}
```

**Output (plain text):**
```
Accepted alert: "Hello world"
```

**Errors:**

| Exit Code | Condition |
|-----------|-----------|
| 1 (GeneralError) | No dialog is currently open |
| 2 (ConnectionError) | Cannot connect to Chrome |
| 3 (TargetError) | Tab not found |
| 4 (TimeoutError) | Command timed out |
| 5 (ProtocolError) | CDP protocol error |

#### `dialog info`

**Output (dialog open — JSON):**
```json
{
  "open": true,
  "type": "prompt",
  "message": "Enter name:",
  "default_value": "default"
}
```

**Output (no dialog — JSON):**
```json
{
  "open": false
}
```

**Output (plain text — open):**
```
Dialog open: prompt — "Enter name:" (default: "default")
```

**Output (plain text — closed):**
```
No dialog open
```

---

## Database / Storage Changes

None. This feature is stateless — no persistent storage needed.

---

## State Management

### Dialog State (in-memory, per-command)

```rust
/// Captured from Page.javascriptDialogOpening event.
struct DialogState {
    dialog_type: String,   // "alert", "confirm", "prompt", "beforeunload"
    message: String,
    default_prompt: String, // Empty for non-prompt dialogs
    url: String,           // Page URL that opened the dialog
}
```

### State Transitions

```
Command start → Page.enable → Subscribe to dialog event
    ↓
[Event arrives] → DialogState captured
    ↓
handle: Page.handleJavaScriptDialog → Success/Error
info: Return captured state (or probe if no event)
```

---

## UI Components

N/A — this is a CLI tool, no UI components.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Event-only detection** | Rely solely on `Page.javascriptDialogOpening` events for dialog state | Simple, clean | Misses dialogs opened before subscription | Rejected — race condition |
| **B: Runtime.evaluate probe** | Use `Runtime.evaluate("0")` to detect blocking dialog | Works for already-open dialogs | Extra round-trip, timeout-based detection is slow | **Selected for fallback** |
| **C: Direct CDP handle attempt** | For `dialog handle`, just call `handleJavaScriptDialog` and let CDP error if no dialog | Simplest, no event subscription needed for handle | Less metadata in response (no dialog type/message without event) | **Selected for handle** — subscribe for metadata, but the handle call itself is the source of truth |
| **D: Separate auto-dismiss daemon** | Run auto-dismiss as a persistent background process | Works across commands | Over-engineered, out of scope | Rejected |

**Design Decision**: For `dialog handle`, we subscribe to `Page.javascriptDialogOpening` to capture dialog metadata (type, message, default_value), then call `Page.handleJavaScriptDialog`. If the CDP call fails with "no dialog open", we return an error. The event data enriches the response but isn't required for the operation itself.

For `dialog info`, we enable the Page domain, subscribe to the event, and then use `Runtime.evaluate` as a probe — if it blocks/times out, a dialog is open and we use the event data. If it completes, no dialog is open.

---

## Security Considerations

- [x] **Input Validation**: `action` is constrained to `accept`/`dismiss` via clap enum; `--text` is arbitrary user input passed as-is to CDP (no injection risk since it's a dialog prompt value)
- [x] **No sensitive data**: Dialog messages are page-controlled; we just relay them
- [x] **Local only**: All CDP communication is localhost

---

## Performance Considerations

- [x] **Minimal overhead**: Single CDP event subscription + single command call
- [x] **No polling**: Event-driven architecture, no busy-waiting
- [x] **Auto-dismiss background task**: Lightweight tokio task that only acts when events arrive
- [x] **Probe timeout**: `dialog info` probe uses a short timeout (~200ms) to avoid stalling

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Command Logic | Unit | Output struct serialization, plain text formatting |
| Dialog Detection | Integration | Probe logic with/without open dialog |
| CLI Args | Unit | Clap parsing for dialog subcommands |
| Feature | BDD (cucumber-rs) | All acceptance criteria from requirements.md |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Dialog opened before event subscription (race) | Medium | Medium | Use Runtime.evaluate probe as fallback in `dialog info` |
| Auto-dismiss interferes with `dialog handle` | Low | Medium | Auto-dismiss flag is only on GlobalOpts, not on dialog commands |
| CDP event timing varies across Chrome versions | Low | Low | Use generous timeouts, test against current Chrome stable |

---

## Open Questions

- (None)

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] State management approach is clear (in-memory, per-command)
- [x] N/A — no UI components
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
