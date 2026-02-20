# Root Cause Analysis: navigate back/forward/reload ignores global --timeout option

**Issue**: #145
**Date**: 2026-02-19
**Status**: Draft
**Author**: Claude

---

## Root Cause

The `execute_back`, `execute_forward`, and `execute_reload` functions in `src/navigate.rs` all receive `global: &GlobalOpts` which contains the user-specified `timeout` field (populated from `--timeout` or `CHROME_CLI_TIMEOUT`). However, all three functions pass the hardcoded constant `DEFAULT_NAVIGATE_TIMEOUT_MS` (30,000ms) to their respective wait functions instead of using `global.timeout`.

This is an oversight from the original implementation. The `execute_url` function correctly uses `args.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS)` for its per-command timeout, but the history navigation commands were never wired to any timeout parameter. The `global.timeout` field is correctly used for the CDP `command_timeout` (via the `cdp_config` helper), but it is not forwarded to the navigation event wait logic.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/navigate.rs` | 251 | `execute_back` passes `DEFAULT_NAVIGATE_TIMEOUT_MS` to `wait_for_history_navigation` |
| `src/navigate.rs` | 313 | `execute_forward` passes `DEFAULT_NAVIGATE_TIMEOUT_MS` to `wait_for_history_navigation` |
| `src/navigate.rs` | 342 | `execute_reload` passes `DEFAULT_NAVIGATE_TIMEOUT_MS` to `wait_for_event` |

### Triggering Conditions

- User specifies `--timeout <ms>` or sets `CHROME_CLI_TIMEOUT=<ms>`
- User runs `navigate back`, `navigate forward`, or `navigate reload`
- The navigation event is delayed or missed (e.g., SPA pushState)
- The command waits the full 30 seconds instead of the user-specified value

---

## Fix Strategy

### Approach

Replace the three hardcoded `DEFAULT_NAVIGATE_TIMEOUT_MS` references with `global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS)`. This is the same pattern used by `execute_url` (with `args.timeout` instead of `global.timeout`), so it is consistent with the existing codebase.

The fix is three single-line changes — one per affected function. No new types, functions, or modules are needed.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/navigate.rs:251` | Replace `DEFAULT_NAVIGATE_TIMEOUT_MS` with `global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS)` | `execute_back` respects user timeout |
| `src/navigate.rs:313` | Replace `DEFAULT_NAVIGATE_TIMEOUT_MS` with `global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS)` | `execute_forward` respects user timeout |
| `src/navigate.rs:342` | Replace `DEFAULT_NAVIGATE_TIMEOUT_MS` with `global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS)` | `execute_reload` respects user timeout |

### Blast Radius

- **Direct impact**: Only the three wait calls in `execute_back`, `execute_forward`, and `execute_reload`
- **Indirect impact**: None — the wait functions (`wait_for_history_navigation`, `wait_for_event`) accept `timeout_ms: u64` as a parameter and are unchanged. No callers other than these three are affected.
- **Risk level**: Low — the change only affects the value passed to existing functions, and the fallback preserves the current default behavior when no timeout is specified.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Default timeout changes when no `--timeout` specified | Low | `unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS)` preserves the 30s default; regression test AC5 verifies this |
| `navigate <URL>` per-command `--timeout` regresses | Low | `execute_url` is not modified; regression test AC6 verifies this |
| CDP `command_timeout` conflicts with navigation wait timeout | Low | These are independent timeouts — `cdp_config` sets the CDP protocol timeout, while the wait functions set the event-wait timeout. No interaction. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
