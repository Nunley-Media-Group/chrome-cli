# Implementation Tasks: Add `audit lighthouse` Command

**Issues**: #169, #231
**Date**: 2026-04-22
**Status**: Amended
**Author**: Claude

---

## Summary

| Phase | Tasks | Description |
|-------|-------|-------------|
| 1: Setup | T001–T002 | CLI types and command dispatch wiring |
| 2: Core | T003–T006 | audit.rs module with binary discovery, execution, parsing, output |
| 3: Testing | T007–T009 | BDD feature file, step registration, unit tests |
| 4: Verification | T010–T011 | Manual smoke test and regression check |
| 5: Enhancement — Issue #231 | T012–T018 | `--install-prereqs` flag, help-text prerequisite surfacing, extended not-found error |

---

## Phase 1: Setup

### T001: Add `audit` command group and `lighthouse` subcommand to CLI

**File**: `src/cli/mod.rs`

**Changes**:
1. Add `Audit(AuditArgs)` variant to the `Command` enum with help text and examples
2. Define `AuditArgs` struct with `#[command(subcommand)] pub command: AuditCommand`
3. Define `AuditCommand` enum with `Lighthouse(AuditLighthouseArgs)` variant
4. Define `AuditLighthouseArgs` struct with:
   - `pub url: Option<String>` — positional, optional URL override
   - `#[arg(long)] pub only: Option<String>` — comma-separated category filter
   - `#[arg(long)] pub output_file: Option<PathBuf>` — path to save full report

**Acceptance Criteria**:
- `agentchrome audit --help` shows the audit command group
- `agentchrome audit lighthouse --help` shows all flags and arguments
- `agentchrome audit` without subcommand exits non-zero with "subcommand" in stderr

**Dependencies**: None

---

### T002: Wire `audit` command dispatch in `main.rs`

**Files**: `src/main.rs`

**Changes**:
1. Add `mod audit;` declaration at the top of main.rs
2. Add match arm: `Command::Audit(args) => audit::execute_audit(&global, args).await,`
3. Add `AuditArgs` to the `use cli::{...}` import list

**Acceptance Criteria**:
- `agentchrome audit lighthouse` routes to `audit::execute_audit`
- Project compiles with `cargo check`

**Dependencies**: T001

---

## Phase 2: Core Implementation

### T003: Create `src/audit.rs` with `execute_audit` entry point

**File**: `src/audit.rs` (new)

**Changes**:
1. Create the module with standard imports (serde, AppError, ExitCode, GlobalOpts, etc.)
2. Implement `pub async fn execute_audit(global: &GlobalOpts, args: &AuditArgs) -> Result<(), AppError>` that matches on `AuditCommand::Lighthouse` and delegates to `execute_lighthouse`
3. Implement `async fn execute_lighthouse(global: &GlobalOpts, args: &AuditLighthouseArgs) -> Result<(), AppError>` as the main orchestration function:
   - Resolve connection to get port
   - Resolve URL (from arg or active page)
   - Find lighthouse binary
   - Validate `--only` categories
   - Build and execute lighthouse command
   - Parse output and print scores summary
   - Optionally save full report

**Acceptance Criteria**:
- Function compiles and is reachable from dispatch
- Returns `AppError::no_session()` when no session exists

**Dependencies**: T001, T002

---

### T004: Implement lighthouse binary discovery and category validation

**File**: `src/audit.rs`

**Changes**:
1. Define `const VALID_CATEGORIES: &[&str] = &["performance", "accessibility", "best-practices", "seo", "pwa"];`
2. Implement `fn find_lighthouse_binary() -> Result<(), AppError>` that runs `lighthouse --version` as a probe:
   - On success, return `Ok(())`
   - On failure (not found / not executable), return `AppError` with message: `"lighthouse binary not found. Install it with: npm install -g lighthouse"` and code `GeneralError`
3. Implement `fn validate_categories(only: &str) -> Result<Vec<&str>, AppError>` that:
   - Splits on commas, trims whitespace
   - Validates each against `VALID_CATEGORIES`
   - Returns error for invalid: `"Invalid category: '<name>'. Valid categories: performance, accessibility, best-practices, seo, pwa"`

**Acceptance Criteria**:
- `find_lighthouse_binary()` returns structured error with install hint when lighthouse is not installed
- `validate_categories("performance,accessibility")` returns `Ok(vec!["performance", "accessibility"])`
- `validate_categories("performance,invalid")` returns error naming the invalid category

**Dependencies**: T003

---

### T005: Implement lighthouse execution and JSON parsing

**File**: `src/audit.rs`

**Changes**:
1. Implement `fn build_lighthouse_command(url: &str, port: u16, categories: Option<&[&str]>) -> std::process::Command` that constructs:
   ```
   lighthouse <URL> --port <PORT> --output json --chrome-flags="--headless"
   [--only-categories=cat1,cat2,...]
   ```
2. Implement `fn run_lighthouse(cmd: &mut std::process::Command) -> Result<serde_json::Value, AppError>` that:
   - Executes the command, captures stdout/stderr
   - On non-zero exit: returns `AppError` with `"Lighthouse audit failed: <stderr trimmed>"` and code `GeneralError`
   - On zero exit: parses stdout as JSON, returns the parsed value
   - On parse failure: returns `AppError` with `"Failed to parse Lighthouse output: <reason>"`

**Acceptance Criteria**:
- Command is constructed with correct flags
- Non-zero exit code produces a structured error with lighthouse's stderr
- Stdout is parsed as JSON successfully

**Dependencies**: T003, T004

---

### T006: Implement score extraction and output formatting

**File**: `src/audit.rs`

**Changes**:
1. Implement `fn extract_scores(lhr: &serde_json::Value, url: &str, requested: Option<&[&str]>) -> Result<serde_json::Value, AppError>` that:
   - Reads `lhr["categories"]` object
   - For each category, extracts `.score` (which may be `null` or a number)
   - If `requested` is `Some`, only includes those categories; others are omitted
   - If `requested` is `None`, includes all 5 categories
   - Builds a `serde_json::Map` with `url` plus category scores
   - Returns the JSON value
2. Implement the `--output-file` logic: if provided, write the raw `lhr` JSON to the file path using `std::fs::write`
3. Print the scores summary JSON to stdout via `println!`

**Output format**:
```json
{"url":"https://example.com","performance":0.91,"accessibility":0.87,"best-practices":0.93,"seo":0.90,"pwa":0.30}
```

When `--only performance,accessibility` is used:
```json
{"url":"https://example.com","performance":0.91,"accessibility":0.87}
```

When a category score is `null` in Lighthouse output:
```json
{"url":"https://example.com","performance":0.91,"pwa":null}
```

**Acceptance Criteria**:
- All 5 categories extracted when no `--only` filter
- Only requested categories present when `--only` is used
- `null` scores preserved as JSON `null`, not omitted
- `--output-file` writes the full Lighthouse JSON report to disk
- Scores summary always printed to stdout

**Dependencies**: T005

---

## Phase 3: Testing

### T007: Create BDD feature file

**File**: `tests/features/audit-lighthouse.feature` (new)

**Changes**:
1. Write Gherkin scenarios covering all 8 acceptance criteria from requirements.md
2. Include CLI-testable scenarios (argument validation, help text, invalid categories)
3. Include Chrome-dependent scenarios (full audit, URL override) marked with comments

**Acceptance Criteria**:
- Feature file covers AC1–AC8
- Scenarios use existing CliWorld step patterns (`Given agentchrome is built`, `When I run "..."`, `Then ...`)

**Dependencies**: T001

---

### T008: Register BDD feature file in test runner

**File**: `tests/bdd.rs`

**Changes**:
1. Add `CliWorld::cucumber()` block at the end of `main()` that runs `tests/features/audit-lighthouse.feature`
2. Filter to CLI-testable scenarios only (argument validation, help text, invalid `--only`, subcommand required)
3. Chrome-dependent scenarios filtered out with a comment explaining they're verified via smoke test

**Acceptance Criteria**:
- `cargo test --test bdd` includes the audit lighthouse feature
- CLI-testable scenarios pass
- Chrome-dependent scenarios are skipped with explanation

**Dependencies**: T007

---

### T009: Add unit tests for category validation and score extraction

**File**: `src/audit.rs` (inline `#[cfg(test)] mod tests`)

**Changes**:
1. Test `validate_categories` with valid, invalid, and mixed inputs
2. Test `extract_scores` with:
   - Full Lighthouse JSON → all 5 scores extracted
   - `--only` filter → only requested categories in output
   - `null` score → preserved as `null` in output
   - Missing `categories` key → error
3. Test `find_lighthouse_binary` error message contains install hint

**Acceptance Criteria**:
- `cargo test --lib` passes all unit tests
- Coverage for happy path, filtering, null handling, and error cases

**Dependencies**: T004, T006

---

## Phase 4: Verification

### T010: Manual smoke test against real Chrome

**Procedure**:
1. `cargo build`
2. `./target/debug/agentchrome connect --launch --headless`
3. `./target/debug/agentchrome navigate https://example.com`
4. `./target/debug/agentchrome audit lighthouse` — verify JSON scores on stdout
5. `./target/debug/agentchrome audit lighthouse --only performance,accessibility` — verify filtered output
6. `./target/debug/agentchrome audit lighthouse --output-file /tmp/lh-report.json` — verify file written + scores on stdout
7. `./target/debug/agentchrome audit lighthouse https://www.saucedemo.com/` — verify URL override
8. `./target/debug/agentchrome audit lighthouse --only invalid` — verify error message
9. SauceDemo baseline: navigate + snapshot against https://www.saucedemo.com/
10. `./target/debug/agentchrome connect disconnect`
11. `pkill -f 'chrome.*--remote-debugging' || true`

**Acceptance Criteria**:
- All ACs verified against real browser
- SauceDemo baseline passes

**Dependencies**: T001–T009

---

### T011: Verify no regressions

**Procedure**:
1. `cargo fmt --check` — no formatting violations
2. `cargo clippy` — no new warnings
3. `cargo test` — all existing tests pass
4. `cargo build` — clean build

**Acceptance Criteria**:
- Zero clippy warnings
- Zero test failures
- Clean build

**Dependencies**: T001–T009

---

## Phase 5: Enhancement — Issue #231 (Prerequisite Handling)

### T012: Add `--install-prereqs` flag to `AuditLighthouseArgs`

**File**: `src/cli/mod.rs`

**Changes**:
1. Add `#[arg(long)] pub install_prereqs: bool` to `AuditLighthouseArgs`
2. Update the `#[command(long_about = "...")]` on `Lighthouse` to include a `PREREQUISITES:` section above the `EXAMPLES:` block stating: `"Requires the lighthouse npm package. Install with: npm install -g lighthouse (or run: agentchrome audit lighthouse --install-prereqs)"`
3. Update the `#[command(about = "...")]` on the `Audit` group so it reads `"Run audits against the current page (requires lighthouse CLI — see 'audit lighthouse --help')"`
4. Verify the flag name does not collide with any global flag (`--port`, `--host`, `--ws-url`, `--page-id`, `--tab`, `--json`, `--verbose`) or existing subcommand flag

**Acceptance Criteria** (AC9, AC12):
- `agentchrome audit lighthouse --help` shows the PREREQUISITES line above EXAMPLES
- `agentchrome --help` and `agentchrome audit --help` both show the prerequisite reference in the `audit` group line
- `agentchrome audit lighthouse --install-prereqs --help` parses without conflict
- `cargo check` passes

**Dependencies**: T001

---

### T013: Implement `install_lighthouse_prereqs()` in `audit.rs`

**File**: `src/audit.rs`

**Changes**:
1. Branch `execute_lighthouse` at the top: `if args.install_prereqs { return install_lighthouse_prereqs(); }` — runs before `resolve_connection` because the install path does not require an active session
2. Implement `fn install_lighthouse_prereqs() -> Result<(), AppError>`:
   - Probe `npm --version` via `std::process::Command`
   - If the probe fails (binary not found or non-zero exit), return `AppError { error: "npm not found on PATH — install Node.js first", code: 1 }`
   - Run `std::process::Command::new("npm").args(["install", "-g", "lighthouse"]).status()` with inherited stdio so npm's progress is visible
   - On non-zero exit, capture (or re-run with captured stderr) and return `AppError { error: "Failed to install lighthouse: <stderr>", code: 1 }`
   - On success, probe `lighthouse --version`; if still not on PATH, return `AppError { error: "lighthouse installed but not on PATH — open a new shell and retry", code: 1 }`
   - Emit `{"installed":"lighthouse","version":"<v>"}` on stdout via `println!` with `serde_json::to_string`
3. Add `#[derive(Serialize)] struct InstallPrereqsResult<'a> { installed: &'a str, version: String }`
4. Windows note: rely on PATHEXT; if `Command::new("npm")` fails, retry with `npm.cmd` before erroring

**Acceptance Criteria** (AC10):
- `agentchrome audit lighthouse --install-prereqs` with npm missing → single JSON error on stderr naming Node.js, non-zero exit
- `agentchrome audit lighthouse --install-prereqs` with npm present and install success → `{"installed":"lighthouse","version":"..."}` on stdout, exit 0
- A subsequent `agentchrome audit lighthouse --help` invocation in a new process still exits 0 (install did not corrupt the tool)
- Cross-invocation persistence: after successful install in process A, process B locates the binary without user action

**Dependencies**: T012

---

### T014: Extend `find_lighthouse_binary` not-found error with `--install-prereqs` hint

**File**: `src/audit.rs`

**Changes**:
1. Update the existing `find_lighthouse_binary` error message from `"lighthouse binary not found. Install it with: npm install -g lighthouse"` to:
   ```
   lighthouse binary not found. Install it with: npm install -g lighthouse
   Or run: agentchrome audit lighthouse --install-prereqs
   ```
2. Ensure the error is emitted as a single `AppError` → one JSON object on stderr (retrospective learning: one invocation = one error object). Do NOT add a second error path; extend the message in place

**Acceptance Criteria** (AC11):
- `agentchrome audit lighthouse <URL>` with lighthouse missing produces exactly one JSON error object on stderr
- The `error` field contains both `npm install -g lighthouse` and `--install-prereqs`
- Exit code is 1

**Dependencies**: T004 (existing), T012

---

### T015: Add BDD scenarios for Issue #231

**File**: `tests/features/audit-lighthouse.feature`

**Changes**: Append scenarios (each tagged with a `# Added by issue #231` comment) covering:
1. `audit lighthouse --help` contains prerequisite text above examples (AC9)
2. `audit --help` contains the audit-group prerequisite reference (AC12)
3. `agentchrome --help` contains the audit-group prerequisite reference (AC12)
4. `audit lighthouse` without lighthouse installed emits one JSON error with both `npm install -g lighthouse` and `--install-prereqs` (AC11)
5. `audit lighthouse --install-prereqs` with npm missing emits structured JSON error naming Node.js (AC10 — CLI-testable via npm-stub shim, OR deferred to smoke test if no stub mechanism exists)

**Acceptance Criteria**:
- Feature file contains the 5 new scenarios
- CLI-testable scenarios (1–4) pass under `cargo test --test bdd`
- Scenario 5 is marked with a comment if deferred to smoke test

**Dependencies**: T012, T013, T014

---

### T016: Register new BDD scenarios and add unit tests

**Files**: `tests/bdd.rs`, `src/audit.rs` (inline `#[cfg(test)]`)

**Changes**:
1. Ensure the amended feature file is still picked up by the existing `CliWorld::cucumber()` block (no new registration needed if the file is globbed)
2. Add unit tests for `install_lighthouse_prereqs`'s error paths via a thin indirection — extract the npm-probe and lighthouse-probe into functions that accept a `Command`-factory closure, then test the error branches by injecting factories that return known failures
3. Unit-test that the extended not-found error string contains both the npm hint AND `--install-prereqs`

**Acceptance Criteria**:
- `cargo test --lib` passes; new tests cover npm-missing, npm-install-failure, and extended-error-string cases
- `cargo test --test bdd` passes including the new scenarios

**Dependencies**: T013, T014, T015

---

### T017: Manual smoke test for Issue #231

**Procedure**:
1. `cargo build`
2. On a shell where `lighthouse` is NOT installed: `./target/debug/agentchrome audit lighthouse --help` → verify PREREQUISITES line above EXAMPLES
3. `./target/debug/agentchrome --help` → verify audit line mentions lighthouse CLI requirement
4. `./target/debug/agentchrome audit --help` → verify same
5. `./target/debug/agentchrome audit lighthouse https://example.com` → verify single JSON error on stderr containing both `npm install -g lighthouse` AND `--install-prereqs`
6. `./target/debug/agentchrome audit lighthouse --install-prereqs` on a machine with npm → verify JSON success payload, exit 0
7. Open a NEW shell; run `./target/debug/agentchrome audit lighthouse` → verify the newly installed binary resolves (cross-invocation persistence check)
8. On a machine without npm (simulate by temporarily renaming npm on PATH): `./target/debug/agentchrome audit lighthouse --install-prereqs` → verify structured JSON error naming Node.js, non-zero exit
9. With lighthouse installed, run `./target/debug/agentchrome audit lighthouse https://example.com` → verify AC5-equivalent no-regression behavior (AC13): exit 0, full scores JSON on stdout, no new fields added

**Acceptance Criteria**:
- All AC9–AC13 behaviors verified against real environment
- No regression vs baseline T010 smoke procedure

**Dependencies**: T012–T016

---

### T018: Regression check for Issue #231

**Procedure**:
1. `cargo fmt --check` — no formatting violations
2. `cargo clippy -- -D warnings` — no new warnings introduced by #231 changes
3. `cargo test` — all existing tests (including T007–T009) still pass
4. `cargo build --release` — clean release build (the `--install-prereqs` branch adds no new crate dependencies)

**Acceptance Criteria** (AC13):
- Zero clippy warnings
- Zero test failures (no regression in T007–T011 scenarios)
- Release binary builds cleanly

**Dependencies**: T012–T017

---

## Dependency Graph

```
T001 (CLI types)
  ├── T002 (dispatch wiring) ── T003 (audit.rs scaffold)
  │                                ├── T004 (binary discovery + validation)
  │                                │     └── T005 (execution + parsing)
  │                                │           └── T006 (score extraction + output)
  │                                │
  └── T007 (feature file) ── T008 (BDD registration)

T004 + T006 ── T009 (unit tests)

T001–T009 ── T010 (smoke test)
T001–T009 ── T011 (regression check)

# Issue #231 (Phase 5)
T001 ── T012 (CLI flag + help text)
T012 ── T013 (install_lighthouse_prereqs impl)
T012 + T004 ── T014 (extend not-found error)
T012 + T013 + T014 ── T015 (BDD scenarios)
T013 + T014 + T015 ── T016 (registration + unit tests)
T012–T016 ── T017 (smoke test)
T012–T017 ── T018 (regression check)
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #169 | 2026-03-16 | Initial task breakdown |
| #231 | 2026-04-22 | Added Phase 5 (T012–T018) for `--install-prereqs`, help-text prerequisite surfacing, extended not-found error |
