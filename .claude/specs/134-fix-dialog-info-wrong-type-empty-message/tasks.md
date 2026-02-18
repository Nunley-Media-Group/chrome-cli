# Tasks: Fix dialog info returning wrong type and empty message

**Issue**: #134
**Date**: 2026-02-17
**Status**: Revised (v2)
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Add cookie-based dialog interceptor infrastructure | [x] |
| T002 | Update dialog info to read cookie metadata | [x] |
| T003 | Add navigation-based fallback for dialog handle | [x] |
| T004 | Install interceptors in common commands | [x] |
| T005 | Add regression test | [ ] |
| T006 | Verify no regressions | [ ] |
| T007 | Manual smoke test | [x] |

---

### T001: Add Cookie-Based Dialog Interceptor Infrastructure

**File(s)**: `src/connection.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [x] `ManagedSession` gains an `install_dialog_interceptors()` async method
- [x] Method injects a JS script via `Runtime.evaluate` that overrides `window.alert`, `window.confirm`, `window.prompt` to store `{type, message, defaultValue}` in a cookie named `__chrome_cli_dialog` before calling the original
- [x] Cookie set is wrapped in `try/catch` — handles `data:` URLs (where cookies are disabled) without breaking the original dialog function
- [x] Cookie is URL-encoded JSON, path=/, max-age=300
- [x] Method also calls `Page.addScriptToEvaluateOnNewDocument` to persist across navigations
- [x] Method is best-effort — errors are silently ignored
- [x] `cargo clippy` passes

### T002: Update Dialog Info to Read Cookie Metadata

**File(s)**: `src/dialog.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [x] Add `read_dialog_cookie()` async fn that sends `Network.getCookies` and parses `__chrome_cli_dialog`
- [x] Returns `(type, message, default_value)` tuple or fallback `("unknown", "", "")`
- [x] Add `probe_dialog_open()` helper — shared dialog detection via `Runtime.evaluate` probe
- [x] `execute_info()` uses `probe_dialog_open()` + `read_dialog_cookie()` when dialog detected
- [x] Remove `drain_dialog_event()` entirely — event-based metadata is unreliable
- [x] Simplify `setup_dialog_session()` — no event subscription needed
- [x] `cargo clippy` passes

### T003: Add Navigation-Based Fallback for Dialog Handle

**File(s)**: `src/dialog.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [x] Add `dismiss_via_navigation()` async fn that uses `Page.getNavigationHistory` + `Page.navigate` to dismiss pre-existing dialogs
- [x] `execute_handle()` tries `Page.handleJavaScriptDialog` first (standard CDP path)
- [x] On "No dialog is showing" error, falls back to: probe → navigation dismiss → verify
- [x] When no dialog is actually open, returns proper `AppError::no_dialog_open()` error
- [x] `read_dialog_cookie()` is called BEFORE handling (metadata available for response)
- [x] `cargo clippy` passes

### T004: Install Interceptors in Common Commands

**File(s)**: `src/navigate.rs`, `src/js.rs`, `src/page.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [x] After `setup_session()` in commands that interact with pages, call `managed.install_dialog_interceptors().await`
- [x] The call is best-effort — failure does not affect the command
- [x] At minimum: `navigate`, `js exec`, `page` commands install interceptors
- [x] `cargo clippy` passes

### T005: Add Regression Test

**File(s)**: `tests/features/134-fix-dialog-info-wrong-type-empty-message.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T002, T003
**Acceptance**:
- [ ] Gherkin feature file covers AC1 (alert type and message), AC2 (confirm type and message), AC3 (no dialog regression), AC4 (handle returns correct metadata)
- [ ] All scenarios tagged `@regression`
- [ ] Feature file registered in `tests/bdd.rs` alongside existing dialog feature runners
- [ ] Tests pass with the fix applied

### T006: Verify No Regressions

**File(s)**: existing test files
**Type**: Verify (no file changes)
**Depends**: T001-T005
**Acceptance**:
- [ ] All existing unit tests pass (`cargo test --lib`)
- [ ] All existing BDD tests pass (`cargo test --test bdd`)
- [ ] `cargo clippy` passes
- [ ] `cargo fmt --check` passes

### T007: Manual Smoke Test

**File(s)**: N/A (manual verification)
**Type**: Verify (no file changes)
**Depends**: T002, T003, T004
**Acceptance**:
- [x] Build chrome-cli in release mode
- [x] Launch Chrome via `connect --launch`
- [x] Navigate to https://example.com (installs interceptors)
- [x] Trigger alert with 5s delay, wait for dialog to appear
- [x] `dialog info` shows `type: "alert"` and correct message
- [x] `dialog handle accept` succeeds and returns correct metadata
- [x] `dialog info` after handle shows `"open": false`
- [x] Confirm dialog + dismiss works
- [x] Prompt dialog with default value shows `default_value` in info
- [x] `dialog handle` with no dialog open returns proper error

---

## Validation Checklist

Before marking complete:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T005)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
