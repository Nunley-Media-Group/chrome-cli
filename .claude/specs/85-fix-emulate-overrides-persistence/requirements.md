# Defect Report: Emulate Set Overrides Do Not Persist Across Commands

**Issue**: #85
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude
**Severity**: High
**Related Spec**: `.claude/specs/21-device-network-viewport-emulation/`

---

## Reproduction

### Steps to Reproduce

1. Launch Chrome: `chrome-cli connect --launch`
2. Set user agent: `chrome-cli emulate set --user-agent "TestBot/1.0" --pretty`
3. Navigate: `chrome-cli navigate "https://www.google.com" --pretty`
4. Verify: `chrome-cli js exec "navigator.userAgent" --pretty`

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (Darwin 25.2.0) |
| **Version / Commit** | chrome-cli 0.1.0 |
| **Browser / Runtime** | Chrome via CDP WebSocket |
| **Configuration** | Default; state file at `~/.chrome-cli/emulate-state.json` |

### Frequency

Always — every cross-command invocation loses the override.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `navigator.userAgent` returns `"TestBot/1.0"` after setting the override in a prior command. Similarly, device scale factor, geolocation, and color scheme overrides persist and are visible to subsequent commands. |
| **Actual** | `navigator.userAgent` returns the original Chrome user agent string. All session-scoped CDP overrides (user-agent, device scale factor, geolocation, color scheme) are lost when the `emulate set` command's CDP session closes. Subsequent commands create fresh sessions without these overrides. |

### Error Output

No error is produced. The command silently succeeds but the override is not in effect for subsequent commands.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: User Agent Persists Across Commands

**Given** I have set `emulate set --user-agent "TestBot/1.0"`
**When** I run `js exec "navigator.userAgent"` in a subsequent command
**Then** the result is `"TestBot/1.0"`

### AC2: Device Scale Factor Persists and Is Reported Correctly

**Given** I have set `emulate set --device-scale 2`
**When** I run `emulate status`
**Then** `deviceScaleFactor` shows `2.0`

### AC3: Geolocation Is Shown in Status

**Given** I have set `emulate set --geolocation "37.7749,-122.4194"`
**When** I run `emulate status`
**Then** the geolocation override is included in the output with `latitude: 37.7749` and `longitude: -122.4194`

### AC4: Reset Clears All Overrides

**Given** overrides are set for user-agent, viewport, geolocation, and color-scheme
**When** I run `emulate reset`
**Then** all overrides are cleared and subsequent commands use Chrome defaults

### AC5: Existing Mobile/Network/CPU Persistence Still Works

**Given** I have set `emulate set --mobile --network slow-4g --cpu 4`
**When** I run `emulate status` in a subsequent command
**Then** `mobile`, `network`, and `cpu` values are reported correctly

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Expand `EmulateState` to include `user_agent`, `device_scale_factor`, `geolocation`, `color_scheme`, and `viewport` fields | Must |
| FR2 | Persist all emulation overrides to `~/.chrome-cli/emulate-state.json` on `emulate set` | Must |
| FR3 | Re-apply persisted overrides when creating new CDP sessions in all command modules | Must |
| FR4 | Report all active overrides (including geolocation) in `emulate status` | Must |
| FR5 | `emulate reset` clears all persisted overrides including new fields | Must |

---

## Out of Scope

- Making CDP sessions persistent (Chrome limitation)
- Per-tab emulation overrides
- Browser-level target attachment as an alternative approach
- Refactoring `setup_session()` into a shared module (each command module has its own copy by design)

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC5)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
