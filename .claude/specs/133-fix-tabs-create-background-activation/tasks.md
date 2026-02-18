# Tasks: Fix tabs create --background not preventing tab activation (regression)

**Issue**: #133
**Date**: 2026-02-17
**Status**: Implemented
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Add HTTP `activate_target` helper in discovery.rs | [x] |
| T002 | Add CDP visibility helpers in tabs.rs | [x] |
| T003 | Update `execute_list` to use CDP visibility instead of `i == 0` | [x] |
| T004 | Update `execute_create` background path to use CDP visibility verification | [x] |
| T005 | Add regression test (Gherkin + step definitions) | [x] |
| T006 | Run smoke test against real headless Chrome | [x] |
| T007 | Verify no regressions | [x] |

---

### T001: Add HTTP `activate_target` Helper

**File(s)**: `src/chrome/discovery.rs`, `src/chrome/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [x] New `pub async fn activate_target(host: &str, port: u16, target_id: &str) -> Result<(), ChromeError>` function added
- [x] Calls `GET /json/activate/{target_id}` via existing `http_get` helper
- [x] Returns `Ok(())` on success (Chrome returns "Target activated" text — no need to parse)
- [x] Propagates `ChromeError::HttpError` on failure
- [x] Re-exported from `src/chrome/mod.rs` alongside existing `query_targets`

**Notes**: Chrome's `/json/activate/{id}` HTTP endpoint returns status 200 with body "Target activated" on success. The `http_get` helper already validates status 200. We only need to discard the body.

---

### T002: Add CDP Visibility Helpers

**File(s)**: `src/tabs.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [x] `check_target_visible(client: &CdpClient, target_id: &str) -> bool` added — creates a CDP session via `client.create_session()`, evaluates `document.visibilityState`, returns `true` if `"visible"`. Returns `false` on any error (graceful degradation).
- [x] `query_visible_target_id(ws_url, page_targets, config) -> Option<String>` added — connects a `CdpClient`, iterates page targets calling `check_target_visible`, returns the first visible target's ID. Returns `None` if all fail.
- [x] Both functions are private to `tabs.rs`

**Notes**: Uses `CdpClient::create_session()` + `CdpSession::send_command()` for session-multiplexed queries over a single browser WebSocket connection. Each page target requires one attach + one evaluate round trip (~5-10ms each).

---

### T003: Update `execute_list` to Use CDP Visibility

**File(s)**: `src/tabs.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [x] `execute_list` calls `query_visible_target_id()` after fetching targets from `/json/list`
- [x] The `active` field is set by matching target ID against the visible ID
- [x] Falls back to `i == 0` when `query_visible_target_id` returns `None`
- [x] `CdpConfig` is constructed via `cdp_config(global)` for the visibility query
- [x] Existing unit tests updated: `first_page_target_is_active` replaced with `visible_target_is_marked_active` and `fallback_to_first_when_no_visible_id`

---

### T004: Update `execute_create` Background Path

**File(s)**: `src/tabs.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [x] `original_active_id` detection uses CDP visibility (`check_target_visible`) to find the truly active tab, falling back to first page target if CDP fails
- [x] `CdpClient` is created before `original_active_id` detection (moved earlier in function)
- [x] Background re-activation uses HTTP `activate_target()` followed by 100ms settle + CDP visibility check
- [x] If original tab is not visible after settle, retries HTTP activation once + another 100ms settle
- [x] `/json/list` polling loop removed (ordering is unreliable in headless mode)
- [x] The non-background path is completely unchanged
- [x] Existing unit tests continue to pass

**Notes**: The stability verification adds ~200ms worst-case (two 100ms sleeps + two CDP checks) but only when Chrome re-activates the new tab during page load. Normal case is ~100ms + one CDP check.

---

### T005: Add Regression Test

**File(s)**: `tests/features/133-fix-tabs-create-background-activation.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T004
**Acceptance**:
- [x] Gherkin feature file created at `tests/features/133-fix-tabs-create-background-activation.feature`
- [x] All scenarios from `feature.gherkin` spec are present
- [x] All scenarios tagged `@regression`
- [x] Step definitions added to `tests/bdd.rs` (reuse existing tab management steps where possible)
- [x] `cargo test --test bdd` compiles and the new feature file parses without errors
- [x] Chrome-dependent scenarios are skipped in CI (consistent with existing BDD patterns)

---

### T006: Run Smoke Test Against Real Headless Chrome

**File(s)**: N/A (manual verification)
**Type**: Verify (no file changes)
**Depends**: T004
**Acceptance**:
- [x] Build: `cargo build`
- [x] Connect: `./target/debug/chrome-cli connect --launch --headless`
- [x] Create foreground tab: `./target/debug/chrome-cli tabs create https://www.google.com`
- [x] Verify active: `./target/debug/chrome-cli tabs list --pretty` shows google.com as `active: true`
- [x] Create background tab: `./target/debug/chrome-cli tabs create https://example.com --background`
- [x] Verify background: `./target/debug/chrome-cli tabs list --pretty` shows google.com still `active: true` and example.com as `active: false`
- [x] Non-background still works: `./target/debug/chrome-cli tabs create https://github.com` — github.com becomes active
- [x] SauceDemo smoke: `./target/debug/chrome-cli navigate https://www.saucedemo.com/` + `./target/debug/chrome-cli page snapshot` succeeds
- [x] Kill orphaned Chrome processes: `pkill -f 'chrome.*--remote-debugging' || true`

---

### T007: Verify No Regressions

**File(s)**: Existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002, T003, T004, T005
**Acceptance**:
- [x] `cargo fmt --check` passes
- [x] `cargo clippy --all-targets` passes with no new warnings
- [x] `cargo test --lib` passes (141 unit tests)
- [x] `cargo test --test bdd` passes (BDD tests)
- [x] No side effects in related code paths (`execute_activate`, `execute_close` are unchanged)

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T005)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
