# Tasks: Improve Error Output Consistency on All Failure Paths

**Issues**: #197
**Date**: 2026-04-21
**Status**: Planning
**Author**: Claude (spec-driven)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 1 | [ ] |
| Backend | 4 | [ ] |
| Frontend | 0 | [ ] |
| Integration | 2 | [ ] |
| Testing | 3 | [ ] |
| **Total** | **10** | |

There is no frontend layer — AgentChrome is a CLI. "Backend" maps to `src/error.rs` and the audited command modules; "Integration" maps to `src/main.rs` clap/dispatch wiring.

---

## Phase 1: Setup

### T001: Audit flagged modules for silent failure paths

**File(s)**: `src/form.rs`, `src/interact.rs`, `src/page.rs` (read-only pass)
**Type**: Investigation
**Depends**: None
**Acceptance**:
- [ ] Every `Result`-returning public entry point in the three modules enumerated
- [ ] Every leaf `Err(...)` construction classified: (a) already `AppError`, (b) converts into `AppError` via `?`, or (c) bypasses `AppError` (must fix)
- [ ] Findings written into `design.md` § Audit Findings with file / line / current behaviour / proposed fix
- [ ] Any path that emits stderr directly (`eprintln!`, `writeln!(stderr, …)`) outside `AppError::print_json_stderr` is flagged for removal

**Notes**: This is the backbone of AC4. The output of T001 drives T003/T004. No code changes in this task — pure audit.

---

## Phase 2: Backend Implementation

### T002: Add `AppError::form_fill_not_fillable` constructor

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] New `form_fill_not_fillable(target, tag, role)` constructor per `design.md`
- [ ] `suggest_alternatives(tag, role)` helper returns a non-empty slice for every (tag, role) pair
- [ ] `custom_json` payload serialises to the schema in `design.md` § API / Interface Changes (contains `error`, `code`, `kind:"not_fillable"`, `element_type`, `suggested_alternatives`)
- [ ] Unit tests cover: (a) tag-only classification, (b) role-only classification, (c) tag-and-role both present, (d) unknown tag falls back to generic alternatives
- [ ] Unit test asserts `custom_json` always contains `error` (string) and `code` (integer) (AC7 invariant)

### T003: Route `form fill` non-fillable path through new `AppError`

**File(s)**: `src/form.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] The code path that currently bypasses `AppError` on non-fillable elements now returns `Err(AppError::form_fill_not_fillable(...))`
- [ ] Element tag and ARIA role are both captured from the snapshot node before constructing the error
- [ ] No `eprintln!` / `anyhow!` / bare-string `Err` remains in the affected branch
- [ ] The fix preserves the existing successful path for legitimate fillable elements (regression test: existing form-fill BDD scenarios still pass)

### T004: Fix silent paths in `interact.rs` identified by T001

**File(s)**: `src/interact.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] Every path flagged "(c) bypasses AppError" in T001 Audit Findings now constructs or propagates an `AppError` with an appropriate `ExitCode`
- [ ] No new error variants needed beyond existing `AppError` constructors (reuse `interaction_failed`, `element_not_found`, `uid_not_found`, `stale_uid`, `element_zero_size`)
- [ ] Clippy clean (`cargo clippy --all-targets`)

### T005: Fix silent paths in `page.rs` identified by T001

**File(s)**: `src/page.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] Every path flagged "(c)" in T001 Audit Findings now constructs or propagates an `AppError`
- [ ] Page-wait timeouts route through `AppError::wait_timeout`
- [ ] Screenshot failures route through `AppError::screenshot_failed`
- [ ] Snapshot failures route through `AppError::snapshot_failed`
- [ ] Clippy clean

---

## Phase 3: Integration

### T006: Add `--uid` / `--selector` syntax-hint detector in `main.rs`

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `syntax_hint(argv)` helper (or equivalent inline block) inspects `std::env::args` in the existing clap-error branch
- [ ] When clap emits `ErrorKind::UnknownArgument` AND argv contains `--uid <val>` or `--selector <val>`, the final `clean` message gains a suffix `". Did you mean: agentchrome <cmd> <val>"` pointing to the correct positional form
- [ ] False-positive guard: the hint is suppressed for subcommands that legitimately accept `--uid` as a flag (currently none, but the guard is coded defensively via checking the clap `Command` tree)
- [ ] Unit tests cover: (a) `interact click --uid s6` → hint appears, (b) an unrelated clap error (e.g., missing required arg) → no hint
- [ ] Exactly one JSON line is still emitted on stderr (AC6)

### T007: Extend top-level `after_long_help` with error-contract documentation

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] The top-level `Cli` `after_long_help` contains a new "ERROR HANDLING" section
- [ ] Section describes the stderr JSON schema: `{error: string, code: 1..5, [optional: kind, recoverable, element_type, …]}`
- [ ] Section lists exit-code meanings: 0=success, 1=general, 2=connection, 3=target, 4=timeout, 5=protocol
- [ ] Section notes "Exactly one JSON object per non-zero exit"
- [ ] `agentchrome --help` output (long form) contains the new text
- [ ] `agentchrome capabilities` manifest reflects updated help text (regenerated automatically from clap)

---

## Phase 4: BDD Testing (Required)

**Every acceptance criterion MUST have a Gherkin test.**

### T008: Create BDD feature file `improve-error-output-consistency.feature`

**File(s)**: `tests/features/improve-error-output-consistency.feature`
**Type**: Create
**Depends**: T003, T006, T007
**Acceptance**:
- [ ] Scenarios map 1:1 to AC1–AC7 (see `feature.gherkin` for the authoritative list)
- [ ] Valid Gherkin syntax (validates under cucumber-rs)
- [ ] Scenarios use concrete element UIDs and tags, not placeholders
- [ ] Includes a test fixture reference to `tests/fixtures/improve-error-output-consistency.html`

### T009: Implement step definitions and test fixture

**File(s)**: `tests/bdd.rs`, `tests/fixtures/improve-error-output-consistency.html`
**Type**: Create / Modify
**Depends**: T008
**Acceptance**:
- [ ] Fixture HTML contains: fillable `<input type="text">`, non-fillable `<div>`, `<canvas>`, `<button>`, and a `role="combobox"` without an editable input
- [ ] Fixture has an HTML comment header listing which ACs each element covers
- [ ] Step definitions in `tests/bdd.rs` cover every new step phrase introduced by T008
- [ ] Steps reuse the existing cucumber World where possible; add new fields only when necessary
- [ ] `cargo test --test bdd` passes with the new scenarios enabled (Chrome-dependent scenarios may be gated per existing `tests/bdd.rs` conventions)

### T010: Manual smoke test against headless Chrome (per `steering/tech.md`)

**File(s)**: (no file changes — verification task)
**Type**: Verify
**Depends**: T003, T004, T005, T006, T007, T008, T009
**Acceptance**:
- [ ] `cargo build` produces a clean debug binary
- [ ] Launch headless Chrome via `./target/debug/agentchrome connect --launch --headless`
- [ ] Navigate to `file://<absolute>/tests/fixtures/improve-error-output-consistency.html`
- [ ] Run `./target/debug/agentchrome page snapshot` to assign UIDs
- [ ] For each AC, run the corresponding command against the fixture and confirm the stderr JSON matches expectations
- [ ] Run `./target/debug/agentchrome interact click --uid <UID>` — confirm stderr contains the "Did you mean:" hint
- [ ] Disconnect with `./target/debug/agentchrome connect disconnect`
- [ ] `pkill -f 'chrome.*--remote-debugging' || true`
- [ ] Results recorded in the verification report when `/verify-code` runs

---

## Dependency Graph

```
T001 (audit) ─┬──▶ T003 (form.rs fix)   ───┐
              │                              │
              ├──▶ T004 (interact.rs fix) ───┤
              │                              │
              └──▶ T005 (page.rs fix)    ───┤
                                              │
T002 (new AppError ctor) ──▶ T003 ────────────┤
                                              ▼
T006 (syntax hint)  ──────────────────────▶ T008 (BDD feature file)
T007 (help text)    ──────────────────────▶ T008
                                              │
                                              ▼
                                            T009 (step defs + fixture)
                                              │
                                              ▼
                                            T010 (smoke test)
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #197 | 2026-04-21 | Initial feature spec |

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies correctly mapped
- [x] Tasks can be completed independently given dependencies
- [x] Acceptance criteria verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks included for each layer (unit in T002, T004, T006; BDD in T008–T009; smoke in T010)
- [x] No circular dependencies
- [x] Tasks in logical execution order
