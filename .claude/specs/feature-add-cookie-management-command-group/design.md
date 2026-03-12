# Design: Cookie Management Command Group

**Issues**: #164
**Date**: 2026-03-11
**Status**: Draft
**Author**: Claude (SDLC)

---

## Overview

This feature adds a new `cookie` command group to agentchrome, exposing Chrome DevTools Protocol cookie management capabilities through four subcommands: `list`, `set`, `delete`, and `clear`. The implementation follows the established command module pattern used by `dialog.rs`, `dom.rs`, and `network.rs`.

The design is straightforward: a new `src/cookie.rs` module receives CLI arguments parsed by clap, establishes a CDP session via the existing `ManagedSession` infrastructure, enables the `Network` domain (reusing the `ensure_domain` pattern from `network.rs`), and calls `Network.getCookies`, `Network.setCookie`, and `Network.deleteCookies`. No new CDP infrastructure or domain enablement patterns are needed.

All four subcommands follow the standard request/response model (no streaming or event subscription), making this a clean addition that requires no architectural changes.

---

## Architecture

### Component Diagram

```
CLI Input (args)
    |
+--------------------+
|   cli/mod.rs       |  CookieArgs, CookieCommand enum, subcommand arg structs
+--------+-----------+
         |
+--------+-----------+
|   main.rs          |  Command::Cookie(args) => cookie::execute_cookie(&global, args)
+--------+-----------+
         |
+--------+-----------+
|   cookie.rs        |  execute_cookie() dispatcher -> execute_list/set/delete/clear
+--------+-----------+
         |
+--------+-----------+
|   ManagedSession   |  ensure_domain("Network"), send_command("Network.*")
+--------+-----------+
         |
+--------+-----------+
|   CDP Client       |  WebSocket JSON-RPC to Chrome
+--------+-----------+
         |
    Chrome Browser
```

### Data Flow

1. User runs `agentchrome cookie <subcommand> [args] [flags]`
2. clap parses into `CookieArgs` containing a `CookieCommand` variant
3. `main.rs` dispatches to `cookie::execute_cookie()`
4. `execute_cookie()` matches the subcommand and calls the specific handler
5. Handler creates a CDP session via `setup_session()` (standard pattern)
6. Handler enables `Network` domain via `managed.ensure_domain("Network")`
7. Handler sends the appropriate CDP command (`Network.getCookies`, etc.)
8. Handler maps the CDP response to output structs and prints JSON (or plain text)

---

## API / Interface Changes

### New CLI Commands

| Command | Type | Purpose |
|---------|------|---------|
| `agentchrome cookie list [--domain DOMAIN] [--all]` | Query | List cookies for current page or all cookies |
| `agentchrome cookie set <name> <value> [--domain DOMAIN] [--path PATH] [--secure] [--http-only] [--same-site POLICY] [--expires TIMESTAMP]` | Mutation | Set a browser cookie |
| `agentchrome cookie delete <name> [--domain DOMAIN]` | Mutation | Delete a specific cookie |
| `agentchrome cookie clear` | Mutation | Delete all cookies |

### CDP Methods Used

| CDP Method | Subcommand | Direction |
|------------|------------|-----------|
| `Network.getCookies` | `cookie list` (default) | Request → Response |
| `Network.getAllCookies` | `cookie list --all` | Request → Response |
| `Network.setCookie` | `cookie set` | Request → Response |
| `Network.deleteCookies` | `cookie delete`, `cookie clear` | Request → Response |

### Request / Response Schemas

#### `cookie list`

**CDP Request** (`Network.getCookies`):
```json
{}
```
With `--all` flag, uses `Network.getAllCookies` instead (no params).

**CDP Response** (both methods):
```json
{
  "cookies": [
    {
      "name": "session_id",
      "value": "abc123",
      "domain": ".example.com",
      "path": "/",
      "expires": 1735689600.0,
      "size": 22,
      "httpOnly": true,
      "secure": true,
      "session": false,
      "sameSite": "Lax"
    }
  ]
}
```

**CLI Output** (mapped to output struct):
```json
[
  {
    "name": "session_id",
    "value": "abc123",
    "domain": ".example.com",
    "path": "/",
    "expires": 1735689600.0,
    "httpOnly": true,
    "secure": true,
    "sameSite": "Lax",
    "size": 22
  }
]
```

#### `cookie set`

**CDP Request** (`Network.setCookie`):
```json
{
  "name": "session_id",
  "value": "abc123",
  "domain": "example.com",
  "path": "/",
  "secure": true,
  "httpOnly": true,
  "sameSite": "Strict",
  "expires": 1735689600
}
```

**CDP Response**:
```json
{
  "success": true
}
```

**CLI Output**:
```json
{
  "success": true,
  "name": "session_id",
  "domain": "example.com"
}
```

**Errors**:

| Code | Condition |
|------|-----------|
| Exit 5 (ProtocolError) | CDP rejects the cookie (e.g., invalid domain) |
| Exit 2 (ConnectionError) | No active Chrome session |

#### `cookie delete`

**CDP Request** (`Network.deleteCookies`):
```json
{
  "name": "session_id",
  "domain": "example.com"
}
```

**CDP Response**: empty result `{}`

**CLI Output**:
```json
{
  "deleted": 1
}
```

#### `cookie clear`

Two-step process:
1. Call `Network.getAllCookies` to get the full cookie list
2. For each cookie, call `Network.deleteCookies` with the cookie's name and domain

**CLI Output**:
```json
{
  "deleted": 5
}
```

---

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `src/cookie.rs` | Cookie command module — output types, session setup, subcommand handlers |
| `tests/features/cookie-management.feature` | BDD Gherkin feature file |

### Modified Files

| File | Change | Rationale |
|------|--------|-----------|
| `src/cli/mod.rs` | Add `CookieArgs`, `CookieCommand`, `CookieListArgs`, `CookieSetArgs`, `CookieDeleteArgs` structs and `Command::Cookie` variant | CLI argument definitions |
| `src/main.rs` | Add `mod cookie;` declaration and `Command::Cookie(args) => cookie::execute_cookie(&global, args).await` match arm | Command dispatch |
| `tests/bdd.rs` | Add step definitions for cookie scenarios | BDD test support |

---

## Detailed Module Design

### `src/cli/mod.rs` — CLI Argument Structs

```rust
#[derive(Args)]
pub struct CookieArgs {
    #[command(subcommand)]
    pub command: CookieCommand,
}

#[derive(Subcommand)]
pub enum CookieCommand {
    /// List cookies for the current page or all cookies
    List(CookieListArgs),
    /// Set a browser cookie
    Set(CookieSetArgs),
    /// Delete a specific cookie by name
    Delete(CookieDeleteArgs),
    /// Clear all cookies
    Clear,
}

#[derive(Args)]
pub struct CookieListArgs {
    /// Filter cookies by domain
    #[arg(long)]
    pub domain: Option<String>,
    /// List all cookies (not scoped to current URL)
    #[arg(long)]
    pub all: bool,
}

#[derive(Args)]
pub struct CookieSetArgs {
    /// Cookie name
    pub name: String,
    /// Cookie value
    pub value: String,
    /// Cookie domain (strongly recommended)
    #[arg(long)]
    pub domain: Option<String>,
    /// Cookie path
    #[arg(long, default_value = "/")]
    pub path: String,
    /// Set cookie as Secure (HTTPS only)
    #[arg(long)]
    pub secure: bool,
    /// Set cookie as HttpOnly (not accessible via JavaScript)
    #[arg(long)]
    pub http_only: bool,
    /// SameSite attribute: Strict, Lax, or None
    #[arg(long, value_name = "POLICY")]
    pub same_site: Option<String>,
    /// Expiry as Unix timestamp (seconds since epoch)
    #[arg(long)]
    pub expires: Option<f64>,
}

#[derive(Args)]
pub struct CookieDeleteArgs {
    /// Cookie name to delete
    pub name: String,
    /// Scope deletion to a specific domain
    #[arg(long)]
    pub domain: Option<String>,
}
```

### `src/cookie.rs` — Module Structure

```
cookie.rs
├── Output types (CookieInfo, SetResult, DeleteResult)
├── print_output() — standard JSON output helper
├── print_*_plain() — plain-text formatters per subcommand
├── cdp_config() — timeout config helper
├── setup_session() — standard session setup (resolve_connection, create_session, ManagedSession)
├── execute_cookie() — top-level dispatcher
├── execute_list() — Network.getCookies / Network.getAllCookies, filter by domain
├── execute_set() — Network.setCookie with optional flags
├── execute_delete() — Network.deleteCookies by name + optional domain
└── execute_clear() — Network.getAllCookies + Network.deleteCookies for each
```

### Output Types

```rust
#[derive(Serialize)]
struct CookieInfo {
    name: String,
    value: String,
    domain: String,
    path: String,
    expires: f64,
    #[serde(rename = "httpOnly")]
    http_only: bool,
    secure: bool,
    #[serde(rename = "sameSite")]
    same_site: String,
    size: u64,
}

#[derive(Serialize)]
struct SetResult {
    success: bool,
    name: String,
    domain: String,
}

#[derive(Serialize)]
struct DeleteResult {
    deleted: u64,
}
```

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Storage domain** | Use `Storage.getCookies` / `Storage.setCookies` / `Storage.clearCookies` | Dedicated cookie domain | Less widely implemented, `Storage.getCookies` requires `browserContextId` parameter | Rejected — Network domain is already enabled and methods are simpler |
| **B: Network domain** | Use `Network.getCookies` / `Network.setCookie` / `Network.deleteCookies` | Already enabled in agentchrome, well-documented, straightforward params | Shared domain with network monitoring | **Selected** — minimal new infrastructure needed |
| **C: Batch delete for clear** | Call `Network.deleteCookies` once per cookie during `clear` | Correct deletion with domain scoping | Multiple CDP calls for many cookies | **Selected** — `Network.deleteCookies` requires name+domain, so per-cookie calls are necessary |
| **D: Single `clearBrowserCookies`** | Use `Network.clearBrowserCookies` for the `clear` subcommand | Single CDP call | Clears ALL cookies across ALL domains/profiles, too aggressive | Rejected — `cookie clear` should clear cookies for the current browsing context, not the entire browser |

**Note on Alternative D reconsideration**: Actually, `Network.clearBrowserCookies` is the simplest approach for `cookie clear` and aligns with the AC ("all cookies are removed"). Since agentchrome operates on a single browser instance, clearing all cookies is the expected behavior. We will use `Network.clearBrowserCookies` for `clear` and report the count by first calling `Network.getAllCookies` to count before clearing.

---

## Security Considerations

- [x] **Authentication**: N/A — local CDP connection, no external auth
- [x] **Authorization**: N/A — user has full control of the browser instance
- [x] **Input Validation**: Cookie name must be non-empty; `--same-site` value validated by clap (if using `ValueEnum`) or by CDP (which returns an error for invalid values)
- [x] **Data Sanitization**: Cookie values are passed through as-is to CDP; no escaping needed as CDP handles serialization
- [x] **Sensitive Data**: Cookie values (including auth tokens) are output in plaintext — this is intentional and consistent with the tool's purpose as a browser automation CLI

---

## Performance Considerations

- [x] **No caching**: Each command invocation creates a fresh CDP session and queries cookies directly — consistent with all other agentchrome commands
- [x] **Batch clear**: `cookie clear` calls `Network.clearBrowserCookies` (single CDP call) after counting via `Network.getAllCookies` — two calls total, no per-cookie iteration needed
- [x] **No pagination**: Cookie lists are typically small (dozens, not thousands); no pagination needed
- [x] **Timeout**: Standard `global.timeout` applies via `CdpConfig`

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| CLI parsing | Unit (BDD) | Argument parsing for all subcommands and flag combinations |
| Command execution | BDD Integration | All 10 acceptance criteria as Gherkin scenarios |
| CDP interaction | Integration | Cookie list, set, delete, clear against real Chrome |
| Output format | BDD | JSON and plain text output format verification |
| Error handling | BDD | Connection error, invalid cookie params |

### BDD Test Approach

Following the existing pattern in `tests/bdd.rs`:
- Add a `CookieWorld` struct (or extend existing world) with step definitions
- Feature file at `tests/features/cookie-management.feature`
- Steps use `Command::new(binary_path).args(...)` to invoke the CLI
- Chrome-dependent scenarios tagged `@chrome` (skipped in CI without Chrome)

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `Network.getAllCookies` deprecated in newer Chrome versions | Low | Medium | Fall back to `Network.getCookies` without URL scoping; monitor Chrome release notes |
| `cookie clear` on a profile with many cookies is slow | Low | Low | `Network.clearBrowserCookies` is a single CDP call, fast regardless of cookie count |
| `--same-site` values differ between CDP versions | Low | Low | Pass value directly to CDP, let Chrome validate; document accepted values |

---

## Open Questions

- None

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #164 | 2026-03-11 | Initial feature spec |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] N/A — No database/storage changes
- [x] N/A — No state management (stateless request/response)
- [x] N/A — No UI components (CLI tool)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
