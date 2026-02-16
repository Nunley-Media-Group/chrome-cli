# Tasks: emulate reset does not restore original viewport dimensions

**Issue**: #100
**Date**: 2026-02-15
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix the defect — add baseline capture and restore | [ ] |
| T002 | Add regression test | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/emulate.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `EmulateState` has a new `baseline_viewport: Option<ViewportState>` field with `#[serde(skip_serializing_if = "Option::is_none")]` and `#[serde(default)]`
- [ ] `execute_set()`: before the first viewport override, if `persisted.baseline_viewport` is `None`, queries `window.innerWidth` / `window.innerHeight` via `Runtime.evaluate` and stores the result as `baseline_viewport`
- [ ] `execute_reset()`: if persisted state contains `baseline_viewport`, calls `Emulation.setDeviceMetricsOverride` with the baseline width/height (deviceScaleFactor: 1, mobile: false) before calling `Emulation.clearDeviceMetricsOverride`
- [ ] Existing `emulate-state.json` files without `baseline_viewport` deserialize without error (backward compatible)
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo fmt --check` passes

**Notes**: The baseline must be captured *before* the first `Emulation.setDeviceMetricsOverride` call in `execute_set()`, using the same `Runtime.evaluate` pattern already used on lines 718–739. Only capture when `persisted.baseline_viewport` is `None` to avoid overwriting the true baseline on subsequent `emulate set` calls. In `execute_reset()`, read the persisted state *before* deleting it so the baseline is available for the restore step.

### T002: Add Regression Test

**File(s)**: `tests/features/100-fix-emulate-reset-viewport.feature`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario reproduces the original bug condition (set viewport → reset → check viewport restored)
- [ ] Scenario tagged `@regression`
- [ ] Test passes with the fix applied
- [ ] Test fails if the fix is reverted (confirms it catches the bug)

### T003: Verify No Regressions

**File(s)**: existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing BDD tests pass (`cargo test --test bdd`)
- [ ] All unit tests pass (`cargo test --lib`)
- [ ] `emulate set` / `emulate status` / `emulate reset` still work for non-viewport overrides (network, CPU, user-agent, geolocation, color-scheme)
- [ ] Existing `85-emulate-overrides-persistence.feature` scenarios pass
- [ ] Existing `74-fix-emulate-status-inaccurate-state.feature` scenarios pass

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
