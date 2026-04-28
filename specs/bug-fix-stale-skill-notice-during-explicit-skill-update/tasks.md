# Tasks: Fix stale-skill notice during explicit skill update

**Issue**: #281
**Date**: 2026-04-28
**Status**: Planning
**Author**: Codex (AI-assisted)

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Suppress pre-dispatch stale notice for explicit skill updates only | [ ] |
| T002 | Add BDD regression coverage for explicit update and related stale-notice behavior | [ ] |
| T003 | Verify focused skill and staleness behavior | [ ] |

---

### T001: Suppress Pre-Dispatch Notice for Explicit Update

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `skill_check::emit_stale_notice_if_any(&config_file)` is skipped only when the parsed command is `Command::Skill(SkillCommand::Update(args))` and `args.tool.is_some()`.
- [ ] `skill update` without `--tool` still runs the pre-dispatch stale-notice path before dispatch.
- [ ] Non-update commands, including `skill list`, still run the pre-dispatch stale-notice path.
- [ ] `src/skill.rs::execute_skill()` remains responsible for explicit update behavior and keeps printing the existing single-target `SkillResult` JSON object.
- [ ] No stale-notice formatting, `TOOLS` registry, install, uninstall, or skill template behavior changes are introduced.

**Notes**: Prefer a small named predicate in `src/main.rs` so the exceptional command path is easy to test and audit.

### T002: Add BDD Regression Coverage

**File(s)**: `tests/features/281-fix-stale-skill-notice-during-explicit-skill-update.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] New feature file contains one `@regression` scenario for each acceptance criterion in `requirements.md`.
- [ ] AC1 scenario plants a stale `copilot-jb` skill in a temp home, runs `agentchrome skill update --tool copilot-jb`, and proves stderr does not contain a stale-skill notice naming `copilot-jb`.
- [ ] AC1/AC2 scenarios assert stdout remains a single JSON object with `tool`, `path`, `action`, and `version` fields and no batch `results` array.
- [ ] AC3 scenario proves an ordinary non-update command still emits the existing active-tool-scoped stale notice when the active installed skill is stale.
- [ ] AC4 scenario proves bare `agentchrome skill update` still updates all stale installed skills and a subsequent invocation emits no stale notice for updated targets.
- [ ] Test helpers isolate `HOME`, `USERPROFILE`, `CODEX_HOME`, active-tool env vars, and cwd so the tests never depend on or mutate the developer's real machine state.

**Notes**: Reuse the existing `SkillWorld` / `StaleSkillWorld` temp-home helpers where practical. Add only the missing assertions needed for selected-tool stale-notice absence and single-target JSON shape.

### T003: Verify Focused Skill Behavior

**File(s)**: existing - no direct changes expected
**Type**: Verify
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo fmt --check` passes.
- [ ] `cargo test --bin agentchrome skill` passes.
- [ ] `cargo test --bin agentchrome skill_check` passes.
- [ ] `cargo test --test bdd` passes, or the nearest supported focused BDD invocation that includes issue #281 and stale-skill scenarios passes.
- [ ] Manual CLI smoke uses a temp `HOME`/cwd, installs or plants a stale `copilot-jb` skill, runs `./target/debug/agentchrome skill update --tool copilot-jb`, and confirms exit 0, single-target JSON stdout, and no self-stale notice on stderr.
- [ ] Manual CLI smoke also confirms a non-update command with a stale active skill still emits the stale notice.

**Notes**: This defect does not require Chrome/CDP or a browser fixture; record the feature exercise as N/A during verification because the affected behavior is pure CLI/filesystem logic.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #281 | 2026-04-28 | Initial defect tasks |

---

## Validation Checklist

- [x] Tasks are focused on the fix - no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
