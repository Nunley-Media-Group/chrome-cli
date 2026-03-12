# Tasks: Cookie Management Command Group

**Issues**: #164
**Date**: 2026-03-11
**Status**: Planning
**Author**: Claude (SDLC)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 4 | [ ] |
| Integration | 2 | [ ] |
| Testing | 3 | [ ] |
| **Total** | **11** | |

Note: No Frontend phase — agentchrome is a CLI tool with no UI layer.

---

## Phase 1: Setup

### T001: Define CLI argument structs for cookie command group

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `CookieArgs` struct with `#[command(subcommand)]` field
- [ ] `CookieCommand` enum with variants: `List(CookieListArgs)`, `Set(CookieSetArgs)`, `Delete(CookieDeleteArgs)`, `Clear`
- [ ] `CookieListArgs` with `--domain` (Option<String>) and `--all` (bool) flags
- [ ] `CookieSetArgs` with positional `name` and `value`, plus optional `--domain`, `--path` (default "/"), `--secure`, `--http-only`, `--same-site`, `--expires` flags
- [ ] `CookieDeleteArgs` with positional `name` and optional `--domain` flag
- [ ] `Command::Cookie(CookieArgs)` variant added to the `Command` enum with `long_about` and `after_long_help` examples
- [ ] `cargo check` passes

**Notes**: Follow the `DialogArgs`/`DialogCommand` pattern. Add descriptive `long_about` and `after_long_help` with usage examples. The `--same-site` flag accepts `Strict`, `Lax`, or `None` as string values.

### T002: Define output types for cookie commands

**File(s)**: `src/cookie.rs`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `CookieInfo` struct with fields: `name`, `value`, `domain`, `path`, `expires` (f64), `http_only` (bool, renamed to `httpOnly`), `secure` (bool), `same_site` (String, renamed to `sameSite`), `size` (u64) — all `Serialize`
- [ ] `SetResult` struct with `success` (bool), `name` (String), `domain` (String)
- [ ] `DeleteResult` struct with `deleted` (u64)
- [ ] `print_output()` helper function matching the standard pattern
- [ ] Plain text formatters: `print_list_plain()`, `print_set_plain()`, `print_delete_plain()`
- [ ] `cdp_config()` and `setup_session()` helper functions matching the standard pattern
- [ ] File compiles with `cargo check`

**Notes**: Follow the exact output formatting pattern from `dialog.rs`. Use `#[serde(rename = "httpOnly")]` and `#[serde(rename = "sameSite")]` for JSON field naming.

---

## Phase 2: Backend Implementation

### T003: Implement `cookie list` subcommand

**File(s)**: `src/cookie.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] `execute_list()` calls `Network.getCookies` (or `Network.getAllCookies` when `--all` is set)
- [ ] Maps CDP response cookies array to `Vec<CookieInfo>`
- [ ] Filters by `--domain` if provided (client-side substring match on domain field)
- [ ] Returns empty JSON array `[]` when no cookies exist (not an error)
- [ ] Supports `--plain` output: `name: value` per line
- [ ] Supports `--pretty` output: pretty-printed JSON
- [ ] `cargo check` passes

**Notes**: `Network.getCookies` returns cookies scoped to the current page URLs. `Network.getAllCookies` returns all cookies regardless of URL. Enable the `Network` domain via `managed.ensure_domain("Network")`.

### T004: Implement `cookie set` subcommand

**File(s)**: `src/cookie.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] `execute_set()` builds `Network.setCookie` params from CLI args
- [ ] Includes `name`, `value`, and all optional flags (`domain`, `path`, `secure`, `httpOnly`, `sameSite`, `expires`)
- [ ] Checks CDP response `success` field; returns error if `false`
- [ ] Returns `SetResult` JSON with `success`, `name`, `domain`
- [ ] Supports `--plain` output: `Set cookie: <name> (domain: <domain>)`
- [ ] `cargo check` passes

**Notes**: `Network.setCookie` returns `{ "success": true/false }`. If success is false, return an `AppError` with `ExitCode::ProtocolError`.

### T005: Implement `cookie delete` subcommand

**File(s)**: `src/cookie.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] `execute_delete()` calls `Network.deleteCookies` with `name` and optional `domain`
- [ ] Returns `DeleteResult` JSON with `deleted: 1`
- [ ] Supports `--plain` output: `Deleted 1 cookie(s)`
- [ ] `cargo check` passes

**Notes**: `Network.deleteCookies` takes `name` (required) plus optional `domain`, `path`, `url`. It returns an empty result `{}` on success.

### T006: Implement `cookie clear` subcommand

**File(s)**: `src/cookie.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] `execute_clear()` first calls `Network.getAllCookies` to count existing cookies
- [ ] Then calls `Network.clearBrowserCookies` to remove all cookies
- [ ] Returns `DeleteResult` JSON with `deleted: <count>`
- [ ] Supports `--plain` output: `Cleared <count> cookie(s)`
- [ ] `cargo check` passes

**Notes**: Count cookies before clearing so the output reports how many were removed. `Network.clearBrowserCookies` takes no params and returns `{}`.

---

## Phase 3: Integration

### T007: Register cookie module and dispatch in main.rs

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T003, T004, T005, T006
**Acceptance**:
- [ ] `mod cookie;` declaration added (alphabetically with other module declarations)
- [ ] `Command::Cookie(args) => cookie::execute_cookie(&global, args).await` match arm added
- [ ] `execute_cookie()` dispatcher matches all four `CookieCommand` variants
- [ ] `cargo build` succeeds
- [ ] Running `agentchrome cookie --help` shows the subcommand group
- [ ] Running `agentchrome cookie list --help` shows list-specific flags

**Notes**: The `execute_cookie()` function is the top-level dispatcher in `src/cookie.rs` that matches `CookieCommand` variants to their handlers.

### T008: Manual smoke test against headless Chrome

**File(s)**: None (verification only)
**Type**: Verify
**Depends**: T007
**Acceptance**:
- [ ] `cargo build` succeeds
- [ ] `./target/debug/agentchrome connect --launch --headless` connects
- [ ] `./target/debug/agentchrome navigate https://www.saucedemo.com/` succeeds
- [ ] `./target/debug/agentchrome cookie list` returns cookies from saucedemo.com
- [ ] `./target/debug/agentchrome cookie set "test_cookie" "test_value" --domain "www.saucedemo.com"` sets a cookie
- [ ] `./target/debug/agentchrome cookie list` shows the newly set cookie
- [ ] `./target/debug/agentchrome cookie delete "test_cookie" --domain "www.saucedemo.com"` removes it
- [ ] `./target/debug/agentchrome cookie list` no longer shows "test_cookie"
- [ ] `./target/debug/agentchrome cookie clear` removes all remaining cookies
- [ ] `./target/debug/agentchrome cookie list` returns `[]`
- [ ] `./target/debug/agentchrome cookie list --all` works (may return empty or browser-level cookies)
- [ ] `./target/debug/agentchrome connect disconnect` succeeds
- [ ] Orphaned Chrome processes killed

---

## Phase 4: BDD Testing

### T009: Create BDD Gherkin feature file

**File(s)**: `tests/features/cookie-management.feature`
**Type**: Create
**Depends**: T007
**Acceptance**:
- [ ] All 10 acceptance criteria from requirements.md have corresponding scenarios
- [ ] Feature uses `Background: Given agentchrome is built`
- [ ] Uses Given/When/Then format consistently
- [ ] Includes CLI argument validation scenarios (testable without Chrome)
- [ ] Valid Gherkin syntax

### T010: Implement BDD step definitions

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T009
**Acceptance**:
- [ ] `CookieWorld` struct with `binary_path`, `stdout`, `stderr`, `exit_code` fields
- [ ] Step definitions for all cookie scenarios
- [ ] CLI argument validation steps run without Chrome
- [ ] Chrome-dependent steps tagged appropriately
- [ ] `cargo test --test bdd` compiles and runs

**Notes**: Follow the `DialogWorld` pattern in `tests/bdd.rs`. CLI validation tests (missing args, invalid subcommand) can run without Chrome.

### T011: Verify no regressions

**File(s)**: None (verification only)
**Type**: Verify
**Depends**: T010
**Acceptance**:
- [ ] `cargo test --lib` passes (unit tests)
- [ ] `cargo test --test bdd` passes (BDD tests)
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy` passes with `all=deny, pedantic=warn`
- [ ] No regressions in existing command modules

---

## Dependency Graph

```
T001 (CLI args) ──┬──▶ T003 (list) ──────┐
                  ├──▶ T004 (set) ───────┤
T002 (types)  ────┤                       ├──▶ T007 (integration) ──▶ T008 (smoke test)
                  ├──▶ T005 (delete) ────┤                                │
                  └──▶ T006 (clear) ─────┘                                ▼
                                                                    T009 (gherkin) ──▶ T010 (steps) ──▶ T011 (verify)
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #164 | 2026-03-11 | Initial feature spec |

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included (BDD + smoke test)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
