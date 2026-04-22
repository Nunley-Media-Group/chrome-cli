# Tasks: Man Page Generation

**Issues**: #27, #232
**Date**: 2026-04-22
**Status**: Planning
**Author**: Claude (writing-specs)

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #27 | 2026-02-14 | Initial task breakdown (T001–T008) |
| #232 | 2026-04-22 | Appended enrichment phase (T012–T017): library accessors, xtask enrichment pipeline, determinism CI guard, BDD coverage |

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 3 | [ ] |
| Backend | 2 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| Enrichment (Issue #232) | 6 | [ ] |
| **Total** | **14** | |

---

## Task Format

Each task follows this structure:

```
### T[NNN]: [Task Title]

**File(s)**: `{layer}/path/to/file`
**Type**: Create | Modify | Delete
**Depends**: T[NNN], T[NNN] (or None)
**Acceptance**:
- [ ] [Verifiable criterion 1]
- [ ] [Verifiable criterion 2]

**Notes**: [Optional implementation hints]
```

Map `{layer}/` placeholders to actual project paths using `structure.md`.

---

## Phase 1: Setup

### T001: Add clap_mangen dependency to Cargo.toml

**File(s)**: `Cargo.toml`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `clap_mangen` added to `[dependencies]` (needed for runtime `agentchrome man`)
- [ ] Version is compatible with clap 4
- [ ] `cargo check` passes

**Notes**: `clap_mangen` must be a regular dependency (not dev-dependency) since the `agentchrome man` subcommand uses it at runtime.

### T002: Create xtask workspace member

**File(s)**: `Cargo.toml`, `xtask/Cargo.toml`, `xtask/src/main.rs`, `.cargo/config.toml`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] `xtask/` directory created with its own `Cargo.toml`
- [ ] `xtask` added to workspace members in root `Cargo.toml`
- [ ] `xtask/Cargo.toml` depends on `clap_mangen` and `clap` (for `Cli::command()` access)
- [ ] `.cargo/config.toml` defines alias: `xtask = "run --package xtask --"`
- [ ] `cargo xtask --help` runs successfully
- [ ] `xtask/src/main.rs` has placeholder structure with `man` subcommand

### T003: Add Man subcommand to CLI definition

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `ManArgs` struct defined with optional `command: Option<String>` field
- [ ] `Command::Man(ManArgs)` variant added to the `Command` enum
- [ ] `about`, `long_about`, and `after_long_help` attributes with usage examples
- [ ] `cargo check` passes
- [ ] `agentchrome man --help` shows usage information

---

## Phase 2: Backend Implementation

### T004: Implement `execute_man()` handler in main.rs

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T001, T003
**Acceptance**:
- [ ] `Command::Man` match arm calls `execute_man(args)`
- [ ] `execute_man()` calls `Cli::command()` to get the clap Command builder
- [ ] Without argument: renders the top-level man page to stdout
- [ ] With argument: finds the named subcommand and renders its man page
- [ ] Invalid subcommand name returns appropriate error
- [ ] Uses `clap_mangen::Man::new(cmd).render(&mut stdout)`
- [ ] Exit code 0 on success, non-zero on error

**Notes**: Follow the same pattern as `execute_completions()` — synchronous, no async, no Chrome connection.

### T005: Implement xtask man subcommand

**File(s)**: `xtask/src/main.rs`
**Type**: Create
**Depends**: T002
**Acceptance**:
- [ ] `cargo xtask man` generates man pages for all commands
- [ ] Recursively walks the command tree (top-level + all nested subcommands)
- [ ] Writes `.1` files to `man/` directory (creates it if needed)
- [ ] File naming: `agentchrome.1`, `agentchrome-connect.1`, `agentchrome-tabs-list.1`, etc.
- [ ] Prints a summary of generated files to stdout
- [ ] Exit code 0 on success

**Notes**: The xtask needs to import `agentchrome`'s `Cli` struct. Since `agentchrome` is a binary crate, the xtask should use `Cli::command()` from the library export in `lib.rs`. May need to re-export `Cli::command()` from `lib.rs`.

---

## Phase 3: Frontend Implementation

### T007: [Client-side model]

**File(s)**: `{presentation-layer}/models/...`
**Type**: Create
**Depends**: T002
**Acceptance**:
- [ ] Model matches API response schema
- [ ] Serialization/deserialization works
- [ ] Immutable with update method (if applicable)
- [ ] Unit tests for serialization

### T008: [Client-side service / API client]

**File(s)**: `{presentation-layer}/services/...`
**Type**: Create
**Depends**: T007
**Acceptance**:
- [ ] All API calls implemented
- [ ] Error handling with typed exceptions
- [ ] Uses project's HTTP client pattern
- [ ] Unit tests pass

### T009: [State management]

**File(s)**: `{presentation-layer}/state/...` or `{presentation-layer}/providers/...`
**Type**: Create
**Depends**: T008
**Acceptance**:
- [ ] State class defined (immutable if applicable)
- [ ] Loading/error states handled
- [ ] State transitions match design spec
- [ ] Unit tests for state transitions

### T010: [UI components]

**File(s)**: `{presentation-layer}/components/...` or `{presentation-layer}/widgets/...`
**Type**: Create
**Depends**: T009
**Acceptance**:
- [ ] Components match design specs
- [ ] Uses project's design tokens (no hardcoded values)
- [ ] Loading/error/empty states
- [ ] Component tests pass

### T011: [Screen / Page]

**File(s)**: `{presentation-layer}/screens/...` or `{presentation-layer}/pages/...`
**Type**: Create
**Depends**: T010
**Acceptance**:
- [ ] Screen layout matches design
- [ ] State management integration working
- [ ] Navigation implemented

---

## Phase 3: Integration

### T006: Wire up lib.rs export and gitignore

**File(s)**: `src/lib.rs`, `.gitignore`
**Type**: Modify
**Depends**: T004, T005
**Acceptance**:
- [ ] `src/lib.rs` re-exports `cli::Cli` (or a function returning `Command`) so xtask can access it
- [ ] `.gitignore` includes `man/` directory (generated files not tracked)
- [ ] `cargo xtask man` successfully generates man pages to `man/`
- [ ] `agentchrome man` successfully renders man pages to stdout

---

## Phase 4: BDD Testing

### T007: Create BDD feature file for man page generation

**File(s)**: `tests/features/man-page-generation.feature`
**Type**: Create
**Depends**: T004
**Acceptance**:
- [ ] All acceptance criteria from requirements.md have corresponding scenarios
- [ ] Uses Given/When/Then format
- [ ] Includes happy path (top-level and subcommand man pages)
- [ ] Includes error case (invalid subcommand)
- [ ] Includes help text scenario
- [ ] Feature file is valid Gherkin syntax

### T008: Add BDD step definitions for man page tests

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T007
**Acceptance**:
- [ ] Any new step definitions needed for man page scenarios are added
- [ ] Existing steps (e.g., "I run", "stdout should contain", "exit code should be") are reused where possible
- [ ] All scenarios in `man-page-generation.feature` pass
- [ ] `cargo test --test bdd` passes with no regressions

---

## Dependency Graph

```
T001 (clap_mangen dep) ──┬──▶ T004 (execute_man handler) ──┐
                         │                                   │
T003 (ManArgs CLI def) ──┘                                   ├──▶ T006 (lib.rs + gitignore)
                                                             │
T002 (xtask workspace) ──────▶ T005 (xtask man cmd) ────────┘
                                                             │
                                                             ▼
                                                    T007 (feature file)
                                                             │
                                                             ▼
                                                    T008 (step definitions)
                                                             │
                                                             ▼
                                                    T012 (lib accessors)
                                                             │
                                                             ▼
                                                    T013 (roff emitter) ──▶ T014 (xtask wiring)
                                                                                     │
                                                             T015 (runtime parity) ◀─┤
                                                                                     │
                                                             T016 (CI determinism)   │
                                                                     │               │
                                                                     └──▶ T017 (BDD coverage)
```

<!-- Added by issue #232 -->

## Phase 5: Enrichment (Issue #232)

### T012: Expose capabilities and examples data to xtask via library accessors

**File(s)**: `src/lib.rs`, `src/capabilities.rs`, `src/examples/mod.rs`
**Type**: Modify
**Depends**: T006
**Acceptance**:
- [ ] `pub fn build_manifest() -> CapabilitiesManifest` (or equivalent accessor) is reachable as `agentchrome::capabilities::build_manifest`
- [ ] `pub fn all_examples() -> Vec<CommandGroupSummary>` is reachable as `agentchrome::examples::all_examples`
- [ ] `CapabilitiesManifest`, `CommandDescriptor`, `SubcommandDescriptor`, `ArgDescriptor`, `FlagDescriptor`, `ExitCodeDescriptor`, `CommandGroupSummary`, `ExampleEntry` types are re-exported at the crate root (or a stable sub-path xtask can import)
- [ ] `cargo check -p xtask` compiles against the new imports
- [ ] No private-module warnings; widen visibility only for the types named above

**Notes**: If a type must stay internal, add a thin view struct in `lib.rs` that xtask consumes instead of widening internal APIs.

### T013: Implement deterministic roff emitter for CAPABILITIES and EXAMPLES sections

**File(s)**: `xtask/src/enrich.rs` (new), `xtask/src/main.rs`
**Type**: Create
**Depends**: T012
**Acceptance**:
- [ ] `enrich_for(cmd_name: &str, manifest: &CapabilitiesManifest, examples: &[CommandGroupSummary]) -> String` returns roff-formatted text for the CAPABILITIES + EXAMPLES sections
- [ ] Emits `.SH CAPABILITIES` with purpose / inputs / flags / exit codes sourced from the manifest entry whose name matches `cmd_name`
- [ ] Emits `.SH EXAMPLES` containing every `ExampleEntry` (cmd + description) for the matching group
- [ ] Iterates input `Vec`s in declared order — no `HashMap` walks, no `sort_unstable`
- [ ] Returns empty string (not an error) when no matching entry exists, so top-level and leaf subcommands without enrichment data pass through cleanly
- [ ] Unit tests cover: (a) a command present in both sources, (b) a command present only in examples, (c) a command present only in capabilities, (d) a command absent from both

**Notes**: Keep roff emission manual and minimal — `.SH`, `.TP`, `.PP`, `.B`. Don't pull in a roff DSL crate; determinism risk is not worth the dependency.

### T014: Wire enrichment into `generate_man_pages()` in xtask

**File(s)**: `xtask/src/main.rs`
**Type**: Modify
**Depends**: T013
**Acceptance**:
- [ ] `render_man_page()` appends the output of `enrich::enrich_for(name, &manifest, &examples)` to the `buf` returned by `clap_mangen::Man::render`
- [ ] Manifest and examples are built once at the top of `generate_man_pages()` and passed by reference into recursion
- [ ] `clap_mangen::Man::date("")` (or the equivalent API) is used to suppress build-date drift if the default emits one
- [ ] `cargo xtask man` runs end-to-end and produces enriched `.1` files in `man/`
- [ ] `man -l man/agentchrome-dialog.1` (or equivalent on macOS: `man ./man/agentchrome-dialog.1`) renders a CAPABILITIES section and an EXAMPLES section showing every entry from `examples dialog`

### T015: Runtime parity — `agentchrome man <cmd>` shows the same enrichment

**File(s)**: `src/man.rs` (or wherever `execute_man` lives), `src/lib.rs`
**Type**: Modify
**Depends**: T013, T014
**Acceptance**:
- [ ] Decide Open Question O1 in design.md: either (a) runtime calls the shared enrichment helper so output matches the packaged file, or (b) runtime reads the packaged `.1` file
- [ ] Document the decision inline with a one-line comment at the call site (no long rationale — the why lives in design.md)
- [ ] `agentchrome man dialog` stdout contains every example from `agentchrome examples dialog`
- [ ] `agentchrome man dialog` stdout contains the CAPABILITIES section content
- [ ] Runtime startup time for `agentchrome man <cmd>` stays under 50 ms on a reference machine (verify with `/usr/bin/time -v` or equivalent; document the measurement in the PR description, not in the code)

### T016: CI determinism guard

**File(s)**: `.github/workflows/ci.yml` (or project's CI file), `xtask/src/main.rs` (if a dedicated `--check` flag is added)
**Type**: Modify
**Depends**: T014
**Acceptance**:
- [ ] CI runs `cargo xtask man` and then `git diff --exit-code man/`, failing the build if the committed files drift from what the xtask produces
- [ ] The guard runs on every PR, not only on main
- [ ] Running the guard twice in a row on a clean tree produces zero diff (true determinism, not coincidental)
- [ ] If `man/` was previously gitignored (per #27 T006), revert that: man pages are now checked in so the determinism guard has something to diff against

**Notes**: Flipping `man/` from gitignored to tracked is a deliberate scope expansion of this issue — record in the PR description. The alternative (check against a cached tarball) is strictly worse.

### T017: BDD coverage for enriched content

**File(s)**: `tests/features/man-page-generation.feature`, `tests/bdd.rs`
**Type**: Modify
**Depends**: T014, T015
**Acceptance**:
- [ ] New scenario: "Man page includes capabilities section" — asserts `agentchrome man dialog` stdout contains "CAPABILITIES"
- [ ] New scenario: "Man page examples match examples subcommand" — runs `agentchrome examples dialog` and `agentchrome man dialog` and asserts every example command from the first appears as a substring in the second
- [ ] New scenario: "Man generation is deterministic" — runs `cargo xtask man` twice, asserts `man/` is byte-identical between runs (suitable for local and CI execution)
- [ ] New scenario (tagged `@requires-225`): "Dialog man page shows cross-process flow" — skipped until #225 lands
- [ ] `cargo test --test bdd` passes with no regressions on existing #27 scenarios

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
