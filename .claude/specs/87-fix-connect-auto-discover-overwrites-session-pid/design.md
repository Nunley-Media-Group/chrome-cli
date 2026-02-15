# Root Cause Analysis: Connect auto-discover overwrites session PID

**Issue**: #87
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude

---

## Root Cause

When `connect --launch` runs, it creates a `ConnectionInfo` with `pid: Some(pid)` and saves it to `~/.chrome-cli/session.json` via `save_session()`. This works correctly.

When a subsequent `connect` (auto-discover) runs, it calls `discover_chrome()` and constructs a new `ConnectionInfo` with `pid: None` hardcoded at `src/main.rs:330`. This new session data is then passed to `save_session()`, which unconditionally overwrites the session file. The PID from the original launch is lost because `save_session()` has no merge/preserve logic — it always creates a fresh `SessionData` from the `ConnectionInfo` it receives.

When `connect --disconnect` later runs, it reads the session file, finds `pid: None`, and skips the process kill. The Chrome process is orphaned.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/main.rs` | 324–334 | Auto-discover branch constructs `ConnectionInfo` with `pid: None` |
| `src/main.rs` | 278–289 | `save_session()` blindly writes what it receives — no merge with existing session |
| `src/main.rs` | 306–317 | Strategy 1 (direct WS URL) has the same pattern — also hardcodes `pid: None` |

### Triggering Conditions

- A session file exists with a non-`None` PID (from a prior `--launch`)
- A `connect` call (auto-discover or direct WS URL) reaches one of the code paths that constructs `ConnectionInfo` with `pid: None`
- The same Chrome instance is still running on the same port

---

## Fix Strategy

### Approach

Add a helper function that reads the existing session before writing a new one. If the existing session has a PID and the port matches the new connection's port, carry the PID forward into the new `SessionData`. This keeps the fix minimal — only the session write path changes; no new CLI flags, no new data structures.

The fix targets `save_session()` in `src/main.rs`. Before constructing the `SessionData`, it reads the existing session via `session::read_session()`. If the existing session has a PID and the port matches, the PID is preserved. Otherwise, the incoming `ConnectionInfo.pid` is used (which is `None` for auto-discover, `Some(pid)` for launch).

Strategy 1 (direct WS URL, lines 306–317) has the same `pid: None` pattern but is lower priority since users explicitly providing a WS URL are unlikely to have launched Chrome via `--launch`. However, the fix naturally covers this path too since it modifies `save_session()`.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/main.rs` | Modify `save_session()` to read existing session and preserve PID when ports match | Fixes the root cause — PID is carried forward across reconnections |
| `src/session.rs` | Add unit test for PID preservation round-trip | Validates the fix at the session layer |

### Blast Radius

- **Direct impact**: `save_session()` in `src/main.rs` (called from 3 places: WS URL strategy, auto-discover, and launch)
- **Indirect impact**: `execute_disconnect()` benefits — PID is now available for process kill
- **Risk level**: Low — the change adds an optional read-before-write; if the read fails or returns `None`, behavior is unchanged

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| PID from a stale session carried into a connection to a different Chrome instance | Low | Port-matching guard: PID only preserved when ports match |
| Read failure on existing session blocks new session write | Low | Read failure is non-fatal — fall back to the incoming `ConnectionInfo.pid` (same as current behavior) |
| Fresh `connect` (no prior session) breaks | Very Low | `read_session()` returns `Ok(None)` for missing files — no PID to preserve, no change in behavior |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Store PID in a separate file (`pid.lock`) | Separate PID from session data so overwrites don't lose it | Over-engineered for this fix; adds file management complexity and doesn't follow existing patterns |
| Merge at the `write_session_to` level in `session.rs` | Make the session writer itself merge with existing data | Violates single-responsibility — the writer should write what it's given; the caller should decide what to write |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
