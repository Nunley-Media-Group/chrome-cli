# Root Cause Analysis: Fix stale-skill notice during explicit skill update

**Issue**: #281
**Date**: 2026-04-28
**Status**: Draft
**Author**: Codex (AI-assisted)

---

## Root Cause

The defect is an ordering bug in the command dispatcher. `src/main.rs::run()` loads configuration, immediately calls `skill_check::emit_stale_notice_if_any(&config_file)`, and only then dispatches the parsed command. That ordering is correct for ordinary commands because the notice is advisory and should appear before the command's own stderr, but it is wrong for `Command::Skill(SkillCommand::Update(Some(tool)))`.

For an explicit update, the selected target is the object being repaired by the current invocation. `emit_stale_notice_if_any()` scans installed skill files before `src/skill.rs::execute_skill()` reaches the `SkillCommand::Update(Some(tool))` arm, so it observes the selected skill in its stale pre-update state and emits guidance to run `agentchrome skill update`. The same invocation then updates that exact target and prints successful single-target JSON, producing contradictory stderr/stdout for automation consumers.

The minimal correction is to suppress the pre-dispatch stale notice only for explicit single-target skill updates. Non-update commands still need the advisory notice, and omitted-`--tool` bare updates still need the existing issue #254 behavior that scans and updates all stale installed skills.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/main.rs` | 173-177 | `run()` loads config and emits the global stale-skill notice before command dispatch. |
| `src/main.rs` | 179-208 | Dispatcher routes `Command::Skill(args)` to `skill::execute_skill()`. |
| `src/skill_check.rs` | 259-274 | `emit_stale_notice_if_any()` applies suppression gates, scans installed skills, and writes the stale notice to stderr. |
| `src/skill.rs` | 596-612 | `update_skill()` verifies a selected skill is installed, rewrites it with current content, and returns `action: "updated"`. |
| `src/skill.rs` | 992-999 | `execute_skill()` routes `Update(Some(tool))` to the explicit single-target updater and `Update(None)` to the bare multi-target updater. |
| `tests/features/skill-staleness.feature` | 51-60, 81-86 | Existing staleness/update scenarios cover bare update and idempotent explicit update, but not self-stale notice suppression for explicit stale updates. |

### Triggering Conditions

- A supported AgentChrome skill file is installed for the selected explicit target.
- The installed file's embedded version marker is older than `CARGO_PKG_VERSION`.
- Stale-skill notice checks are enabled by environment and config.
- The user runs `agentchrome skill update --tool <tool>`.
- The selected tool is in the active-tool scope or the no-active-tool fallback scans all installed stale skills.

---

## Fix Strategy

### Approach

Add a small dispatch predicate in `src/main.rs` that decides whether the pre-dispatch stale notice should run. The predicate should return `false` only for `Command::Skill(SkillCommand::Update(args))` when `args.tool.is_some()`. In all other cases it should return `true`, including `skill update` without `--tool`, `skill install`, `skill uninstall`, `skill list`, and every non-skill command.

Keep the actual update implementation in `src/skill.rs` unchanged. The explicit update arm already preserves the correct single-target behavior: it resolves the requested `ToolName`, calls `update_skill()`, and prints a `SkillResult` JSON object. Avoid passing selected-tool state into `skill_check.rs`; the notice module should keep its existing role of formatting stale installed-skill notices when the dispatcher asks it to run.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/main.rs` | Guard the `skill_check::emit_stale_notice_if_any(&config_file)` call behind a predicate such as `should_emit_stale_notice_for_command(&cli.command)`. | Prevents the global stale notice from observing the selected explicit update target before it is repaired. |
| `src/main.rs` | Match `Command::Skill(SkillCommand::Update(args))` with `args.tool.is_some()` as the only suppression case. | Preserves ordinary stale notices and keeps bare update behavior from issue #254 intact. |
| `src/skill.rs` | No semantic change expected. Keep explicit `Update(Some(tool))` on `tool_for_name()` -> `update_skill()` -> single-target `print_output()`. | Maintains the existing success output contract and target-selection semantics. |
| `src/skill_check.rs` | No semantic change expected. Existing suppression gates and formatting stay centralized there. | Avoids widening the fix into stale-notice formatting or active-tool scoping. |
| `tests/features/281-fix-stale-skill-notice-during-explicit-skill-update.feature` | Add one `@regression` scenario per acceptance criterion. | Captures the self-stale suppression, explicit JSON contract, non-update stale notices, and bare update no-regression behavior. |
| `tests/bdd.rs` | Register the new feature file and extend the existing skill/staleness test worlds with any missing stdout/stderr assertions. | Keeps tests isolated with temp homes and exercises the real binary path. |

### Blast Radius

- **Direct impact**: Pre-dispatch stale-notice emission in `src/main.rs`.
- **Indirect impact**: Explicit `agentchrome skill update --tool <tool>` stderr behavior changes from "may include global stale notice" to "does not include the self-stale pre-update notice".
- **Risk level**: Low to Medium. The code change is a narrow dispatcher guard, but it sits in the global command path and must not accidentally suppress notices for non-update commands or bare update.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Stale notices are accidentally suppressed for non-update commands. | Medium | AC3 adds a non-update stale-notice scenario that must still emit the active-tool-scoped notice. |
| Bare `skill update` without `--tool` loses its multi-target update behavior. | Medium | AC4 keeps the existing issue #254 bare-update flow under regression coverage. |
| Explicit update stdout changes from a single `SkillResult` object to batch/no-op JSON. | Low | AC1 and AC2 assert `tool`, `path`, `action`, and `version` fields and absence of batch `results`. |
| The fix changes stale-notice wording instead of only suppressing the contradictory path. | Low | Tests should assert notice absence/presence by stale-notice prefix and target name, not by rewriting `format_notice()`. |
| Test fixtures touch real user skill files. | Low | BDD tasks must keep `HOME`, `USERPROFILE`, and cwd isolated in temp directories. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Filter the selected update target out of `skill_check::stale_tools_for_notice()` while still emitting notices for unrelated stale tools. | Thread command context into the stale-notice scanner and remove only the selected tool from notice candidates. | More invasive than needed, makes the notice module command-aware, and can still produce noisy stderr during an explicit repair command. |
| Move the stale notice to run after command dispatch. | Let update commands repair the skill before the notice scanner runs. | Breaks the existing "notice before command stderr/streaming output" contract and risks changing many unrelated commands. |
| Suppress stale notices for all `skill` subcommands. | Skip the global notice whenever the top-level command is `skill`. | Too broad; `skill list` and other non-update skill commands should retain current stale-notice behavior. |
| Change explicit update to call bare update internally. | Make `--tool` share the multi-target scan and skip self-notice indirectly. | Violates the single-target output and target-selection contract. |

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #281 | 2026-04-28 | Initial defect design |

---

## Validation Checklist

- [x] Root cause is identified with specific code references
- [x] Fix is minimal - no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
