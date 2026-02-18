# Root Cause Analysis: Page screenshot --uid fails with 'Could not find node' (regression of #115)

**Issue**: #132
**Date**: 2026-02-17
**Status**: Verified
**Author**: Claude

---

## Root Cause

In `src/page.rs`, `resolve_uid_clip()` used `DOM.describeNode` with a `backendNodeId` to obtain a transient `nodeId`, then passed that `nodeId` to `DOM.getBoxModel`. However, the transient `nodeId` returned by `DOM.describeNode` is not anchored in Chrome's document tree, so `DOM.getBoxModel` fails with "Could not find node with given id".

The fix from issue #115 (PR #124) added `ensure_domain("DOM")` but this alone does not populate the document tree. During verification of this fix, adding `DOM.getDocument` (to mirror `resolve_selector_clip()`) was also proven insufficient — the `nodeId` from `DOM.describeNode` remains unanchored regardless.

The actual solution is simpler: `DOM.getBoxModel` accepts `backendNodeId` directly as a parameter, so the intermediate `DOM.describeNode` step is unnecessary.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/page.rs` | 697-741 | `resolve_uid_clip()` — resolves a UID to a clip region for element screenshots; used `DOM.describeNode` + transient `nodeId` instead of passing `backendNodeId` directly to `DOM.getBoxModel` |

### Triggering Conditions

- User runs `page snapshot` to establish UIDs, then `page screenshot --uid <uid>`
- `resolve_uid_clip()` calls `DOM.describeNode` which returns a transient `nodeId`
- `DOM.getBoxModel` with this transient `nodeId` fails because Chrome cannot find the node in any document tree

---

## Fix Strategy

### Approach

Remove the `ensure_domain("DOM")`, `DOM.getDocument`, and `DOM.describeNode` calls from `resolve_uid_clip()`. Instead, pass the `backendNodeId` (from the snapshot UID map) directly to `DOM.getBoxModel`, which accepts it as a parameter. This eliminates the transient `nodeId` problem entirely and reduces the function from 5 CDP calls to 1.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/page.rs` | Replace `ensure_domain` + `DOM.getDocument` + `DOM.describeNode` + `DOM.getBoxModel(nodeId)` with `DOM.getBoxModel(backendNodeId)` in `resolve_uid_clip()` | `DOM.getBoxModel` accepts `backendNodeId` directly, bypassing the broken transient `nodeId` path |

### Blast Radius

- **Direct impact**: `resolve_uid_clip()` in `src/page.rs` — simplified to fewer CDP calls
- **Indirect impact**: None — `resolve_uid_clip()` is only called from the `page screenshot --uid` code path; `resolve_selector_clip()` and `js exec --uid` paths are unaffected
- **Risk level**: Low — uses a documented CDP parameter (`backendNodeId` on `DOM.getBoxModel`)

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `DOM.getBoxModel` with `backendNodeId` behaves differently | Very Low | This is a documented CDP parameter; verified against real Chrome in smoke tests |
| Breaking `js exec --uid` path | Very Low | That path uses `DOM.resolveNode` in `src/js.rs`, which is completely separate and not modified |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
