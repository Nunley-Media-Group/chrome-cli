# Tasks: Machine-Readable Capabilities Manifest Subcommand

**Issue**: #30
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude (spec generation)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend | 2 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **7** | |

---

## Phase 1: Setup

### T001: Add CapabilitiesArgs struct and Command::Capabilities variant

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `CapabilitiesArgs` struct with `command: Option<String>` and `compact: bool` fields
- [ ] `Command::Capabilities(CapabilitiesArgs)` variant added to `Command` enum
- [ ] Help text includes `long_about` and `after_long_help` with usage examples
- [ ] Variant placed between `Examples` and `Completions` in the enum (or at a logical position among meta-commands)
- [ ] `cargo check` passes

**Notes**: Follow the pattern of `ExamplesArgs` — a simple struct with an optional string and a bool flag. The `--command` flag uses `#[arg(long)]`, and `--compact` uses `#[arg(long)]`.

### T002: Define output types for the capabilities manifest

**File(s)**: `src/capabilities.rs` (create)
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `CapabilitiesManifest` struct with `name`, `version`, `commands`, `global_flags`, `exit_codes`
- [ ] `CommandDescriptor` struct with `name`, `description`, `subcommands`
- [ ] `SubcommandDescriptor` struct with `name`, `description`, `args`, `flags`
- [ ] `ArgDescriptor` struct with `name`, `type_name` (serialized as `"type"`), `required`, `description`
- [ ] `FlagDescriptor` struct with `name`, `type_name`, `required`, `default`, `values`, `description`
- [ ] `ExitCodeDescriptor` struct with `code`, `name`, `description`
- [ ] All structs derive `Serialize`
- [ ] Optional fields use `#[serde(skip_serializing_if = "Option::is_none")]`
- [ ] `type_name` fields use `#[serde(rename = "type")]`
- [ ] `cargo check` passes

---

## Phase 2: Backend Implementation

### T003: Implement clap tree walking and manifest generation

**File(s)**: `src/capabilities.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `fn build_manifest(cmd: &clap::Command, compact: bool) -> CapabilitiesManifest` builds the full manifest
- [ ] `fn visit_command(cmd: &clap::Command) -> CommandDescriptor` extracts a top-level command with its subcommands
- [ ] `fn visit_subcommand(parent_name: &str, cmd: &clap::Command) -> SubcommandDescriptor` extracts name, description, args, flags
- [ ] `fn extract_args(cmd: &clap::Command) -> Vec<ArgDescriptor>` extracts positional arguments
- [ ] `fn extract_flags(cmd: &clap::Command) -> Vec<FlagDescriptor>` extracts flags (long options)
- [ ] `fn infer_type(arg: &clap::Arg) -> String` infers type from heuristics (enum, bool, integer, path, string)
- [ ] `fn extract_default(arg: &clap::Arg) -> Option<serde_json::Value>` extracts default values
- [ ] `fn global_flags(cmd: &clap::Command) -> Vec<FlagDescriptor>` extracts global flags from root command
- [ ] `fn exit_codes() -> Vec<ExitCodeDescriptor>` returns static exit code documentation
- [ ] Handles commands with subcommands (tabs, page, interact, form, perf, dialog, emulate, js, console, network)
- [ ] Handles flat commands without subcommands (connect, dom)
- [ ] Handles hybrid commands (navigate: direct args + subcommands)
- [ ] Compact mode produces `CommandDescriptor` with `subcommands: None` (names + descriptions only)
- [ ] All clap `Command` variants from the enum are represented in the output
- [ ] Hidden arguments (e.g., internal clap args) are excluded

### T004: Implement execute_capabilities dispatcher and formatting

**File(s)**: `src/capabilities.rs`
**Type**: Modify
**Depends**: T001, T003
**Acceptance**:
- [ ] `pub fn execute_capabilities(global: &GlobalOpts, args: &CapabilitiesArgs) -> Result<(), AppError>` implemented
- [ ] Calls `Cli::command()` to get the clap command tree
- [ ] Calls `build_manifest()` to generate the manifest
- [ ] If `--command <CMD>`: filters manifest to matching command or returns `AppError` with `ExitCode::GeneralError`
- [ ] Default output: compact JSON via `serde_json::to_string`
- [ ] `--pretty` output: indented JSON via `serde_json::to_string_pretty`
- [ ] `print_output()` helper follows `examples.rs` pattern
- [ ] Returns exit code 0 on success

**Notes**: Unlike `examples`, this command always outputs JSON (even without `--json` flag) since the manifest is inherently structured data. The `--plain` flag is not applicable — JSON is always the output format.

---

## Phase 3: Integration

### T005: Wire capabilities command into main dispatcher

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `mod capabilities;` added to module declarations
- [ ] `CapabilitiesArgs` imported from `cli` module (if needed by pattern)
- [ ] `Command::Capabilities(args) => capabilities::execute_capabilities(&global, args)` added to match in `run()`
- [ ] No `async` — this is a sync call (same as `execute_completions`, `execute_man`, `execute_examples`)
- [ ] `cargo build` succeeds
- [ ] Running `chrome-cli capabilities` prints valid JSON with all commands
- [ ] Running `chrome-cli capabilities --pretty` prints indented JSON
- [ ] Running `chrome-cli capabilities --command navigate` prints navigate-only manifest
- [ ] Running `chrome-cli capabilities --compact` prints minimal manifest
- [ ] Running `chrome-cli capabilities --command nonexistent` exits with code 1 and error on stderr

---

## Phase 4: Testing

### T006: Create BDD feature file for capabilities command

**File(s)**: `tests/features/capabilities.feature`
**Type**: Create
**Depends**: T005
**Acceptance**:
- [ ] All 10 acceptance criteria from requirements.md have corresponding scenarios
- [ ] Uses Given/When/Then format
- [ ] Includes error handling scenario (unknown command)
- [ ] Feature file is valid Gherkin syntax
- [ ] Includes data-driven scenario for per-command coverage
- [ ] Includes scenario validating global_flags and exit_codes presence

### T007: Add unit tests for capabilities module

**File(s)**: `src/capabilities.rs` (inline `#[cfg(test)]` module)
**Type**: Modify
**Depends**: T003, T004
**Acceptance**:
- [ ] Test: `build_manifest()` returns manifest with correct `name` and `version`
- [ ] Test: manifest contains all expected command names (sync test against clap tree)
- [ ] Test: each command has a non-empty description
- [ ] Test: `global_flags()` returns entries for port, host, ws-url, timeout, tab, etc.
- [ ] Test: `exit_codes()` returns all 6 exit codes (0-5)
- [ ] Test: `infer_type()` returns "enum" for args with possible values
- [ ] Test: `infer_type()` returns "bool" for SetTrue/SetFalse args
- [ ] Test: compact mode omits subcommands details
- [ ] Test: `--command` filter returns only the specified command
- [ ] Test: `--command` with unknown name returns error
- [ ] Test: commands with subcommands (e.g., tabs) have `subcommands` field populated
- [ ] Test: enum flags (e.g., --wait-until) have `values` array
- [ ] `cargo test` passes

---

## Dependency Graph

```
T001 (CLI args) ──┐
                  ├──▶ T004 (dispatcher) ──▶ T005 (wire into main) ──▶ T006 (BDD)
T002 (types) ─┬──┘
              │
              └──▶ T003 (tree walking) ──▶ T004
                                            │
                                            └──▶ T007 (unit tests)
```

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included for each layer
- [x] No circular dependencies
- [x] Tasks are in logical execution order
