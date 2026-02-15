# Tasks: Fix --enable-automation flag missing from Chrome launch

**Issue**: #70
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix the defect | [ ] |
| T002 | Add regression test | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/chrome/launcher.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `--enable-automation` is added to the Chrome command builder at lines 148–152
- [ ] The flag is unconditional (not gated on headless/headed mode)
- [ ] The flag appears after `--no-default-browser-check` and before the headless conditional
- [ ] No unrelated changes included in the diff

**Notes**: Add `.arg("--enable-automation")` to the command builder chain in `launch_chrome()`, right after `.arg("--no-default-browser-check")` on line 152. Follow the fix strategy from design.md.

### T002: Add Regression Test

**File(s)**: `tests/features/70-fix-enable-automation-flag.feature`, `src/chrome/launcher.rs` (unit test)
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin feature file created with scenarios for AC1, AC2, AC3
- [ ] All scenarios tagged `@regression`
- [ ] Unit test added in `launcher.rs` `mod tests` that verifies the command args include `--enable-automation`
- [ ] Tests pass with the fix applied

**Notes**: The Gherkin scenarios serve as documentation and BDD acceptance tests. The unit test in `launcher.rs` provides fast, CI-friendly verification that the flag is present. Since `Command` args aren't directly inspectable post-build, consider testing via a helper or by verifying the launched Chrome process's command line.

### T003: Verify No Regressions

**File(s)**: [existing test files]
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo test` passes (all existing unit and integration tests)
- [ ] `cargo clippy` passes with no new warnings
- [ ] No side effects in the `connect --launch` path (per blast radius from design.md)

---

## Validation Checklist

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
