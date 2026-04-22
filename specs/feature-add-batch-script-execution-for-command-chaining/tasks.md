# Tasks: Batch Script Execution

**Issues**: #199
**Date**: 2026-04-21
**Status**: Planning
**Author**: Rich Nunley

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 2 | [ ] |
| Backend (runner core) | 5 | [ ] |
| CLI integration | 3 | [ ] |
| Integration (cross-feature) | 2 | [ ] |
| Testing | 4 | [ ] |
| **Total** | **16** | |

Reference `steering/structure.md` — command modules live at `src/<name>.rs` or `src/<name>/mod.rs`; clap derive types live in `src/cli/mod.rs`; BDD lives in `tests/features/` + `tests/bdd.rs`.

---

## Phase 1: Setup

### T001: Create `src/script/` module skeleton

**File(s)**: `src/script/mod.rs`, `src/script/parser.rs`, `src/script/runner.rs`, `src/script/dispatch.rs`, `src/script/context.rs`, `src/script/result.rs`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] Module compiles with stub `pub fn run(...)` signatures
- [ ] Registered via `mod script;` in `src/lib.rs` and `src/main.rs`
- [ ] No clippy warnings under `-D warnings -W pedantic`

### T002: Define script schema types (serde)

**File(s)**: `src/script/parser.rs`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] `Script`, `Step`, `CmdStep`, `IfStep`, `LoopStep`, `LoopKind::{Count, While}` structs/enums derive `Serialize`, `Deserialize`, `Debug`
- [ ] serde tag/untagged choices match the JSON shapes from design.md
- [ ] Unit tests parse one happy-path sample per shape and reject malformed samples with a useful error
- [ ] Schema rejects: empty `commands`, loop without `count`/`while`, `while` without `max`, step with multiple top-level forms (`cmd` + `if`)

---

## Phase 2: Backend — Runner Core

### T003: Command-entry dispatch table

**File(s)**: `src/script/dispatch.rs`, plus small adapters in existing modules (`src/navigate.rs`, `src/page/mod.rs`, `src/js.rs`, `src/form.rs`, `src/interact.rs`, `src/tabs.rs`, `src/console.rs`, `src/dialog.rs`)
**Type**: Create + Modify
**Depends**: T001
**Acceptance**:
- [ ] `dispatch::invoke(argv: &[String], ctx: &mut VarContext, session: &mut Session) -> Result<serde_json::Value>` routes to the right command module
- [ ] Adapters in each listed module expose `pub fn run_from_argv(session, argv) -> Result<Value>` that internally re-parses the argv via clap's `try_parse_from` and returns the command's structured output
- [ ] Dispatch returns a typed error for unknown subcommands (caught by `--dry-run`)
- [ ] Unit tests cover: known command round-trip, unknown command error, adapter error surfacing

### T004: VarContext + argument substitution

**File(s)**: `src/script/context.rs`
**Type**: Create
**Depends**: T002
**Acceptance**:
- [ ] `VarContext { prev, vars, cwd_script }` implemented
- [ ] `substitute(argv, ctx) -> Vec<String>` handles whole-token `$prev`, `$vars.<name>`, and inline interpolation per design
- [ ] Unknown variable produces `SubstitutionError::Undefined(name)`
- [ ] Unit tests cover: whole-token swap, JSON-object substitution (serialized), inline path, unknown variable path

### T005: Expression evaluator via CDP `Runtime.evaluate`

**File(s)**: `src/script/eval.rs` (new file)
**Type**: Create
**Depends**: T003
**Acceptance**:
- [ ] `eval_bool(session, expr, ctx, loop_index) -> Result<bool>` prefixes preamble binding `$prev`, `$vars`, `$i`
- [ ] Returns `Err` on Chrome evaluation exceptions
- [ ] Unit tests (using a fake CDP session) cover truthy, falsy, and thrown-exception outcomes

### T006: Sequential runner + if/else selection

**File(s)**: `src/script/runner.rs`
**Type**: Modify
**Depends**: T003, T004, T005
**Acceptance**:
- [ ] `run(script, &mut session, opts) -> RunReport` walks `commands` linearly
- [ ] `If` steps select `then` or `else`; non-selected branch entries emit `status: "skipped"`
- [ ] `--fail-fast` halts at first `error`; continue-on-error accumulates all results
- [ ] Per-step `duration_ms` + overall `total_ms` populated
- [ ] `bind` stores step output under `ctx.vars`
- [ ] Unit tests cover: happy path, fail-fast abort, continue-on-error, if-true, if-false, bind then reference

### T007: Loop execution (count + while) with max guard

**File(s)**: `src/script/runner.rs`
**Type**: Modify
**Depends**: T006
**Acceptance**:
- [ ] Count loop iterates exactly N times; exposes `$i`
- [ ] While loop re-evaluates expression per iteration; aborts at `max` with one stderr warning JSON
- [ ] Loop iteration entries carry `loop_index`
- [ ] Nested loops: `$i` scopes to innermost loop
- [ ] Unit tests cover: count=0, count=3, while-true-bounded, while-false-first-iter, max-tripped warning

---

## Phase 3: CLI Integration

### T008: Clap surface for `script run`

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `Command::Script(ScriptArgs)` variant added with `about`, `long_about`, `after_long_help` per design.md
- [ ] `ScriptSubcommand::Run(RunArgs)` with `<file>` positional, `--fail-fast`, `--dry-run` flags
- [ ] `after_long_help` contains ≥ 3 worked EXAMPLES (file, stdin via `-`, `--fail-fast`)
- [ ] `script --help` and `script run --help` render with examples and descriptions
- [ ] No flag/positional collides with any global flag (verified by manual `agentchrome --help` inspection)

### T009: Dispatch `script run` in `main.rs`

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T002, T006, T007, T008
**Acceptance**:
- [ ] Reads file or stdin based on `<file>` value (`-` → stdin)
- [ ] Requires an active session unless `--dry-run`; returns exit code 2 with standard error JSON when session missing
- [ ] Exit code mapping: 0 on success (even with mixed results), 1 on `--fail-fast` abort, 2 on no-session, 1 on parse/validation errors
- [ ] Output: single `RunReport` JSON on stdout respecting `--json`/`--pretty`

### T010: `--dry-run` implementation

**File(s)**: `src/script/runner.rs`, `src/main.rs`
**Type**: Modify
**Depends**: T002, T003
**Acceptance**:
- [ ] Runs parser + schema validation + subcommand-name lookup against the dispatch table
- [ ] Does not open a CDP session or call `Runtime.evaluate`
- [ ] Emits a `{ dispatched: false, ok: true, steps: N }` JSON on success
- [ ] Emits the standard error JSON + exit 1 on any validation failure
- [ ] Unit + BDD coverage (AC9)

---

## Phase 4: Integration (Cross-Feature)

### T011: Register `script` in capabilities manifest

**File(s)**: `src/capabilities.rs`
**Type**: Modify
**Depends**: T008
**Acceptance**:
- [ ] `agentchrome capabilities --json` output includes a `script` top-level entry
- [ ] Entry lists `run` subcommand with `--fail-fast` and `--dry-run` flags
- [ ] Existing capabilities manifest schema unchanged for other commands (no regressions)
- [ ] BDD AC12 covers

### T012: `examples script` built-in

**File(s)**: `src/examples/mod.rs` (or the existing examples dispatcher), new `src/examples/script.rs` or comparable data file
**Type**: Create + Modify
**Depends**: T008
**Acceptance**:
- [ ] `agentchrome examples script` prints ≥ 3 example scripts (sequential, conditional, loop)
- [ ] `agentchrome examples script --json` returns a structured listing per the project's listing/detail pattern (summary fields only on listing path; full body on detail — per `steering/tech.md` progressive-disclosure rule)
- [ ] BDD AC13 covers

---

## Phase 5: BDD Testing (Required)

**Every acceptance criterion MUST have a Gherkin test.** Reference `steering/tech.md` — cucumber-rs 0.21, `tests/features/*.feature`, steps in `tests/bdd.rs`.

### T013: Create BDD feature file

**File(s)**: `tests/features/batch-script-execution.feature`
**Type**: Create
**Depends**: T009, T010, T011, T012
**Acceptance**:
- [ ] Scenario per AC1–AC16
- [ ] Valid Gherkin syntax; cucumber-rs parses without error
- [ ] Uses deterministic fixtures (no external network)

### T014: Implement step definitions

**File(s)**: `tests/bdd.rs`, `tests/fixtures/batch-script-execution.html`, `tests/fixtures/scripts/*.json`
**Type**: Create + Modify
**Depends**: T013
**Acceptance**:
- [ ] World state covers script file paths, last exit code, captured stdout/stderr JSON
- [ ] Steps use `cargo run --bin agentchrome` or direct library calls per project pattern
- [ ] Fixture HTML covers the DOM elements referenced by loop/conditional scenarios
- [ ] Fixture scripts cover: sequential, conditional (both branches), count loop, while + max, bind + reference, fail-fast abort, dry-run ok, dry-run error
- [ ] `cargo test --test bdd` passes locally

### T015: Unit tests (supplementary)

**File(s)**: `src/script/parser.rs`, `src/script/context.rs`, `src/script/runner.rs`, `src/script/dispatch.rs`
**Type**: Create (inline `#[cfg(test)]` modules)
**Depends**: T002, T004, T006, T007
**Acceptance**:
- [ ] Parser: malformed schema variants covered (see T002)
- [ ] Substitution: all cases covered (see T004)
- [ ] Runner: control-flow cases covered (see T006, T007)
- [ ] Dispatch: unknown command + adapter error paths covered
- [ ] `cargo test --lib` passes

### T016: Manual smoke test against real headless Chrome

**File(s)**: (no code changes; verification script) `tests/fixtures/batch-script-execution.html`, `tests/fixtures/scripts/smoke.json`
**Type**: Verify
**Depends**: T009, T010, T011, T012, T013, T014, T015
**Acceptance**:
- [ ] `cargo build` succeeds
- [ ] `./target/debug/agentchrome connect --launch --headless` starts a headless instance
- [ ] `./target/debug/agentchrome script run tests/fixtures/scripts/smoke.json` exits 0 with a well-formed result JSON
- [ ] `./target/debug/agentchrome script run --fail-fast tests/fixtures/scripts/broken.json` exits 1 with one stderr JSON error
- [ ] Stdin form (`cat smoke.json | agentchrome script run -`) produces identical output
- [ ] `./target/debug/agentchrome connect disconnect` cleanly shuts down
- [ ] `pkill -f 'chrome.*--remote-debugging' || true` leaves no orphaned Chrome
- [ ] Results recorded in the `/verify-code` report per `steering/tech.md`

---

## Dependency Graph

```
T001 ──┬──▶ T002 ──┬──▶ T004 ──┐
       │           │           │
       ├──▶ T003 ──┼──▶ T005 ──┼──▶ T006 ──▶ T007 ──▶ T009
       │           │                                  │
       │           └──▶ T010 ───────────────────────▶ T009
       │
       └──▶ T008 ──▶ T009
                   │
                   ├──▶ T011
                   └──▶ T012

T009, T010, T011, T012 ──▶ T013 ──▶ T014
T002, T004, T006, T007, T003 ──▶ T015
T013, T014, T015 ──────────────▶ T016
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #199 | 2026-04-21 | Initial feature spec |

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `steering/structure.md`)
- [x] Test tasks included for each layer (unit, BDD, manual smoke)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
- [x] Manual smoke test task included (T016) per `steering/tech.md`
- [x] clap help metadata tasks (T008) per `steering/tech.md`
- [x] Capabilities + examples integration tasks (T011, T012)
