# Tasks: Script bind stores raw command output envelope for `js exec`

**Issues**: #248
**Date**: 2026-04-23
**Status**: Planning
**Author**: Rich Nunley

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Unwrap `js exec` envelope at the bind site in the script runner | [x] |
| T002 | Add `@regression` BDD scenarios + fixture for the bind contract | [x] |
| T003 | Verify no regressions in adjacent bind behaviour | [x] |

---

### T001: Fix the Defect — Unwrap `js exec` envelope at bind site

**File(s)**: `src/script/runner.rs` (around lines 128–134)
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] When `cmd_step.cmd` starts with `["js", "exec", ...]` and the returned `value` is a JSON object containing a `"result"` key, `ctx.bind(bind_name, ...)` receives `value["result"]` rather than the full envelope.
- [ ] For all other commands (`navigate`, `page find`, `page text`, `page screenshot`, …), the bound value is unchanged — still the raw value returned by `invoke(...)`.
- [ ] `ctx.set_prev(value.clone())` continues to receive the **full** value (the envelope) so the step's own result shape in `StepResult.output` is preserved for logs/reporting.
- [ ] `agentchrome js exec ...` invoked outside a script produces identical stdout JSON to before (envelope unchanged on the wire).
- [ ] `cargo fmt`, `cargo clippy`, and `cargo build` are clean.

**Notes**: Keep the change localized — one `if let` or `match` guard before the existing `ctx.bind(...)` call. Do not modify `src/js.rs` or `src/script/context.rs`.

### T002: Add Regression Tests

**File(s)**: `tests/features/batch-script-execution.feature`, `tests/bdd.rs` (embedded script fixtures section around lines 490–566)
**Type**: Modify + Create
**Depends**: T001
**Acceptance**:
- [ ] A new `@regression` scenario covers AC1: `js exec` of `document.title` bound to `t`, then an `if` step using `$vars.t.includes('…')` — asserts the branch is taken and no `TypeError` is raised.
- [ ] A new `@regression` scenario covers AC2: `js exec` returning an object literal, bound to `obj`, asserts a downstream reference to `$vars.obj.<field>` resolves correctly (object is bound directly, not wrapped in an envelope).
- [ ] A new `@regression` scenario covers AC3: an existing-style `page find` + `$vars.match[0].uid` chain continues to resolve identically.
- [ ] A new fixture file is embedded in `tests/bdd.rs` following the existing pattern (`simple.json`, `conditional.json`, `page-find.json` at lines 490–566).
- [ ] `cargo test --test bdd` passes with T001 applied; at least one of the new scenarios demonstrably **fails** when T001 is reverted (confirms the test catches the bug).

**Notes**: Reuse the existing BDD step vocabulary — do not introduce new step definitions unless strictly required. Tag every new scenario `@regression` per the defect-variant contract.

### T003: Verify No Regressions

**File(s)**: existing tests only (no file changes)
**Type**: Verify
**Depends**: T001, T002
**Acceptance**:
- [ ] Full `cargo test` suite passes.
- [ ] Full `cargo test --test bdd` suite passes, including all pre-existing batch-script-execution scenarios (AC17–AC20).
- [ ] Manual smoke: `agentchrome js exec "1+1"` on the CLI still emits `{"result":2,"type":"number","truncated":false}` (envelope preserved outside scripts).
- [ ] `grep` of `tests/` and `docs/` confirms no remaining test expectations of the old envelope-as-bind-value shape.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #248 | 2026-04-23 | Initial defect spec |

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (`src/script/runner.rs`, `tests/features/`, `tests/bdd.rs`)
