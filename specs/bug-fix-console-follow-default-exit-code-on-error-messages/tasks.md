# Tasks: console follow default exit code on error messages

**Issue**: #228
**Date**: 2026-04-22
**Status**: Planning
**Author**: Rich Nunley

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Add `--fail-on-error` flag and flip default exit behavior in `console follow` | [ ] |
| T002 | Add regression BDD scenarios for both default and opt-in modes | [ ] |
| T003 | Verify no regressions in related console/help behavior | [ ] |

---

### T001: Fix the Defect

**File(s)**:
- `src/cli/mod.rs` — `ConsoleFollowArgs` struct and `Follow` subcommand `long_about`/`after_long_help`
- `src/console.rs` — `execute_follow` post-loop exit decision
- `src/examples/commands.rs` — add `--fail-on-error` example entry
- `README.md`, `docs/claude-code.md` — narrative updates where behavior is described

**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `ConsoleFollowArgs` gains `pub fail_on_error: bool` with `#[arg(long)]`, defaulting to `false`
- [ ] `execute_follow` returns `AppError` only when `args.fail_on_error && saw_errors` is true; otherwise returns `Ok(())` on timeout / Ctrl+C
- [ ] Help text for `console follow --help` describes both default monitoring (exit 0) and `--fail-on-error` assertion mode (exit 1) with a worked example for each
- [ ] `examples` subcommand includes a `console follow --fail-on-error --timeout <ms>` entry with a short description
- [ ] `README.md` and `docs/claude-code.md` narrative text no longer implies `console follow` fails by default on `console.error`
- [ ] Bug reproduction from `requirements.md` no longer reproduces: `console follow --timeout 3000` returns exit 0 when `console.error` is observed
- [ ] No unrelated changes included in the diff

**Notes**: Keep the `AppError` message and exit code exactly as today (`"Error-level console messages were seen"`, `ExitCode::GeneralError`) so AC2's contract remains byte-identical for `--fail-on-error` callers.

### T002: Add Regression Test

**File(s)**:
- `tests/features/228-console-follow-default-exit-code.feature` — new scenarios tagged `@regression`
- `tests/bdd.rs` — register the new scenarios (follow existing pattern for `Console follow help shows all flags` and similar help-only scenarios)
- Step definitions in `tests/bdd.rs` if any new step phrasing is required (reuse existing `I run "..."`, `the exit code should be...`, `stdout should contain`, `stderr should contain` steps)

**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenarios cover AC1 (default exit 0 despite `console.error`), AC2 (`--fail-on-error` preserves exit 1 and JSON stderr), and AC3 (help documents both modes)
- [ ] All three scenarios tagged `@regression`
- [ ] Chrome-dependent scenarios (AC1, AC2) use the project's existing Chrome-required pattern — match how `tests/features/console.feature` gates streaming scenarios. If executing live streaming scenarios is not part of the current BDD harness, leave them as commented `# Scenario:` blocks following the same convention as the rest of `console.feature`, and keep AC3 (help-only, no Chrome) as the live regression gate.
- [ ] AC3 help-only scenario runs in the BDD harness and passes with the fix applied
- [ ] AC3 help-only scenario fails if T001's help-text change is reverted (confirms it catches the regression)
- [ ] Step definitions reuse existing patterns in `tests/bdd.rs` — no new step crates or helpers introduced

### T003: Verify No Regressions

**File(s)**: existing test files under `tests/features/` and `tests/bdd.rs`

**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo test` passes
- [ ] Existing `Console follow help shows all flags` scenario still passes (now additionally asserting `--fail-on-error`)
- [ ] `103-fix-console-read-empty-array.feature` and `146-console-read-runtime-messages.feature` regression scenarios for `console follow --timeout 2000` still pass
- [ ] `console read`, `console read --errors-only`, and one-shot console paths are unchanged (no behavior drift per design blast-radius table)
- [ ] Manual smoke: reproduce the issue steps from `requirements.md` — observe exit 0 without flag and exit 1 with `--fail-on-error`

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #228 | 2026-04-22 | Initial defect report |
