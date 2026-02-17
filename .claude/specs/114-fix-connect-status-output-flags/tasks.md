# Tasks: Fix connect --status ignoring --pretty and --plain output format flags

**Issue**: #114
**Date**: 2026-02-16
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix `execute_status()` to respect output format flags | [ ] |
| T002 | Add regression test | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `execute_status()` checks `global.output.pretty` and uses `serde_json::to_string_pretty()` when true
- [ ] `execute_status()` checks `global.output.plain` and outputs human-readable key-value text when true
- [ ] Default (no flag) output remains compact single-line JSON via `serde_json::to_string()`
- [ ] A `format_plain_status()` helper produces human-readable text for `StatusInfo`
- [ ] No unrelated changes included in the diff
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy` passes

**Notes**: Follow the pattern in `src/tabs.rs` — `print_output()` (line 67) for pretty/compact JSON, and `format_plain_table()` (line 82) for plain text. Add an equivalent local `print_output()` and `format_plain_status()` in `main.rs`. The plain text format should show key-value pairs for each `StatusInfo` field (ws_url, port, pid, timestamp, reachable).

### T002: Add Regression Test

**File(s)**: `tests/features/114-fix-connect-status-output-flags.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin feature file covers AC1 (pretty JSON), AC2 (plain text), and AC3 (default compact JSON)
- [ ] All scenarios tagged `@regression`
- [ ] Step definitions implemented in `tests/bdd.rs`
- [ ] Tests pass with the fix applied
- [ ] Tests would fail if the fix were reverted (confirms they catch the bug)

### T003: Verify No Regressions

**File(s)**: Existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing tests pass (`cargo test`)
- [ ] No side effects in related code paths (only `execute_status()` modified; `print_json()` and other callers unchanged)

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
