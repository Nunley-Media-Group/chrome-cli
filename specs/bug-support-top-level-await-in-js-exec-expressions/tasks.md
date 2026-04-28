# Tasks: Support top-level await in js exec expressions

**Issue**: #279
**Date**: 2026-04-27
**Status**: Planning
**Author**: Codex (write-spec)

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix expression evaluation params for top-level await | [ ] |
| T002 | Add regression BDD scenarios, step bindings, and focused unit coverage | [ ] |
| T003 | Verify no regressions with focused Rust and live-browser checks | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/js.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `execute_expression_with_context` includes `replMode: true` in the params passed to `Runtime.evaluate`.
- [ ] The existing block wrapper (`{ <code> }`) remains in place so `let` and `const` declarations stay scoped to each invocation.
- [ ] Existing params remain intact: `returnByValue`, `awaitPromise`, `generatePreview`, and optional `contextId`.
- [ ] The fix is made in the shared expression helper rather than duplicating logic at primary, frame, worker, or script-runner call sites.
- [ ] No unrelated changes are made to code-source resolution, `--uid` function execution, console capture, truncation, plain/JSON output formatting, or error handling.

**Notes**: If adding unit coverage is easier with a pure helper, extract the params construction into a small function such as `runtime_evaluate_params(code, await_promise, context_id)`. Keep the helper private to `src/js.rs`.

### T002: Add Regression Coverage

**File(s)**: `tests/features/279-support-top-level-await-in-js-exec-expressions.feature`, `tests/bdd.rs`, `src/js.rs`
**Type**: Create + Modify
**Depends**: T001
**Acceptance**:
- [ ] New Gherkin feature file exists at `tests/features/279-support-top-level-await-in-js-exec-expressions.feature` with every scenario tagged `@regression`.
- [ ] Scenarios map 1:1 to AC1-AC3 in `requirements.md`.
- [ ] Step definitions in `tests/bdd.rs` follow the existing cucumber-rs patterns and can execute the debug binary against a connected Chrome instance for the success-path assertions.
- [ ] A focused unit test in `src/js.rs` asserts the `Runtime.evaluate` params include `replMode: true`, preserve `awaitPromise`, preserve `returnByValue`, and retain `contextId` when supplied.
- [ ] The regression scenario for direct top-level await fails before T001 and passes after T001.

### T003: Verify No Regressions

**File(s)**: none (verification only)
**Type**: Verify
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo fmt --check` exits 0.
- [ ] `cargo test --lib js::tests` exits 0, or the nearest focused unit-test command for `src/js.rs` exits 0.
- [ ] `cargo test --test bdd -- 279-support-top-level-await-in-js-exec-expressions` exits 0, or the repository-supported equivalent focused BDD filter exits 0.
- [ ] Existing JavaScript execution BDD coverage for promise awaiting and scope isolation still passes.
- [ ] Manual exercise with the debug binary against headless Chrome confirms:
  - `./target/debug/agentchrome js exec 'await Promise.resolve("done")' --pretty` exits 0 and returns `"done"`.
  - `./target/debug/agentchrome js exec 'new Promise(r => setTimeout(() => r("done"), 100))' --pretty` still exits 0 and returns `"done"`.
  - Two consecutive same-name `let` or `const` declarations still exit 0.
- [ ] No orphaned Chrome processes are left running after the live-browser exercise.

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix - no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #279 | 2026-04-27 | Initial defect tasks |
