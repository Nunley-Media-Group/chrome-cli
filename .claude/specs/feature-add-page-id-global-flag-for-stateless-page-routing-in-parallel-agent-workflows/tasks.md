# Tasks: Add --page-id Global Flag for Stateless Page Routing

**Issues**: #170
**Date**: 2026-03-12
**Status**: Planning
**Author**: Claude (SDLC)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 1 | [ ] |
| Backend | 2 | [ ] |
| Integration | 1 | [ ] |
| Testing | 3 | [ ] |
| **Total** | **7** | |

---

## Phase 1: Setup

### T001: Add `--page-id` field to `GlobalOpts`

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `GlobalOpts` has a `page_id: Option<String>` field
- [ ] Field has `#[arg(long, global = true, conflicts_with = "tab")]` attribute
- [ ] Help text reads: `"Explicit page target ID (bypasses session state; conflicts with --tab)"`
- [ ] `agentchrome --help` shows the new `--page-id` flag
- [ ] `agentchrome page text --page-id X --tab 0` exits with code 1 and a conflict error on stderr
- [ ] `cargo build` succeeds (compilation will fail until T002 updates call sites)

**Notes**: Place the field directly after the `tab` field for logical grouping. The `conflicts_with = "tab"` attribute handles AC5 (mutual exclusivity) entirely via clap -- no runtime code needed. Verify that the new arg name `page_id` does not collide with any existing global or subcommand arg names (retrospective learning: parameter name collisions).

---

## Phase 2: Backend Implementation

### T002: Modify `resolve_target()` to accept and prioritize `page_id`

**File(s)**: `src/connection.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `resolve_target()` signature adds `page_id: Option<&str>` as the fourth parameter
- [ ] When `page_id` is `Some`, the function queries targets and calls `select_target(&targets, Some(pid))` directly, returning immediately without reading the session file
- [ ] When `page_id` is `None`, all existing logic is unchanged (session fallback chain intact)
- [ ] When `page_id` is `Some` but the ID doesn't exist, `AppError::target_not_found()` is returned with exit code 3
- [ ] Existing unit tests for `select_target()` still pass
- [ ] `cargo clippy` passes

**Notes**: The `page_id` branch should be the very first check in the function body, before the existing `tab.is_none()` session check. Since `page_id` and `tab` are mutually exclusive (enforced by clap), they cannot both be `Some` at this point. The `page_id` lookup reuses `select_target()` which already handles by-ID matching.

### T003: Update all command module call sites

**File(s)**: `src/navigate.rs`, `src/page/mod.rs`, `src/js.rs`, `src/form.rs`, `src/interact.rs`, `src/console.rs`, `src/network.rs`, `src/emulate.rs`, `src/perf.rs`, `src/dialog.rs`, `src/dom.rs`, `src/cookie.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Every call to `resolve_target()` passes `global.page_id.as_deref()` as the new fourth argument
- [ ] All 12 modules compile without errors
- [ ] No functional changes to command logic beyond the added parameter
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` passes

**Notes**: Each module follows the same pattern. The change in each file is mechanical:
```rust
// Before:
let target = resolve_target(&conn.host, conn.port, global.tab.as_deref()).await?;
// After:
let target = resolve_target(&conn.host, conn.port, global.tab.as_deref(), global.page_id.as_deref()).await?;
```
Also update `apply_config_defaults()` in `src/main.rs` to include `page_id: cli_global.page_id.clone()` in the constructed `GlobalOpts`.

---

## Phase 3: Integration

### T004: Verify config merging and end-to-end flag propagation

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `apply_config_defaults()` includes `page_id: cli_global.page_id.clone()` in the returned `GlobalOpts`
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] Running `agentchrome page text --page-id X` (against no Chrome) produces a connection error, not a panic (confirms flag is threaded through correctly)

**Notes**: The `page_id` is not supported in config files (intentionally stateless), so it's simply cloned from CLI args without config merging.

---

## Phase 4: Testing

### T005: Create BDD feature file

**File(s)**: `tests/features/page-id-global-flag.feature`
**Type**: Create
**Depends**: T004
**Acceptance**:
- [ ] Feature file contains scenarios for all 7 acceptance criteria from requirements.md
- [ ] Scenarios use Given/When/Then format
- [ ] Scenarios tagged `@requires-chrome` where Chrome interaction is needed
- [ ] Feature file is valid Gherkin syntax
- [ ] Scenario for mutual exclusivity (AC5) does NOT require `@requires-chrome`

### T006: Implement BDD step definitions

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] Step definitions for all new scenarios are implemented
- [ ] Steps reuse existing step definition patterns from the file (CLI invocation via `Command::new`)
- [ ] All non-`@requires-chrome` scenarios pass: `cargo test --test bdd`
- [ ] Step definitions for `@requires-chrome` scenarios are implemented (may be skipped in CI)

**Notes**: The project uses a single `tests/bdd.rs` file for all step definitions. Follow existing patterns for CLI invocation steps (process spawning, exit code assertion, stderr content assertion). The mutual exclusivity test (AC5) can run without Chrome since clap validates before any connection is made.

### T007: Smoke test against real Chrome

**File(s)**: (no file changes -- manual verification)
**Type**: Verify
**Depends**: T004
**Acceptance**:
- [ ] Build debug binary: `cargo build`
- [ ] Launch headless Chrome: `./target/debug/agentchrome connect --launch --headless`
- [ ] Create a second tab: `./target/debug/agentchrome tabs create https://www.saucedemo.com/`
- [ ] List tabs to get target IDs: `./target/debug/agentchrome tabs list`
- [ ] Run page text with `--page-id` targeting the SauceDemo tab by its target ID
- [ ] Verify output contains SauceDemo content
- [ ] Run page text with `--page-id` targeting the first tab (about:blank)
- [ ] Verify output does NOT contain SauceDemo content
- [ ] Run with nonexistent `--page-id`: verify exit code 3 and JSON error on stderr
- [ ] Run with both `--page-id` and `--tab`: verify exit code 1 and conflict error
- [ ] SauceDemo smoke: navigate + snapshot succeeds
- [ ] Disconnect: `./target/debug/agentchrome connect disconnect`
- [ ] Kill orphaned Chrome: `pkill -f 'chrome.*--remote-debugging' || true`

---

## Dependency Graph

```
T001 ──▶ T002 ──▶ T003 ──▶ T004 ──▶ T005 ──▶ T006
                                  │
                                  └──▶ T007
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #170 | 2026-03-12 | Initial feature spec |

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included (BDD + smoke)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
