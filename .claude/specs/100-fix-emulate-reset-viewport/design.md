# Root Cause Analysis: emulate reset does not restore original viewport dimensions

**Issue**: #100
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude

---

## Root Cause

The `execute_reset()` function in `src/emulate.rs` (line 880) calls `Emulation.clearDeviceMetricsOverride` to remove the viewport override. This CDP command tells Chrome to stop applying a device metrics override, but it does **not** resize the browser window back to its original dimensions. The viewport was physically changed by the earlier `Emulation.setDeviceMetricsOverride` call, and clearing the override simply removes the CDP-level constraint without restoring the original window geometry.

The fundamental issue is that the codebase has no concept of "baseline viewport dimensions." When `emulate set --viewport 375x667` is called, the original viewport (e.g., 756x417) is not captured or stored anywhere. When `emulate reset` runs, it has no record of what dimensions to restore to. The persisted `EmulateState` only tracks override values, not pre-override baselines.

After `clearDeviceMetricsOverride`, Chrome reports `window.innerWidth` / `window.innerHeight` as whatever the current window geometry happens to be — which in headless mode retains the overridden dimensions rather than reverting to the original.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/emulate.rs` | 878–882 | `execute_reset()` — calls `clearDeviceMetricsOverride` without restoring original dimensions |
| `src/emulate.rs` | 82–99 | `EmulateState` — no field for baseline viewport |
| `src/emulate.rs` | 702–773 | `execute_set()` viewport section — sets override without capturing baseline |
| `src/emulate.rs` | 775–811 | `execute_set()` persistence section — persists override but not baseline |

### Triggering Conditions

- A viewport override is applied via `emulate set --viewport WxH`
- `emulate reset` is called afterward
- The viewport remains at the overridden dimensions because `clearDeviceMetricsOverride` does not physically resize the window

---

## Fix Strategy

### Approach

Capture the baseline viewport dimensions before the first viewport override is applied, persist them alongside the emulation state, and use them during `emulate reset` to actively restore the viewport via `Emulation.setDeviceMetricsOverride` (with the baseline dimensions) followed by `Emulation.clearDeviceMetricsOverride`.

The fix adds a `baseline_viewport` field to `EmulateState`. In `execute_set()`, before applying a viewport override for the first time (when no baseline exists yet), the current viewport dimensions are queried via `Runtime.evaluate` and stored as the baseline. In `execute_reset()`, if a baseline viewport exists in the persisted state, the reset function first calls `Emulation.setDeviceMetricsOverride` with the baseline dimensions to physically resize the viewport back, then calls `Emulation.clearDeviceMetricsOverride` to remove the override constraint, and finally deletes the state file as it does today.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/emulate.rs` (EmulateState) | Add `baseline_viewport: Option<ViewportState>` field | Store the original viewport dimensions before any override |
| `src/emulate.rs` (execute_set) | Before first viewport override, query and persist baseline dimensions if `baseline_viewport` is `None` | Capture original state for later restoration |
| `src/emulate.rs` (execute_reset) | Before `clearDeviceMetricsOverride`, if baseline exists, call `setDeviceMetricsOverride` with baseline dimensions, then clear | Physically restore the original viewport size |

### Blast Radius

- **Direct impact**: `src/emulate.rs` — `EmulateState` struct, `execute_set()`, `execute_reset()`
- **Indirect impact**: `apply_emulate_state()` reads `EmulateState` but ignores unknown fields (serde defaults); `execute_status()` reads the state file but does not use `baseline_viewport`. The existing `emulate-state.json` files will deserialize cleanly because the new field is `Option` with `skip_serializing_if`.
- **Risk level**: Low — the change is additive (new optional field) and only modifies the reset path behavior

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Existing state files missing `baseline_viewport` cause deserialization errors | Low | Field is `Option<ViewportState>` with `#[serde(skip_serializing_if = "Option::is_none")]` and defaults to `None` — backward compatible |
| Reset behavior changes for users who did not set viewport overrides | Low | Reset only uses baseline if it exists; otherwise falls through to existing `clearDeviceMetricsOverride` behavior |
| Baseline captured at wrong time (after an override already applied) | Low | Only capture baseline when `persisted.baseline_viewport` is `None`, ensuring it's captured once before the first override |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Query viewport via JS before clearing override, then re-apply | Would query the *overridden* dimensions, not the original baseline | Does not solve the problem — need the pre-override dimensions |
| Use `Browser.getWindowBounds` / `Browser.setWindowBounds` | CDP browser-level APIs for window geometry | These operate on the browser window, not the viewport content area; behavior differs between headed and headless mode; more complex and less portable |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
