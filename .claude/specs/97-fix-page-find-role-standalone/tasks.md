# Tasks: page find --role does not work as standalone search criterion

**Issue**: #97
**Date**: 2026-02-15
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix the validation guard and error message | [ ] |
| T002 | Add regression test scenarios | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] Validation guard at line 513 accepts `--role` as a standalone criterion (condition becomes `args.query.is_none() && args.selector.is_none() && args.role.is_none()`)
- [ ] Error message updated to mention `--role` as a valid option (e.g., `"a text query, --selector, or --role is required"`)
- [ ] `page find --role textbox` no longer returns an error
- [ ] `page find` with no arguments still returns an error
- [ ] No unrelated changes in the diff

**Notes**: Follow the fix strategy from design.md. The `search_tree()` function in `snapshot.rs` already handles empty query + role filter correctly — no changes needed there.

### T002: Add Regression Test

**File(s)**: `tests/features/element-finding.feature`, `.claude/specs/97-fix-page-find-role-standalone/feature.gherkin`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario for role-only search added (AC1)
- [ ] Gherkin scenario for empty role result added (AC3)
- [ ] Existing "Combined role and text query" scenario preserved (AC2 — already exists)
- [ ] Existing "Neither query nor selector provided" scenario updated to reflect new error message
- [ ] All scenarios tagged `@regression` in the spec feature file
- [ ] Tests pass with the fix applied

### T003: Verify No Regressions

**File(s)**: existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] All existing tests pass (`cargo test`)
- [ ] `cargo clippy` passes with no new warnings
- [ ] `page find <text>` still works
- [ ] `page find --selector <css>` still works
- [ ] `page find <text> --role <role>` still works
- [ ] `page find` with no arguments still errors

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
