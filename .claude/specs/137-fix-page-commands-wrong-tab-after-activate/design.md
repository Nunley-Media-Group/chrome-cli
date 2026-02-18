# Root Cause Analysis: Page commands target wrong tab after tabs activate

**Issue**: #137
**Date**: 2026-02-17
**Status**: Draft
**Author**: Claude

---

## Root Cause

The bug has two contributing factors that combine to create unreliable behavior across CLI invocations:

**1. No cross-invocation state for active tab.** Each `chrome-cli` command is a separate OS process. When `tabs activate <id>` runs, it sends `Target.activateTarget` via CDP and polls `/json/list` until the activated tab appears first. But after the process exits and the WebSocket closes, there is no mechanism to communicate which tab was activated to the next CLI invocation. The session file (`~/.chrome-cli/session.json`) stores `ws_url`, `port`, `pid`, and `timestamp` — but not which tab is active.

**2. Default target selection relies on `/json/list` ordering.** In `resolve_target()`, when `--tab` is not specified, `select_target()` picks the first target with `target_type == "page"` from Chrome's `/json/list` HTTP endpoint. This endpoint does not reliably reflect CDP-driven activation changes across process boundaries, especially in headless mode. The result is that page commands attach a CDP session to whichever tab Chrome happens to list first — which may be `about:blank`, the previously active tab, or an unrelated tab.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/connection.rs` | 120–143 | `select_target()` — picks first page-type target when `tab` is `None` |
| `src/connection.rs` | 150–157 | `resolve_target()` — calls `query_targets()` then `select_target()` |
| `src/session.rs` | 7–14 | `SessionData` — lacks `active_tab_id` field |
| `src/tabs.rs` | 294–333 | `execute_activate()` — does not persist the activated tab ID |
| `src/main.rs` | 318–337 | `save_session()` — constructs `SessionData` without `active_tab_id` |

### Triggering Conditions

- Multiple tabs are open
- `tabs activate <id>` is run, then a page command is run as a separate CLI invocation
- Chrome's `/json/list` has not updated its ordering (or has reordered) between invocations
- No `--tab` flag is provided to the page command

---

## Fix Strategy

### Approach

Persist the activated tab's target ID in the session file and prefer it when resolving the default target. This is a minimal, data-flow-only change: `tabs activate` writes `active_tab_id` into `SessionData`, and `resolve_target()` reads it back as the preferred default when `--tab` is absent. If the persisted target no longer exists (tab was closed), the existing first-page heuristic serves as a graceful fallback.

This approach avoids modifying `select_target()` (which remains a pure function for testability) and instead adds the session-aware logic to `resolve_target()`, which already performs I/O.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/session.rs` | Add `active_tab_id: Option<String>` to `SessionData` with `#[serde(skip_serializing_if = "Option::is_none", default)]` | Persists the activated tab ID across invocations. `Option` + `skip_serializing_if` + `default` ensures backward/forward compatibility — existing session files without the field deserialize with `None`, and new session files omit it when unset. |
| `src/tabs.rs` | In `execute_activate()`, after successful activation, read the session file, set `active_tab_id` to the activated target's ID, and write it back | Writes the activated tab ID so the next CLI invocation can read it |
| `src/connection.rs` | In `resolve_target()`, when `tab` is `None`, check the session file for `active_tab_id`. If found and the target exists in the target list, use it. Otherwise fall back to the existing first-page heuristic. | Reads the persisted active tab ID and uses it as the default target |
| `src/main.rs` | In `save_session()`, preserve `active_tab_id` from the existing session when reconnecting to the same port (same pattern as PID preservation) | Prevents `connect` commands from clearing the active tab state |

### Blast Radius

- **Direct impact**: `session.rs` (struct change), `tabs.rs` (write active tab), `connection.rs` (read active tab), `main.rs` (preserve on reconnect)
- **Indirect impact**: All commands that call `resolve_target()` — `page.rs`, `navigate.rs`, `js.rs`, `console.rs`, `network.rs`, `emulate.rs`, `interact.rs`, `dialog.rs`, `perf.rs`, `form.rs`. These all benefit from the fix without any code changes, since they all delegate to `resolve_target()`.
- **Risk level**: Low — the change only affects the `tab == None` path (no `--tab` flag). Explicit `--tab` usage is completely unaffected. Backward compatibility is maintained by serde defaults.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Stale `active_tab_id` points to closed tab | Low | `resolve_target()` falls back to first-page heuristic when persisted target is not found in target list |
| `save_session()` clears `active_tab_id` on reconnect | Low | Preserve from existing session when reconnecting to same port (same pattern as PID preservation) |
| Session file format change breaks old CLI versions | Low | `active_tab_id` is optional with `skip_serializing_if` — old versions will silently ignore the unknown field |
| `--tab` flag behavior changes | None | Explicit `--tab` takes precedence unconditionally; `active_tab_id` is only consulted when `tab` is `None` |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Use CDP `document.visibilityState` query in `resolve_target()` | Query each target's visibility state to find the active tab (similar to `tabs list` approach) | Requires establishing a CDP session per target and evaluating JS in each — adds latency and complexity to every command invocation. Session file approach is zero-overhead for the common case. |
| Store active tab in a separate file (not session.json) | Write active tab ID to a dedicated file like `~/.chrome-cli/active-tab` | Unnecessary fragmentation — `SessionData` already exists for cross-invocation state. Adding a field is simpler and atomic. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
