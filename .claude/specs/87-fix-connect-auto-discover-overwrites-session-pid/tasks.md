# Tasks: Connect auto-discover overwrites session PID

**Issue**: #87
**Date**: 2026-02-15
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix `save_session()` to preserve PID from existing session | [ ] |
| T002 | Add regression test (Gherkin + unit) | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `save_session()` reads the existing session before writing
- [ ] If an existing session has a PID and the port matches the new connection, the PID is preserved
- [ ] If the port differs, the PID is not carried forward (prevents stale PID injection)
- [ ] If no existing session exists, behavior is unchanged (`pid` comes from `ConnectionInfo`)
- [ ] Read failure on existing session is non-fatal — falls back to current behavior
- [ ] No unrelated changes in the diff

**Notes**: Modify `save_session()` to call `session::read_session()` and conditionally carry forward the PID. The `ConnectionInfo.pid` takes priority when it is `Some` (i.e., from `--launch`); the existing session PID is only used as a fallback when `ConnectionInfo.pid` is `None` and ports match.

### T002: Add Regression Test

**File(s)**: `tests/features/87-fix-connect-auto-discover-overwrites-session-pid.feature`, `src/session.rs` (unit tests)
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin feature file with `@regression` tag covers AC1 and AC2
- [ ] Unit test in `src/session.rs` validates PID preservation when ports match
- [ ] Unit test validates PID is not preserved when ports differ
- [ ] Tests pass with the fix applied
- [ ] Tests would fail if the fix were reverted (confirms the test catches the bug)

### T003: Verify No Regressions

**File(s)**: existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo test --lib` passes (all unit tests)
- [ ] `cargo test --test bdd` passes (all BDD tests)
- [ ] `cargo clippy` passes with no new warnings
- [ ] No side effects in `save_session()` callers (`execute_connect`, `execute_launch`)

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
