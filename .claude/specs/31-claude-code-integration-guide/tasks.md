# Tasks: Claude Code Integration Guide

**Issue**: #31
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude (nmg-sdlc)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 1 | [ ] |
| Content — Integration Guide | 4 | [ ] |
| Content — CLAUDE.md Template | 1 | [ ] |
| Integration — README Update | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **9** | |

---

## Phase 1: Setup

### T001: Create docs/ and examples/ directories

**File(s)**: `docs/`, `examples/`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `docs/` directory exists at project root
- [ ] `examples/` directory exists at project root

**Notes**: These directories don't exist yet. Both are needed for the guide and template files.

---

## Phase 2: Content — Integration Guide

### T002: Write integration guide — Introduction and Discovery sections

**File(s)**: `docs/claude-code.md`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] File exists at `docs/claude-code.md`
- [ ] Contains Introduction section explaining chrome-cli is built for AI agents
- [ ] Contains Discovery & Setup section covering PATH, `--help`, `capabilities`, `examples`
- [ ] Contains a step-by-step setup checklist
- [ ] All commands referenced are valid chrome-cli commands

**Notes**: This task creates the file and writes the first two sections. Reference `chrome-cli capabilities` output to ensure command accuracy.

### T003: Write integration guide — Workflow and Efficiency sections

**File(s)**: `docs/claude-code.md`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Contains "Common Workflows" section with at least four workflows: testing web apps, scraping data, debugging UI issues, form automation
- [ ] Each workflow shows complete command sequences with realistic URLs/data
- [ ] Contains "Recommended Workflow Loops" section with interaction loop and data extraction loop
- [ ] Contains "Efficiency Tips" section covering batch commands, `--wait-until`, `page text` vs `page snapshot`, minimizing round-trips
- [ ] Workflow loop diagrams are included (ASCII art or markdown)

### T004: Write integration guide — Error Handling and Best Practices sections

**File(s)**: `docs/claude-code.md`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] Contains "Error Handling for AI Agents" section with exit code conventions, common error patterns, recovery strategies, `--timeout` usage
- [ ] Contains "Best Practices" checklist: snapshot before interact, `--json`, exit codes, `--timeout`, `form fill` over `interact type`, `console follow`/`network follow`
- [ ] Error handling section covers: connection refused, element not found, timeout, page not loaded
- [ ] Each best practice includes a brief rationale

### T005: Write integration guide — Example Conversation and Reference sections

**File(s)**: `docs/claude-code.md`
**Type**: Modify
**Depends**: T003, T004
**Acceptance**:
- [ ] Contains "Example Conversation" section showing a realistic multi-turn Claude Code session
- [ ] Example demonstrates: connecting, navigating, taking snapshot, interacting with elements, verifying results
- [ ] Example shows error handling and recovery
- [ ] Contains "Reference" section linking to `chrome-cli capabilities`, `chrome-cli examples`, and man pages
- [ ] All commands in the example are valid and use realistic output

---

## Phase 3: Content — CLAUDE.md Template

### T006: Create CLAUDE.md example template

**File(s)**: `examples/CLAUDE.md.example`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] File exists at `examples/CLAUDE.md.example`
- [ ] Contains "Browser Automation" header with project intro
- [ ] Contains Quick Start with: connect, navigate, snapshot, screenshot commands
- [ ] Contains Key Commands section with: `capabilities`, `examples`, `page snapshot`, `interact click`, `form fill`
- [ ] Contains recommended workflow loop: snapshot → identify → interact → verify
- [ ] Contains tips: JSON output, exit codes, timeouts, `form fill` preference
- [ ] Commands match the current chrome-cli interface
- [ ] Template is concise (under 60 lines) and immediately usable

---

## Phase 4: Integration — README Update

### T007: Update README.md Claude Code Integration section

**File(s)**: `README.md`
**Type**: Modify
**Depends**: T002, T006
**Acceptance**:
- [ ] Existing "Claude Code Integration" section (lines 265-288) is replaced
- [ ] New section contains a brief summary (2-3 sentences)
- [ ] Links to `docs/claude-code.md` for the full guide
- [ ] Links to `examples/CLAUDE.md.example` for the template
- [ ] No broken markdown links
- [ ] Surrounding sections (Architecture above, Contributing below) are unaffected

---

## Phase 5: BDD Testing

### T008: Create BDD feature file for documentation verification

**File(s)**: `tests/features/claude-code-guide.feature`
**Type**: Create
**Depends**: T005, T006, T007
**Acceptance**:
- [ ] Feature file exists with valid Gherkin syntax
- [ ] All 8 acceptance criteria from requirements.md have corresponding scenarios
- [ ] Scenarios verify file existence, section presence, and content correctness
- [ ] Uses Given/When/Then format consistently

### T009: Implement BDD step definitions for documentation tests

**File(s)**: `tests/steps/claude_code_guide_steps.rs` (or inline in existing BDD runner)
**Type**: Create
**Depends**: T008
**Acceptance**:
- [ ] Step definitions cover all scenarios in the feature file
- [ ] File existence checks use `std::path::Path::new(...).exists()`
- [ ] Content checks use string matching on file contents
- [ ] All BDD tests pass with `cargo test --test bdd`

---

## Dependency Graph

```
T001 (setup dirs)
 ├──▶ T002 (guide: intro + discovery)
 │     ├──▶ T003 (guide: workflows + efficiency)
 │     │     └──▶ T005 (guide: example conversation + reference)
 │     ├──▶ T004 (guide: errors + best practices)
 │     │     └──▶ T005
 │     └──▶ T007 (README update)
 └──▶ T006 (CLAUDE.md template)
       └──▶ T007

T005, T006, T007 ──▶ T008 (BDD feature file) ──▶ T009 (step definitions)
```

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure
- [x] Test tasks are included (T008, T009)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
