# Tasks: Fix perf record --duration reporting incorrect duration_ms

**Issue**: #118
**Date**: 2026-02-16
**Status**: Planning
**Author**: Claude

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix the duration timer placement | [ ] |
| T002 | Add regression test | [ ] |
| T003 | Verify no regressions | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/perf.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `start_time = Instant::now()` is captured in `execute_record()` before `Tracing.start`
- [ ] `stop_and_collect()` accepts a `start_time: Instant` parameter instead of creating its own
- [ ] The local `let start_time = Instant::now();` inside `stop_and_collect()` is removed
- [ ] `duration_ms` in the output reflects the full recording period, not just collection overhead
- [ ] No unrelated changes included in the diff

**Notes**: Follow the fix strategy from design.md. The change is two lines moved + one function signature change. Keep it minimal.

### T002: Add Regression Test

**File(s)**: `tests/features/118-fix-perf-record-duration.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario tests that `perf record --duration 2000` reports `duration_ms` approximately 2000
- [ ] Gherkin scenario tests that `perf record --reload --duration 3000` reports `duration_ms` ≥ 3000
- [ ] Regression scenario verifies output structure (file, duration_ms, size_bytes, vitals)
- [ ] All scenarios tagged `@regression`
- [ ] Step definitions implemented in `tests/bdd.rs`
- [ ] Tests pass with the fix applied

### T003: Verify No Regressions

**File(s)**: existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo test --lib` passes (unit tests)
- [ ] `cargo test --test bdd` passes (BDD tests)
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt --check` passes

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
