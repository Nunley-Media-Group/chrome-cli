# Tasks: Add Page Wait Command

**Issues**: #163
**Date**: 2026-03-12
**Status**: Planning
**Author**: Claude

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 3 | [ ] |
| Backend | 3 | [ ] |
| Integration | 1 | [ ] |
| Testing | 5 | [ ] |
| **Total** | **12** | |

---

## Phase 1: Setup

### T001: Add `globset` dependency to Cargo.toml

**File(s)**: `Cargo.toml`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `globset = "0.4"` added to `[dependencies]` section
- [ ] `cargo check` passes with the new dependency

**Notes**: The `globset` crate provides URL-appropriate glob matching where `*` matches across `/` characters.

### T002: Define `PageWaitArgs` and add `Wait` variant to `PageCommand`

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `PageWaitArgs` struct defined with `--url`, `--text`, `--selector`, `--network-idle`, and `--interval` fields
- [ ] `--url`, `--text`, `--selector`, and `--network-idle` use `group = "condition"` to enforce exactly one condition
- [ ] `#[command(arg_required_else_help = true)]` attribute on `PageWaitArgs`
- [ ] `--interval` defaults to `100` (milliseconds)
- [ ] `Wait(PageWaitArgs)` variant added to `PageCommand` enum with `/// Wait until a condition is met on the current page` doc comment
- [ ] `cargo check` passes

**Notes**: Use `#[arg(long, group = "condition")]` on each condition flag. The `network_idle` field is `bool` (presence flag); others are `Option<String>`.

### T003: Add `wait_timeout` error constructor to `AppError`

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `pub fn wait_timeout(timeout_ms: u64, condition: &str) -> Self` added to `impl AppError`
- [ ] Returns `ExitCode::TimeoutError` (code 4)
- [ ] Message format: `"Wait timed out after {timeout_ms}ms: {condition}"`
- [ ] `cargo check` passes

---

## Phase 2: Backend Implementation

### T004: Create `src/page/wait.rs` — core structure and condition dispatch

**File(s)**: `src/page/wait.rs`
**Type**: Create
**Depends**: T001, T002, T003
**Acceptance**:
- [ ] `WaitResult` struct defined with `condition`, `matched`, `url`, `title`, and optional `pattern`/`text`/`selector` fields (using `skip_serializing_if`)
- [ ] `pub async fn execute_wait(global: &GlobalOpts, args: &PageWaitArgs) -> Result<(), AppError>` implemented
- [ ] Function calls `setup_session(global)` and `ensure_domain("Runtime")`
- [ ] Timeout derived from `global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS)`
- [ ] Dispatches to poll-based path for `--url`/`--text`/`--selector` or event-driven path for `--network-idle`
- [ ] On success, calls `get_page_info()` for URL/title and outputs via `print_output()`
- [ ] Supports `--plain` output format
- [ ] `cargo check` passes

**Notes**: Import `setup_session`, `get_page_info`, `print_output`, `cdp_config` from `super`. Import `DEFAULT_NAVIGATE_TIMEOUT_MS` from `crate::navigate`.

### T005: Implement poll-based condition checking (`--url`, `--text`, `--selector`)

**File(s)**: `src/page/wait.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `--url`: Compiles glob pattern with `GlobBuilder::new(pattern).literal_separator(false).build()` and matches against `location.href` via `Runtime.evaluate`
- [ ] `--text`: Evaluates `document.body.innerText.includes(...)` via `Runtime.evaluate` — text value properly JSON-encoded with `serde_json::to_string()` before embedding in JS expression to prevent injection
- [ ] `--selector`: Evaluates `document.querySelector(...) !== null` via `Runtime.evaluate` — selector value properly escaped
- [ ] Immediate pre-check: condition evaluated once before entering poll loop; returns immediately if already satisfied (AC7)
- [ ] Poll loop: `tokio::time::sleep(Duration::from_millis(args.interval))` between checks
- [ ] Deadline enforcement: returns `AppError::wait_timeout()` when elapsed time exceeds timeout
- [ ] Invalid glob pattern produces a clear error (exit code 1) before any CDP interaction
- [ ] JS evaluation errors during polling are caught and retried on next interval (page may be navigating)
- [ ] `cargo check` passes

**Notes**: For `--url`, fetch `location.href` as a string and match in Rust with `globset`. For `--text` and `--selector`, the entire check runs in JS returning a boolean.

### T006: Implement event-driven `--network-idle` path

**File(s)**: `src/page/wait.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] Enables `Network` domain via `managed.ensure_domain("Network")`
- [ ] Subscribes to `Network.requestWillBeSent`, `Network.loadingFinished`, `Network.loadingFailed`
- [ ] Calls `navigate::wait_for_network_idle(req_rx, fin_rx, fail_rx, timeout_ms)`
- [ ] Returns immediately when network is already idle (inherent behavior of `wait_for_network_idle` — idle timer starts at 0 in-flight) (AC6)
- [ ] On success, retrieves page info and outputs `WaitResult` with `condition: "network-idle"`
- [ ] `cargo check` passes

**Notes**: Direct reuse of `wait_for_network_idle()` from `src/navigate.rs` — no modifications needed to that function.

---

## Phase 3: Integration

### T007: Wire wait module into page dispatcher

**File(s)**: `src/page/mod.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `mod wait;` declaration added alongside other submodule declarations
- [ ] `PageCommand::Wait(wait_args) => wait::execute_wait(global, wait_args).await,` arm added to `execute_page` match
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo fmt --check` passes

---

## Phase 4: Testing

### T008: Create BDD feature file

**File(s)**: `tests/features/page-wait.feature`
**Type**: Create
**Depends**: T007
**Acceptance**:
- [ ] All 8 acceptance criteria from requirements.md mapped to scenarios
- [ ] Valid Gherkin syntax
- [ ] Scenarios are independent (no shared mutable state)
- [ ] Uses concrete examples from requirements.md

### T009: Implement BDD step definitions

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T008
**Acceptance**:
- [ ] All step definitions for page-wait scenarios implemented
- [ ] Steps follow existing patterns in `tests/bdd.rs`
- [ ] `cargo test --test bdd` passes (Chrome-independent scenarios)

### T010: Add unit tests for glob URL matching

**File(s)**: `src/page/wait.rs`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] `#[cfg(test)] mod tests` block with unit tests for glob matching
- [ ] Tests cover: wildcard match, literal match, no match, pattern with `*` across `/`, empty pattern edge case
- [ ] `cargo test --lib` passes

### T011: Manual smoke test against real Chrome

**File(s)**: (no file changes — execution only)
**Type**: Verify
**Depends**: T007
**Acceptance**:
- [ ] Build debug binary: `cargo build`
- [ ] Connect to headless Chrome: `./target/debug/agentchrome connect --launch --headless`
- [ ] Navigate to SauceDemo: `./target/debug/agentchrome navigate https://www.saucedemo.com/`
- [ ] Test `--text`: `./target/debug/agentchrome page wait --text "Swag Labs"` returns successfully with JSON
- [ ] Test `--url`: `./target/debug/agentchrome page wait --url "*saucedemo*"` returns successfully with JSON
- [ ] Test `--selector`: `./target/debug/agentchrome page wait --selector "#login-button"` returns successfully with JSON
- [ ] Test `--network-idle`: `./target/debug/agentchrome page wait --network-idle` returns successfully with JSON
- [ ] Test timeout: `./target/debug/agentchrome page wait --text "nonexistent" --timeout 2000` exits with code 4
- [ ] Test no condition: `./target/debug/agentchrome page wait` shows help/error
- [ ] Disconnect and kill Chrome: `./target/debug/agentchrome connect disconnect && pkill -f 'chrome.*--remote-debugging' || true`

### T012: Verify no regressions

**File(s)**: (no file changes — execution only)
**Type**: Verify
**Depends**: T007, T009
**Acceptance**:
- [ ] `cargo test --lib` passes (all unit tests)
- [ ] `cargo test --test bdd` passes (all BDD tests)
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo fmt --check` passes
- [ ] No changes to existing page subcommand behavior

---

## Dependency Graph

```
T001 (globset dep) ──────┐
                         ├──▶ T004 (core wait.rs) ──┬──▶ T005 (poll conditions) ──▶ T010 (unit tests)
T002 (CLI args) ─────────┤                          │
                         │                          ├──▶ T006 (network idle)
T003 (error ctor) ───────┘                          │
                                                    └──▶ T007 (wire dispatcher) ──┬──▶ T008 (feature file) ──▶ T009 (step defs)
                                                                                  ├──▶ T011 (smoke test)
                                                                                  └──▶ T012 (regressions)
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #163 | 2026-03-12 | Initial task breakdown |

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included (BDD + unit + smoke)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
