# Design: Add Page Wait Command

**Issues**: #163
**Date**: 2026-03-12
**Status**: Draft
**Author**: Claude

---

## Overview

This feature adds a `page wait` subcommand that blocks until a user-specified condition is met on the current page. It supports four condition types: URL glob matching (`--url`), text content search (`--text`), CSS selector presence (`--selector`), and network idle detection (`--network-idle`). Exactly one condition must be specified per invocation.

The implementation follows the established page subcommand pattern: a new `src/page/wait.rs` module with a `PageWaitArgs` struct in `src/cli/mod.rs`, dispatched through `src/page/mod.rs`. For `--url`, `--text`, and `--selector`, the command polls via `Runtime.evaluate` at a configurable interval (default 100ms). For `--network-idle`, it reuses the existing event-driven `wait_for_network_idle()` infrastructure from `src/navigate.rs`. All conditions check immediately before entering the wait loop to return instantly when already satisfied.

A new external dependency on the `globset` crate is required for URL glob pattern matching. No other architectural changes are needed — the command integrates cleanly into the existing page module structure.

---

## Architecture

### Component Diagram

```
CLI Layer (src/cli/mod.rs)
    │
    │  PageWaitArgs { url, text, selector, network_idle, interval }
    │
    ▼
Page Dispatcher (src/page/mod.rs)
    │
    │  PageCommand::Wait(args) → wait::execute_wait(global, args)
    │
    ▼
Wait Module (src/page/wait.rs)  ◄── NEW
    │
    ├─── Poll Loop (--url, --text, --selector)
    │       │
    │       ▼
    │    Runtime.evaluate  ──►  Chrome (JS evaluation)
    │       │
    │       ├── --url:  glob match against location.href
    │       ├── --text: document.body.innerText.includes(text)
    │       └── --selector: document.querySelector(sel) !== null
    │
    └─── Event-Driven (--network-idle)
            │
            ▼
         wait_for_network_idle()  ◄── reused from src/navigate.rs
            │
            ├── Network.requestWillBeSent
            ├── Network.loadingFinished
            └── Network.loadingFailed
```

### Data Flow

```
1. CLI parses PageWaitArgs, validates exactly one condition flag is set
2. execute_wait() calls setup_session() to connect to Chrome
3. Enables Runtime domain (always), Network domain (if --network-idle)
4. Performs immediate condition check:
   a. If condition already met → build result JSON, print, return Ok
   b. If not met → enter wait loop
5. Wait loop:
   a. Poll conditions: sleep(interval) → Runtime.evaluate → check result
   b. Network idle: subscribe to Network events → wait_for_network_idle()
6. On match: get_page_info() → build WaitResult → print_output() → return Ok
7. On timeout: return AppError with ExitCode::TimeoutError (code 4)
```

---

## API / Interface Changes

### New CLI Subcommand

| Command | Type | Purpose |
|---------|------|---------|
| `agentchrome page wait` | Subcommand | Wait until a condition is met on the current page |

### CLI Arguments: `PageWaitArgs`

```rust
/// Wait until a condition is met on the current page
#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct PageWaitArgs {
    /// Wait for the page URL to match a glob pattern
    #[arg(long, group = "condition")]
    pub url: Option<String>,

    /// Wait for text to appear in the page content
    #[arg(long, group = "condition")]
    pub text: Option<String>,

    /// Wait for a CSS selector to match an element in the DOM
    #[arg(long, group = "condition")]
    pub selector: Option<String>,

    /// Wait for network activity to settle (no requests for 500ms)
    #[arg(long, group = "condition")]
    pub network_idle: bool,

    /// Poll interval in milliseconds (for --url, --text, --selector)
    #[arg(long, default_value = "100")]
    pub interval: u64,
}
```

The `group = "condition"` attribute with clap ensures exactly one condition is specified. The `arg_required_else_help` attribute shows help when no arguments are provided.

### Output Schema

**Success (stdout):**

```json
{
  "condition": "url",
  "matched": true,
  "url": "https://example.com/dashboard",
  "title": "Dashboard",
  "pattern": "*/dashboard*",
  "text": null,
  "selector": null
}
```

| Field | Type | Description |
|-------|------|-------------|
| `condition` | `"url"` \| `"text"` \| `"selector"` \| `"network-idle"` | Which condition was checked |
| `matched` | `true` | Always true on success |
| `url` | String | Current page URL at time of match |
| `title` | String | Current page title at time of match |
| `pattern` | String \| null | Glob pattern (for `--url`), null otherwise |
| `text` | String \| null | Search text (for `--text`), null otherwise |
| `selector` | String \| null | CSS selector (for `--selector`), null otherwise |

**Timeout Error (stderr):**

```json
{"error": "Wait timed out after 3000ms: text \"never-appearing-text\" not found", "code": 4}
```

Uses existing `AppError` serialization. A new `wait_timeout()` constructor will be added to `AppError` for wait-specific messages.

---

## New Module: `src/page/wait.rs`

### Core Types

```rust
#[derive(Serialize)]
struct WaitResult {
    condition: String,
    matched: bool,
    url: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selector: Option<String>,
}
```

Note: Optional condition-specific fields use `skip_serializing_if` to omit null fields from output, keeping responses clean. Only the field relevant to the active condition appears.

### Core Function: `execute_wait`

```
async fn execute_wait(global, args):
    session = setup_session(global)
    ensure_domain("Runtime")
    timeout_ms = global.timeout.unwrap_or(30_000)
    deadline = now() + timeout_ms

    if args.network_idle:
        return execute_network_idle_wait(session, timeout_ms, global)

    // Determine condition checker
    condition = match (args.url, args.text, args.selector):
        url  → UrlCondition(GlobMatcher::new(url))
        text → TextCondition(text)
        sel  → SelectorCondition(sel)

    // Immediate check
    if condition.check(session).await? → return success

    // Poll loop
    loop:
        sleep(interval)
        if now() > deadline → return timeout error
        if condition.check(session).await? → return success
```

### Condition Checking

Each condition is evaluated via `Runtime.evaluate`:

| Condition | JavaScript Expression | Match Criteria |
|-----------|----------------------|----------------|
| `--url` | `location.href` | Glob pattern matches the URL string |
| `--text` | `document.body.innerText.includes("...")` | Returns `true` |
| `--selector` | `document.querySelector("...") !== null` | Returns `true` |

For `--url`, the glob matching happens in Rust (not JS) using the `globset` crate against the URL returned by `location.href`. This allows proper glob semantics without injecting glob logic into the browser.

For `--text` and `--selector`, the entire check runs in JS via `Runtime.evaluate`, returning a boolean.

### Network Idle Path

The `--network-idle` path is handled separately because it's event-driven, not polled:

```
async fn execute_network_idle_wait(session, timeout_ms, global):
    ensure_domain("Network")
    req_rx  = subscribe("Network.requestWillBeSent")
    fin_rx  = subscribe("Network.loadingFinished")
    fail_rx = subscribe("Network.loadingFailed")
    wait_for_network_idle(req_rx, fin_rx, fail_rx, timeout_ms)?
    (url, title) = get_page_info(session)
    return WaitResult { condition: "network-idle", ... }
```

This directly reuses `navigate::wait_for_network_idle()` with no modifications needed. The function already handles the 500ms idle threshold and timeout.

---

## External Dependency

### `globset` Crate

| Crate | Version | Purpose |
|-------|---------|---------|
| `globset` | `0.4` | URL glob pattern matching for `--url` condition |

**Why `globset` over `glob`**: The `glob` crate is filesystem-oriented (matches paths with `/` separators). `globset` provides general-purpose glob matching that works correctly on URL strings. It's maintained by the same author as `ripgrep` (Andrew Galloway / BurntSushi) and is well-tested.

**Usage pattern:**
```rust
use globset::GlobBuilder;

let glob = GlobBuilder::new(pattern)
    .literal_separator(false)  // * matches across /
    .build()
    .map_err(|e| AppError { message: format!("Invalid glob pattern: {e}"), ... })?;
let matcher = glob.compile_matcher();

// In poll loop:
let url: String = /* from Runtime.evaluate */;
if matcher.is_match(&url) { /* condition met */ }
```

Setting `literal_separator(false)` ensures `*` matches across `/` characters in URLs, so `*/dashboard*` matches `https://example.com/dashboard`.

---

## Error Handling

### New Error Constructor

```rust
impl AppError {
    pub fn wait_timeout(timeout_ms: u64, condition: &str) -> Self {
        Self {
            message: format!("Wait timed out after {timeout_ms}ms: {condition}"),
            code: ExitCode::TimeoutError,
            custom_json: None,
        }
    }
}
```

### Error Cases

| Scenario | Error Message | Exit Code |
|----------|---------------|-----------|
| Timeout waiting for URL | `Wait timed out after 30000ms: url "*/dashboard*" not matched` | 4 |
| Timeout waiting for text | `Wait timed out after 3000ms: text "Products" not found` | 4 |
| Timeout waiting for selector | `Wait timed out after 30000ms: selector "#results-table" not found` | 4 |
| Timeout waiting for network idle | `Wait timed out after 30000ms: network-idle` | 4 |
| Invalid glob pattern | `Invalid glob pattern: ...` | 1 |
| No condition specified | Clap validation error (structured JSON via existing interceptor) | 1 |
| Connection failure | Existing connection error path | 2 |

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: JS-only evaluation** | Run all condition checks entirely in JS, including glob matching | No new Rust dependency | No standard glob in JS; would need to inject a glob implementation or use regex | Rejected — glob semantics are cleaner in Rust |
| **B: Regex for URL matching** | Use `--url` with regex instead of glob | No new dependency (regex already present) | Glob is more intuitive for URL matching; issue specifically requests glob | Rejected — per issue recommendation |
| **C: Shared wait utility module** | Extract a new `src/wait.rs` module for all wait logic | Centralizes wait infrastructure | Over-engineering for this scope; `wait_for_network_idle` is fine in navigate.rs | Rejected — unnecessary abstraction |
| **D: `globset` crate for glob** | Use `globset` for URL pattern matching in Rust | Well-maintained, general-purpose, correct semantics for URLs | New dependency | **Selected** |

---

## Security Considerations

- [x] **Input Validation**: `--url` glob pattern validated at parse time by `globset`; invalid patterns produce a clear error before any CDP interaction
- [x] **Input Validation**: `--text` and `--selector` are passed to `Runtime.evaluate` — text is embedded in a JS string literal and must be properly escaped to prevent injection. Use `serde_json::to_string()` to JSON-encode the text value before embedding in the JS expression
- [x] **Input Validation**: `--selector` is passed to `document.querySelector()` — CSS selectors are not an injection vector in this context, but malformed selectors will cause a JS exception that must be caught and reported
- [x] **No sensitive data**: The command only reads page state; it does not modify anything

---

## Performance Considerations

- [x] **Poll interval**: Default 100ms keeps CDP overhead low (10 calls/second) while providing responsive detection
- [x] **Immediate check**: Checking condition before entering the poll loop avoids a wasted 100ms sleep when the condition is already met
- [x] **Event-driven network idle**: No polling overhead for `--network-idle`; purely event-driven via CDP subscriptions
- [x] **Single JS evaluation**: Each poll iteration makes exactly one `Runtime.evaluate` call (for poll-based conditions)

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| CLI argument parsing | BDD | Validates condition group enforcement, default values, help text |
| Poll-based wait (--url, --text, --selector) | BDD | Condition match, timeout, immediate return when pre-satisfied |
| Event-driven wait (--network-idle) | BDD | Network settles, already idle, timeout |
| Error handling | BDD | Timeout messages, invalid glob, no condition specified |
| Output format | BDD | JSON structure, field presence/absence, exit codes |
| Glob matching | Unit | Pattern edge cases (wildcard, literal, empty) |

BDD scenarios will be defined in `tests/features/page-wait.feature` with step definitions in `tests/bdd.rs`.

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `Runtime.evaluate` fails during polling (page navigating) | Medium | Low | Catch JS evaluation errors in poll loop; retry on next interval rather than failing immediately |
| Glob pattern semantics surprise users (e.g., `*` vs `**`) | Low | Low | Document that `*` matches across `/` in URLs; `literal_separator(false)` ensures intuitive behavior |
| High-frequency polling causes CDP backpressure | Low | Medium | Default 100ms interval is conservative; `--interval` allows tuning |
| `--text` check misses text in iframes | Low | Low | Documented as checking `document.body.innerText` (main frame only); iframe support is out of scope |

---

## Open Questions

- [x] Should `--network-idle` reuse `wait_for_network_idle()` directly? → Yes, direct reuse with no modifications needed
- [x] Which glob crate? → `globset` for URL-appropriate matching

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #163 | 2026-03-12 | Initial technical design |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Architecture follows existing project patterns (page subcommand pattern per `structure.md`)
- [x] All API/interface changes documented with schemas (CLI args, output JSON)
- [x] Database/storage changes planned with migrations (N/A — no storage)
- [x] State management approach is clear (stateless command; poll or event-driven)
- [x] UI components and hierarchy defined (N/A — CLI only)
- [x] Security considerations addressed (JS injection prevention, input validation)
- [x] Performance impact analyzed (poll interval, immediate check, event-driven network idle)
- [x] Testing strategy defined (BDD + unit for glob)
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
