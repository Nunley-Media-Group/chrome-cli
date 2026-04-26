# Tasks: Fix bare `skill update` installed-skill detection

**Issue**: #254
**Date**: 2026-04-26
**Status**: Planning
**Author**: Codex (AI-assisted)

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Add installed-skill inventory and successful no-op handling | [ ] |
| T002 | Add regression BDD coverage for stale, current, missing, and explicit-target behavior | [ ] |
| T003 | Verify focused skill-command and staleness test suites | [ ] |

---

### T001: Add Installed-Skill Inventory and No-Op Handling

**File(s)**: `src/skill.rs`, `src/skill_check.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `src/skill_check.rs` exposes a structured inventory helper that scans every supported `TOOLS` path and distinguishes missing, installed-current, and installed-stale AgentChrome skills.
- [ ] `stale_tools()` uses the inventory helper so stale notices and bare update remain aligned.
- [ ] `agentchrome skill update` without `--tool` updates every stale installed skill found by the registry scan, without requiring active-tool environment variables or parent-process signals.
- [ ] If the registry scan finds installed skills but none are stale, the command exits 0 and emits structured informational JSON on stdout.
- [ ] If the registry scan finds no installed AgentChrome skills, the command exits 0 and emits structured informational JSON on stdout.
- [ ] No-op outcomes do not write JSON errors to stderr.
- [ ] Explicit `agentchrome skill update --tool <name>` keeps the current `SkillResult` JSON object shape and still errors when that named tool has no installed skill.

**Notes**: Keep the fix scoped to omitted-`--tool` update behavior. Do not change `install`, `uninstall`, skill template content, or the `TOOLS` registry.

### T002: Add Regression BDD Coverage

**File(s)**: `tests/features/254-fix-skill-update-auto-detect.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] New feature file contains one `@regression` scenario for each acceptance criterion in `requirements.md`.
- [ ] Scenarios use temp home / temp Codex home fixtures and clear tool-detection env vars so tests do not depend on the developer's real machine state.
- [ ] AC1 scenario plants at least two stale installed skills and proves bare update updates both without active-tool signals.
- [ ] AC2 scenario plants at least one current-version installed skill and proves bare update exits 0 with an all-up-to-date message.
- [ ] AC3 scenario plants no skills and proves bare update exits 0 with a no-skills-installed message.
- [ ] AC4 scenario plants two stale skills, runs explicit `--tool claude-code`, and proves only the named skill updates while stdout remains a single-target JSON object.
- [ ] The AC1 scenario fails if bare update is changed back to `resolve_tool(None)` / active-tool-only behavior.

**Notes**: Prefer extending the existing `SkillWorld` / `StaleSkillWorld` helpers in `tests/bdd.rs` rather than introducing a parallel binary harness.

### T003: Verify Focused Skill Behavior

**File(s)**: existing - no direct changes expected
**Type**: Verify
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo fmt --check` passes.
- [ ] `cargo test --bin agentchrome skill` passes.
- [ ] `cargo test --bin agentchrome skill_check` passes.
- [ ] `cargo test --test bdd skill` or the nearest supported focused BDD invocation for skill scenarios passes.
- [ ] `cargo test --test bdd` passes if the focused BDD command cannot isolate skill scenarios reliably.
- [ ] Manual smoke with a temp `HOME` confirms `agentchrome skill update` with no installed skills exits 0 and does not touch the real user home.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #254 | 2026-04-26 | Initial defect tasks |

---

## Validation Checklist

- [x] Tasks are focused on the fix - no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
