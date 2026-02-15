# Tasks: Fix Emulate Set Overrides Persistence

**Issue**: #85
**Date**: 2026-02-15
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Expand EmulateState and persist all overrides | [ ] |
| T002 | Add apply_emulate_state helper and integrate into all command modules | [ ] |
| T003 | Fix emulate status to read all persisted fields | [ ] |
| T004 | Add regression test | [ ] |

---

### T001: Expand EmulateState and Persist All Overrides

**File(s)**: `src/emulate.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `EmulateState` struct includes `user_agent: Option<String>`, `device_scale_factor: Option<f64>`, `geolocation: Option<GeolocationState>` (new helper struct with `latitude: f64, longitude: f64`), `color_scheme: Option<String>`, `viewport: Option<ViewportState>` (new helper struct with `width: u32, height: u32`)
- [ ] All new fields use `#[serde(skip_serializing_if = "Option::is_none")]`
- [ ] New helper structs derive `Debug, Clone, Serialize, Deserialize`
- [ ] `execute_set` persists user_agent, device_scale_factor, geolocation, color_scheme, and viewport alongside existing mobile/network/cpu
- [ ] `--no-user-agent` and `--no-geolocation` clear their respective persisted fields (set to `None`)
- [ ] Existing state files without new fields deserialize correctly (backward compatible via `Option` + `Default`)
- [ ] `cargo clippy` passes

**Notes**: The persistence merge logic in `execute_set` (lines 581-592) needs expansion. Each new flag should conditionally update the corresponding persisted field, mirroring the existing pattern for mobile/network/cpu.

### T002: Add apply_emulate_state Helper and Integrate Into All Command Modules

**File(s)**: `src/emulate.rs`, `src/js.rs`, `src/navigate.rs`, `src/form.rs`, `src/interact.rs`, `src/page.rs`, `src/network.rs`, `src/console.rs`, `src/perf.rs`, `src/dialog.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] New `pub async fn apply_emulate_state(managed: &mut ManagedSession) -> Result<(), AppError>` function in `src/emulate.rs`
- [ ] Function reads persisted state via `read_emulate_state()`
- [ ] If no state file exists, returns `Ok(())` immediately (no-op)
- [ ] Re-applies active overrides via CDP: user-agent (`Emulation.setUserAgentOverride`), device metrics (`Emulation.setDeviceMetricsOverride` for viewport/scale/mobile), geolocation (`Emulation.setGeolocationOverride`), color scheme (`Emulation.setEmulatedMedia`), network (`Network.emulateNetworkConditions` with `ensure_domain`), CPU (`Emulation.setCPUThrottlingRate`)
- [ ] Skips overrides at default values (e.g., `None` fields, `mobile: false` with no viewport)
- [ ] All 9 command modules call `apply_emulate_state(&mut managed)` after session creation
- [ ] `cargo clippy` passes
- [ ] `cargo build` succeeds

**Notes**: Call `apply_emulate_state` after `setup_session()` in each module's command execution functions. The emulate module's own commands (`emulate set`, `emulate reset`, `emulate status`) do NOT call this helper — they manage state directly.

### T003: Fix Emulate Status to Read All Persisted Fields

**File(s)**: `src/emulate.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `execute_status` reads geolocation from persisted state instead of hardcoding `None`
- [ ] `execute_status` reads device_scale_factor from persisted state (using persisted value if present, falling back to JavaScript-queried `devicePixelRatio`)
- [ ] `execute_status` reads user_agent from persisted state (using persisted value if present, falling back to JavaScript-queried `navigator.userAgent`)
- [ ] `execute_status` reads color_scheme from persisted state (using persisted value if present, falling back to JavaScript-queried media query)
- [ ] `emulate status` output includes geolocation when an override is active
- [ ] `cargo clippy` passes

**Notes**: The status command currently queries some values via JavaScript (which returns Chrome defaults since overrides aren't active in the new session). For overridden values, the persisted state should take precedence.

### T004: Add Regression Test

**File(s)**: `tests/features/85-emulate-overrides-persistence.feature`
**Type**: Create
**Depends**: T001, T002, T003
**Acceptance**:
- [ ] Gherkin feature file covers all 5 acceptance criteria from requirements.md
- [ ] All scenarios tagged `@regression`
- [ ] Scenarios use concrete data from reproduction steps
- [ ] Feature file is valid Gherkin syntax

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T004)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
