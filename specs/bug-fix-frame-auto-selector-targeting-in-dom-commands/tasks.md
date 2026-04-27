# Tasks: Fix frame auto selector targeting in DOM commands

**Issue**: #275
**Date**: 2026-04-27
**Status**: Planning
**Author**: Codex (write-spec)

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Add selector-aware auto frame resolution | [ ] |
| T002 | Wire DOM select to selector-aware frame auto targeting | [ ] |
| T003 | Add regression coverage | [ ] |
| T004 | Verify no regressions and smoke test | [ ] |

---

### T001: Add Selector-Aware Auto Frame Resolution

**File(s)**: `src/frame.rs`, `src/output.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] Auto-frame target handling distinguishes UID targets from selector targets without changing public CLI syntax
- [ ] Existing UID lookup behavior, including the snapshot-state fast path, remains unchanged
- [ ] Selector auto-search enumerates frames in document order and returns the first frame whose document contains the selector
- [ ] Same-origin frame selector checks use execution-context-based querying rather than the main-frame DOM root
- [ ] Exhausting all frames without a match returns `AppError::element_not_in_any_frame()`

**Notes**: Prefer a typed helper, for example an `AutoFrameTarget` enum, over passing loosely interpreted strings through the shared resolver. Keep the selector search bounded by the existing frame scan limit.

### T002: Wire DOM Select to Selector-Aware Frame Auto Targeting

**File(s)**: `src/dom.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `execute_select` passes its selector argument to frame auto resolution when `--frame auto` is present
- [ ] `dom --frame auto select body` can resolve a child frame before executing the existing DOM selection logic
- [ ] Successful auto-selected DOM output includes frame context for the frame that was selected
- [ ] Explicit `dom --frame 1 select body` behavior is unchanged
- [ ] No unrelated DOM subcommand behavior is changed

**Notes**: Preserve existing large-response output handling. If adding a frame field changes the result shape, scope it to auto-frame output and document it in the regression scenario.

### T003: Add Regression Coverage

**File(s)**: `tests/features/275-fix-frame-auto-selector-targeting-in-dom-commands.feature`, `tests/bdd.rs`, optionally `tests/fixtures/iframe-frame-targeting.html`
**Type**: Create / Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] Gherkin scenarios cover AC1 through AC4 from `requirements.md`
- [ ] Every scenario is tagged `@regression`
- [ ] Existing `tests/fixtures/iframe-frame-targeting.html` is reused unless a smaller deterministic fixture is needed
- [ ] BDD registration follows the existing pattern for Chrome-dependent frame/DOM scenarios
- [ ] Test coverage includes the no-match error contract and the UID auto-targeting preservation case

**Notes**: If the full BDD runner cannot execute Chrome-dependent scenarios in CI, register the feature with a filtered runner and document manual exercise coverage in T004.

### T004: Verify No Regressions and Smoke Test

**File(s)**: existing test files, `tests/fixtures/iframe-frame-targeting.html`
**Type**: Verify (no file changes expected)
**Depends**: T001, T002, T003
**Acceptance**:
- [ ] `cargo fmt --check` passes
- [ ] `cargo test --lib` passes
- [ ] `cargo test --test bdd` passes or reports only intentionally skipped Chrome-dependent scenarios
- [ ] `cargo clippy --all-targets` passes
- [ ] Manual smoke test builds a fresh debug binary, launches headless Chrome, loads an iframe page, runs `dom --frame auto select body`, verifies exit code 0 and frame context, confirms `dom --frame 1 select body`, then disconnects Chrome
- [ ] No orphaned Chrome processes remain after the smoke test

**Notes**: Follow `steering/tech.md` manual smoke-test requirements. Prefer the committed iframe fixture through a `file://` URL for deterministic verification; the issue's public test page can be used as an additional check.

---

## Validation Checklist

Before marking complete:

- [x] Tasks are focused on the fix - no feature work
- [x] Regression test is included
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #275 | 2026-04-27 | Initial defect report |
