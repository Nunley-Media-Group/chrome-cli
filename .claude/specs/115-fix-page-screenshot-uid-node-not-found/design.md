# Root Cause Analysis: Page screenshot --uid fails with 'Could not find node with given id'

**Issue**: #115
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude

---

## Root Cause

The `resolve_uid_clip()` function in `src/page.rs` (line 696) uses CDP commands `DOM.describeNode` and `DOM.getBoxModel` to resolve an accessibility UID to a bounding box for element screenshots. These commands require the DOM domain to be active. However, `resolve_uid_clip()` accepts an immutable `&ManagedSession` reference and does not — and cannot — call `managed.ensure_domain("DOM")` (which requires `&mut self`).

The caller `execute_screenshot()` (line 869) does call `managed.ensure_domain("DOM").await?` before invoking `resolve_uid_clip()`. However, because `resolve_uid_clip` only holds an immutable reference, the DOM domain enablement done by the caller can become stale or ineffective. The CDP session may not have the DOM domain in a usable state by the time the inner function issues `DOM.describeNode`, resulting in the "Could not find node with given id" error.

In contrast, `execute_with_uid()` in `src/js.rs` (line 434) takes `&mut ManagedSession` and calls `managed.ensure_domain("DOM").await?` immediately before its own `DOM.resolveNode` call. This pattern works reliably because the domain enablement and the CDP command share the same mutable session context without any intervening operations.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/page.rs` | 696–734 | `resolve_uid_clip()` — resolves UID to bounding box via DOM.describeNode + DOM.getBoxModel |
| `src/page.rs` | 868–872 | `execute_screenshot()` — calls `ensure_domain("DOM")` then `resolve_uid_clip()` |

### Triggering Conditions

- A `page snapshot` has been run, assigning UIDs to DOM elements
- `page screenshot --uid <uid>` is invoked, triggering the `resolve_uid_clip()` code path
- The DOM domain is not in a usable state when `DOM.describeNode` is called inside `resolve_uid_clip()`

---

## Fix Strategy

### Approach

Change `resolve_uid_clip()` to accept `&mut ManagedSession` instead of `&ManagedSession`, and move the `managed.ensure_domain("DOM").await?` call inside the function, immediately before the first DOM CDP command. This follows the same defensive pattern used by `execute_with_uid()` in `src/js.rs:434`.

Remove the now-redundant `managed.ensure_domain("DOM").await?` call from the caller at line 869 in `execute_screenshot()`, since the function itself is now responsible for ensuring the domain is active.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/page.rs:696` | Change `resolve_uid_clip(managed: &ManagedSession, ...)` to `resolve_uid_clip(managed: &mut ManagedSession, ...)` | Allows the function to call `ensure_domain` |
| `src/page.rs:710` | Add `managed.ensure_domain("DOM").await?` before the `DOM.describeNode` call | Ensures DOM domain is active at point of use |
| `src/page.rs:869` | Remove `managed.ensure_domain("DOM").await?` from the UID branch in `execute_screenshot()` | Redundant — `resolve_uid_clip` now handles it internally |

### Blast Radius

- **Direct impact**: `resolve_uid_clip()` in `src/page.rs` — signature change from `&` to `&mut`
- **Indirect impact**: `execute_screenshot()` is the only caller of `resolve_uid_clip()` and already has `&mut ManagedSession`, so the signature change is compatible
- **Risk level**: Low

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `js exec --uid` stops working | Low — no changes to `src/js.rs` | AC2 regression test verifies `js exec --uid` still works |
| Screenshot by CSS selector breaks | Low — `resolve_selector_clip` is not modified | Existing tests cover selector-based screenshots |
| Double DOM.enable call if domain already active | None — `ensure_domain` is idempotent (no-op if already enabled) | Built into `ManagedSession::ensure_domain` at `src/connection.rs:192` |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
