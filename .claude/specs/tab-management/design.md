# Design: Tab Management Commands

**Issue**: #7
**Date**: 2026-02-11
**Status**: Draft
**Author**: Claude (spec-driven development)

---

## Overview

This feature implements the `tabs` subcommand group (`list`, `create`, `close`, `activate`) by layering CLI argument parsing and output formatting on top of the existing CDP client and connection infrastructure. The implementation adds a new `tabs` command module to the binary crate, expands the CLI enum from a unit variant to a nested subcommand group, and uses a hybrid approach: the HTTP JSON API for listing targets (reusing `query_targets()`) and the CDP WebSocket protocol for mutating commands (`Target.createTarget`, `Target.closeTarget`, `Target.activateTarget`).

The feature follows the established patterns: `resolve_connection()` for finding Chrome, `select_target()` for tab ID/index resolution, `CdpClient::send_command()` for browser-level CDP commands, and `print_json()` for output formatting.

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                         CLI Layer                                 │
│  cli/mod.rs: TabsArgs, TabsCommand enum (List, Create, Close,    │
│              Activate) with per-subcommand args structs            │
└────────────────────────────────┬─────────────────────────────────┘
                                 │
                                 ▼
┌──────────────────────────────────────────────────────────────────┐
│                      Command Layer                                │
│  main.rs (or tabs module): execute_tabs() dispatches to           │
│  execute_list(), execute_create(), execute_close(),               │
│  execute_activate() — each resolves connection, performs           │
│  CDP operation, formats output                                    │
└─────────────────┬──────────────────────┬────────────────────────┘
                  │                      │
          ┌───────▼───────┐     ┌────────▼────────┐
          │  HTTP API     │     │  CDP WebSocket   │
          │  /json/list   │     │  CdpClient       │
          │  (tabs list)  │     │  (create/close/  │
          │               │     │   activate)      │
          └───────┬───────┘     └────────┬────────┘
                  │                      │
                  └──────────┬───────────┘
                             ▼
                   Chrome Browser (CDP)
```

### Data Flow

#### `tabs list`

```
1. Parse CLI args (--all, --plain)
2. resolve_connection(host, port, ws_url) → ResolvedConnection { host, port }
3. query_targets(host, port) → Vec<TargetInfo>   (HTTP GET /json/list)
4. Filter: exclude non-"page" targets
5. Filter: unless --all, exclude chrome:// and chrome-extension:// URLs
6. Map to output structs with index and active flag (first page target = active)
7. Format as JSON array or plain text table → stdout
```

#### `tabs create [URL]`

```
1. Parse CLI args (url, --background, --timeout)
2. resolve_connection(host, port, ws_url) → ResolvedConnection { ws_url }
3. CdpClient::connect(ws_url) → CdpClient
4. client.send_command("Target.createTarget", {url, background}) → result
5. Extract targetId from result
6. query_targets(host, port) → find new target info (url, title)
7. Format as JSON object → stdout
```

#### `tabs close <ID>...`

```
1. Parse CLI args (one or more target IDs/indices)
2. resolve_connection(host, port, ws_url) → ResolvedConnection
3. query_targets(host, port) → Vec<TargetInfo>
4. Count page targets; if closing would leave 0 pages → error
5. Resolve each target via select_target()
6. CdpClient::connect(ws_url) → CdpClient
7. For each target: client.send_command("Target.closeTarget", {targetId})
8. Re-query remaining count
9. Format as JSON object → stdout
```

#### `tabs activate <ID>`

```
1. Parse CLI args (target ID/index)
2. resolve_connection(host, port, ws_url) → ResolvedConnection
3. resolve_target(host, port, tab) → TargetInfo
4. CdpClient::connect(ws_url) → CdpClient
5. client.send_command("Target.activateTarget", {targetId})
6. Format as JSON object → stdout
```

---

## API / Interface Changes

### CLI Subcommand Structure

The `Tabs` variant in the `Command` enum changes from a unit variant to a struct variant containing nested subcommands:

| Subcommand | Args | Description |
|------------|------|-------------|
| `tabs list` | `--all` | List open tabs |
| `tabs create [URL]` | `--background`, `--timeout <MS>` | Create a new tab |
| `tabs close <TAB_ID>...` | (positional, required, multiple) | Close tab(s) |
| `tabs activate <TAB_ID>` | `--quiet` | Activate (focus) a tab |

### Output Schemas

#### `tabs list` (JSON)

```json
[
  {
    "id": "ABC123DEF456",
    "url": "https://google.com",
    "title": "Google",
    "active": true
  },
  {
    "id": "GHI789JKL012",
    "url": "https://github.com",
    "title": "GitHub",
    "active": false
  }
]
```

#### `tabs list --plain`

```
  #  ID            TITLE               URL                       ACTIVE
  0  ABC123DEF456  Google              https://google.com        *
  1  GHI789JKL012  GitHub              https://github.com
```

#### `tabs create`

```json
{
  "id": "MNO345PQR678",
  "url": "https://example.com",
  "title": "Example Domain"
}
```

#### `tabs close`

```json
{
  "closed": ["ABC123DEF456"],
  "remaining": 2
}
```

#### `tabs activate`

```json
{
  "activated": "GHI789JKL012",
  "url": "https://github.com",
  "title": "GitHub"
}
```

### Error Responses

All errors use the existing `AppError` JSON format: `{"error": "message", "code": N}`

| Condition | Exit Code | Message |
|-----------|-----------|---------|
| No Chrome connection | 2 | "No Chrome instance found..." |
| Tab not found | 3 | "Tab 'X' not found..." |
| Close last tab | 3 | "Cannot close the last tab" |
| CDP protocol error | 5 | Protocol error detail |

---

## CDP Commands Used

| Operation | CDP Method | Level | Parameters |
|-----------|-----------|-------|------------|
| List tabs | HTTP `/json/list` | N/A | None |
| Create tab | `Target.createTarget` | Browser | `url: String`, `background: bool` |
| Close tab | `Target.closeTarget` | Browser | `targetId: String` |
| Activate tab | `Target.activateTarget` | Browser | `targetId: String` |

All mutating commands use browser-level `CdpClient::send_command()` — no session attachment is required since these are `Target` domain operations.

---

## Module Structure

### New Files

| File | Purpose |
|------|---------|
| `src/tabs.rs` | Tab command handlers: `execute_tabs()`, plus internal functions for list/create/close/activate |

### Modified Files

| File | Change |
|------|--------|
| `src/cli/mod.rs` | Change `Tabs` from unit variant to `Tabs(TabsArgs)` with nested `TabsCommand` subcommand enum |
| `src/main.rs` | Update `run()` match arm from `Command::Tabs => not_implemented()` to dispatch to `tabs::execute_tabs()` |
| `src/error.rs` | Add `AppError::last_tab()` constructor for close-last-tab error |

### Unchanged Files

| File | Why Unchanged |
|------|---------------|
| `src/lib.rs` | `tabs.rs` lives in the binary crate (`main.rs`), not the library — it's command-level orchestration, not reusable infrastructure |
| `src/connection.rs` | Existing `resolve_connection()` and `select_target()` are reused as-is |
| `src/cdp/*` | CdpClient API is sufficient for browser-level Target commands |
| `src/chrome/*` | `query_targets()` is reused as-is |

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: HTTP API only** | Use `/json/new`, `/json/close`, `/json/activate` endpoints | No WebSocket needed, simpler | No `--background` support, limited error detail, Chrome may not support all endpoints | Rejected — insufficient control |
| **B: CDP WebSocket only** | Use `Target.getTargets` for list too | Consistent connection type | Requires WebSocket for read-only operation; existing `query_targets()` works well | Rejected — unnecessary complexity |
| **C: Hybrid (HTTP list + CDP mutate)** | HTTP for list, CDP WebSocket for create/close/activate | Pragmatic, reuses existing code, full control for mutations | Two connection types in one command group | **Selected** — best balance of simplicity and capability |
| **D: Library module** | Put tab logic in `src/lib.rs` as `pub mod tabs` | Reusable by other crates | Over-engineering; tab handlers are CLI-specific orchestration | Rejected — YAGNI |

---

## Security Considerations

- [x] **Input Validation**: Tab IDs are validated through existing `select_target()` — either numeric index or target ID string
- [x] **URL Validation**: URLs passed to `tabs create` are forwarded to Chrome as-is; Chrome handles URL validation
- [x] **Local-only**: Inherits `warn_if_remote_host()` behavior from connection resolution
- [x] **No secrets**: No credentials or tokens are stored or transmitted

---

## Performance Considerations

- [x] **Connection reuse**: Each command opens a fresh CDP connection (stateless CLI design — Chrome handles connection pooling)
- [x] **HTTP for reads**: `tabs list` uses lightweight HTTP instead of WebSocket
- [x] **No pagination**: Tab count is bounded by practical browser limits (hundreds, not millions)
- [x] **Timeout**: Inherits global `--timeout` for CDP command timeout

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Unit | `#[test]` in `src/tabs.rs` | URL filtering logic, plain text formatting, output serialization |
| Unit | `#[test]` in `src/error.rs` | New `last_tab()` error constructor |
| Unit | `#[test]` in `src/cli/mod.rs` | Clap argument parsing for TabsCommand |
| Integration (BDD) | `tests/features/tab-management.feature` | End-to-end acceptance criteria against real Chrome |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `Target.activateTarget` not supported in all Chrome versions | Low | Medium | Fall back to session-based `Page.bringToFront` if needed |
| Active tab detection is inaccurate | Low | Low | Document that "active" uses `/json/list` ordering (first page target) |
| Race condition when closing multiple tabs | Low | Low | Close sequentially; re-query count after all closes |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] State management: stateless CLI, no local state beyond session file
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
