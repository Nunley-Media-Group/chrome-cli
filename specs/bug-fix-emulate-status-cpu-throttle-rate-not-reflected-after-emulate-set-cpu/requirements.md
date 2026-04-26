# Defect Report: emulate status omits CPU and geolocation after emulate set

**Issue**: #253
**Date**: 2026-04-26
**Status**: Approved
**Author**: Codex
**Severity**: High
**Related Spec**: specs/feature-device-network-viewport-emulation/

---

## Reproduction

### Steps to Reproduce

1. Launch or connect to a Chrome session with CDP enabled.
2. Run `agentchrome emulate set --cpu 4 --geolocation 37.7749,-122.4194 --color-scheme light`.
3. Confirm the `emulate set` JSON output includes `"cpu": 4` and a `geolocation` object.
4. Run `agentchrome emulate status` in a later CLI invocation against the same AgentChrome home and Chrome session.
5. Observe that the status JSON omits `cpu` and `geolocation`.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS exercise session reported in issue #253 |
| **Version / Commit** | Branch `253-fix-emulate-status-cpu-throttle-rate-not-reflected-after-emulate-set-cpu` at `9db7b4f` |
| **Browser / Runtime** | Chrome via CDP WebSocket |
| **Configuration** | Default AgentChrome state path, `~/.agentchrome/emulate-state.json` |

### Frequency

Always in the reported exercise path: `emulate set` confirms the values in its own output, but `emulate status` does not report them.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | Values accepted by `emulate set --cpu 4` and `emulate set --geolocation 37.7749,-122.4194` are persisted to AgentChrome emulation state and are reported by a later `emulate status` invocation as `"cpu": 4` and `geolocation.latitude/geolocation.longitude`. |
| **Actual** | `emulate status` omits the optional `cpu` and `geolocation` fields, which means `execute_status` is seeing `None` for both values even though `emulate set` just reported them as applied. |

### Error Output

No error is emitted. The failure is silent because `EmulateStatusOutput` uses `skip_serializing_if = "Option::is_none"` for both optional fields.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: CPU throttle is shown after set

**Given** Chrome is running with CDP enabled and no prior emulation state is active
**When** I run `agentchrome emulate set --cpu 4`
**And** I run `agentchrome emulate status` in a later CLI invocation using the same AgentChrome home
**Then** the status JSON includes `"cpu": 4`

### AC2: Geolocation is shown after set

**Given** Chrome is running with CDP enabled and no prior geolocation override is active
**When** I run `agentchrome emulate set --geolocation 37.7749,-122.4194`
**And** I run `agentchrome emulate status` in a later CLI invocation using the same AgentChrome home
**Then** the status JSON includes `geolocation.latitude` equal to `37.7749`
**And** the status JSON includes `geolocation.longitude` equal to `-122.4194`

### AC3: Reset clears CPU and geolocation from status

**Given** Chrome is running with CDP enabled
**And** CPU and geolocation overrides have been set and are visible in `emulate status`
**When** I run `agentchrome emulate reset`
**And** I run `agentchrome emulate status`
**Then** the status JSON omits `cpu`
**And** the status JSON omits `geolocation`

### AC4: State persistence path has executable regression coverage

**Given** the defect fix is implemented
**When** the focused emulate tests run under `cargo test`
**Then** an executable test proves CPU and geolocation values written by the set path are read back by the status path
**And** the test fails if CPU or geolocation are not written to the persisted emulation state

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Identify the exact reason `execute_status` receives `None` for `cpu` and `geolocation` after a successful `emulate set` invocation. | Must |
| FR2 | Ensure `emulate set --cpu <rate>` writes the selected CPU throttle rate to `~/.agentchrome/emulate-state.json` before returning success. | Must |
| FR3 | Ensure `emulate set --geolocation LAT,LONG` writes latitude and longitude to `~/.agentchrome/emulate-state.json` before returning success. | Must |
| FR4 | Ensure `emulate status` reads CPU and geolocation from the same persisted state path used by `emulate set`. | Must |
| FR5 | Preserve the existing absent-value JSON contract: after `emulate reset`, optional `cpu` and `geolocation` fields are omitted, not emitted as `null`. | Must |
| FR6 | Add focused regression coverage for the set-to-status persistence path; serialization-only tests are not sufficient. | Should |

---

## Out of Scope

- Changing the `emulate set` output format.
- Changing other emulation state fields such as network, viewport, user-agent, color scheme, device scale, or mobile, except where necessary to avoid regressing them.
- Changing the AgentChrome state file location or global output contract.
- Introducing a long-running daemon or persistent CDP session.

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario covers reset behavior
- [x] Fix scope is minimal and bounded to emulation state persistence
- [x] Out of scope is defined
