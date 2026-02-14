# Requirements: Device, Network, and Viewport Emulation

**Issue**: #21
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude (SDLC)

---

## User Story

**As a** developer or automation engineer
**I want** to emulate different devices, network conditions, geolocations, and color schemes via the CLI
**So that** I can test how web pages behave under various device and network constraints without physical devices

---

## Background

The MCP server's `emulate` tool provides comprehensive device emulation including network throttling, CPU throttling, geolocation, user agent, color scheme, and viewport settings. The `resize_page` tool adjusts viewport dimensions. Issue #21 brings these capabilities to the CLI as the `emulate` subcommand group and a `page resize` shorthand, using CDP's `Network.emulateNetworkConditions`, `Emulation.setCPUThrottlingRate`, `Emulation.setGeolocationOverride`, `Emulation.setUserAgentOverride`, `Emulation.setEmulatedMedia`, and `Emulation.setDeviceMetricsOverride` methods.

---

## Acceptance Criteria

### AC1: Set network emulation profile

**Given** Chrome is running with CDP enabled and a page is loaded
**When** I run `chrome-cli emulate set --network slow-4g`
**Then** network throttling is applied via CDP
**And** the command returns JSON with the current emulation settings including the network profile

**Example**:
- Given: Chrome open on `https://example.com`
- When: `chrome-cli emulate set --network slow-4g`
- Then: Output includes `"network": "slow-4g"` and exit code 0

### AC2: Set CPU throttling rate

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli emulate set --cpu 4`
**Then** CPU throttling rate 4 is applied via `Emulation.setCPUThrottlingRate`
**And** the command returns JSON with the current emulation settings including the CPU rate

### AC3: Set geolocation override

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli emulate set --geolocation 37.7749,-122.4194`
**Then** geolocation is overridden via `Emulation.setGeolocationOverride`
**And** the command returns JSON with current emulation settings including latitude and longitude

### AC4: Clear geolocation override

**Given** Chrome is running with CDP enabled and geolocation is overridden
**When** I run `chrome-cli emulate set --no-geolocation`
**Then** the geolocation override is cleared
**And** the command returns JSON confirming geolocation is no longer overridden

### AC5: Set custom user agent

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli emulate set --user-agent "Mozilla/5.0 Custom"`
**Then** the user agent is overridden via `Emulation.setUserAgentOverride`
**And** the command returns JSON with the current emulation settings

### AC6: Reset user agent to default

**Given** Chrome is running with CDP enabled and user agent is overridden
**When** I run `chrome-cli emulate set --no-user-agent`
**Then** the user agent override is cleared (reset to browser default)
**And** the command returns JSON confirming the reset

### AC7: Set color scheme emulation

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli emulate set --color-scheme dark`
**Then** the `prefers-color-scheme` media feature is set to `dark` via `Emulation.setEmulatedMedia`
**And** the command returns JSON with the current emulation settings

### AC8: Set viewport dimensions

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli emulate set --viewport 375x667`
**Then** the viewport is resized via `Emulation.setDeviceMetricsOverride`
**And** the command returns JSON with current emulation settings including viewport dimensions

### AC9: Set device scale factor

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli emulate set --device-scale 2`
**Then** the device pixel ratio is set to 2 via `Emulation.setDeviceMetricsOverride`
**And** the command returns JSON with current emulation settings

### AC10: Enable mobile emulation

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli emulate set --mobile --viewport 375x667`
**Then** mobile emulation is enabled (touch events, mobile viewport meta) via `Emulation.setDeviceMetricsOverride`
**And** touch emulation is enabled via `Emulation.setTouchEmulationEnabled`
**And** the command returns JSON with mobile emulation active

### AC11: Combine multiple emulation settings

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli emulate set --network slow-4g --viewport 375x667 --mobile --color-scheme dark`
**Then** all specified settings are applied together
**And** the command returns JSON reflecting all active emulation settings

### AC12: Target specific tab for emulation

**Given** Chrome is running with CDP enabled and multiple tabs are open
**When** I run `chrome-cli emulate set --network 3g --tab 2`
**Then** network throttling is applied only to tab 2
**And** other tabs are not affected

### AC13: Reset all emulation overrides

**Given** Chrome is running with CDP enabled and emulation overrides are active
**When** I run `chrome-cli emulate reset`
**Then** all emulation overrides are cleared (network, CPU, geolocation, user agent, color scheme, viewport)
**And** the command returns JSON confirmation

### AC14: Show current emulation status

**Given** Chrome is running with CDP enabled and some emulation overrides are active
**When** I run `chrome-cli emulate status`
**Then** the command returns JSON with all currently active emulation settings
**And** settings that are not overridden show their default/inactive state

### AC15: Page resize shorthand

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli page resize 1280x720`
**Then** the viewport is resized to 1280x720 via `Emulation.setDeviceMetricsOverride`
**And** the command returns JSON: `{"width": 1280, "height": 720}`

### AC16: Invalid network profile produces error

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli emulate set --network invalid-profile`
**Then** the command exits with a non-zero exit code
**And** an error message lists the valid network profiles

### AC17: Invalid viewport format produces error

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli emulate set --viewport badformat`
**Then** the command exits with a non-zero exit code
**And** an error message indicates the expected `WIDTHxHEIGHT` format

### AC18: Invalid geolocation format produces error

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli emulate set --geolocation not-a-coord`
**Then** the command exits with a non-zero exit code
**And** an error message indicates the expected `LAT,LONG` format

### AC19: CPU throttling rate out of range produces error

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli emulate set --cpu 0`
**Then** the command exits with a non-zero exit code
**And** an error message indicates the valid range (1-20)

### AC20: Set network to offline mode

**Given** Chrome is running with CDP enabled
**When** I run `chrome-cli emulate set --network offline`
**Then** the network is fully offline via `Network.emulateNetworkConditions`
**And** the command returns JSON with `"network": "offline"`

### AC21: Disable network throttling

**Given** Chrome is running with CDP enabled and network throttling is active
**When** I run `chrome-cli emulate set --network none`
**Then** network throttling is disabled
**And** the command returns JSON with `"network": "none"`

### Generated Gherkin Preview

```gherkin
Feature: Device, network, and viewport emulation
  As a developer or automation engineer
  I want to emulate different devices, network conditions, geolocations, and color schemes
  So that I can test web page behavior under various device and network constraints

  Background:
    Given Chrome is running with CDP enabled

  Scenario: Set network emulation to slow-4g
    When I run "chrome-cli emulate set --network slow-4g"
    Then the JSON output should contain "network" set to "slow-4g"
    And the exit code should be 0

  Scenario: Set CPU throttling
    When I run "chrome-cli emulate set --cpu 4"
    Then the JSON output should contain "cpu" set to 4
    And the exit code should be 0

  # ... all ACs become scenarios
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `emulate set` applies one or more emulation overrides | Must | Core command |
| FR2 | `emulate reset` clears all emulation overrides | Must | Undo mechanism |
| FR3 | `emulate status` reports current emulation state | Must | Observability |
| FR4 | `page resize <WIDTHxHEIGHT>` sets viewport dimensions | Must | Shorthand |
| FR5 | Network profiles: offline, slow-4g, 4g, 3g, none | Must | Predefined profiles |
| FR6 | CPU throttling rate (1-20) | Must | CDP `setCPUThrottlingRate` |
| FR7 | Geolocation override and clear | Must | CDP `setGeolocationOverride` |
| FR8 | User agent override and clear | Must | CDP `setUserAgentOverride` |
| FR9 | Color scheme emulation (dark, light, auto) | Must | CDP `setEmulatedMedia` |
| FR10 | Viewport dimensions with `--viewport WxH` | Must | CDP `setDeviceMetricsOverride` |
| FR11 | Device pixel ratio with `--device-scale` | Should | CDP `setDeviceMetricsOverride` |
| FR12 | Mobile emulation flag `--mobile` | Should | Touch + mobile viewport |
| FR13 | Tab targeting via `--tab` global option | Must | Per-tab emulation |
| FR14 | JSON output with all current emulation settings | Must | Consistent output |
| FR15 | Plain text output mode for `--plain` flag | Should | Human-readable alternative |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Emulation commands complete in < 500ms (CDP round-trip) |
| **Security** | All CDP connections localhost only (per tech.md) |
| **Reliability** | Graceful error handling for unsupported CDP methods |
| **Platforms** | macOS, Linux, Windows (per product.md) |
| **Compatibility** | Works with Chrome/Chromium supporting CDP Emulation and Network domains |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| network | enum string | One of: offline, slow-4g, 4g, 3g, none | No |
| cpu | integer | Range 1-20 | No |
| geolocation | string | Format: `LAT,LONG` (float,float) | No |
| no-geolocation | flag | Boolean flag | No |
| user-agent | string | Non-empty string | No |
| no-user-agent | flag | Boolean flag | No |
| color-scheme | enum string | One of: dark, light, auto | No |
| viewport | string | Format: `WIDTHxHEIGHT` (positive integers) | No |
| device-scale | float | Positive number | No |
| mobile | flag | Boolean flag | No |
| tab | string | Tab ID or numeric index | No (global opt) |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| network | string or null | Active network profile name |
| cpu | integer or null | Active CPU throttling rate |
| geolocation | object or null | `{latitude, longitude}` if set |
| userAgent | string or null | Custom user agent if set |
| colorScheme | string or null | Forced color scheme if set |
| viewport | object or null | `{width, height}` if overridden |
| deviceScaleFactor | float or null | Device pixel ratio if set |
| mobile | boolean | Whether mobile emulation is active |

---

## Dependencies

### Internal Dependencies
- [x] Issue #4 — CDP client (complete)
- [x] Issue #6 — Session management (complete)

### External Dependencies
- Chrome/Chromium with CDP `Emulation` and `Network` domain support

### Blocked By
- None (all dependencies resolved)

---

## Out of Scope

- Named device presets (e.g., `--device "iPhone 14"`) — future enhancement
- Persistent emulation profiles saved to disk
- Network request interception (separate feature, #16)
- Timezone emulation
- Vision deficiency emulation
- Media type emulation (print/screen) — only color scheme

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Command latency | < 500ms | Time from CLI invocation to output |
| CDP coverage | All 6 CDP domains exercised | Network, CPU, Geo, UA, Media, Viewport |
| Error coverage | All invalid inputs produce clear errors | BDD error scenarios pass |

---

## Open Questions

- None — all requirements are clear from the issue specification.

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements (CDP methods referenced for clarity only)
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified (AC16-AC19)
- [x] Dependencies are identified and resolved
- [x] Out of scope is defined
- [x] Open questions are documented (none)
