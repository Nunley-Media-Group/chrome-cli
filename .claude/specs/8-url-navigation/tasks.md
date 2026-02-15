# Tasks: URL Navigation

**Issue**: #8
**Date**: 2026-02-11
**Status**: Planning
**Author**: Claude (spec-driven development)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 4 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **9** | |

---

## Phase 1: Setup

### T001: Define CLI subcommand types for `navigate`

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `Navigate` variant changes from unit variant to `Navigate(NavigateArgs)`
- [ ] `NavigateArgs` struct uses `#[command(args_conflicts_with_subcommands = true)]`
- [ ] `NavigateArgs` has optional `#[command(subcommand)] command: Option<NavigateCommand>`
- [ ] `NavigateArgs` has `#[command(flatten)] url_args: NavigateUrlArgs` for the URL path
- [ ] `NavigateUrlArgs` has positional `url: Option<String>`, `--wait-until` (WaitUntil enum, default load), `--timeout <MS>` (Option<u64>), `--ignore-cache` bool flag
- [ ] `NavigateCommand` enum has `Back`, `Forward`, `Reload(NavigateReloadArgs)` variants
- [ ] `NavigateReloadArgs` has `--ignore-cache` bool flag
- [ ] `WaitUntil` enum has `Load`, `Domcontentloaded`, `Networkidle`, `None` variants with `#[derive(Clone, Copy, ValueEnum)]`
- [ ] Note: `--tab` is already a global option on `GlobalOpts`, no need to add per-subcommand
- [ ] `cargo check` passes with no errors
- [ ] `chrome-cli navigate --help` shows URL arg and subcommands

**Notes**: Use clap `args_conflicts_with_subcommands` so that `navigate <URL>` works without a subcommand keyword, while `navigate back/forward/reload` are subcommands. The global `--tab` and `--timeout` flags already exist; `--timeout` on `NavigateUrlArgs` is a navigate-specific override (for the wait strategy timeout, distinct from the global CDP command timeout). If the user provides `--timeout` on the navigate URL args, use it as the navigation wait timeout; otherwise default to 30000ms.

### T002: Add error constructors for navigation failures

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AppError::navigation_failed(error_text: &str)` constructor added, returns `AppError { message: format!("Navigation failed: {error_text}"), code: ExitCode::GeneralError }`
- [ ] `AppError::navigation_timeout(timeout_ms: u64, strategy: &str)` constructor added, returns `AppError { message: format!("Navigation timed out after {timeout_ms}ms waiting for {strategy}"), code: ExitCode::TimeoutError }`
- [ ] Unit tests added for both constructors verifying message content and exit code
- [ ] `cargo test --lib` passes

---

## Phase 2: Backend Implementation

### T003: Implement `execute_url` handler (URL navigation with wait strategies)

**File(s)**: `src/navigate.rs` (create new file)
**Type**: Create
**Depends**: T001, T002
**Acceptance**:
- [ ] Module has `pub async fn execute_navigate(global: &GlobalOpts, args: &NavigateArgs) -> Result<(), AppError>` that dispatches based on subcommand vs positional URL
- [ ] If `args.command` is `Some(NavigateCommand::Back)` → call `execute_back()`
- [ ] If `args.command` is `Some(NavigateCommand::Forward)` → call `execute_forward()`
- [ ] If `args.command` is `Some(NavigateCommand::Reload(..))` → call `execute_reload()`
- [ ] If `args.url_args.url` is `Some(url)` → call `execute_url()`
- [ ] If neither subcommand nor URL → return error "No URL or subcommand provided"
- [ ] `execute_url()` resolves connection via `resolve_connection()`
- [ ] Resolves target tab via `resolve_target()` (uses global `--tab`)
- [ ] Connects via `CdpClient::connect()` and `client.create_session(target_id)`
- [ ] Wraps session in `ManagedSession::new()`
- [ ] Enables Page domain via `managed.ensure_domain("Page")`
- [ ] Enables Network domain via `managed.ensure_domain("Network")`
- [ ] Subscribes to wait strategy events BEFORE sending `Page.navigate`
- [ ] Subscribes to `Network.responseReceived` for HTTP status extraction
- [ ] Sends `Page.navigate` with `{url}` params (and `transitionType: "typed"` if `--ignore-cache` is set, or uses separate cache-bypass approach)
- [ ] Checks `Page.navigate` response for `errorText` field — if present, returns `AppError::navigation_failed(errorText)`
- [ ] Waits for the selected strategy event(s) with timeout
- [ ] Extracts HTTP status from `Network.responseReceived` where `type == "Document"` (first matching event)
- [ ] Gets page title via `Runtime.evaluate("document.title")`
- [ ] Gets final URL via `Runtime.evaluate("location.href")`
- [ ] Outputs JSON: `{url, title, status}` via `print_output()`
- [ ] `cargo check` passes

**Notes**: The `--ignore-cache` flag for URL navigation can be implemented by enabling Network domain and calling `Network.setCacheDisabled(cacheDisabled: true)` before navigating, then restoring afterward. Alternatively, use the `Page.navigate` approach and note that `transitionType` doesn't disable cache — `Network.setCacheDisabled` is the correct CDP method.

### T004: Implement wait strategy functions

**File(s)**: `src/navigate.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `wait_for_load(session, timeout)` subscribes to `Page.loadEventFired`, waits with `tokio::select!` between event and timeout
- [ ] `wait_for_dom_content(session, timeout)` subscribes to `Page.domContentEventFired`, same pattern
- [ ] `wait_for_network_idle(session, timeout)` subscribes to `Network.requestWillBeSent`, `Network.loadingFinished`, `Network.loadingFailed`
- [ ] Network idle tracks `in_flight_count: u32`, increments on `requestWillBeSent`, decrements (saturating) on `loadingFinished`/`loadingFailed`
- [ ] Network idle uses a 500ms idle timer: when `in_flight_count` reaches 0, starts 500ms countdown; if new request arrives, resets timer
- [ ] Network idle has overall timeout from `--timeout`
- [ ] `wait_none()` is a no-op (returns immediately)
- [ ] All wait functions return `Result<(), AppError>` — `Ok(())` on success, `Err(navigation_timeout)` on timeout
- [ ] `cargo check` passes

**Notes**: The wait functions should accept the `CdpSession` (not `ManagedSession`) reference for subscribing, since `ManagedSession` delegates to the inner session. Actually, since `ManagedSession` doesn't expose `subscribe()`, the wait functions should be set up before wrapping in `ManagedSession`, or we should pass the session's subscribe capability. The simplest approach: subscribe via the session before wrapping in `ManagedSession`, or subscribe after domain enabling through the inner session. Consider adding a `subscribe()` method to `ManagedSession` that delegates to the inner session.

**Updated approach**: Add a `subscribe(&self, method: &str)` method to `ManagedSession` in `src/connection.rs` that delegates to `self.session.subscribe(method)`. This keeps the pattern consistent and allows navigate.rs to only interact with `ManagedSession`.

### T005: Implement `execute_back` and `execute_forward` handlers

**File(s)**: `src/navigate.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_back()` resolves connection, target, creates session + ManagedSession
- [ ] Enables Page domain
- [ ] Sends `Page.getNavigationHistory` → extracts `currentIndex` and `entries`
- [ ] If `currentIndex == 0` → returns current page info (not an error, just a no-op)
- [ ] Otherwise, gets `entries[currentIndex - 1]` → `entryId`
- [ ] Subscribes to `Page.loadEventFired`
- [ ] Sends `Page.navigateToHistoryEntry` with `{entryId}`
- [ ] Waits for `Page.loadEventFired` with timeout
- [ ] Gets final URL and title (from history entry or `Runtime.evaluate`)
- [ ] Outputs JSON: `{url, title}`
- [ ] `execute_forward()` follows same pattern but navigates to `entries[currentIndex + 1]`
- [ ] If `currentIndex == entries.len() - 1` → returns current page info (no-op)
- [ ] `cargo check` passes

### T006: Implement `execute_reload` handler

**File(s)**: `src/navigate.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `execute_reload()` resolves connection, target, creates session + ManagedSession
- [ ] Enables Page domain
- [ ] Subscribes to `Page.loadEventFired`
- [ ] Sends `Page.reload` with `{ignoreCache: true}` if `--ignore-cache` flag is set, else `{}`
- [ ] Waits for `Page.loadEventFired` with timeout
- [ ] Gets current URL via `Runtime.evaluate("location.href")`
- [ ] Gets current title via `Runtime.evaluate("document.title")`
- [ ] Outputs JSON: `{url, title}`
- [ ] `cargo check` passes

---

## Phase 3: Integration

### T007: Wire navigate command dispatch in main.rs and expose subscribe on ManagedSession

**File(s)**: `src/main.rs`, `src/connection.rs`
**Type**: Modify
**Depends**: T003, T004, T005, T006
**Acceptance**:
- [ ] `mod navigate;` declaration added to `main.rs`
- [ ] `run()` match arm updated: `Command::Navigate(args) => navigate::execute_navigate(&cli.global, args).await`
- [ ] `ManagedSession` in `src/connection.rs` gets a new `subscribe(&self, method: &str)` method that delegates to `self.session.subscribe(method)`
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo clippy` passes with project's lint settings (all=deny, pedantic=warn)
- [ ] Running `chrome-cli navigate --help` shows URL positional arg and back/forward/reload subcommands
- [ ] Running `chrome-cli navigate back --help` works
- [ ] Running `chrome-cli navigate reload --help` shows --ignore-cache flag

---

## Phase 4: Testing

### T008: Create BDD feature file for URL navigation

**File(s)**: `tests/features/url-navigation.feature`
**Type**: Create
**Depends**: T007
**Acceptance**:
- [ ] Feature file covers all 18 acceptance criteria from requirements.md
- [ ] Uses Background for common Chrome-running precondition
- [ ] Scenarios are independent and self-contained
- [ ] Valid Gherkin syntax
- [ ] Scenario names match AC names from requirements

### T009: Add unit tests for navigate command logic

**File(s)**: `src/navigate.rs`, `src/error.rs`, `src/connection.rs`
**Type**: Modify
**Depends**: T003, T004, T005, T006, T007
**Acceptance**:
- [ ] Test: `WaitUntil` enum default is `Load`
- [ ] Test: navigate error constructors (navigation_failed, navigation_timeout)
- [ ] Test: history back at index 0 is a no-op
- [ ] Test: history forward at last entry is a no-op
- [ ] Test: `Page.navigate` errorText detection
- [ ] Test: output struct serialization (NavigateResult, HistoryResult)
- [ ] Test: ManagedSession.subscribe() returns a receiver (mock CDP test, similar to existing managed_session_enables_domain_once test)
- [ ] All tests pass with `cargo test`

---

## Dependency Graph

```
T001 ──┬──▶ T003 ──┬──▶ T004
       │           ├──▶ T005
T002 ──┘           ├──▶ T006
                   │
                   └──┬──▶ T007 ──▶ T008
                      │
                      └──▶ T009
```

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure
- [x] Test tasks are included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
