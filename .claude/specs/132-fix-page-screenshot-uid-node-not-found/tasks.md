# Tasks: Page screenshot --uid fails with 'Could not find node' (regression of #115)

**Issue**: #132
**Date**: 2026-02-17
**Status**: Complete
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix the defect | [x] |
| T002 | Add regression test | [x] |
| T003 | Manual smoke test | [x] |
| T004 | Verify no regressions | [x] |

---

### T001: Fix the Defect

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [x] `resolve_uid_clip()` passes `backendNodeId` directly to `DOM.getBoxModel` instead of using `DOM.describeNode` + transient `nodeId`
- [x] Bug no longer reproduces using the steps from requirements.md
- [x] No unrelated changes included in the diff

**Notes**: Removed `ensure_domain("DOM")`, `DOM.getDocument`, and `DOM.describeNode` calls. Pass `backendNodeId` from the snapshot UID map directly to `DOM.getBoxModel`, which accepts it as a parameter. The original approach (adding `DOM.getDocument`) was proven insufficient during verification — the transient `nodeId` from `DOM.describeNode` is not anchored in the document tree regardless.

### T002: Add Regression Test

**File(s)**: `tests/features/132-fix-page-screenshot-uid-node-not-found.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [x] Gherkin scenario verifies `backendNodeId` is passed directly to `DOM.getBoxModel` and `DOM.describeNode` is not used
- [x] Scenario tagged `@regression`
- [x] Step definitions implemented in `tests/bdd.rs`
- [x] Test passes with the fix applied
- [x] Chrome-dependent AC scenarios documented (commented out, consistent with project pattern)

### T003: Manual Smoke Test

**File(s)**: N/A (manual verification)
**Type**: Verify (no file changes)
**Depends**: T001
**Acceptance**:
- [x] Build debug binary: `cargo build`
- [x] Launch headless Chrome: `./target/debug/chrome-cli connect --launch --headless`
- [x] Navigate to a page: `./target/debug/chrome-cli navigate https://www.google.com`
- [x] Run snapshot: `./target/debug/chrome-cli page snapshot`
- [x] Run screenshot by UID: `./target/debug/chrome-cli page screenshot --uid s1 --file /tmp/element.png` — succeeds (36x16 PNG)
- [x] Run js exec by UID: `./target/debug/chrome-cli js exec --uid s1 "(el) => el.tagName"` — succeeds (returns "A")
- [x] SauceDemo smoke test: navigate to https://www.saucedemo.com/, run `page snapshot`, screenshot Login button by UID (450x21 PNG)
- [x] Disconnect and kill orphaned Chrome processes

### T004: Verify No Regressions

**File(s)**: Existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [x] `cargo test --test bdd` passes
- [x] `cargo test --lib` passes (141 tests)
- [x] `cargo clippy` passes with no errors
- [x] `cargo fmt --check` passes
- [x] No side effects in `resolve_selector_clip()` or `js exec --uid` paths

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
