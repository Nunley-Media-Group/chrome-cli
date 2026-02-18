# Tasks: Page commands target wrong tab after tabs activate

**Issue**: #137
**Date**: 2026-02-17
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Add `active_tab_id` to `SessionData` and preserve on reconnect | [ ] |
| T002 | Persist active tab in `execute_activate()` | [ ] |
| T003 | Prefer persisted active tab in `resolve_target()` | [ ] |
| T004 | Add regression test | [ ] |
| T005 | Run smoke test against headless Chrome | [ ] |
| T006 | Verify no regressions | [ ] |

---

### T001: Add `active_tab_id` to `SessionData` and preserve on reconnect

**File(s)**: `src/session.rs`, `src/main.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `SessionData` has `active_tab_id: Option<String>` with `#[serde(skip_serializing_if = "Option::is_none", default)]`
- [ ] Existing session files without `active_tab_id` deserialize successfully with `None`
- [ ] `save_session()` in `main.rs` preserves `active_tab_id` from the existing session when reconnecting to the same port (matching PID preservation pattern)
- [ ] Session round-trip test passes with and without `active_tab_id`
- [ ] `cargo test --lib` passes

**Notes**: The `default` serde attribute ensures backward compatibility — missing fields deserialize as `None`. In `save_session()`, read the existing session once and extract both `pid` and `active_tab_id` from it, rather than reading the session file twice.

### T002: Persist active tab in `execute_activate()`

**File(s)**: `src/tabs.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] After successful `Target.activateTarget` and polling, `execute_activate()` reads the current session, sets `active_tab_id` to the activated target's ID, and writes it back
- [ ] Failure to persist is non-fatal (warning on stderr, command still succeeds)
- [ ] `cargo test --lib` passes

**Notes**: Read the existing session with `session::read_session()`, update `active_tab_id` and `timestamp`, write back with `session::write_session()`. If no session exists (unlikely since `resolve_connection` would have failed), skip persisting silently.

### T003: Prefer persisted active tab in `resolve_target()`

**File(s)**: `src/connection.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] When `tab` is `None`, `resolve_target()` checks the session file for `active_tab_id`
- [ ] If `active_tab_id` is set and the target exists in the target list, that target is returned
- [ ] If `active_tab_id` is set but the target does not exist (tab was closed), falls back to the existing first-page heuristic
- [ ] When `tab` is `Some(value)` (explicit `--tab`), `active_tab_id` is not consulted
- [ ] `cargo test --lib` passes
- [ ] Existing `select_target` unit tests still pass unchanged

**Notes**: Keep `select_target()` as a pure function — add the session-aware logic in `resolve_target()` only. Try the persisted ID via `select_target(&targets, Some(&active_id))`, and if it returns `Err`, fall back to `select_target(&targets, None)`.

### T004: Add regression test

**File(s)**: `tests/features/137-fix-page-commands-wrong-tab-after-activate.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T003
**Acceptance**:
- [ ] Gherkin scenarios cover AC1 (page text from activated tab), AC2 (page screenshot from activated tab), AC3 (explicit --tab still works), AC4 (persistence across invocations)
- [ ] All scenarios tagged `@regression`
- [ ] Step definitions implemented in `tests/bdd.rs`
- [ ] `cargo test --test bdd` passes

### T005: Run smoke test against headless Chrome

**File(s)**: None (manual verification)
**Type**: Verify
**Depends**: T003
**Acceptance**:
- [ ] Build debug binary: `cargo build`
- [ ] Launch headless Chrome: `./target/debug/chrome-cli connect --launch --headless`
- [ ] Reproduce original bug steps from requirements.md and confirm the bug no longer occurs
- [ ] Run SauceDemo smoke test (navigate + snapshot)
- [ ] Disconnect and kill orphaned Chrome processes

### T006: Verify no regressions

**File(s)**: Existing test files
**Type**: Verify (no file changes)
**Depends**: T003, T004
**Acceptance**:
- [ ] `cargo test` passes (all unit, integration, and BDD tests)
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo fmt --check` passes
- [ ] No side effects in related code paths (per blast radius from design.md)

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T004)
- [x] Smoke test is included (T005)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
