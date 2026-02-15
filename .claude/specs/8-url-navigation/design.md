# Design: URL Navigation

**Issue**: #8
**Date**: 2026-02-11
**Status**: Draft
**Author**: Claude (spec-driven development)

---

## Overview

This feature implements the `navigate` subcommand group (`<URL>`, `back`, `forward`, `reload`) by introducing session-level CDP communication to chrome-cli. Unlike `tabs`, which uses browser-level `CdpClient::send_command()` for Target domain operations, navigation requires attaching to a specific tab target via `CdpSession` and enabling the Page and Network domains.

The key technical challenge is implementing wait strategies — configurable policies that determine when a navigation is "complete." These use CDP event subscriptions (`CdpSession::subscribe()`) to listen for `Page.loadEventFired`, `Page.domContentEventFired`, and network activity events. The network idle strategy introduces in-flight request tracking with a 500ms quiescence window.

The implementation follows the established binary-crate command module pattern from `tabs.rs`: a new `src/navigate.rs` module with command handlers, CLI arg types added to `src/cli/mod.rs`, and dispatch wired in `src/main.rs`.

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                         CLI Layer                                 │
│  cli/mod.rs: NavigateArgs, NavigateCommand enum                   │
│  (Url, Back, Forward, Reload) with per-subcommand args            │
└────────────────────────────────┬─────────────────────────────────┘
                                 │
                                 ▼
┌──────────────────────────────────────────────────────────────────┐
│                      Command Layer                                │
│  navigate.rs: execute_navigate() dispatches to                    │
│  execute_url(), execute_back(), execute_forward(),                │
│  execute_reload()                                                 │
│                                                                   │
│  Wait strategies (WaitStrategy enum):                             │
│  - wait_for_load()           → Page.loadEventFired                │
│  - wait_for_dom_content()    → Page.domContentEventFired          │
│  - wait_for_network_idle()   → Network.* events + 500ms timer     │
│  - none                      → return immediately                 │
└─────────────────┬──────────────────────┬────────────────────────┘
                  │                      │
          ┌───────▼───────┐     ┌────────▼────────┐
          │  Connection   │     │  CDP Session     │
          │  Layer        │     │  (Page + Network │
          │  resolve_     │     │   domains)       │
          │  connection() │     │  ManagedSession  │
          │  resolve_     │     │  subscribe()     │
          │  target()     │     │  send_command()  │
          └───────┬───────┘     └────────┬────────┘
                  │                      │
                  └──────────┬───────────┘
                             ▼
                   Chrome Browser (CDP)
```

### Data Flow

#### `navigate <URL>`

```
1. Parse CLI args (url, --wait-until, --timeout, --ignore-cache, --tab)
2. resolve_connection(host, port, ws_url) → ResolvedConnection
3. resolve_target(host, port, tab) → TargetInfo
4. CdpClient::connect(ws_url) → CdpClient
5. client.create_session(target_id) → CdpSession
6. ManagedSession::new(session)
7. managed.ensure_domain("Page")
8. managed.ensure_domain("Network") — only if wait=networkidle or need status
9. Subscribe to wait events BEFORE sending Page.navigate
10. session.send_command("Page.navigate", {url, ..}) → check errorText
11. If errorText present → return navigation error (DNS, SSL, etc.)
12. Wait for strategy events with timeout:
    - load: Page.loadEventFired
    - domcontentloaded: Page.domContentEventFired
    - networkidle: 0 in-flight requests for 500ms
    - none: skip waiting
13. Collect Network.responseReceived for main frame → HTTP status
14. Query page state: Runtime.evaluate("document.title") for title, use navigated URL
15. Return JSON: {url, title, status}
```

#### `navigate back`

```
1. Parse CLI args (--tab)
2. resolve_connection → resolve_target → create session
3. managed.ensure_domain("Page")
4. session.send_command("Page.getNavigationHistory") → {currentIndex, entries}
5. If currentIndex == 0 → already at start, return current page info
6. entry = entries[currentIndex - 1]
7. Subscribe to Page.loadEventFired
8. session.send_command("Page.navigateToHistoryEntry", {entryId: entry.id})
9. Wait for Page.loadEventFired with timeout
10. Return JSON: {url, title}
```

#### `navigate forward`

```
1. Same as back, but navigate to entries[currentIndex + 1]
2. If currentIndex == entries.len() - 1 → already at end, return current page info
```

#### `navigate reload`

```
1. Parse CLI args (--ignore-cache, --tab)
2. resolve_connection → resolve_target → create session
3. managed.ensure_domain("Page")
4. Subscribe to Page.loadEventFired
5. session.send_command("Page.reload", {ignoreCache})
6. Wait for Page.loadEventFired with timeout
7. Get current URL and title via Runtime.evaluate
8. Return JSON: {url, title}
```

---

## API / Interface Changes

### CLI Subcommand Structure

The `Navigate` variant in the `Command` enum changes from a unit variant to `Navigate(NavigateArgs)` with nested subcommands. Since `navigate <URL>` should also work (not just `navigate url <URL>`), the URL subcommand is the default (no subcommand keyword needed).

| Subcommand | Args | Description |
|------------|------|-------------|
| `navigate <URL>` | `--wait-until <EVENT>`, `--timeout <MS>`, `--ignore-cache`, `--tab <ID>` | Navigate to URL |
| `navigate back` | `--tab <ID>` | Go back in history |
| `navigate forward` | `--tab <ID>` | Go forward in history |
| `navigate reload` | `--ignore-cache`, `--tab <ID>` | Reload current page |

**Clap design**: Use `#[command(subcommand)]` with an enum that has `Back`, `Forward`, `Reload` variants, and a catch-all `Url(NavigateUrlArgs)` variant using `#[command(external_subcommand)]` or by making the URL a default subcommand. The simplest approach: make `NavigateCommand` an enum where `Url` is the variant for URL navigation, but use clap's default subcommand feature or restructure so `navigate <URL>` works without a subcommand keyword.

**Recommended approach**: Use `clap::Subcommand` with the URL variant having `#[command(name = "to")]` or make the URL positional on the parent `NavigateArgs` using a flattened approach. After examining clap's capabilities, the cleanest pattern is:

```rust
pub enum NavigateCommand {
    /// Navigate back in browser history
    Back,
    /// Navigate forward in browser history
    Forward,
    /// Reload the current page
    Reload(NavigateReloadArgs),
}

pub struct NavigateArgs {
    /// URL to navigate to (omit for back/forward/reload subcommands)
    pub url: Option<String>,
    /// Subcommand (back, forward, reload)
    pub command: Option<NavigateCommand>,
    // ... shared flags
}
```

However, clap doesn't natively support "optional subcommand with fallback to positional." The cleanest approach that avoids user confusion: keep all four as explicit subcommands but make the URL one named `to`:

- `chrome-cli navigate to <URL>` — but the issue says `chrome-cli navigate <URL>`

**Final approach**: Use a flat argument structure with mutual exclusion:

```rust
pub struct NavigateArgs {
    /// URL to navigate to
    pub url: Option<String>,
    /// Navigate back in browser history
    #[arg(long)]
    pub back: bool,
    /// Navigate forward in browser history
    #[arg(long)]
    pub forward: bool,
    /// Reload the current page
    #[arg(long)]
    pub reload: bool,
    // ... other flags
}
```

Wait — the issue specifies `navigate back`, `navigate forward`, `navigate reload` as subcommands (not flags). Let me reconsider.

**Adopted approach**: Use clap subcommands where `Back`, `Forward`, `Reload` are subcommand variants. For URL navigation without a subcommand keyword, we put the URL as an optional positional arg on the parent `NavigateArgs` and dispatch based on whether a subcommand or positional URL is present. Clap supports this with `#[command(args_conflicts_with_subcommands = true)]`.

```rust
#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct NavigateArgs {
    #[command(subcommand)]
    pub command: Option<NavigateCommand>,
    #[command(flatten)]
    pub url_args: Option<NavigateUrlArgs>,
}
```

This allows both `chrome-cli navigate https://example.com` and `chrome-cli navigate back`.

### Output Schemas

#### `navigate <URL>` (JSON)

```json
{
  "url": "https://example.com/",
  "title": "Example Domain",
  "status": 200
}
```

#### `navigate back` / `navigate forward` / `navigate reload` (JSON)

```json
{
  "url": "https://example.com/",
  "title": "Example Domain"
}
```

### Error Responses

All errors use the existing `AppError` JSON format: `{"error": "message", "code": N}`

| Condition | Exit Code | Message |
|-----------|-----------|---------|
| No Chrome connection | 2 | "No Chrome instance found..." |
| Tab not found | 3 | "Tab 'X' not found..." |
| DNS resolution failure | 1 | "Navigation failed: net::ERR_NAME_NOT_RESOLVED" |
| SSL error | 1 | "Navigation failed: net::ERR_CERT_..." |
| Navigation timeout | 4 | "Navigation timed out after Xms waiting for {strategy}" |
| Already at history start (back) | 0 | Returns current page info (no-op, not an error) |
| Already at history end (forward) | 0 | Returns current page info (no-op, not an error) |

---

## CDP Commands Used

| Operation | CDP Method | Domain | Level | Parameters |
|-----------|-----------|--------|-------|------------|
| Navigate to URL | `Page.navigate` | Page | Session | `url: String`, `transitionType?: String` |
| Get history | `Page.getNavigationHistory` | Page | Session | None |
| Navigate to history entry | `Page.navigateToHistoryEntry` | Page | Session | `entryId: i64` |
| Reload page | `Page.reload` | Page | Session | `ignoreCache?: bool` |
| Enable Page domain | `Page.enable` | Page | Session | None |
| Enable Network domain | `Network.enable` | Network | Session | None |
| Get page title | `Runtime.evaluate` | Runtime | Session | `expression: "document.title"` |

### CDP Events Consumed

| Event | Domain | Used By |
|-------|--------|---------|
| `Page.loadEventFired` | Page | `load` wait strategy |
| `Page.domContentEventFired` | Page | `domcontentloaded` wait strategy |
| `Network.requestWillBeSent` | Network | `networkidle` (increment in-flight count) |
| `Network.loadingFinished` | Network | `networkidle` (decrement in-flight count) |
| `Network.loadingFailed` | Network | `networkidle` (decrement in-flight count) |
| `Network.responseReceived` | Network | HTTP status extraction for main frame |

---

## Wait Strategy Implementation

### Wait for Load

```
1. Subscribe to Page.loadEventFired via session.subscribe()
2. Send Page.navigate
3. tokio::select! {
     event = rx.recv() => Ok(()),
     _ = tokio::time::sleep(timeout) => Err(timeout)
   }
```

### Wait for DOMContentLoaded

Same as load, but subscribes to `Page.domContentEventFired`.

### Wait for Network Idle

```
1. Subscribe to Network.requestWillBeSent, Network.loadingFinished, Network.loadingFailed
2. Track in_flight_count: u32 = 0
3. Send Page.navigate
4. Loop:
   tokio::select! {
     event = request_rx.recv() => { in_flight_count += 1; reset idle timer }
     event = finished_rx.recv() => { in_flight_count = in_flight_count.saturating_sub(1); if 0, start 500ms idle timer }
     event = failed_rx.recv() => { in_flight_count = in_flight_count.saturating_sub(1); if 0, start 500ms idle timer }
     _ = idle_timer (500ms) => { if in_flight_count == 0, break Ok(()) }
     _ = overall_timeout => { break Err(timeout) }
   }
```

### Wait for None

Return immediately after `Page.navigate` completes (the CDP command response itself).

---

## HTTP Status Code Extraction

To get the HTTP status code of the main document:

1. Enable the Network domain before navigation
2. Subscribe to `Network.responseReceived`
3. After `Page.navigate`, the first `Network.responseReceived` event where `type == "Document"` contains `response.status`
4. Store the status code and include it in the output

If Network events are unavailable (e.g., cached page), default to status 0 or omit the field.

---

## Module Structure

### New Files

| File | Purpose |
|------|---------|
| `src/navigate.rs` | Navigate command handlers: `execute_navigate()`, plus internal functions for URL/back/forward/reload and wait strategies |

### Modified Files

| File | Change |
|------|--------|
| `src/cli/mod.rs` | Change `Navigate` from unit variant to `Navigate(NavigateArgs)` with nested `NavigateCommand` enum, add `NavigateUrlArgs`, `NavigateReloadArgs`, `WaitUntil` enum |
| `src/main.rs` | Update match arm from `Command::Navigate => not_implemented()` to dispatch to `navigate::execute_navigate()`, add `mod navigate;` |
| `src/error.rs` | Add `AppError::navigation_failed()` and `AppError::navigation_timeout()` constructors |

### Unchanged Files

| File | Why Unchanged |
|------|---------------|
| `src/lib.rs` | Navigate handlers are CLI-specific orchestration in the binary crate |
| `src/connection.rs` | Existing `resolve_connection()`, `resolve_target()`, `ManagedSession` are reused as-is |
| `src/cdp/*` | CdpClient, CdpSession, and subscribe() APIs are sufficient |
| `src/tabs.rs` | No interaction with tab management |

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Browser-level commands only** | Use `Page.navigate` via browser-level `CdpClient` | No session needed | Cannot subscribe to per-tab events; Page domain requires session attachment | Rejected — wait strategies require session |
| **B: JavaScript-based navigation** | Use `Runtime.evaluate("location.href = ...")` | Works without Page domain | No wait strategy, no errorText for DNS/SSL, no status code | Rejected — inferior error handling |
| **C: Session-level with ManagedSession** | Attach to target, enable domains lazily, use event subscriptions | Full CDP capabilities, proper error handling, wait strategies work | More complex setup (attach + enable) | **Selected** — necessary for requirements |
| **D: Flag-based subcommands** | `navigate --back`, `navigate --forward` instead of subcommands | Simpler clap structure | Against issue spec; `navigate back` reads better | Rejected — issue specifies subcommands |
| **E: Separate wait module in lib.rs** | Put wait strategy logic in a reusable library module | Reusable by future commands (e.g., `page screenshot` may also wait) | Over-engineering for now; can extract later | Rejected — YAGNI, keep in navigate.rs for now |

---

## Security Considerations

- [x] **Input Validation**: URLs are forwarded to Chrome as-is; Chrome handles URL validation and security (blocks `javascript:`, `data:` with cross-origin restrictions, etc.)
- [x] **Local-only**: Inherits `warn_if_remote_host()` behavior from connection resolution
- [x] **No secrets**: No credentials or tokens stored or transmitted
- [x] **No arbitrary code execution**: Navigation commands do not execute user-supplied JavaScript (that's the `js` command)

---

## Performance Considerations

- [x] **Session reuse**: Each navigate command creates one CdpSession per invocation (stateless CLI design)
- [x] **Event subscription setup**: Subscribe to events BEFORE sending navigation to avoid race conditions
- [x] **Network idle efficiency**: Uses CDP events (push) rather than polling; 500ms idle window is standard (matches Puppeteer)
- [x] **Domain enabling**: ManagedSession avoids redundant `{domain}.enable` calls within a single command
- [x] **Timeout**: Configurable via `--timeout`, defaults to 30s, applies to wait strategy not CDP transport

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Unit | `#[test]` in `src/navigate.rs` | WaitUntil parsing, error message formatting, history navigation edge cases (at start/end) |
| Unit | `#[test]` in `src/error.rs` | New error constructors |
| Unit | `#[test]` in `src/cli/mod.rs` | Clap arg parsing for NavigateCommand variants |
| Integration (BDD) | `tests/features/url-navigation.feature` | All 18 acceptance criteria |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Race between subscribe and navigate | Medium | High | Always subscribe BEFORE sending Page.navigate; CDP buffers events per session |
| Network idle never stabilizes (long-polling, WebSockets) | Medium | Medium | Overall timeout prevents hang; 500ms window is pragmatic |
| `Page.navigate` errorText format changes across Chrome versions | Low | Low | Use substring matching for common errors (net::ERR_*) |
| History navigation entry IDs not stable | Low | Low | Get fresh history before each back/forward; no caching |
| `Runtime.evaluate` for title fails if page is error page | Low | Low | Default to empty string on evaluation failure |

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
