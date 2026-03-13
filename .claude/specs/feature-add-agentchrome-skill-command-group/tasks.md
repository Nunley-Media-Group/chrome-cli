# Tasks: Add agentchrome skill Command Group

**Issues**: #172
**Date**: 2026-03-12
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
| **Total** | **13** | |

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
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #172 | 2026-03-12 | Initial feature spec |

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
