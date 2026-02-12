# Tasks: Page Text Extraction

**Issue**: #9
**Date**: 2026-02-11
**Status**: Planning
**Author**: Claude (writing-specs)

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

### T001: Add error helper constructors

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AppError::element_not_found(selector)` returns message `"Element not found for selector: {selector}"` with `ExitCode::GeneralError`
- [ ] `AppError::evaluation_failed(description)` returns message `"Text extraction failed: {description}"` with `ExitCode::GeneralError`
- [ ] Unit tests for both constructors verify message content and exit code

**Notes**: Follow the existing pattern of `navigation_failed()`, `target_not_found()`, etc.

### T002: Add CLI argument types for `page text`

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `PageArgs` struct with `#[command(subcommand)]` field of type `PageCommand`
- [ ] `PageCommand` enum with `Text(PageTextArgs)` variant
- [ ] `PageTextArgs` struct with `--selector` (`Option<String>`) argument
- [ ] `Command::Page` variant changed from unit to `Page(PageArgs)`
- [ ] `cargo build` compiles without errors
- [ ] `chrome-cli page text --help` shows the selector option and global flags

**Notes**: Follow `TabsArgs`/`TabsCommand` pattern. The `--plain`, `--pretty`, `--json`, `--tab` flags are already global.

---

## Phase 2: Backend Implementation

### T003: Implement page text extraction command

**File(s)**: `src/page.rs` (new file)
**Type**: Create
**Depends**: T001, T002
**Acceptance**:
- [ ] `PageTextResult` struct with `text`, `url`, `title` fields, derives `Serialize`
- [ ] `execute_page()` dispatches `PageCommand::Text` to `execute_text()`
- [ ] `execute_text()` follows the session setup pattern from `navigate.rs`:
  - Resolves connection and target via `resolve_connection` / `resolve_target`
  - Creates `CdpClient`, `CdpSession`, `ManagedSession`
  - Enables `Runtime` domain via `ensure_domain`
- [ ] Without `--selector`: evaluates `document.body.innerText` via `Runtime.evaluate`
- [ ] With `--selector`: evaluates IIFE that calls `querySelector(selector).innerText` with null detection
- [ ] Fetches page URL and title (same `Runtime.evaluate` pattern as `navigate.rs::get_page_info`)
- [ ] Returns `PageTextResult` with extracted text, URL, and title
- [ ] `returnByValue: true` is set in the `Runtime.evaluate` params
- [ ] CSS selector quotes are escaped in the JS expression to prevent breakage
- [ ] `print_output()` helper handles `--json` and `--pretty` (same as `navigate.rs`)
- [ ] `--plain` mode prints only the raw text string to stdout (no JSON)
- [ ] Non-existent selector returns `AppError::element_not_found(selector)`
- [ ] JS evaluation exceptions return `AppError::evaluation_failed(description)`
- [ ] Empty/blank pages return `PageTextResult` with empty `text` field (not an error)
- [ ] `cdp_config()` helper for timeout (same pattern as other modules)
- [ ] Unit tests for `PageTextResult` serialization (JSON fields present, correct types)

**Notes**: Reuse `setup_session` / `get_page_info` pattern from navigate.rs. The IIFE for selector extraction should return `{ __error: "not_found" }` sentinel when `querySelector` returns null, so we can distinguish "element not found" from "element has empty text".

### T004: Wire page command into main dispatcher

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `mod page;` declaration added
- [ ] `Command::Page(args)` match arm calls `page::execute_page(&cli.global, args).await`
- [ ] Previous `Err(AppError::not_implemented("page"))` is removed
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy` passes (all=deny, pedantic=warn)

---

## Phase 3: Integration

### T005: Verify end-to-end with cargo clippy and existing tests

**File(s)**: (all modified files)
**Type**: Verify
**Depends**: T004
**Acceptance**:
- [ ] `cargo clippy --all-targets -- -D warnings` passes with zero warnings
- [ ] `cargo fmt --check` passes
- [ ] `cargo test --lib` passes (all unit tests including new ones)
- [ ] `cargo build` succeeds
- [ ] `chrome-cli page text --help` displays expected usage info

---

## Phase 4: Testing

### T006: Create BDD feature file for page text extraction

**File(s)**: `tests/features/page-text-extraction.feature`
**Type**: Create
**Depends**: T004
**Acceptance**:
- [ ] All 10 acceptance criteria from `requirements.md` are Gherkin scenarios
- [ ] Uses `Background:` for shared Chrome setup
- [ ] Valid Gherkin syntax
- [ ] Scenarios are independent and declarative

### T007: Implement BDD step definitions for page text extraction

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T006
**Acceptance**:
- [ ] Step definitions exist for all scenarios in `page-text-extraction.feature`
- [ ] Steps follow existing cucumber-rs patterns from the project
- [ ] `cargo test --test bdd` compiles (tests may skip if no Chrome available)

---

## Dependency Graph

```
T001 ──┐
       ├──▶ T003 ──▶ T004 ──▶ T005
T002 ──┘                │
                        ├──▶ T006 ──▶ T007
                        │
                        └──▶ (done)
```

T001 and T002 can be done in parallel (no interdependency).
T006 and T007 can proceed once T004 is complete.
T005 is a verification gate before merging.

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] BDD test tasks included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
