# Tasks: Fix navigate back/forward/reload ignoring global --timeout option

**Issue**: #145
**Date**: 2026-02-19
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix the defect | [ ] |
| T002 | Add regression test | [ ] |
| T003 | Run smoke test | [ ] |
| T004 | Verify no regressions | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/navigate.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `execute_back` (line 251) uses `global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS)` instead of `DEFAULT_NAVIGATE_TIMEOUT_MS`
- [ ] `execute_forward` (line 313) uses `global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS)` instead of `DEFAULT_NAVIGATE_TIMEOUT_MS`
- [ ] `execute_reload` (line 342) uses `global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS)` instead of `DEFAULT_NAVIGATE_TIMEOUT_MS`
- [ ] No unrelated changes included in the diff
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` passes

**Notes**: Follow the same pattern as `execute_url` which uses `args.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS)`. The only difference is that history/reload commands read from `global.timeout` (the global `--timeout` flag) rather than a per-command `args.timeout`.

### T002: Add Regression Test

**File(s)**: `tests/features/145-fix-navigate-timeout.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin feature file covers all 6 acceptance criteria from requirements.md
- [ ] All scenarios tagged `@regression`
- [ ] Step definitions implemented in `tests/bdd.rs` (reuse existing steps where possible)
- [ ] `cargo test --test bdd` passes

### T003: Run Smoke Test

**File(s)**: (no file changes)
**Type**: Verify
**Depends**: T001, T002
**Acceptance**:
- [ ] Build in debug mode: `cargo build`
- [ ] Launch headless Chrome: `./target/debug/chrome-cli connect --launch --headless`
- [ ] Navigate to a page, then navigate back with `--timeout 5000` — verify it completes successfully
- [ ] Navigate forward with `--timeout 5000` — verify it completes successfully
- [ ] Reload with `--timeout 5000` — verify it completes successfully
- [ ] Run SauceDemo baseline: navigate to `https://www.saucedemo.com/` and take a snapshot
- [ ] Disconnect and kill orphaned Chrome processes

### T004: Verify No Regressions

**File(s)**: (existing test files)
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo test --lib` passes (unit tests)
- [ ] `cargo test --test bdd` passes (BDD tests)
- [ ] `cargo clippy` passes
- [ ] `cargo fmt --check` passes
- [ ] No side effects in `execute_url` or other navigate code paths

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
