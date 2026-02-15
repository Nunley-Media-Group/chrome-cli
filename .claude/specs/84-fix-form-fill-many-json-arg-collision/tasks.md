# Tasks: form fill-many panics due to 'json' arg name collision

**Issue**: #84
**Date**: 2026-02-15
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

**File(s)**: `src/cli/mod.rs`, `src/form.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `FormFillManyArgs::json` is renamed to `FormFillManyArgs::input` in `src/cli/mod.rs` (line 1582)
- [ ] `args.json` is changed to `args.input` in `src/form.rs` (line 422)
- [ ] `cargo build` succeeds without errors
- [ ] `cargo clippy` passes with no new warnings
- [ ] No unrelated changes included in the diff

**Notes**: Follow the fix strategy from design.md. The `#[arg(value_name = "JSON")]` attribute should remain so help text is unchanged.

### T002: Add Regression Test

**File(s)**: `tests/features/form.feature`, `tests/bdd.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario verifies `form fill-many` accepts a positional JSON argument without panicking
- [ ] Scenario tagged `@regression`
- [ ] Step definitions reuse existing `CliWorld` steps (given chrome-cli is built, when I run, then exit code)
- [ ] Scenario added to `FORM_TESTABLE_SCENARIOS` in `tests/bdd.rs`
- [ ] `cargo test --test bdd` passes for the new scenario

**Notes**: Since the actual fill-many command requires a running Chrome instance, the regression test should verify that the CLI at least parses the arguments without panicking (exit code != 101). A test that runs the command without Chrome should produce a connection error, not a panic.

### T003: Verify No Regressions

**File(s)**: Existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing BDD tests pass: `cargo test --test bdd`
- [ ] All existing unit tests pass: `cargo test --lib`
- [ ] `cargo clippy` reports no errors
- [ ] `form fill-many --help` still shows `--file` and `--include-snapshot` options

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
