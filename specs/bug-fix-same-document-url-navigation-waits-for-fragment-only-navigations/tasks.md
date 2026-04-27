# Tasks: Fix same-document URL navigation waits for fragment-only navigations

**Issue**: #277
**Date**: 2026-04-27
**Status**: Planning
**Author**: Codex (write-spec)

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Add same-document completion handling to URL navigation waits | [ ] |
| T002 | Keep URL navigation helper callers in sync | [ ] |
| T003 | Add regression coverage | [ ] |
| T004 | Verify no regressions and smoke test | [ ] |

---

### T001: Add Same-Document Completion Handling to URL Navigation Waits

**File(s)**: `src/navigate.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `load` and `domcontentloaded` URL waits subscribe to `Page.navigatedWithinDocument` before `Page.navigate`
- [ ] A fragment-only same-document URL navigation resolves on `Page.navigatedWithinDocument` instead of timing out while waiting for `Page.loadEventFired` or `Page.domContentEventFired`
- [ ] Cross-document URL navigation still resolves on the selected load-style event
- [ ] Existing timeout behavior remains when neither the selected load-style event nor same-document completion arrives
- [ ] `networkidle` and `none` paths are unchanged

**Notes**: Prefer a small URL-specific helper over broad changes to `wait_for_event`. Preserve error messages and exit code 4 for real timeouts.

### T002: Keep URL Navigation Helper Callers in Sync

**File(s)**: `src/navigate.rs`, `src/diagnose/mod.rs`, `src/script/dispatch.rs`
**Type**: Modify / Verify
**Depends**: T001
**Acceptance**:
- [ ] `execute_url` and `navigate_and_wait` share the same same-document completion behavior, either through a shared helper or by routing `execute_url` through `navigate_and_wait`
- [ ] `agentchrome diagnose <url>` URL mode continues to propagate navigation errors and output shape correctly
- [ ] Script-runner `navigate` URL commands continue to return `{ "url", "title", "status" }`
- [ ] `--ignore-cache` and `--wait-for-selector` behavior from top-level `agentchrome navigate <url>` is preserved
- [ ] No unrelated command modules are changed

**Notes**: The defect is reported on top-level `agentchrome navigate <url>`, but the duplicated helper has the same event-wait pattern. Audit both paths so the root pattern is closed.

### T003: Add Regression Coverage

**File(s)**: `tests/features/277-fix-same-document-url-navigation-waits-for-fragment-only-navigations.feature`, `tests/bdd.rs`, `tests/fixtures/same-document-url-navigation.html`
**Type**: Create / Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] Gherkin scenarios cover AC1 through AC3 from `requirements.md`
- [ ] Every scenario is tagged `@regression`
- [ ] A deterministic fixture provides at least two fragment targets, such as `#S06` and `#S07`, with no external network dependencies
- [ ] BDD registration follows the existing Chrome-dependent navigation-regression convention in `tests/bdd.rs`
- [ ] Coverage proves the load-style same-document bug would fail without the fix and pass with it

**Notes**: Use the public `https://qaplayground.vercel.app/` reproduction for manual smoke coverage, but prefer a committed fixture for deterministic automated coverage.

### T004: Verify No Regressions and Smoke Test

**File(s)**: existing test files, `tests/fixtures/same-document-url-navigation.html`
**Type**: Verify (no file changes expected)
**Depends**: T001, T002, T003
**Acceptance**:
- [ ] `cargo fmt --check` passes
- [ ] `cargo test --lib` passes
- [ ] `cargo test --test bdd` passes or reports only intentionally skipped Chrome-dependent scenarios
- [ ] `cargo clippy --all-targets` passes
- [ ] Feature Exercise Gate: build the debug binary, launch headless Chrome, navigate to the deterministic fixture, then verify same-document fragment navigation with `--wait-until load` and `--wait-until domcontentloaded`
- [ ] Issue reproduction smoke: repeat the `https://qaplayground.vercel.app/` steps from `requirements.md` and confirm both fragment navigations exit 0 with the final fragment URL
- [ ] Cross-document smoke: navigate to `https://example.com/ --wait-until load` and confirm stdout includes `url`, `title`, and `status`
- [ ] Disconnect Chrome and confirm no orphaned Chrome processes remain

**Notes**: Follow `steering/tech.md` cleanup requirements for AgentChrome-managed headless Chrome.

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
| #277 | 2026-04-27 | Initial defect report |
