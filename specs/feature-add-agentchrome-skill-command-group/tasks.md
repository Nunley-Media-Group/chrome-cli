# Tasks: Add agentchrome skill Command Group

**Issues**: #172, #214, #263, #268
**Date**: 2026-04-25
**Status**: Planning
**Author**: Claude (AI-assisted)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 5 | [ ] |
| Integration | 2 | [ ] |
| Testing | 3 | [ ] |
| Documentation | 1 | [ ] |
| Gemini CLI (#214) | 5 | [ ] |
| Codex Support (#263) | 8 | [ ] |
| Multi-Target Install/Update (#268) | 8 | [x] |
| **Total** | **34** | |

---

## Phase 1: Setup

### T001: Define CLI types for skill command group

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `SkillArgs` struct with `SkillCommand` subcommand enum added
- [ ] `SkillCommand` has variants: `Install(SkillInstallArgs)`, `Uninstall(SkillToolArgs)`, `Update(SkillToolArgs)`, `List`
- [ ] `SkillInstallArgs` and `SkillToolArgs` have `--tool` flag with `ToolName` value enum
- [ ] `ToolName` enum has variants: `ClaudeCode`, `Windsurf`, `Aider`, `Continue`, `CopilotJb`, `Cursor`
- [ ] `Command::Skill(SkillArgs)` variant added to `Command` enum with `long_about` and `after_long_help`
- [ ] `cargo check` passes with no errors

**Notes**: Follow the `CookieArgs` / `CookieCommand` pattern exactly. Add clap `long_about` and `after_long_help` with examples. Use `#[arg(long, value_enum)]` for the `--tool` flag.

### T002: Add skill module to main.rs dispatch and lib.rs

**File(s)**: `src/main.rs`, `src/lib.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `mod skill;` declared in `main.rs`
- [ ] `Command::Skill(args) => skill::execute_skill(&global, args),` added to `run()` match (non-async, no `.await`)
- [ ] `cargo check` passes

**Notes**: This is a non-async dispatch like `examples` and `capabilities`. Do NOT add to `lib.rs` — the skill module is binary-only (no library consumers need it).

---

## Phase 2: Backend Implementation

### T003: Implement tool registry and detection heuristic

**File(s)**: `src/skill.rs`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] `ToolInfo` struct with name, detection description, and `InstallMode` defined
- [ ] Static `TOOLS` array with all 6 supported tools
- [ ] `detect_tool()` function implements 3-tier detection: env vars → parent process → config dirs
- [ ] Detection returns `Option<&ToolInfo>` — `None` if no tool detected
- [ ] Priority order: env vars first, then parent process, then config dir existence
- [ ] Unit tests for each detection tier

**Notes**: For parent process detection, check `std::env::var("_")` on Unix (contains invoking process path). Use `std::fs::metadata` to check config dir existence. Keep detection best-effort — `--tool` flag is the reliable override.

### T004: Implement install logic (standalone, append-section, config-patch)

**File(s)**: `src/skill.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `install_skill()` function resolves tool from `--tool` flag or detection
- [ ] Standalone mode: creates parent dirs, writes skill file
- [ ] Append-section mode: reads existing file, replaces or appends `<!-- agentchrome:start -->` / `<!-- agentchrome:end -->` delimited section
- [ ] Config-patch mode (Aider): writes standalone file + adds `read` entry to `~/.aider.conf.yml`
- [ ] Skill content generated from static template with version stamp from `VERSION` file or clap version
- [ ] Returns JSON result on stdout: `{"tool", "path", "action": "installed", "version"}`
- [ ] Idempotent: re-install overwrites without error
- [ ] Unit tests for each install mode

**Notes**: Use `env!("CARGO_PKG_VERSION")` for the version stamp at compile time. Expand `~` via `dirs::home_dir()` or `std::env::var("HOME")`. For Aider YAML config, use simple line-based manipulation (check if `read:` section exists, check if path already listed) rather than a full YAML parser to avoid a new dependency.

### T005: Implement uninstall logic

**File(s)**: `src/skill.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `uninstall_skill()` function resolves tool from `--tool` flag or detection
- [ ] Standalone mode: deletes the skill file, removes empty parent dirs
- [ ] Append-section mode: reads file, removes `<!-- agentchrome:start -->` to `<!-- agentchrome:end -->` block, writes back
- [ ] Config-patch mode (Aider): deletes skill file + removes `read` entry from config
- [ ] Returns JSON result: `{"tool", "path", "action": "uninstalled"}`
- [ ] Graceful if file doesn't exist (still returns success JSON)
- [ ] Unit tests for each uninstall mode

### T006: Implement update logic

**File(s)**: `src/skill.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `update_skill()` delegates to the same write logic as install
- [ ] Returns JSON result: `{"tool", "path", "action": "updated", "version"}`
- [ ] If no skill currently installed, returns error on stderr
- [ ] Unit tests

### T007: Implement list logic with installed status

**File(s)**: `src/skill.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `list_tools()` iterates the tool registry
- [ ] For each tool, resolves the install path and checks if the file exists
- [ ] Returns JSON: `{"tools": [{"name", "detection", "path", "installed": bool}]}`
- [ ] Respects `--pretty` output flag
- [ ] Unit tests for list output structure

---

## Phase 3: Integration

### T008: Wire dispatcher and error handling

**File(s)**: `src/skill.rs`
**Type**: Modify
**Depends**: T002, T003, T004, T005, T006, T007
**Acceptance**:
- [ ] `execute_skill()` function matches on `SkillCommand` variants and dispatches to install/uninstall/update/list
- [ ] All errors use `AppError` with `ExitCode::GeneralError`
- [ ] Unknown tool detection error includes `custom_json` with supported tools list
- [ ] JSON output on stdout, JSON errors on stderr consistent with global contract
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` passes with no warnings

### T009: Update README with skill install/update setup flow

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T008
**Acceptance**:
- [ ] "Claude Code Integration" section updated: step 2 becomes `agentchrome skill install` instead of manually dropping a CLAUDE.md template
- [ ] `agentchrome skill update` documented as the post-upgrade step
- [ ] Existing CLAUDE.md template approach preserved as an alternative/manual option
- [ ] Command reference table includes `skill` entry

---

## Phase 4: Testing

### T010: Create BDD feature file

**File(s)**: `tests/features/skill-command-group.feature`
**Type**: Create
**Depends**: T008
**Acceptance**:
- [ ] All 12 acceptance criteria from requirements.md are scenarios
- [ ] Uses Given/When/Then format
- [ ] Valid Gherkin syntax
- [ ] Covers: install, uninstall, update, list, auto-detect, --tool flag, error case, idempotency, cross-validation

### T011: Implement BDD step definitions

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T010
**Acceptance**:
- [ ] `SkillWorld` struct defined with temp dir for isolated testing
- [ ] Step definitions for all scenarios
- [ ] Tests use temp directories to avoid modifying the real filesystem
- [ ] Tests pass: `cargo test --test bdd`

### T012: Smoke test against real environment

**File(s)**: (manual verification)
**Type**: Verify
**Depends**: T008
**Acceptance**:
- [ ] `cargo build` succeeds
- [ ] `./target/debug/agentchrome skill list` returns valid JSON with all 6 tools
- [ ] `./target/debug/agentchrome skill install --tool claude-code` writes file to expected path
- [ ] `./target/debug/agentchrome skill list` shows `"installed": true` for claude-code
- [ ] `./target/debug/agentchrome skill update --tool claude-code` replaces file with current version
- [ ] `./target/debug/agentchrome skill uninstall --tool claude-code` removes the file
- [ ] `./target/debug/agentchrome skill list` shows `"installed": false` for claude-code
- [ ] `./target/debug/agentchrome skill install` (no --tool) either auto-detects or shows error with supported tools
- [ ] SauceDemo baseline: `./target/debug/agentchrome connect --launch --headless && ./target/debug/agentchrome navigate https://www.saucedemo.com/ && ./target/debug/agentchrome page snapshot && ./target/debug/agentchrome connect disconnect`
- [ ] Kill orphaned Chrome: `pkill -f 'chrome.*--remote-debugging' || true`

---

## Phase 5: Documentation

### T013: Update command reference and examples

**File(s)**: `src/examples.rs`
**Type**: Modify
**Depends**: T008
**Acceptance**:
- [ ] `skill` command group added to the `all_examples()` function
- [ ] Examples include: `skill install`, `skill install --tool cursor`, `skill list`, `skill uninstall`, `skill update`
- [ ] `cargo test --lib` passes (examples tests)

---

## Phase 6: Gemini CLI Support (Issue #214)

### T014: Add Gemini variant to ToolName enum

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None (existing infrastructure from T001 is already implemented)
**Acceptance**:
- [ ] `Gemini` variant added to `ToolName` enum
- [ ] `cargo check` passes with no errors

**Notes**: Simple addition to the existing `ValueEnum` derive enum. Clap automatically derives the `--tool gemini` CLI value from the variant name.

### T015: Add Gemini to tool registry, name mapping, and detection

**File(s)**: `src/skill.rs`
**Type**: Modify
**Depends**: T014
**Acceptance**:
- [ ] `ToolInfo` entry added to `TOOLS` array with name `"gemini"`, detection description `"GEMINI_* env var or ~/.gemini/ directory exists"`, and `InstallMode::Standalone { path_template: "~/.gemini/instructions/agentchrome.md" }`
- [ ] `tool_for_name` match arm added: `ToolName::Gemini => "gemini"`
- [ ] Tier 1 detection: `has_env_prefix("GEMINI_")` check added in `detect_tool()` after the `CURSOR_*` check
- [ ] Tier 3 detection: `home.join(".gemini").is_dir()` check added in `detect_tool()` after the `~/.cursor/` check
- [ ] `cargo check` passes

**Notes**: Follows the exact same pattern as the existing 6 tools. Standalone install mode means no special append-section or config-patching logic needed. Detection ordering: Tier 1 GEMINI_* env vars (after CURSOR_*), Tier 3 ~/.gemini/ directory (after ~/.cursor/).

### T016: Update unit tests for Gemini

**File(s)**: `src/skill.rs`
**Type**: Modify
**Depends**: T015
**Acceptance**:
- [ ] `tool_registry_has_six_tools` assertion updated to `TOOLS.len() == 7` (and test renamed to `tool_registry_has_seven_tools`)
- [ ] `tool_for_name_maps_all_variants` test includes `assert_eq!(tool_for_name(&ToolName::Gemini).name, "gemini")`
- [ ] `list_output_has_all_tools` assertion updated for 7 tools
- [ ] `cargo test --lib` passes with all tests green

**Notes**: All existing tests remain; only counts and Gemini-specific assertions are added.

### T017: Update README with Gemini support

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T015
**Acceptance**:
- [ ] Gemini CLI listed as a supported tool wherever the other 6 tools are mentioned in skill installer documentation
- [ ] Install path `~/.gemini/instructions/agentchrome.md` documented
- [ ] Detection method (GEMINI_* env var or ~/.gemini/ directory) documented
- [ ] If there is a supported tools table or list, Gemini is included

**Notes**: Follow the existing documentation pattern. The README already has a "Claude Code Integration" section with `agentchrome skill install`; Gemini should appear in any tool enumeration.

### T018: Smoke test Gemini install/uninstall/list

**File(s)**: (manual verification)
**Type**: Verify
**Depends**: T015, T016
**Acceptance**:
- [ ] `cargo build` succeeds
- [ ] `./target/debug/agentchrome skill list` returns valid JSON with 7 tools including `gemini`
- [ ] `./target/debug/agentchrome skill install --tool gemini` writes file to `~/.gemini/instructions/agentchrome.md`
- [ ] `./target/debug/agentchrome skill list` shows `"installed": true` for gemini
- [ ] `./target/debug/agentchrome skill update --tool gemini` replaces file with current version
- [ ] `./target/debug/agentchrome skill uninstall --tool gemini` removes the file and cleans empty dirs
- [ ] `./target/debug/agentchrome skill list` shows `"installed": false` for gemini

---

## Phase 7: Codex Support (Issue #263)

### T019: Add Codex CLI enum and registry mapping

**File(s)**: `src/cli/mod.rs`, `src/skill.rs`
**Type**: Modify
**Depends**: None (existing skill infrastructure from T001-T008 is already implemented)
**Acceptance**:
- [ ] `Codex` variant added to `ToolName` enum in `src/cli/mod.rs`
- [ ] `tool_for_name` maps `ToolName::Codex` to `"codex"`
- [ ] `TOOLS` includes a Codex `ToolInfo` entry with detection text `CODEX_HOME env var or ~/.codex/ directory exists`
- [ ] Codex uses standalone skill installation at `$CODEX_HOME/skills/agentchrome/SKILL.md` with fallback behavior defined in T020
- [ ] `cargo check` passes

**Notes**: Keep this as an enum/registry extension. Do not add new command variants or Codex-specific JSON output.

### T020: Implement CODEX_HOME-aware path resolution

**File(s)**: `src/skill.rs`
**Type**: Modify
**Depends**: T019
**Acceptance**:
- [ ] `resolve_path()` recognizes the exact `$CODEX_HOME/` prefix used by the Codex path template
- [ ] When `CODEX_HOME` is set to a non-empty value, paths resolve under that directory
- [ ] When `CODEX_HOME` is unset or empty, paths resolve under `~/.codex/`
- [ ] Existing `~/` path resolution remains unchanged for all other tools
- [ ] Unit tests cover set, unset, and empty `CODEX_HOME`

**Notes**: Do not implement general environment-variable expansion. This issue only needs the Codex home-root rule.

### T021: Add Codex detection without changing priority semantics

**File(s)**: `src/skill.rs`
**Type**: Modify
**Depends**: T019, T020
**Acceptance**:
- [ ] Tier 1 detection selects Codex when `CODEX_HOME` is set and no higher-priority explicit tool env signal applies
- [ ] Tier 3 detection selects Codex when `~/.codex/` exists and no higher-priority config directory applies
- [ ] Existing detection behavior for Claude Code, Windsurf, Aider, Cursor, Gemini, Continue, and Copilot JB remains unchanged
- [ ] Unit tests cover `CODEX_HOME`, `~/.codex/`, and mixed-signal priority behavior

**Notes**: No parent-process detection is required for Codex in this issue.

### T022: Extend Codex lifecycle BDD coverage

**File(s)**: `tests/features/skill-command-group.feature`, `tests/bdd.rs`
**Type**: Modify
**Depends**: T019, T020, T021
**Acceptance**:
- [ ] BDD scenarios cover `agentchrome skill install --tool codex` with `CODEX_HOME` set
- [ ] BDD scenarios cover explicit Codex install fallback to `~/.codex` when `CODEX_HOME` is unset
- [ ] BDD scenarios cover Codex entry in `skill list`
- [ ] BDD scenarios cover Codex auto-detection via `CODEX_HOME` and `~/.codex/`
- [ ] BDD scenarios cover Codex update and uninstall
- [ ] BDD helpers use temp homes and temp `CODEX_HOME` directories, never the real user Codex directory

### T023: Extend staleness coverage for Codex

**File(s)**: `tests/features/skill-staleness.feature`, `tests/bdd.rs`
**Type**: Modify
**Depends**: T019, T020
**Acceptance**:
- [ ] Codex-only stale skill scenario asserts the single-tool notice names `codex`
- [ ] Multi-tool stale scenario includes Codex in the aggregated stale-tool list
- [ ] Suppression scenarios continue to pass for Codex via `AGENTCHROME_NO_SKILL_CHECK=1` and config
- [ ] Tests plant stale Codex skill files under temp `CODEX_HOME` or temp `~/.codex`

**Notes**: `src/skill_check.rs` should not need special-case logic if Codex is correctly added to `TOOLS` and `resolve_path()`.

### T024: Update unit tests for Codex registry and paths

**File(s)**: `src/skill.rs`, `src/skill_check.rs`
**Type**: Modify
**Depends**: T019, T020, T021
**Acceptance**:
- [ ] Registry count assertion updated from 7 to 8
- [ ] `tool_for_name_maps_all_variants` includes Codex
- [ ] List output assertions include Codex
- [ ] Path resolution tests cover `$CODEX_HOME` set/unset/empty behavior
- [ ] Detection tests cover Codex env and config-dir signals
- [ ] Relevant staleness formatting or integration tests include Codex
- [ ] `cargo test --lib skill` or equivalent focused unit tests pass

### T025: Update Codex documentation

**File(s)**: `README.md`, `docs/codex.md`, `examples/AGENTS.md.example`
**Type**: Modify
**Depends**: T019
**Acceptance**:
- [ ] README lists Codex as a supported skill installer target
- [ ] README shows `agentchrome skill install --tool codex`
- [ ] `docs/codex.md` recommends the native Codex skill install path and documents `$CODEX_HOME` fallback behavior
- [ ] `examples/AGENTS.md.example` mentions the Codex skill install command where setup guidance is presented

### T026: Verify Codex skill workflow

**File(s)**: (focused verification)
**Type**: Verify
**Depends**: T019, T020, T021, T022, T023, T024, T025
**Acceptance**:
- [ ] `cargo fmt --check` passes
- [ ] `cargo check` passes
- [ ] Focused unit tests for skill registry/path/detection pass
- [ ] Focused BDD scenarios for `tests/features/skill-command-group.feature` pass or are run as part of `cargo test --test bdd`
- [ ] Focused BDD scenarios for `tests/features/skill-staleness.feature` pass or are run as part of `cargo test --test bdd`
- [ ] Manual smoke with temp `CODEX_HOME`: install → list shows installed → update → uninstall → list shows not installed

---

## Phase 8: Multi-Target Install/Update (Issue #268)

### T027: Define multi-target skill output types

**File(s)**: `src/skill.rs`
**Type**: Modify
**Depends**: T019-T026
**Acceptance**:
- [x] Add a serializable batch output type with `results` as the top-level field
- [x] Add a serializable per-target outcome type with `tool`, `path`, `action`, `version`, `status`, and optional `error`
- [x] Single-target `SkillResult` output remains unchanged for explicit `--tool` invocations
- [x] Unit tests cover success and failure serialization

**Notes**: Keep the batch shape specific to omitted-`--tool` install/update. Do not change `skill list`, explicit install/update/uninstall, or explicit error contracts.

### T028: Implement detected-target collection for bare install

**File(s)**: `src/skill.rs`
**Type**: Modify
**Depends**: T027
**Acceptance**:
- [x] Bare `skill install` collects every supported tool with a positive detection signal
- [x] Collection preserves registry order for deterministic output
- [x] Detection checks cover env-var, parent-process, and config-directory signals already documented for each tool
- [x] No detected target is skipped merely because a higher-priority target also exists
- [x] Empty detection still returns an actionable JSON error listing supported tools and detection methods

**Notes**: This helper complements `detect_tool()` rather than replacing it. Existing first-match detection remains available for command paths that still need one target.

### T029: Implement stale-installed target collection for bare update

**File(s)**: `src/skill.rs`, `src/skill_check.rs`
**Type**: Modify
**Depends**: T027
**Acceptance**:
- [x] Bare `skill update` collects every supported target with an installed AgentChrome skill older than the running binary
- [x] Target collection uses the same version-marker parsing and path resolution rules as the staleness notice
- [x] Missing installs and unreadable non-stale installs are skipped consistently with `src/skill_check.rs`
- [x] Empty stale-target collection returns an actionable JSON error stating no stale installed AgentChrome skills were found
- [x] Unit tests prove Codex plus at least one other tool are selected in the same stale scan

**Notes**: Prefer shared structured helpers over parsing the human-readable stale notice line.

### T030: Wire explicit vs bare install/update dispatch

**File(s)**: `src/skill.rs`, `src/cli/mod.rs`
**Type**: Modify
**Depends**: T028, T029
**Acceptance**:
- [x] `skill install --tool <name>` still calls the single-target install path and returns the existing object shape
- [x] `skill update --tool <name>` still calls the single-target update path and returns the existing object shape
- [x] `skill uninstall --tool <name>` remains single-target
- [x] Bare `skill install` executes install for every detected target and returns batch JSON
- [x] Bare `skill update` executes update for every stale installed target and returns batch JSON
- [x] Multi-target execution attempts every target before returning
- [x] Any per-target failure is represented in `results` and makes the process exit non-zero
- [x] `src/cli/mod.rs` long help explains bare multi-target behavior and explicit single-target behavior

### T031: Add multi-target BDD scenarios

**File(s)**: `tests/features/skill-command-group.feature`, `tests/features/skill-staleness.feature`
**Type**: Modify
**Depends**: T030
**Acceptance**:
- [x] Scenario covers bare update refreshing every stale installed skill
- [x] Scenario covers bare update updating a lower-priority stale install despite a higher-priority detection signal
- [x] Scenario covers bare install installing into all detected agents
- [x] Scenario covers explicit `--tool` remaining single-target
- [x] Scenario covers multi-target partial failure reporting per target with non-zero exit
- [x] Scenario covers a multi-tool stale notice being cleared by bare `agentchrome skill update`

### T032: Implement BDD steps and temp-home fixtures

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T031
**Acceptance**:
- [x] Steps can plant stale AgentChrome skill files for Codex plus at least one other tool in the same temp home
- [x] Steps can create multiple detection signals without touching the real user home
- [x] Steps can assert batch JSON contains all expected tools, paths, actions, versions, and per-target statuses
- [x] Steps can simulate or provoke a per-target write/resolve failure without leaving temp artifacts behind
- [x] Steps verify a subsequent invocation emits no stale notice after bare update succeeds

### T033: Add focused unit coverage for target selection and batch semantics

**File(s)**: `src/skill.rs`, `src/skill_check.rs`
**Type**: Modify
**Depends**: T030
**Acceptance**:
- [x] Unit tests cover detected-target collection with multiple simultaneous signals
- [x] Unit tests cover stale-target collection with multiple stale installed skills
- [x] Unit tests cover explicit-target single-result serialization remains unchanged
- [x] Unit tests cover batch partial failure serialization and exit-code decision
- [x] Unit tests cover shared stale-scan behavior remains aligned with notice formatting

### T034: Verify multi-target skill workflow

**File(s)**: (focused verification)
**Type**: Verify
**Depends**: T032, T033
**Acceptance**:
- [x] `cargo fmt --check` passes
- [x] Focused `cargo test` coverage for `skill` and `skill_check` passes
- [x] Focused BDD coverage for `tests/features/skill-command-group.feature` passes or is run as part of `cargo test --test bdd`
- [x] Focused BDD coverage for `tests/features/skill-staleness.feature` passes or is run as part of `cargo test --test bdd`
- [x] Manual temp-home smoke proves bare update clears a multi-tool stale notice in one invocation
- [x] Manual temp-home smoke proves bare install writes multiple detected targets in one invocation

---

## Dependency Graph

```
T001 ──┬──▶ T002 ──────────────────────────────────────┐
       │                                                │
       └──▶ T003 ──▶ T004 ──▶ T005                     │
                │       │       │                       ▼
                │       └──▶ T006                     T008 ──▶ T009
                │                                       │
                └──▶ T007                               ├──▶ T010 ──▶ T011
                                                        │
                                                        ├──▶ T012
                                                        │
                                                        └──▶ T013

Phase 6 (Issue #214 — independent of Phases 1-5, runs on existing infrastructure):

T014 ──▶ T015 ──┬──▶ T016 ──▶ T018
                │
                └──▶ T017

Phase 7 (Issue #263 — independent of Phases 1-6, runs on existing infrastructure):

T019 ──▶ T020 ──▶ T021 ──┬──▶ T022 ──┐
       │                 ├──▶ T023 ──┤
       │                 └──▶ T024 ──┤
       └──▶ T025 ───────────────────┤
                                    ▼
                                   T026

Phase 8 (Issue #268 — builds on shipped skill installer and Codex support):

T027 ──┬──▶ T028 ──┐
       └──▶ T029 ──┴──▶ T030 ──┬──▶ T031 ──▶ T032 ──┐
                                └──▶ T033 ───────────┤
                                                      ▼
                                                     T034
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #172 | 2026-03-12 | Initial feature spec |
| #214 | 2026-04-16 | Add Phase 6: Gemini CLI support (T014–T018) |
| #263 | 2026-04-24 | Add Phase 7: Codex skill installer support (T019–T026) |
| #268 | 2026-04-25 | Add Phase 8: multi-target bare skill install/update support (T027–T034) |

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included for each layer
- [x] No circular dependencies
- [x] Tasks are in logical execution order
