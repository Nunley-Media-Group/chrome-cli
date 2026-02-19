# Design: Console Read Runtime Messages

**Issue**: #146
**Date**: 2026-02-18
**Status**: Draft
**Author**: Claude (writing-specs)

---

## Overview

This change replaces the page-reload strategy in `execute_read()` with a CDP replay buffer drain. CDP caches `Runtime.consoleAPICalled` events and replays them when `Runtime.enable` is called on a new session. By subscribing to console events *before* enabling the Runtime domain, all cached messages (from page load and runtime interactions alike) are captured without reloading the page.

The fix is a net code reduction (~110 lines removed, ~25 lines added) confined entirely to `src/console.rs`. No CLI argument changes, no new modules, no changes to the CDP client or transport layer. The `console follow` path is untouched.

---

## Architecture

### Component Diagram

Only the `console.rs` command module changes. All other layers remain identical:

```
CLI Input (args)
    |
    v
main.rs (dispatch)
    |
    v
console.rs::execute_read()      <-- MODIFIED: replay buffer strategy
    |
    v
ManagedSession                   <-- UNCHANGED (subscribe + ensure_domain)
    |
    v
CdpClient / Transport           <-- UNCHANGED
    |
    v
Chrome Browser (CDP Runtime domain replay buffer)
```

### Data Flow -- Before (Current)

```
1. Create CDP session
2. Enable Runtime domain (ensure_domain("Runtime"))
3. Enable Page domain (ensure_domain("Page"))
4. Subscribe to Runtime.consoleAPICalled
5. Subscribe to Page.frameNavigated
6. Subscribe to Page.loadEventFired
7. Send Page.reload command
8. Collect events during reload (nav tracking, load event, idle window)
9. Navigation-aware filter (last 3 navigations or current)
10. Apply type/limit/page filters
11. Output JSON
```

### Data Flow -- After (New)

```
1. Create CDP session
2. Subscribe to Runtime.consoleAPICalled
3. Enable Runtime domain (ensure_domain("Runtime"))
   --> CDP replays all cached console events immediately
4. Drain events with idle timeout (200ms idle, 5s absolute max)
5. Apply type/limit/page filters
6. Output JSON
```

**Critical ordering**: Step 2 must come before step 3. The `subscribe()` call registers a listener with the transport layer without sending any CDP command. The `ensure_domain("Runtime")` call sends `Runtime.enable`, which triggers Chrome's replay buffer. If the subscription isn't registered first, replayed events are dropped silently.

This pattern is already established in `connection.rs:309-337` (`spawn_auto_dismiss`), which subscribes to `Page.javascriptDialogOpening` before calling `Page.enable` for the same reason.

---

## API / Interface Changes

**None.** The CLI interface, argument structure, output schema, and exit codes are all unchanged. The change is purely internal to the collection strategy.

| Aspect | Before | After |
|--------|--------|-------|
| CLI args | Same | Same |
| Output schema | Same | Same |
| Exit codes | Same | Same |
| `--include-preserved` | Navigation-aware (last 3 navs) | All replayed events (no navigation context in buffer) |

### `--include-preserved` Behavior Change

The CDP replay buffer does not tag events with navigation context. All replayed events are returned regardless of when they were generated. This means:

- Without `--include-preserved`: all replayed events are returned (the buffer doesn't distinguish navigations)
- With `--include-preserved`: same behavior (no navigation filtering possible)

This is acceptable because the replay buffer inherently includes all events from the current page's lifecycle, which aligns with the feature's goal of capturing runtime interaction messages.

---

## Detailed Changes

### File: `src/console.rs`

#### Remove

| Item | Lines | Rationale |
|------|-------|-----------|
| `RawConsoleEvent` struct | 66-69 | No longer needed -- navigation tracking removed |
| `DEFAULT_RELOAD_TIMEOUT_MS` constant | 386 | No page reload |
| `POST_LOAD_IDLE_MS` constant | 388-389 | Replaced by a simpler idle timeout |
| Page domain enable | 400-401 | No page reload needed |
| `Page.frameNavigated` subscription | 413-421 | No navigation tracking |
| `Page.loadEventFired` subscription | 423-431 | No load event tracking |
| `Page.reload` command | 433-441 | Core of the old approach -- removed entirely |
| Navigation tracking variables (`current_nav_id`, `page_loaded`) | 445-446 | No navigation tracking |
| `idle_deadline` logic | 448, 451-458, 481-489 | Replaced by simple idle timeout |
| Navigation-aware `tokio::select!` branches | 460-494 | Replaced by single-branch drain loop |
| Navigation-aware filtering | 496-510 | No navigation context in replay buffer |

#### Add

| Item | Rationale |
|------|-----------|
| `DEFAULT_DRAIN_TIMEOUT_MS` constant (5000) | Absolute max timeout for replay buffer drain |
| `IDLE_DRAIN_MS` constant (200) | Idle timeout -- if no event arrives within 200ms, drain is complete |
| Subscribe to `Runtime.consoleAPICalled` **before** `ensure_domain("Runtime")` | Critical ordering for replay buffer capture |
| Simple drain loop: `tokio::select!` with console event receive + idle timeout | Collects all replayed events until 200ms of silence or 5s absolute max |

#### Preserve (Unchanged)

| Item | Rationale |
|------|-----------|
| `setup_session()` | Session creation is identical |
| `parse_console_event()` / `parse_console_event_detail()` | Event parsing is identical |
| Type filtering (`resolve_type_filter`, `filter_by_type`) | Applied post-collection as before |
| Pagination (`paginate`) | Applied post-filtering as before |
| Detail mode (MSG_ID lookup) | Operates on collected events as before |
| Plain text and JSON output | Output formatting unchanged |
| All unit tests for helpers | Helper functions are unchanged |

### Pseudocode -- New `execute_read()`

```rust
async fn execute_read(global, args) {
    let (client, mut managed) = setup_session(global).await?;
    let total_timeout = global.timeout.unwrap_or(DEFAULT_DRAIN_TIMEOUT_MS);

    // CRITICAL: Subscribe BEFORE enabling Runtime domain
    let mut console_rx = managed.subscribe("Runtime.consoleAPICalled").await?;
    managed.ensure_domain("Runtime").await?;

    // Drain replay buffer with idle timeout
    let mut events = Vec::new();
    let absolute_deadline = now() + total_timeout;
    loop {
        let remaining = absolute_deadline - now();
        if remaining.is_zero() { break; }
        let idle = min(IDLE_DRAIN_MS, remaining);
        select! {
            event = console_rx.recv() => {
                events.push(event.params);
            }
            () = sleep(idle) => break,
        }
    }

    // Detail mode, list mode, filtering, pagination, output -- unchanged
    if let Some(msg_id) = args.msg_id { ... }
    let messages = events.iter().enumerate()
        .filter_map(|(i, e)| parse_console_event(e, i)).collect();
    let messages = apply_type_filter(messages, args);
    let messages = paginate(messages, args.limit, args.page);
    print_output(&messages, &global.output)
}
```

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Page reload (current)** | Reload page to regenerate console events | Works for page-load messages | Destroys runtime state, loses interaction messages, multi-second latency | Rejected -- cannot capture runtime messages |
| **B: CDP replay buffer (proposed)** | Subscribe before `Runtime.enable`, drain replayed events | Captures all messages (page-load + runtime), preserves page state, ~200ms, net code reduction | `--include-preserved` loses navigation-aware filtering | **Selected** |
| **C: Persistent background listener** | Long-running daemon that captures all events | Most complete solution | Massive complexity, out of scope, overkill for CLI tool | Rejected -- over-engineering |
| **D: `Console.messageAdded` domain** | Use deprecated Console domain for historical messages | Alternative data source | Deprecated, may be removed from Chrome, less detail than Runtime events | Rejected -- deprecated API |

---

## Security Considerations

- [x] **No new inputs**: No new CLI arguments or user-controlled data paths
- [x] **No new network surface**: Same CDP localhost connection as before
- [x] **No secrets**: No credentials or tokens involved

---

## Performance Considerations

- [x] **Faster**: ~200ms idle drain vs multi-second page reload cycle
- [x] **Less CDP traffic**: No Page domain enable, no reload command, no navigation/load events
- [x] **No page reload**: Avoids re-executing page scripts, re-fetching resources

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Unit | `#[test]` in `console.rs` | Helper functions (unchanged): `parse_console_event`, `filter_by_type`, `paginate`, `format_console_args`, `timestamp_to_iso` |
| BDD | Gherkin + cucumber-rs | All 7 acceptance criteria from requirements.md |
| Smoke | Manual against headless Chrome | Cross-invocation `js exec` + `console read`, page state preservation, SauceDemo baseline |

### BDD Test Approach

Since BDD tests run without a real Chrome instance, the Gherkin scenarios will validate:
- CLI argument parsing and output format expectations
- Filter and pagination behavior on collected messages
- The subscribe-before-enable ordering (via scenario structure, not Chrome)

The smoke test during `/verifying-specs` validates actual CDP behavior against a real Chrome instance.

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| CDP replay buffer behavior differs across Chrome versions | Low | High | Validated against Chrome 144; this is documented CDP behavior for `Runtime.enable` |
| `--include-preserved` loses navigation-aware filtering | Low | Low | The flag still works (returns all events); navigation context wasn't reliable anyway since the replay buffer doesn't tag navigations |
| Subscribe-before-enable ordering is accidentally reversed in future edits | Low | High | Add code comment explaining the critical ordering requirement; BDD test validates message capture |

---

## Open Questions

- (none -- approach validated per issue #146 comments)

---

## Validation Checklist

- [x] Architecture follows existing project patterns (subscribe-before-enable pattern from `spawn_auto_dismiss`)
- [x] No API/interface changes
- [x] No database/storage changes
- [x] No state management changes beyond the collection strategy
- [x] No UI changes (CLI tool)
- [x] Security considerations addressed (no new surface)
- [x] Performance impact analyzed (improvement)
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
