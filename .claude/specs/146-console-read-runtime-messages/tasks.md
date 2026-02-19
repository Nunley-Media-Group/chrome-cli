# Tasks: Console Read Runtime Messages

**Issue**: #146
**Date**: 2026-02-18
**Status**: Planning
**Author**: Claude (writing-specs)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 1 | [ ] |
| Backend | 2 | [ ] |
| Integration | 0 | N/A |
| Testing | 3 | [ ] |
| **Total** | **6** | |

---

## Phase 1: Setup

### T001: Remove reload-based constants and types

**File(s)**: `src/console.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `RawConsoleEvent` struct (lines 66-69) is removed
- [ ] `DEFAULT_RELOAD_TIMEOUT_MS` constant (line 386) is removed
- [ ] `POST_LOAD_IDLE_MS` constant (lines 388-389) is removed
- [ ] New constants added: `DEFAULT_DRAIN_TIMEOUT_MS` (5000) and `IDLE_DRAIN_MS` (200)
- [ ] `cargo check` passes with no errors

**Notes**: This is a preparatory cleanup before rewriting `execute_read()`. The new constants serve the same role (timeout control) but with names reflecting the drain strategy.

---

## Phase 2: Backend Implementation

### T002: Rewrite `execute_read()` to use CDP replay buffer

**File(s)**: `src/console.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `execute_read()` subscribes to `Runtime.consoleAPICalled` BEFORE calling `ensure_domain("Runtime")` (critical ordering for replay buffer capture)
- [ ] Page domain is not enabled (`ensure_domain("Page")` call removed)
- [ ] No `Page.reload` command is sent
- [ ] No `Page.frameNavigated` or `Page.loadEventFired` subscriptions
- [ ] No navigation tracking variables (`current_nav_id`, `page_loaded`, `idle_deadline`)
- [ ] Events are collected with a simple drain loop: receive events until 200ms of idle or 5s absolute timeout
- [ ] `--include-preserved` flag still accepted but all replayed events are returned regardless (no navigation filtering)
- [ ] Detail mode (MSG_ID lookup) still works on collected events
- [ ] Type filtering, pagination, and output formatting are unchanged
- [ ] `#[allow(clippy::too_many_lines)]` annotation can be removed (function is now ~25 lines)
- [ ] `cargo check` passes
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt --check` passes

**Notes**: Follow the subscribe-before-enable pattern from `connection.rs:309-337` (`spawn_auto_dismiss`). The drain loop is a single `tokio::select!` with two branches: event receive and idle timeout. Reset the idle timer on each received event.

### T003: Update unit tests for removed types

**File(s)**: `src/console.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Any unit tests referencing `RawConsoleEvent` are removed or updated
- [ ] Existing unit tests for helper functions (`parse_console_event`, `filter_by_type`, `paginate`, `format_console_args`, `timestamp_to_iso`, `extract_stack_trace`, `map_cdp_type`, `is_error_level`) continue to pass
- [ ] `cargo test --lib` passes

**Notes**: The helper function unit tests should all pass without changes since only the collection strategy changed, not the helpers.

---

## Phase 3: Testing

### T004: Create BDD feature file

**File(s)**: `tests/features/146-console-read-runtime-messages.feature`
**Type**: Create
**Depends**: T002
**Acceptance**:
- [ ] All 7 acceptance criteria from requirements.md are represented as scenarios
- [ ] Uses Given/When/Then format matching project conventions
- [ ] Background section establishes shared preconditions
- [ ] Chrome-dependent scenarios tagged `@requires-chrome`
- [ ] Feature file is valid Gherkin syntax
- [ ] Scenarios are independent and self-contained

### T005: Register BDD feature and implement step definitions

**File(s)**: `tests/bdd.rs`, `tests/features/146-console-read-runtime-messages.feature`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] Feature file registered in `tests/bdd.rs` with `filter_run_and_exit` (all scenarios require Chrome, so filter returns `false` -- matching the #103 pattern)
- [ ] Step definitions reuse existing steps where possible (`Given chrome-cli is built`, `When I run`, `Then the exit code should be`)
- [ ] Any new step definitions needed are added to `tests/bdd.rs`
- [ ] `cargo test --test bdd` passes (all scenarios are filtered out in CI, but syntax is valid)

### T006: Run smoke test against headless Chrome

**File(s)**: (no file changes -- verification only)
**Type**: Verify
**Depends**: T002, T005
**Acceptance**:
- [ ] Build in debug mode: `cargo build`
- [ ] Launch headless Chrome: `./target/debug/chrome-cli connect --launch --headless`
- [ ] Navigate to a page: `./target/debug/chrome-cli navigate https://www.saucedemo.com/`
- [ ] Generate runtime console messages: `./target/debug/chrome-cli js exec "console.log('smoke-test'); console.error('smoke-error')"`
- [ ] Verify `console read` captures runtime messages: `./target/debug/chrome-cli console read` returns entries with "smoke-test" and "smoke-error"
- [ ] Verify `--errors-only` filter: `./target/debug/chrome-cli console read --errors-only` returns only the error entry
- [ ] Verify page state preserved: `./target/debug/chrome-cli page snapshot` still shows the SauceDemo page (not a reload/blank state)
- [ ] Verify accumulated messages: run another `js exec` with new messages, then `console read` shows all prior + new messages
- [ ] Verify `console follow` still works: `./target/debug/chrome-cli console follow --timeout 2000`
- [ ] Disconnect: `./target/debug/chrome-cli connect disconnect`
- [ ] Kill orphaned Chrome processes: `pkill -f 'chrome.*--remote-debugging' || true`
- [ ] All existing tests pass: `cargo test`
- [ ] Clippy passes: `cargo clippy`

---

## Dependency Graph

```
T001 ──▶ T002 ──┬──▶ T003
                │
                ├──▶ T004 ──▶ T005
                │
                └──▶ T006
```

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] BDD test tasks are included (T004, T005)
- [x] Smoke test task is included (T006)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
