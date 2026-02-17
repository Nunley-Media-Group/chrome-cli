# Tasks: Page screenshot --uid fails with 'Could not find node with given id'

**Issue**: #115
**Date**: 2026-02-16
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix `resolve_uid_clip` to ensure DOM domain | [ ] |
| T002 | Add regression test | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `resolve_uid_clip()` signature changed to accept `&mut ManagedSession`
- [ ] `managed.ensure_domain("DOM").await?` added inside `resolve_uid_clip()` before the `DOM.describeNode` call
- [ ] Redundant `managed.ensure_domain("DOM").await?` removed from the UID branch in `execute_screenshot()` (line 869)
- [ ] `cargo build` succeeds without errors
- [ ] `cargo clippy` passes without warnings

**Notes**: Follow the fix strategy from design.md. The only file changed is `src/page.rs`. The change is two-part: (1) move `ensure_domain` into the function, (2) update the function signature from `&ManagedSession` to `&mut ManagedSession`. The caller already has `&mut` access.

### T002: Add Regression Test

**File(s)**: `tests/features/115-fix-page-screenshot-uid-node-not-found.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin feature file created with scenarios for AC1 (screenshot by UID) and AC2 (js exec by UID)
- [ ] All scenarios tagged `@regression`
- [ ] Step definitions implemented in `tests/bdd.rs`
- [ ] `cargo test --test bdd` passes for the new scenarios

### T003: Verify No Regressions

**File(s)**: (existing test files)
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing tests pass (`cargo test`)
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy` passes
- [ ] No side effects in related code paths (screenshot by selector, js exec by UID)

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
