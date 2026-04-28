# Tasks: Clean HTML-to-Markdown Conversion for Agentic Scraping

**Issues**: #269
**Date**: 2026-04-27
**Status**: Planning
**Author**: Codex (write-spec)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 3 | [ ] |
| Backend | 6 | [ ] |
| Integration | 3 | [ ] |
| Testing | 5 | [ ] |
| **Total** | 17 | |

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

### T001: Add permissive parsing, conversion, and URL-fetch dependencies

**File(s)**: `Cargo.toml`, `Cargo.lock`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] HTML parser/tree manipulation dependency is MIT, Apache-2.0, or MIT OR Apache-2.0 compatible
- [ ] HTML-to-Markdown converter dependency is MIT, Apache-2.0, or MIT OR Apache-2.0 compatible
- [ ] URL-fetch dependency is permissively licensed and configured without unnecessary heavy features
- [ ] `cargo tree` does not introduce GPL-only dependencies

**Notes**: Baseline candidates from design are `kuchiki`, `quick_html2md`, and `ureq`; preserve behavior if an equally permissive alternative is selected.

### T002: Define the `agentchrome markdown` CLI surface

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `Command::Markdown(MarkdownArgs)` exists
- [ ] `--file`, `--stdin`, and `--url` are mutually exclusive
- [ ] `--base-url`, `--selector`, `--strip-links`, `--include-images`, and `--max-input-bytes` are defined with doc comments and value parsers
- [ ] `agentchrome markdown --help` includes short help, long help, and examples for page, file, stdin, URL, plain, selector, strip-links, and include-images modes
- [ ] Clap validation errors still emit a single JSON object on stderr through existing error handling

### T003: Wire command dispatch and module ownership

**File(s)**: `src/main.rs`, `src/markdown.rs`, `src/lib.rs`
**Type**: Create / Modify
**Depends**: T002
**Acceptance**:
- [ ] `src/main.rs` declares and dispatches the new command module
- [ ] `src/markdown.rs` owns command execution and conversion internals
- [ ] Library exposure is added only if tests or reusable helpers require it
- [ ] No unrelated command dispatch behavior changes

---

## Phase 2: Backend Implementation

### T004: Implement source acquisition for browser pages

**File(s)**: `src/markdown.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] Default source connects to the current page using existing connection and target-resolution helpers
- [ ] Global `--tab`, `--page-id`, `--timeout`, and dialog auto-dismiss behavior remain consistent with other browser-page commands
- [ ] Browser source returns HTML, page URL, base URL, and title
- [ ] Browser-source CDP/evaluation failures map to existing typed errors

### T005: Implement source acquisition for file, stdin, and URL

**File(s)**: `src/markdown.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `--file` reads bounded UTF-8/HTML bytes and reports unreadable paths as one JSON stderr error
- [ ] `--stdin` reads bounded stdin without blocking indefinitely beyond normal stdin semantics
- [ ] `--url` accepts only `http` and `https`
- [ ] URL fetch observes the effective timeout and input limit
- [ ] URL DNS/TLS/connect failures and timeouts map to the design's typed errors
- [ ] `--base-url` is accepted for file/stdin and rejected or ignored deterministically for page/url per the final implementation contract

### T006: Build the cleanup pipeline

**File(s)**: `src/markdown.rs`
**Type**: Modify
**Depends**: T004, T005
**Acceptance**:
- [ ] HTML is parsed into a manipulable tree
- [ ] hard-removal nodes are removed consistently
- [ ] hidden nodes and obvious boilerplate containers are removed
- [ ] primary-region selection prefers `main`, `[role="main"]`, and `article`
- [ ] `removed_node_count` and `primary_region` metadata are populated where determinable

### T007: Implement selector scoping

**File(s)**: `src/markdown.rs`
**Type**: Modify
**Depends**: T006
**Acceptance**:
- [ ] `--selector` scopes output to matching subtree or subtrees in document order
- [ ] selector mode bypasses primary-region narrowing but still performs hard cleanup
- [ ] no-match selectors return one structured JSON error with target exit code semantics
- [ ] invalid selectors return one structured JSON error without panic

### T008: Normalize links, images, tables, and code blocks

**File(s)**: `src/markdown.rs`
**Type**: Modify
**Depends**: T006, T007
**Acceptance**:
- [ ] relative links resolve against page URL, fetched URL, or `--base-url`
- [ ] default output preserves useful links and omits images
- [ ] `--strip-links` unwraps anchors while preserving link text
- [ ] `--include-images` preserves useful image references with alt text and resolved URLs
- [ ] code fences preserve language hints when available
- [ ] content tables remain readable and layout-only tables are simplified without losing text

### T009: Convert cleaned HTML to Markdown and emit result

**File(s)**: `src/markdown.rs`, `src/output.rs` (only if shared helper support is needed)
**Type**: Modify
**Depends**: T008
**Acceptance**:
- [ ] JSON output includes `markdown`, `source`, and `metadata`
- [ ] optional fields that cannot be determined serialize as `null`
- [ ] `--plain` emits only Markdown when below threshold
- [ ] JSON and plain modes use shared large-response behavior
- [ ] summary metadata is useful when large output is offloaded

---

## Phase 3: Integration

### T010: Update examples and discoverability surfaces

**File(s)**: `src/examples.rs`, `src/capabilities.rs`, `xtask/src/main.rs` (only if required)
**Type**: Modify
**Depends**: T002, T009
**Acceptance**:
- [ ] built-in examples mention practical `agentchrome markdown` workflows if examples are manually curated
- [ ] capabilities output reflects the new command and flags through clap-derived metadata
- [ ] generated man pages include the new command's long help and examples
- [ ] no stale "not supported" wording exists for the command

### T011: Preserve adjacent command behavior

**File(s)**: `src/page/text.rs`, `src/dom.rs`, `src/page/mod.rs`, `src/main.rs`
**Type**: Verify / Modify only if necessary
**Depends**: T009
**Acceptance**:
- [ ] `page text` output and selector behavior are unchanged
- [ ] `page snapshot` output and compact behavior are unchanged
- [ ] `dom get-html` output and plain mode are unchanged
- [ ] shared output helper changes, if any, do not alter existing command contracts

### T012: Add error constructors only where they improve typed behavior

**File(s)**: `src/error.rs`, `src/markdown.rs`
**Type**: Modify
**Depends**: T005, T007, T009
**Acceptance**:
- [ ] new errors produce exactly one JSON object on stderr
- [ ] exit code mapping matches design.md
- [ ] errors include actionable messages without leaking raw HTML content
- [ ] existing error constructors are reused when they already express the condition

---

## Phase 4: Testing

### T013: Add unit tests for source and cleanup behavior

**File(s)**: `src/markdown.rs`
**Type**: Modify
**Depends**: T006, T008, T009
**Acceptance**:
- [ ] tests cover source-option validation and input-size bounds
- [ ] tests cover hard removals and boilerplate removals
- [ ] tests cover primary-region selection and selector scoping
- [ ] tests cover URL resolution, strip-links, include-images, code fences, and tables
- [ ] tests cover JSON serialization of optional `null` fields

### T014: Add executable BDD feature coverage

**File(s)**: `tests/features/clean-html-markdown.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T009
**Acceptance**:
- [ ] every acceptance criterion in `requirements.md` has a matching Gherkin scenario
- [ ] BDD steps are implemented in `tests/bdd.rs`
- [ ] scenarios verify success output, plain output, errors, and source metadata
- [ ] BDD feature can be run with `cargo test --test bdd -- --input tests/features/clean-html-markdown.feature`

### T015: Add deterministic HTML fixtures

**File(s)**: `tests/fixtures/clean-html-markdown.html`, optional helper fixture files under `tests/fixtures/`
**Type**: Create
**Depends**: T014
**Acceptance**:
- [ ] fixture contains primary content plus navigation, header, footer, cookie banner, scripts, styles, SVG, hidden nodes, sidebar content, links, images, code, and tables
- [ ] fixture has comments identifying which ACs it covers
- [ ] fixture is self-contained and deterministic
- [ ] no external network dependencies are required for local fixture scenarios

### T016: Verify documentation and generated surfaces

**File(s)**: `src/cli/mod.rs`, generated man output (not committed unless project convention requires), capabilities output
**Type**: Verify
**Depends**: T010
**Acceptance**:
- [ ] `agentchrome markdown --help` includes required examples
- [ ] `agentchrome capabilities` includes the command and flags
- [ ] `cargo xtask man` renders the markdown command content
- [ ] shell completion generation still succeeds

### T017: Run focused verification and manual smoke

**File(s)**: no source file expected; verification evidence in issue comment during verify-code
**Type**: Verify
**Depends**: T013, T014, T015, T016
**Acceptance**:
- [ ] `cargo fmt --check` passes
- [ ] `cargo test --lib` passes
- [ ] focused BDD command for `clean-html-markdown.feature` passes
- [ ] `cargo clippy --all-targets` passes
- [ ] manual smoke builds a fresh debug binary, launches headless Chrome, navigates to `tests/fixtures/clean-html-markdown.html`, runs `agentchrome markdown`, verifies AC outputs, disconnects, and checks for orphaned Chrome processes

---

## Dependency Graph

```text
T001 -> T002 -> T003
              |-> T004 -> T006 -> T007 -> T008 -> T009
              |-> T005 -------^
T009 -> T010 -> T016
T009 -> T011
T009 -> T012
T009 -> T013 -> T014 -> T015 -> T017
T010 --------^
T011 --------^
T012 --------^
T016 --------^
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #269 | 2026-04-27 | Initial feature spec |

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently given dependencies
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure
- [x] Test tasks are included for each layer
- [x] No circular dependencies
- [x] Tasks are in logical execution order
