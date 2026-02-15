# Root Cause Analysis: tabs create --background does not keep previously active tab focused

**Issue**: #82
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude

---

## Root Cause

The `execute_create` function in `src/tabs.rs` (lines 156–189) correctly passes `"background": true` to Chrome's `Target.createTarget` CDP command when the `--background` flag is set. However, Chrome does not reliably honor this parameter — the newly created tab still becomes the active (focused) tab regardless of the `background` field.

This is a known Chrome DevTools Protocol behavior: the `background` parameter in `Target.createTarget` is advisory, not guaranteed. Chrome's tab activation logic may override it depending on the browser's internal state, platform, or version. The current implementation has no fallback to compensate when Chrome ignores the request.

The fix is straightforward: before creating the new tab, record the currently active tab's `targetId`. After creation, check whether the original tab is still active. If Chrome activated the new tab despite the `background` flag, explicitly re-activate the original tab using `Target.activateTarget` — a CDP command already used by `execute_activate` in the same file (line 250–253).

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/tabs.rs` | 156–189 | `execute_create` — creates tab but does not compensate when Chrome ignores `background` |

### Triggering Conditions

- The `--background` flag is passed to `tabs create`
- Chrome ignores the `background: true` parameter in `Target.createTarget` (always in current Chrome versions)
- No fallback logic exists to re-activate the previously active tab

---

## Fix Strategy

### Approach

When `background` is `true`, the fix adds three steps to `execute_create`:

1. **Before creation**: Query the current targets and identify the first `"page"` target (the active tab) by its `targetId`. This mirrors how `execute_list` determines the active tab (the first page target returned by Chrome's HTTP endpoint is the active one).
2. **Create the tab**: Send `Target.createTarget` as before (still passing `background: true` as a best-effort hint).
3. **After creation**: Re-activate the original tab via `Target.activateTarget` with the saved `targetId`. This is a no-op if Chrome already honored the background flag, and a corrective action if it didn't.

The re-activation step uses `Target.activateTarget`, the same CDP command already used by `execute_activate` (line 250–253). This keeps the fix consistent with existing patterns.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/tabs.rs` | In `execute_create`, when `background` is true: (1) query targets before creation to get the active tab ID, (2) after creation, send `Target.activateTarget` for the original tab | Compensates for Chrome not honoring the `background` parameter |

### Blast Radius

- **Direct impact**: `execute_create` in `src/tabs.rs` — the only function modified
- **Indirect impact**: None. `execute_create` is called only from `execute_tabs` dispatch (line 22). The added CDP calls (`query_targets`, `Target.activateTarget`) are already used elsewhere in the same file. No shared state is modified.
- **Risk level**: Low — the change is additive (extra CDP calls after creation) and only executes when `--background` is true. The non-background path is completely unchanged.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `tabs create` without `--background` changes behavior | None | The new logic is gated on `if background { ... }` — non-background path is untouched |
| Re-activation fails if the original tab was closed between creation and re-activation | Very Low | Edge case: user would have to close the tab in the milliseconds between CDP calls. Not worth guarding against. |
| Extra `query_targets` call adds latency to `--background` creates | Low | One HTTP GET to Chrome's `/json` endpoint adds <10ms. Acceptable for correctness. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Always re-activate after background create | Skip the "check if still active" logic and always send `Target.activateTarget` | **Selected** — simpler, and re-activating an already-active tab is a harmless no-op |
| Remove the `background` parameter from `createTarget` | Since Chrome ignores it anyway | Rejected — keeping it as a hint is correct; future Chrome versions may honor it, and our re-activation is a safe fallback |
| Use CDP events to detect activation change | Subscribe to `Target.targetInfoChanged` to detect if the new tab was activated | Rejected — over-engineered; adds event subscription complexity for a simple sequential fix |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
