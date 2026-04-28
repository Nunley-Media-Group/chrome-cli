# Tasks: Iframe guidance advertises --frame command shapes the parser rejects

**Issue**: #286
**Date**: 2026-04-28
**Status**: Planning
**Author**: Codex (write-spec)

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Correct frame command guidance strings | [ ] |
| T002 | Add parser-backed regression coverage | [ ] |
| T003 | Regenerate docs and verify no regressions | [ ] |
| T004 | Run real-browser smoke verification | [ ] |

---

### T001: Correct Frame Command Guidance Strings

**File(s)**: `src/examples/strategies.rs`, `src/examples_data.rs`, `src/cli/mod.rs`, `src/diagnose/detectors.rs`, `src/diagnose/patterns.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `examples strategies iframes` advertises accepted `--frame` placement for every frame-targeted command
- [ ] `examples strategies scorm` and other strategy guidance sharing the same stale page-frame pattern are corrected
- [ ] `examples page` frame examples use `agentchrome page --frame 1 <subcommand>` for page group commands
- [ ] `diagnose --current` iframe, overlay, Storyline, and SCORM suggestions use concrete accepted command examples
- [ ] Already accepted examples for `dom`, `js`, `interact`, `form`, `media`, and `network list` remain accepted and are not rewritten into a rejected shape
- [ ] No core frame-targeting execution code changes are required

**Notes**: Treat the current parser as canonical: group-scoped `--frame` for `page`, `dom`, `js`, `interact`, `form`, and `media`; `network list --frame` for network filtering.

### T002: Add Parser-Backed Regression Coverage

**File(s)**: `tests/features/286-iframe-guidance-advertises-frame-command-shapes-the-parser-rejects.feature`, `tests/bdd.rs`, optionally `tests/features/examples-strategies.feature`, `tests/features/diagnose.feature`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] Every scenario in the issue-specific feature file is tagged `@regression`
- [ ] Strategy guide regression coverage runs `agentchrome examples strategies iframes --json` and validates every advertised frame command parses successfully
- [ ] Diagnose regression coverage validates iframe-related suggestions contain concrete commands that parse successfully
- [ ] Cross-surface coverage checks help, examples, and man-page frame guidance for accepted placement
- [ ] Parser-validation helpers normalize placeholders such as `N`, `<uid>`, `<selector>`, `<script>`, and coordinate placeholders to concrete sample values before parsing
- [ ] Tests fail if a rejected `agentchrome page snapshot --frame 1`-style command is reintroduced

**Notes**: Prefer `agentchrome::command().try_get_matches_from(...)` for parser checks so these assertions do not require Chrome. Use Chrome only where the scenario is validating live diagnose output or frame targeting behavior.

### T003: Regenerate Docs and Verify No Regressions

**File(s)**: `man/*.1`, existing test files
**Type**: Modify / Verify
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo xtask man` regenerates man pages from corrected clap/examples sources
- [ ] A repository search finds no stale rejected `page <subcommand> --frame` guidance in source, tests, README, docs, or generated man pages except where it is deliberately used as a negative regression example
- [ ] `cargo fmt --check` passes
- [ ] `cargo test --lib` passes
- [ ] `cargo test --test bdd -- --input tests/features/286-iframe-guidance-advertises-frame-command-shapes-the-parser-rejects.feature --fail-fast` passes
- [ ] `cargo test --test bdd -- --input tests/features/examples-strategies.feature --fail-fast` passes
- [ ] `cargo test --test bdd -- --input tests/features/diagnose.feature --fail-fast` passes
- [ ] `cargo clippy --all-targets` passes

**Notes**: The focused BDD command shape follows the AgentChrome harness convention for running a single feature file.

### T004: Run Real-Browser Smoke Verification

**File(s)**: `tests/fixtures/iframe-frame-targeting.html`, existing source files
**Type**: Verify (no file changes expected)
**Depends**: T001, T002, T003
**Acceptance**:
- [ ] Build a fresh debug binary with `cargo build`
- [ ] Launch headless Chrome with `./target/debug/agentchrome connect --launch --headless`
- [ ] Navigate to a deterministic iframe fixture or equivalent iframe test page
- [ ] Confirm `./target/debug/agentchrome page --frame 1 snapshot --compact` succeeds
- [ ] Confirm a previously advertised rejected shape such as `./target/debug/agentchrome page snapshot --frame 1 --compact` is no longer present in guidance
- [ ] Confirm `./target/debug/agentchrome dom --frame 1 select body` and `./target/debug/agentchrome form --frame 1 fill <uid> <value>` still use accepted shapes and do not regress
- [ ] Disconnect Chrome and verify no orphaned Chrome processes remain

**Notes**: Follow the manual smoke-test requirement in `steering/tech.md`. Public sites such as `qaplayground.vercel.app` can be used as an additional check, but a committed fixture should be preferred for deterministic verification.

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
| #286 | 2026-04-28 | Initial defect report |
