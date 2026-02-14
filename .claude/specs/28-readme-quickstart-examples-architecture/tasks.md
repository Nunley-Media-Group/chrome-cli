# Tasks: README with Quick-Start, Examples, and Architecture Overview

**Issue**: #28
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude (writing-specs)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 1 | [ ] |
| Content | 8 | [ ] |
| Integration | 1 | [ ] |
| Testing | 1 | [ ] |
| **Total** | **11** | |

---

## Phase 1: Setup

### T001: Capture current CLI help output for reference

**File(s)**: N/A (reference data only)
**Type**: N/A
**Depends**: None
**Acceptance**:
- [ ] `chrome-cli --help` output captured for command reference table
- [ ] All 16 top-level commands identified with descriptions

**Notes**: Run `chrome-cli --help` and use its output as the source of truth for command descriptions. This ensures the README stays synchronized with the actual implementation.

---

## Phase 2: Content (README Sections)

### T002: Write header section with badges and description

**File(s)**: `README.md`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] H1 heading contains "chrome-cli"
- [ ] One-line description matches Cargo.toml description
- [ ] CI badge links to GitHub Actions ci.yml workflow
- [ ] License badge displays "MIT/Apache-2.0"
- [ ] Crates.io badge commented out with TODO note

### T003: Write Features section

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Bullet list covers: tab management, navigation, page inspection, screenshots, JavaScript execution, form filling, network monitoring, performance tracing, device emulation, dialog handling
- [ ] Comparison table contrasts chrome-cli with alternatives (no Node.js, standalone binary, shell-native)

### T004: Write Installation section

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Pre-built binary download instructions with curl one-liner
- [ ] `cargo install chrome-cli` command present
- [ ] Build from source instructions (git clone, cargo build --release)
- [ ] Supported platforms table lists all 5 release targets (macOS ARM, macOS Intel, Linux x64, Linux ARM, Windows)

### T005: Write Quick Start section

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Contains numbered steps (at least 5)
- [ ] Includes `chrome-cli connect` command
- [ ] Includes `chrome-cli navigate` command
- [ ] Includes `chrome-cli page snapshot` or equivalent inspection command
- [ ] Each step has a code block with the actual command

### T006: Write Usage Examples section

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T005
**Acceptance**:
- [ ] Contains examples for: screenshot, text extraction, JS execution, form filling, network monitoring
- [ ] Each example has copy-pasteable commands in code blocks
- [ ] Lengthy examples wrapped in `<details>` collapsible sections
- [ ] Commands match actual CLI syntax from --help output

### T007: Write Command Reference section

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] Table lists all 16 top-level commands (connect, tabs, navigate, page, dom, js, console, network, interact, form, emulate, perf, dialog, config, completions, man)
- [ ] Each command has a brief description
- [ ] Note directing users to `chrome-cli <command> --help` and man pages for details

### T008: Write Architecture section

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] ASCII diagram showing CLI → Command → CDP → Chrome flow
- [ ] Describes CDP over WebSocket communication
- [ ] Mentions session management (connect/disconnect, session file)
- [ ] Notes performance characteristics (native Rust, fast startup)

### T009: Write Claude Code Integration, Contributing, and License sections

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Claude Code section explains AI agent usage
- [ ] Claude Code section includes example CLAUDE.md snippet
- [ ] Contributing section has: prerequisites (Rust, Chrome), build command, test command, lint commands
- [ ] License section states dual MIT/Apache-2.0 with links to both files

---

## Phase 3: Integration

### T010: Assemble and review full README

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T002, T003, T004, T005, T006, T007, T008, T009
**Acceptance**:
- [ ] All sections present in correct order (Header → Features → Installation → Quick Start → Usage Examples → Command Reference → Architecture → Claude Code → Contributing → License)
- [ ] No broken Markdown links
- [ ] Collapsible sections render correctly
- [ ] Total README under 500 lines
- [ ] Table of contents not needed (GitHub auto-generates one)

---

## Phase 4: BDD Testing

### T011: Create BDD feature file and step definitions

**File(s)**: `tests/features/readme.feature`, `tests/steps/readme_steps.rs`
**Type**: Create
**Depends**: T010
**Acceptance**:
- [ ] Feature file covers all 11 acceptance criteria from requirements.md
- [ ] Step definitions parse README.md and verify section presence
- [ ] Tests verify badge syntax, command mentions, and section headings
- [ ] All BDD scenarios pass

---

## Dependency Graph

```
T001 ──────────────────────────────────────────┐
                                                │
T002 ──┬──▶ T003                                │
       ├──▶ T004                                │
       ├──▶ T005 ──▶ T006                       │
       ├──▶ T007 ◀────────────────────────────┘
       ├──▶ T008
       └──▶ T009
                 │
T003, T004, T005, T006, T007, T008, T009 ──▶ T010 ──▶ T011
```

**Parallelizable groups:**
- T001 can run in parallel with T002
- T003, T004, T005, T008, T009 can run in parallel (all depend only on T002)
- T006 depends on T005; T007 depends on T001

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure
- [x] Test tasks are included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
