# Root Cause Analysis: Emulate Set Overrides Do Not Persist Across Commands

**Issue**: #85
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude

---

## Root Cause

Each chrome-cli command creates an independent CDP WebSocket session via the per-module `setup_session()` function. CDP emulation overrides (e.g., `Emulation.setUserAgentOverride`, `Emulation.setDeviceMetricsOverride`, `Emulation.setGeolocationOverride`, `Emulation.setEmulatedMedia`) are session-scoped: they apply only for the lifetime of the WebSocket connection that issued them. When the `emulate set` command completes and its session closes, all overrides are lost.

Issue #74 introduced a persistence mechanism (`EmulateState` struct serialized to `~/.chrome-cli/emulate-state.json`) but only for three fields: `mobile`, `network`, and `cpu`. These fields were chosen because they cannot be queried via JavaScript and were needed for accurate `emulate status` reporting. However, the persistence was read-only — `emulate status` reads the file but no other command re-applies the persisted state to new sessions.

The bug has two dimensions: (1) the `EmulateState` struct is incomplete — it lacks `user_agent`, `device_scale_factor`, `geolocation`, `color_scheme`, and `viewport`; and (2) no code exists to re-apply persisted overrides when new CDP sessions are created by other commands (navigate, js, interact, etc.).

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/emulate.rs` | 68–75 | `EmulateState` struct — missing fields for user_agent, device_scale_factor, geolocation, color_scheme, viewport |
| `src/emulate.rs` | 581–592 | Persistence in `execute_set` — only persists mobile, network, cpu |
| `src/emulate.rs` | 690–761 | `execute_status` — reads persisted state but geolocation is always `None` |
| `src/emulate.rs` | 606–684 | `execute_reset` — deletes state file but doesn't know about new fields |
| `src/emulate.rs` | 257–267 | `setup_session` — creates fresh CDP session, never reads persisted state |
| `src/js.rs` | 93–105 | `setup_session` — same pattern, no state restoration |
| `src/navigate.rs` | 89–99 | `setup_session` — same pattern, no state restoration |
| `src/form.rs` | 117–127 | `setup_session` — same pattern, no state restoration |
| `src/interact.rs` | 212–222 | `setup_session` — same pattern, no state restoration |
| `src/page.rs` | 120–130 | `setup_session` — same pattern, no state restoration |
| `src/network.rs` | 228–238 | `setup_session` — same pattern, no state restoration |
| `src/console.rs` | 144–154 | `setup_session` — same pattern, no state restoration |
| `src/perf.rs` | 130–140 | `setup_session` — same pattern, no state restoration |
| `src/dialog.rs` | 110–120 | `setup_session` — same pattern, no state restoration |

### Triggering Conditions

- User runs `emulate set` with any override flag (user-agent, device-scale, geolocation, color-scheme)
- User then runs any other command (navigate, js exec, interact, etc.)
- The second command creates a fresh CDP session that lacks the overrides from the first command
- This always reproduces — it is the fundamental architecture of per-invocation CDP sessions

---

## Fix Strategy

### Approach

Expand `EmulateState` to include all emulation override fields, persist them in `emulate set`, and create a shared `apply_emulate_state()` helper function that reads the persisted state file and re-applies all active overrides via CDP commands. Each command module's `setup_session()` will call this helper after creating the managed session.

The helper function will be defined as a public function in `src/emulate.rs` (where `EmulateState` and its I/O functions already live). Each command module already imports from `emulate.rs` or can easily do so. The helper will:

1. Read `~/.chrome-cli/emulate-state.json` via `read_emulate_state()`
2. If state exists and has active overrides, issue the corresponding CDP commands
3. Skip overrides that are at their default values (e.g., `mobile: false`, `user_agent: None`)
4. Require `Network` domain enablement only if network throttling is active

This is the minimal fix: it keeps the existing per-module `setup_session()` pattern intact and only adds a single function call after session creation.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/emulate.rs` (struct) | Add `user_agent: Option<String>`, `device_scale_factor: Option<f64>`, `geolocation: Option<GeolocationState>`, `color_scheme: Option<String>`, `viewport: Option<ViewportState>` to `EmulateState` | Complete the persisted state to cover all override types |
| `src/emulate.rs` (execute_set) | Persist new fields alongside existing mobile/network/cpu | Ensure all overrides survive session boundaries |
| `src/emulate.rs` (new fn) | Add `pub async fn apply_emulate_state(managed: &mut ManagedSession) -> Result<(), AppError>` | Central function to re-apply all persisted overrides to a new session |
| `src/emulate.rs` (execute_status) | Read geolocation from persisted state instead of hardcoding `None` | Fix geolocation always showing as absent in status |
| `src/emulate.rs` (execute_reset) | No structural change needed — `delete_emulate_state()` already deletes the whole file | Reset already works correctly for file deletion |
| `src/js.rs` | Call `apply_emulate_state(&mut managed)` after `setup_session()` | Re-apply overrides for JS execution commands |
| `src/navigate.rs` | Call `apply_emulate_state(&mut managed)` after `setup_session()` | Re-apply overrides for navigation commands |
| `src/form.rs` | Call `apply_emulate_state(&mut managed)` after `setup_session()` | Re-apply overrides for form interaction commands |
| `src/interact.rs` | Call `apply_emulate_state(&mut managed)` after `setup_session()` | Re-apply overrides for DOM interaction commands |
| `src/page.rs` | Call `apply_emulate_state(&mut managed)` after `setup_session()` | Re-apply overrides for page commands |
| `src/network.rs` | Call `apply_emulate_state(&mut managed)` after `setup_session()` | Re-apply overrides for network commands |
| `src/console.rs` | Call `apply_emulate_state(&mut managed)` after `setup_session()` | Re-apply overrides for console commands |
| `src/perf.rs` | Call `apply_emulate_state(&mut managed)` after `setup_session()` | Re-apply overrides for performance commands |
| `src/dialog.rs` | Call `apply_emulate_state(&mut managed)` after `setup_session()` | Re-apply overrides for dialog commands |

### Blast Radius

- **Direct impact**: `src/emulate.rs` — struct expansion, new helper function, updated persistence in `execute_set` and reading in `execute_status`
- **Indirect impact**: All 9 command modules (`js.rs`, `navigate.rs`, `form.rs`, `interact.rs`, `page.rs`, `network.rs`, `console.rs`, `perf.rs`, `dialog.rs`) gain a single function call after session setup
- **Risk level**: Medium — touches many files but each change is a single line addition; the helper is self-contained and fails gracefully (missing state file = no-op)

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| State file format change breaks existing persisted state | Low | Use `#[serde(skip_serializing_if = "Option::is_none")]` on all new fields; existing state files without new fields deserialize correctly to `None`/defaults via `#[derive(Default)]` |
| `apply_emulate_state` failure blocks command execution | Low | Make override re-application best-effort: log warnings but don't fail the command if an individual CDP call fails; alternatively, propagate errors since a broken emulation state should be surfaced |
| Network domain enablement conflicts in network.rs | Low | `ensure_domain("Network")` is idempotent — calling it twice (once in apply, once in the command) has no effect |
| Performance overhead from re-applying overrides | Low | File read + 0-7 CDP calls adds ~10-20ms; within the 50ms startup budget only if state exists |
| `emulate set` within same session double-applies | None | `emulate set` applies overrides directly, then persists; `apply_emulate_state` is not called from within `emulate set`'s own session |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Browser-level target attachment | Attach to browser target instead of page target, so overrides persist across page sessions | Requires significant architectural change to CDP client; not all emulation commands work at browser level; higher blast radius |
| Shared session pool | Keep a long-lived CDP session and reuse it across commands | Contradicts chrome-cli's stateless per-invocation design; adds complexity for process management |
| Only persist to file, don't re-apply | Expand `EmulateState` but only use it for `emulate status` reporting | Doesn't actually fix the bug — overrides still wouldn't be active in subsequent commands |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
