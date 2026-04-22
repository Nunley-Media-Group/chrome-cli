# Tasks: Fix js exec --plain zero-byte output for empty strings

**Issue**: #229
**Date**: 2026-04-22
**Status**: Planning
**Author**: Rich Nunley

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix `--plain` empty-string emission in `execute_exec` and `execute_in_worker` | [ ] |
| T002 | Add regression BDD scenarios + step bindings and unit coverage | [ ] |
| T003 | Verify no regressions (build, unit, clippy, fmt, feature-exercise smoke test) | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/js.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] In `execute_exec` (`src/js.rs`, `--plain` branch around lines 404-413), the `Value::String(s)` arm returns `"\"\""` when `s.is_empty()` and `s.clone()` otherwise.
- [ ] The identical change is applied in `execute_in_worker` (`src/js.rs`, `--plain` branch around lines 569-577).
- [ ] `agentchrome js exec "''" --plain` against a live Chrome produces `""` on stdout (2 bytes, exit code 0).
- [ ] `agentchrome js exec "'hello'" --plain` still produces `hello` on stdout with no added quoting or trailing newline.
- [ ] No unrelated changes in the diff (no touching `emit_plain`, no JSON-mode edits, no refactors).

**Notes**: Follow the minimal-fix strategy from `design.md`. Keep the two call sites byte-identical after the patch so future deduplication is trivial.

### T002: Add Regression Coverage

**File(s)**: `tests/features/229-fix-js-exec-plain-zero-byte-output-for-empty-strings.feature`, `tests/bdd.rs`, `src/js.rs` (inline `#[cfg(test)] mod tests`)
**Type**: Create + Modify
**Depends**: T001
**Acceptance**:
- [ ] New Gherkin feature file exists at `tests/features/229-fix-js-exec-plain-zero-byte-output-for-empty-strings.feature` with every scenario tagged `@regression`.
- [ ] Scenarios map 1:1 to AC1–AC4 in `requirements.md` (empty-string fix, non-empty-string regression guard, non-string-types regression guard, JSON-mode regression guard).
- [ ] Step definitions are added to `tests/bdd.rs` following the existing cucumber-rs patterns for `js exec` scenarios; scenarios that require a live Chrome are gated the same way sibling `js` BDD scenarios are gated today.
- [ ] A unit test in `src/js.rs` covers the plain-mode formatting decision directly (extract the empty-string branch into a small pure helper if that makes the test straightforward, or assert on the computed `text` value via a helper). The test fails if the empty-string special case is reverted.
- [ ] `cargo test --test bdd` and `cargo test --lib` both pass.

### T003: Verify No Regressions

**File(s)**: none (verification only)
**Type**: Verify
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo build` exits 0.
- [ ] `cargo test --lib` exits 0.
- [ ] `cargo clippy --all-targets` exits 0.
- [ ] `cargo fmt --check` exits 0.
- [ ] Feature Exercise Gate: with a fixture at `tests/fixtures/js-exec-plain-empty-string.html` that exposes elements whose `innerText` is `""` and `"hello"`, run the debug binary against headless Chrome and confirm AC1–AC3 outputs match expectations; confirm AC4 by running `--pretty` and default JSON on the empty-string expression and observing `"result": ""` and `"type": "string"`.
- [ ] Orphaned Chrome processes killed (`pkill -f 'chrome.*--remote-debugging' || true`).

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #229 | 2026-04-22 | Initial defect tasks |
