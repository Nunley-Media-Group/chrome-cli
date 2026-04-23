# Tasks: Fix form fill-many field name (`uid` → `target`)

**Issues**: #246
**Date**: 2026-04-23
**Status**: Planning
**Author**: Rich Nunley

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Rename `FillEntry.uid` → `target` with `uid` serde alias; update error, help, and examples | [ ] |
| T002 | Add regression scenarios covering `target`, `uid` alias, and help/error copy | [ ] |
| T003 | Verify no regressions in the broader `form fill` / `fill-many` flows | [ ] |

---

### T001: Rename `FillEntry` field and update all user-visible copy

**File(s)**:
- `src/form.rs` (struct `FillEntry` at 63–68; `execute_fill_many` at 782–820; unit tests at 1423–1450)
- `src/cli/mod.rs` (top-level example at 448; `FillMany` clap metadata at 2760–2773; `FormFillManyArgs.input` doc at 2852)

**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `FillEntry` has `#[serde(alias = "uid")] target: String` (field renamed, alias preserved).
- [ ] Error at `src/form.rs:783` reads `expected array of {target, value} objects`.
- [ ] `FillMany` `long_about` and `after_long_help` examples use `target`; trailing note acknowledges `uid` still works.
- [ ] `src/cli/mod.rs:448` top-level example uses `{"target": ..., "value": ...}`.
- [ ] Call sites `entry.uid` → `entry.target` throughout `execute_fill_many`.
- [ ] `cargo build` and `cargo clippy --all-targets` pass.

**Notes**: Apply the minimal fix from design.md §Fix Strategy. Do not touch `FillResult`, `resolve_target_to_backend_node_id`, or any other `form` subcommand.

### T002: Add regression scenarios

**File(s)**:
- `tests/features/246-fix-fill-many-target-field-name.feature` (new)
- `tests/bdd.rs` / step definitions as needed (follow pattern of `tests/features/136-fix-form-fill-textarea.feature` and the step glue in `tests/bdd.rs`)
- `src/form.rs` (extend the existing `FillEntry` deserialization unit tests at 1423–1450)

**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] New `.feature` file is tagged `@regression` and covers AC1 (`target` accepted), AC2 (`uid` alias accepted), and AC3 (help + error copy reference `target`).
- [ ] Unit test: `[{"target":"s1","value":"John"}]` deserializes with `target == "s1"`.
- [ ] Unit test: `[{"uid":"s1","value":"John"}]` still deserializes (alias) with `target == "s1"`.
- [ ] Unit test: malformed payload returns a `serde_json` error; BDD step asserts the wrapped CLI error message contains `target`.
- [ ] `cargo test` passes with the fix; the new unit assertions fail if the serde alias is removed.

**Notes**: Mirror the structure of `specs/bug-fix-form-fill-many-json-arg-collision/feature.gherkin` — process-level assertions against `agentchrome form fill-many --help` and stderr of an invalid payload.

### T003: Verify no regressions

**File(s)**: [existing tests — no file changes]
**Type**: Verify
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo test` (full suite) passes.
- [ ] `cargo test --test bdd` passes, including pre-existing `fill-many` JSON-arg-collision and form-fill scenarios.
- [ ] `agentchrome examples form` output reviewed: any lingering `uid` wording in strategies/examples is either unrelated (e.g., `form fill <uid>` copy for `form fill` itself is fine) or updated.

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
