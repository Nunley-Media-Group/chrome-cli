# Design: Add --page-id Global Flag for Stateless Page Routing

**Issues**: #170
**Date**: 2026-03-12
**Status**: Draft
**Author**: Claude (SDLC)

---

## Overview

This feature adds a `--page-id` global flag to the CLI that provides stateless, session-independent page targeting. The change is architecturally minimal: a new field in `GlobalOpts`, a modified `resolve_target()` signature with a priority branch, and call-site updates in all 13 command modules that use target resolution.

When `--page-id` is provided, `resolve_target()` skips the session `active_tab_id` fallback entirely and performs a direct target ID lookup against Chrome's live target list. This gives parallel agents a stable routing mechanism that avoids the shared session singleton. The existing `--tab` flag and all fallback behavior remain untouched.

The design follows existing patterns: clap derive for the flag definition, `conflicts_with` for mutual exclusivity with `--tab`, and the same `select_target()` by-ID lookup that `--tab` already uses.

---

## Architecture

### Component Diagram

```
CLI Input: --page-id <target-id>
    |
    v
┌──────────────────────────────────┐
│  CLI Layer (cli/mod.rs)          │
│  GlobalOpts { page_id: Option }  │  <-- NEW FIELD
│  conflicts_with = "tab"          │  <-- CLAP VALIDATION
└────────────┬─────────────────────┘
             |
             v
┌──────────────────────────────────┐
│  main.rs                         │
│  apply_config_defaults()         │  <-- CLONE page_id
│  run() dispatches &global        │
└────────────┬─────────────────────┘
             |
             v
┌──────────────────────────────────┐
│  Command Modules                 │
│  (navigate, page, js, form, ...) │
│  setup_session() calls           │
│  resolve_target() with page_id   │  <-- UPDATED CALL SITES
└────────────┬─────────────────────┘
             |
             v
┌──────────────────────────────────┐
│  connection.rs                   │
│  resolve_target(host, port,      │
│    tab, page_id)                 │  <-- MODIFIED SIGNATURE
│                                  │
│  Priority: page_id > tab >       │
│    session active_tab_id >       │
│    first page target             │
└────────────┬─────────────────────┘
             |
             v
┌──────────────────────────────────┐
│  Chrome (CDP /json/list)         │
│  Returns target list             │
└──────────────────────────────────┘
```

### Data Flow

```
1. User invokes: agentchrome page text --page-id ABC123
2. clap parses GlobalOpts.page_id = Some("ABC123")
   - If --tab also present: clap exits with conflict error (exit code 1)
3. main.rs clones page_id through apply_config_defaults()
4. Command module calls resolve_target(&host, port, tab=None, page_id=Some("ABC123"))
5. resolve_target() sees page_id is Some:
   a. Queries Chrome for targets via /json/list
   b. Calls select_target(&targets, Some("ABC123")) -- direct ID lookup
   c. If found: returns target (no session read)
   d. If not found: returns AppError::target_not_found("ABC123") (exit code 3)
6. Command proceeds with the resolved target -- no session write for active_tab_id
```

---

## API / Interface Changes

### CLI Flag Addition

| Flag | Type | Global | Conflicts With | Purpose |
|------|------|--------|----------------|---------|
| `--page-id <TARGET_ID>` | `Option<String>` | Yes | `--tab` | Stateless page routing by CDP target ID |

### Rust Interface Changes

#### `GlobalOpts` (src/cli/mod.rs)

```rust
// NEW field added to GlobalOpts struct
/// Explicit page target ID (bypasses session state; conflicts with --tab)
#[arg(long, global = true, conflicts_with = "tab")]
pub page_id: Option<String>,
```

#### `resolve_target()` (src/connection.rs)

**Current signature:**
```rust
pub async fn resolve_target(
    host: &str,
    port: u16,
    tab: Option<&str>,
) -> Result<TargetInfo, AppError>
```

**New signature:**
```rust
pub async fn resolve_target(
    host: &str,
    port: u16,
    tab: Option<&str>,
    page_id: Option<&str>,
) -> Result<TargetInfo, AppError>
```

**New logic (prepended to existing function body):**
```rust
// When --page-id is provided, bypass session entirely
if let Some(pid) = page_id {
    let targets = query_targets(host, port).await?;
    return select_target(&targets, Some(pid)).cloned();
}
// ... existing logic unchanged below ...
```

#### Command Module Call Sites (13 modules)

**Current pattern:**
```rust
let target = resolve_target(&conn.host, conn.port, global.tab.as_deref()).await?;
```

**New pattern:**
```rust
let target = resolve_target(&conn.host, conn.port, global.tab.as_deref(), global.page_id.as_deref()).await?;
```

**Affected modules:**
| Module | File | Function | Line |
|--------|------|----------|------|
| navigate | `src/navigate.rs` | `setup_session()` | ~93 |
| page | `src/page/mod.rs` | `setup_session()` | ~56 |
| js | `src/js.rs` | `setup_session()` | ~81 |
| form | `src/form.rs` | `execute_form()` | ~138 |
| interact | `src/interact.rs` | `setup_session()` | ~221 |
| console | `src/console.rs` | `setup_session()` | ~142 |
| network | `src/network.rs` | `setup_session()` | ~220 |
| emulate | `src/emulate.rs` | `setup_session()` | ~456 |
| perf | `src/perf.rs` | `setup_session()` | ~134 |
| dialog | `src/dialog.rs` | `setup_session()` | ~134 |
| dom | `src/dom.rs` | `setup_session()` | ~142 |
| cookie | `src/cookie.rs` | `setup_session()` | ~104 |

#### Config Merging (src/main.rs)

In `apply_config_defaults()`, add the new field alongside `tab`:
```rust
page_id: cli_global.page_id.clone(),
```

No config file support for `page_id` (intentionally stateless -- out of scope per requirements).

### Error Responses

| Condition | Exit Code | Error Message |
|-----------|-----------|---------------|
| `--page-id` and `--tab` both specified | 1 (GeneralError) | Clap conflict error (automatic) |
| `--page-id` with nonexistent ID | 3 (TargetError) | `"Tab '<id>' not found. Run 'agentchrome tabs list' to see available tabs."` |

Both error paths reuse existing infrastructure: clap's `conflicts_with` for mutual exclusivity, and `AppError::target_not_found()` for missing targets.

---

## State Management

### Session File (`~/.agentchrome/session.json`)

**No changes to `SessionData` struct.** The `--page-id` flag is intentionally stateless:

- `resolve_target()` does not read `active_tab_id` from the session when `page_id` is `Some`
- No command writes `active_tab_id` based on `--page-id`
- `tabs activate` continues to write `active_tab_id` as before (unaffected)

### Resolution Priority Chain (Updated)

```
1. --page-id <target-id>    → direct ID lookup, no session I/O     [NEW]
2. --tab <index-or-id>      → index or ID lookup, no session read  [EXISTING]
3. session active_tab_id    → ID lookup from persisted state        [EXISTING]
4. first page target        → fallback to first "page" type target  [EXISTING]
```

Steps 2-4 execute only when `--page-id` is not provided.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Extend `--tab` semantics** | Add a "stateless" mode to `--tab` (e.g., `--tab !ABC123` with a prefix) | No new flag | Overloads existing flag, confusing UX, breaks backward compatibility | Rejected -- semantic overload |
| **B: New `--page-id` flag** | Dedicated flag that bypasses session entirely | Clear intent, no backward compat risk, self-documenting | One more global flag | **Selected** |
| **C: `--target` flag** | More generic name | Slightly shorter | Confusing with CDP's `targetId` terminology ambiguity; `--page-id` explicitly says "page" | Rejected -- less specific |

---

## Security Considerations

- [x] **Input Validation**: Target ID is validated against Chrome's live target list -- no injection risk since it's matched by string equality against `TargetInfo.id`
- [x] **No new credentials or secrets**: The flag is a plain string argument
- [x] **No new file I/O**: No session reads or writes added for this code path
- [x] **Existing security posture unchanged**: All connections remain localhost-only per `tech.md`

---

## Performance Considerations

- [x] **No additional overhead**: `--page-id` code path queries `/json/list` (same as existing flow) and does one linear scan. The session file read is _skipped_, making `--page-id` marginally faster than the no-flag default.
- [x] **No caching needed**: Target lists are ephemeral per invocation
- [x] **No pagination**: Target list is bounded by Chrome's open tabs (typically < 100)

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| `select_target()` | Unit (`src/connection.rs`) | Existing tests already cover by-ID lookup; no new unit tests needed for this function |
| `resolve_target()` | Unit (`src/connection.rs`) | New test: `page_id` bypasses session; new test: `page_id` not found returns error |
| `GlobalOpts` clap validation | BDD | `--page-id` + `--tab` conflict produces exit code 1 |
| End-to-end | BDD | AC1-AC7 scenarios in `feature.gherkin` |
| Backward compatibility | Existing tests | All existing tests must pass without modification (no `page_id` means existing fallback chain) |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Missing a call site in a command module | Low | Medium -- that command wouldn't support `--page-id` | Grep for all `resolve_target` call sites; the compiler will enforce the new parameter |
| Target ID format confusion (users passing tab index to `--page-id`) | Low | Low -- will get "not found" error | Flag help text clarifies it expects a CDP target ID |
| Breaking `--tab` behavior during refactor | Low | High -- backward compatibility regression | `--tab` code path is untouched; existing tests cover it |

---

## Open Questions

(None.)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #170 | 2026-03-12 | Initial feature spec |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] State management approach is clear (stateless -- no session I/O)
- [x] No UI components needed (CLI-only feature)
- [x] Security considerations addressed
- [x] Performance impact analyzed (neutral to positive)
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
