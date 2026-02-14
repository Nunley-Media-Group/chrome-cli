# Tasks: Device, Network, and Viewport Emulation

**Issue**: #21
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude (SDLC)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 4 | [ ] |
| Integration | 2 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **10** | |

---

## Phase 1: Setup

### T001: Add emulate CLI argument types and enums

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `EmulateArgs` struct with `EmulateCommand` subcommand enum defined
- [ ] `EmulateCommand` has variants: `Set(EmulateSetArgs)`, `Reset`, `Status`
- [ ] `EmulateSetArgs` has all flags: `--network`, `--cpu`, `--geolocation`, `--no-geolocation`, `--user-agent`, `--no-user-agent`, `--color-scheme`, `--viewport`, `--device-scale`, `--mobile`
- [ ] `NetworkProfile` enum: `Offline`, `Slow4g`, `FourG`, `ThreeG`, `None` with `ValueEnum` derive
- [ ] `ColorScheme` enum: `Dark`, `Light`, `Auto` with `ValueEnum` derive
- [ ] `--cpu` uses `value_parser` with range 1..=20
- [ ] `PageResizeArgs` struct added with `size: String` positional arg
- [ ] `PageCommand::Resize(PageResizeArgs)` variant added
- [ ] `Command::Emulate` changed from unit variant to `Emulate(EmulateArgs)`
- [ ] `cargo check` passes

**Notes**: Follow existing patterns like `PerfArgs`/`PerfCommand`, `NetworkArgs`/`NetworkCommand`. The `--geolocation` and `--no-geolocation` should use `conflicts_with`. Same for `--user-agent` and `--no-user-agent`.

### T002: Add emulate-specific error constructors

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AppError::emulation_failed(description: &str)` constructor added
- [ ] `AppError::invalid_viewport(input: &str)` constructor added
- [ ] `AppError::invalid_geolocation(input: &str)` constructor added
- [ ] Each returns `ExitCode::GeneralError`
- [ ] Unit tests for each new error constructor

---

## Phase 2: Backend Implementation

### T003: Create emulate module with parsing helpers

**File(s)**: `src/emulate.rs`
**Type**: Create
**Depends**: T001, T002
**Acceptance**:
- [ ] `parse_viewport(input: &str) -> Result<(u32, u32), AppError>` parses `WIDTHxHEIGHT`
- [ ] `parse_geolocation(input: &str) -> Result<(f64, f64), AppError>` parses `LAT,LONG`
- [ ] `network_profile_params(profile: &NetworkProfile) -> serde_json::Value` returns CDP params
- [ ] Output structs: `EmulateStatusOutput`, `EmulateResetOutput`, `ResizeOutput`
- [ ] `pub async fn execute_emulate(global: &GlobalOpts, args: &EmulateArgs) -> Result<(), AppError>` dispatcher
- [ ] Module follows existing patterns: `setup_session()`, `cdp_config()`, output formatting
- [ ] Unit tests for `parse_viewport`, `parse_geolocation`, `network_profile_params`

### T004: Implement `emulate set` command

**File(s)**: `src/emulate.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `--network`: calls `managed.ensure_domain("Network")` then `Network.emulateNetworkConditions` with profile params
- [ ] `--cpu`: calls `Emulation.setCPUThrottlingRate` with `{"rate": N}`
- [ ] `--geolocation`: calls `Emulation.setGeolocationOverride` with `{latitude, longitude, accuracy: 1}`
- [ ] `--no-geolocation`: calls `Emulation.clearGeolocationOverride`
- [ ] `--user-agent`: calls `Emulation.setUserAgentOverride` with `{"userAgent": STRING}`
- [ ] `--no-user-agent`: calls `Emulation.setUserAgentOverride` with `{"userAgent": ""}`
- [ ] `--color-scheme`: calls `Emulation.setEmulatedMedia` with `{"features": [{"name": "prefers-color-scheme", "value": SCHEME}]}`; `auto` clears with empty string value
- [ ] `--viewport`: calls `Emulation.setDeviceMetricsOverride` with width, height, deviceScaleFactor (default 1), mobile (default false)
- [ ] `--device-scale`: integrates with viewport override's `deviceScaleFactor`
- [ ] `--mobile`: integrates with viewport override's `mobile` field + calls `Emulation.setTouchEmulationEnabled` with `{"enabled": true}`
- [ ] Multiple flags can be combined in a single invocation
- [ ] Returns JSON `EmulateStatusOutput` with all settings applied
- [ ] Supports `--plain`, `--json`, `--pretty` output modes

### T005: Implement `emulate reset` command

**File(s)**: `src/emulate.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] Calls `Network.emulateNetworkConditions` with no-throttling params (after ensuring Network domain)
- [ ] Calls `Emulation.setCPUThrottlingRate` with `{"rate": 1}`
- [ ] Calls `Emulation.clearGeolocationOverride`
- [ ] Calls `Emulation.setUserAgentOverride` with `{"userAgent": ""}`
- [ ] Calls `Emulation.setEmulatedMedia` with `{"features": [{"name": "prefers-color-scheme", "value": ""}]}`
- [ ] Calls `Emulation.clearDeviceMetricsOverride`
- [ ] Calls `Emulation.setTouchEmulationEnabled` with `{"enabled": false}`
- [ ] Returns JSON `{"reset": true}`

### T006: Implement `emulate status` command

**File(s)**: `src/emulate.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] Queries detectable settings via JavaScript execution:
  - `window.innerWidth` / `window.innerHeight` for viewport
  - `navigator.userAgent` for user agent
  - `window.matchMedia('(prefers-color-scheme: dark)').matches` for color scheme
- [ ] Returns JSON `EmulateStatusOutput` with detected values
- [ ] Non-detectable settings (network, cpu) reported as `null`

---

## Phase 3: Integration

### T007: Wire emulate module into main dispatcher and add `page resize`

**File(s)**: `src/main.rs`, `src/page.rs`
**Type**: Modify
**Depends**: T004, T005, T006
**Acceptance**:
- [ ] `mod emulate;` added to `src/main.rs`
- [ ] `Command::Emulate(args)` dispatches to `emulate::execute_emulate(&cli.global, args).await`
- [ ] `PageCommand::Resize` dispatches to `execute_resize()` in `page.rs`
- [ ] `execute_resize()` parses `WIDTHxHEIGHT`, calls `Emulation.setDeviceMetricsOverride`, returns `{"width": N, "height": N}`
- [ ] `cargo build` succeeds with no warnings

### T008: Verify all output modes work (JSON, pretty, plain)

**File(s)**: `src/emulate.rs`
**Type**: Modify
**Depends**: T007
**Acceptance**:
- [ ] `--json` outputs compact JSON (default)
- [ ] `--pretty` outputs indented JSON
- [ ] `--plain` outputs human-readable text summary
- [ ] Error output goes to stderr as JSON

---

## Phase 4: BDD Testing

### T009: Create BDD feature file

**File(s)**: `tests/features/emulate.feature`
**Type**: Create
**Depends**: T007
**Acceptance**:
- [ ] All 21 acceptance criteria from requirements.md have corresponding scenarios
- [ ] Includes Background with shared Chrome CDP precondition
- [ ] Uses Scenario Outline for network profile variations
- [ ] Valid Gherkin syntax
- [ ] Error scenarios included (AC16-AC19)

### T010: Implement unit tests for parsing and profiles

**File(s)**: `src/emulate.rs` (inline `#[cfg(test)]` module)
**Type**: Create
**Depends**: T003
**Acceptance**:
- [ ] Tests for `parse_viewport`: valid (`1280x720`, `375x667`), invalid (`bad`, `0x0`, `abc`, `-1x100`)
- [ ] Tests for `parse_geolocation`: valid (`37.7749,-122.4194`), invalid (`not-a-coord`, `37.7749`)
- [ ] Tests for `network_profile_params`: verifies each profile produces correct CDP parameters
- [ ] Tests for output struct serialization
- [ ] `cargo test` passes

---

## Dependency Graph

```
T001 ──┬──▶ T003 ──┬──▶ T004 ──┐
       │           ├──▶ T005 ──┼──▶ T007 ──▶ T008
       │           ├──▶ T006 ──┘
       │           └──▶ T010
       │
T002 ──┘
                         T007 ──▶ T009
```

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure
- [x] Test tasks are included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
