# Tasks: Fix network list showing size 0 for most requests

**Issue**: #117
**Date**: 2026-02-16
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

**File(s)**: `src/network.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] A `resolve_size` helper function is added that takes `encoded_data_length: Option<u64>` and `response_headers: &serde_json::Value` and returns `Option<u64>`
- [ ] The helper returns `encoded_data_length` when it is `Some(n)` where `n > 0`
- [ ] The helper falls back to parsing `content-length` from `response_headers` when `encoded_data_length` is 0 or `None`
- [ ] The helper uses case-insensitive header lookup for `content-length`
- [ ] `builder_to_summary` (line ~773) calls `resolve_size` instead of directly using `builder.encoded_data_length`
- [ ] `execute_get` detail construction (line ~980) calls `resolve_size` instead of directly using `builder.encoded_data_length`
- [ ] Follow mode (line ~1125) calls `resolve_size` with the `InFlightRequest.response_headers` before passing size to `emit_stream_event`
- [ ] No unrelated changes included in the diff

**Notes**: Follow the fix strategy from design.md. CDP normalizes header names to lowercase, but use case-insensitive matching as a safety measure. Use `str::parse::<u64>().ok()` for safe `content-length` parsing.

### T002: Add Regression Test

**File(s)**: `tests/features/117-fix-network-list-size-zero.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Gherkin scenario reproduces the original bug condition (size 0 fallback to content-length)
- [ ] Scenario verifying non-zero `encodedDataLength` is preserved (regression guard)
- [ ] Scenario for detail view (`network get`) size fallback
- [ ] All scenarios tagged `@regression`
- [ ] Step definitions implemented in `tests/bdd.rs`
- [ ] Tests pass with the fix applied

### T003: Verify No Regressions

**File(s)**: Existing test files
**Type**: Verify (no file changes)
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo test --test bdd` passes (all BDD tests)
- [ ] `cargo test --lib` passes (all unit tests)
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo fmt --check` passes
- [ ] No side effects in related code paths (network list, get, follow)

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
