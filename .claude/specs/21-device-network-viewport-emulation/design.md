# Design: Device, Network, and Viewport Emulation

**Issue**: #21
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude (SDLC)

---

## Overview

This feature implements the `emulate` subcommand group with three subcommands (`set`, `reset`, `status`) and adds a `page resize` shorthand. The implementation follows the same patterns as existing command modules (`network.rs`, `console.rs`, etc.) — a new `src/emulate.rs` module with CLI args in `src/cli/mod.rs`, session setup via `resolve_connection`/`resolve_target`, and CDP calls through `ManagedSession`.

The emulate module coordinates six CDP domains/methods: `Network.emulateNetworkConditions` for network throttling, `Emulation.setCPUThrottlingRate` for CPU throttling, `Emulation.setGeolocationOverride` for geolocation, `Emulation.setUserAgentOverride` for user agent, `Emulation.setEmulatedMedia` for color scheme, and `Emulation.setDeviceMetricsOverride` for viewport/device emulation. The `Emulation` domain commands work without explicit domain enabling; `Network.emulateNetworkConditions` requires `Network.enable`.

---

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      CLI Layer (clap)                        │
│  EmulateArgs → EmulateCommand::Set | Reset | Status          │
│  PageCommand::Resize (new variant)                           │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   Command Layer (emulate.rs)                  │
│  execute_emulate() dispatches to:                            │
│    execute_set()    — apply one or more overrides             │
│    execute_reset()  — clear all overrides                     │
│    execute_status() — query current emulation state           │
│                                                              │
│  page.rs: execute_resize() — viewport shorthand              │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                  CDP Layer (ManagedSession)                   │
│  Network.emulateNetworkConditions                            │
│  Emulation.setCPUThrottlingRate                              │
│  Emulation.setGeolocationOverride / clearGeolocationOverride │
│  Emulation.setUserAgentOverride                              │
│  Emulation.setEmulatedMedia                                  │
│  Emulation.setDeviceMetricsOverride / clearDeviceMetrics...  │
│  Emulation.setTouchEmulationEnabled                          │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. User runs: chrome-cli emulate set --network slow-4g --viewport 375x667
2. CLI layer parses args into EmulateSetArgs struct
3. execute_set() establishes CDP session via setup_session()
4. For --network: ensures Network domain, calls Network.emulateNetworkConditions
5. For --viewport: calls Emulation.setDeviceMetricsOverride
6. Builds EmulateStatusOutput aggregating all applied settings
7. Outputs JSON to stdout, exits 0
```

---

## API / Interface Changes

### New CLI Commands

| Command | Purpose |
|---------|---------|
| `chrome-cli emulate set [OPTIONS]` | Apply emulation overrides |
| `chrome-cli emulate reset` | Clear all emulation overrides |
| `chrome-cli emulate status` | Show current emulation settings |
| `chrome-cli page resize <SIZE>` | Shorthand for viewport resize |

### CLI Argument Structures

```rust
// EmulateArgs — top-level subcommand group
pub struct EmulateArgs {
    pub command: EmulateCommand,
}

pub enum EmulateCommand {
    Set(EmulateSetArgs),
    Reset,
    Status,
}

pub struct EmulateSetArgs {
    pub network: Option<NetworkProfile>,      // --network
    pub cpu: Option<u32>,                     // --cpu (1-20)
    pub geolocation: Option<String>,          // --geolocation LAT,LONG
    pub no_geolocation: bool,                 // --no-geolocation
    pub user_agent: Option<String>,           // --user-agent STRING
    pub no_user_agent: bool,                  // --no-user-agent
    pub color_scheme: Option<ColorScheme>,    // --color-scheme
    pub viewport: Option<String>,             // --viewport WIDTHxHEIGHT
    pub device_scale: Option<f64>,            // --device-scale FACTOR
    pub mobile: bool,                         // --mobile
}

pub enum NetworkProfile { Offline, Slow4g, FourG, ThreeG, None }
pub enum ColorScheme { Dark, Light, Auto }

// PageResizeArgs — page resize subcommand
pub struct PageResizeArgs {
    pub size: String,  // WIDTHxHEIGHT
}
```

### Output Schemas

#### `emulate set` / `emulate status`

```json
{
  "network": "slow-4g",
  "cpu": 4,
  "geolocation": { "latitude": 37.7749, "longitude": -122.4194 },
  "userAgent": "Mozilla/5.0 Custom",
  "colorScheme": "dark",
  "viewport": { "width": 375, "height": 667 },
  "deviceScaleFactor": 2.0,
  "mobile": true
}
```

Fields are `null` when not overridden.

#### `emulate reset`

```json
{
  "reset": true
}
```

#### `page resize`

```json
{
  "width": 1280,
  "height": 720
}
```

### Errors

| Condition | Error message pattern |
|-----------|----------------------|
| Invalid network profile | Handled by clap's `ValueEnum` — automatic error |
| Invalid viewport format | `"Invalid viewport format: expected WIDTHxHEIGHT (e.g. 1280x720): {input}"` |
| Invalid geolocation format | `"Invalid geolocation format: expected LAT,LONG (e.g. 37.7749,-122.4194): {input}"` |
| CPU rate out of range | Handled by clap's `value_parser` range validation |
| CDP protocol error | `"Emulation failed: {description}"` |

---

## Database / Storage Changes

None. Emulation settings are ephemeral CDP overrides per tab session. No persistent storage needed.

---

## State Management

Emulation state is managed entirely by Chrome's CDP — the CLI is stateless. Each `emulate set` invocation applies overrides to the target tab's CDP session. `emulate status` queries what can be inferred (though CDP doesn't provide a "get current emulation" query, the status command will report what was set in the current CLI session).

**Implementation approach for `emulate status`**: Since CDP doesn't have explicit "get emulation state" methods, `status` will apply a JavaScript-based query approach:
- Network: not queryable; report "unknown" unless just set
- Viewport: query via `window.innerWidth` / `window.innerHeight`
- User agent: query via `navigator.userAgent`
- Geolocation: attempt `navigator.geolocation.getCurrentPosition`
- Color scheme: query via `window.matchMedia('(prefers-color-scheme: dark)')`
- CPU: not queryable; report "unknown" unless just set

**Alternative (selected)**: Keep it simple — `emulate status` runs the JavaScript queries for what's detectable and reports `null` for what isn't. This matches the CLI's stateless design.

---

## Network Profile Definitions

| Profile | Offline | Latency (ms) | Download (bytes/s) | Upload (bytes/s) |
|---------|---------|--------------|--------------------|--------------------|
| offline | true | 0 | 0 | 0 |
| slow-4g | false | 150 | 1_600_000 | 750_000 |
| 4g | false | 20 | 4_000_000 | 3_000_000 |
| 3g | false | 100 | 750_000 | 250_000 |
| none | false | 0 | -1 | -1 |

These match the Chromium DevTools network presets.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Separate subcommands per domain** | `emulate network`, `emulate viewport`, etc. | Clear separation | Too many commands, can't combine settings in one call | Rejected |
| **B: Single `set` with combined flags** | `emulate set --network X --viewport Y` | Compose multiple settings, concise | Slightly complex arg parsing | **Selected** |
| **C: Named device presets** | `emulate device "iPhone 14"` | Convenient | Requires maintaining device database, out of scope | Deferred |

---

## Security Considerations

- [x] **Input Validation**: All user inputs validated (viewport format, geolocation range, CPU range, enum values)
- [x] **No sensitive data**: Emulation settings are not sensitive
- [x] **Local only**: CDP connections restricted to localhost per existing policy

---

## Performance Considerations

- Multiple CDP calls in `emulate set` are sequential (not parallelized) — each takes ~5-20ms, total well under 500ms target
- No caching needed — commands are one-shot

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Input parsing | Unit | Viewport format parser, geolocation parser, network profiles |
| CDP integration | BDD | All acceptance criteria from requirements |
| Error handling | Unit + BDD | Invalid inputs, CDP failures |
| Output format | Unit | JSON serialization of status output |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| CDP `Emulation.setDeviceMetricsOverride` requires all fields | Med | Med | Always send complete object (width, height, deviceScaleFactor, mobile) |
| Network domain must be enabled for throttling | Low | Low | Use `managed.ensure_domain("Network")` before network calls |
| `emulate status` can't fully detect all active overrides | Med | Low | Use JS queries for detectable settings, `null` for others |

---

## Open Questions

- None.

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] State management approach is clear (stateless, CDP-managed)
- [x] Security considerations addressed
- [x] Performance impact analyzed (well within targets)
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
