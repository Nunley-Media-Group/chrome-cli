# Requirements: Man Page Generation

**Issues**: #27, #232
**Date**: 2026-04-22
**Status**: Draft
**Author**: Claude (writing-specs)

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #27 | 2026-02-14 | Initial feature spec — clap_mangen-based generation + runtime `agentchrome man` subcommand |
| #232 | 2026-04-22 | Enrich generated man pages with capabilities-manifest entries and all examples at xtask build time (single source of truth: the `capabilities` and `examples` subcommands) |

---

## User Story

**As a** developer or automation engineer using agentchrome
**I want** Unix man pages generated from the CLI definition
**So that** I can access documentation through the standard `man` command or inline via `agentchrome man`

---

## Background

Man pages are the standard documentation format on Unix systems. Users expect to find documentation via `man agentchrome` after installing a CLI tool. The `clap_mangen` crate can generate roff-format man pages directly from clap's `Command` definition, keeping documentation automatically in sync with the CLI structure.

Issue #26 (comprehensive help text) has been completed, meaning all commands now have detailed `about`, `long_about`, and `after_long_help` attributes. This rich help text will flow directly into the generated man pages, producing high-quality documentation without manual authoring.

---

## Acceptance Criteria

### AC1: Generate man page for the top-level command

**Given** the agentchrome binary is built
**When** I run the man page generation process
**Then** a man page file `agentchrome.1` is produced
**And** it is valid roff-format man page content

### AC2: Generate man pages for all subcommands

**Given** the agentchrome binary is built
**When** I run the man page generation process
**Then** man page files are produced for each subcommand (e.g., `agentchrome-connect.1`, `agentchrome-tabs.1`, `agentchrome-navigate.1`)
**And** nested subcommands also get man pages (e.g., `agentchrome-tabs-list.1`)

### AC3: Man pages include standard sections

**Given** a generated man page for any command
**When** I inspect its content
**Then** it includes NAME, SYNOPSIS, and DESCRIPTION sections
**And** it includes an OPTIONS section listing all flags and arguments

### AC4: Build script generates man pages to man/ directory

**Given** the agentchrome source tree
**When** I run the build script or xtask to generate man pages
**Then** all man pages are written to the `man/` directory at the project root
**And** the generation completes without errors

### AC5: agentchrome man subcommand displays man page inline

**Given** the agentchrome binary is built
**When** I run `agentchrome man`
**Then** the top-level man page content is displayed to stdout
**And** the exit code is 0

### AC6: agentchrome man subcommand displays subcommand man page

**Given** the agentchrome binary is built
**When** I run `agentchrome man connect`
**Then** the man page for the `connect` subcommand is displayed to stdout
**And** the exit code is 0

### AC7: agentchrome man with invalid subcommand produces error

**Given** the agentchrome binary is built
**When** I run `agentchrome man nonexistent`
**Then** an error message indicates the subcommand is not found
**And** the exit code is non-zero

### AC8: Man page generation uses clap_mangen crate

**Given** the project dependencies
**When** I inspect the man page generation implementation
**Then** it uses the `clap_mangen` crate to generate from the clap `Command` definition
**And** man pages stay automatically in sync with CLI definition changes

### AC9: Man pages are generated via xtask command

**Given** the agentchrome source tree
**When** I run `cargo xtask man`
**Then** man pages are generated for all commands and subcommands
**And** they are written to the `man/` directory

### AC10: agentchrome man help text describes usage

**Given** the agentchrome binary is built
**When** I run `agentchrome man --help`
**Then** the help text explains how to display man pages for commands
**And** lists available subcommand names

<!-- Added by issue #232 -->

### AC11: Man pages include capabilities manifest entry

**Given** a user runs `agentchrome man <cmd>` for any command
**When** the man page renders
**Then** the man page includes the capabilities-manifest entry for that command — purpose, inputs, outputs, and exit codes — rendered in man-page format (roff)
**And** the entry content is sourced from the same structured data that powers `agentchrome capabilities`, not hand-maintained in `after_long_help`

### AC12: Man pages reflect every example from the examples subcommand

**Given** a user runs `agentchrome man <cmd>`
**When** the man page renders
**Then** the EXAMPLES section contains every entry that `agentchrome examples <cmd>` returns for that command
**And** no example shown by `examples <cmd>` is missing from the man page
**And** the set of examples in the man page is not a hand-curated subset of `after_long_help`

### AC13: Dialog man page shows cross-process flow (after #225 ships)

**Given** issue #225 has landed with the cross-process dialog API (either `dialog watch` or composite `interact click --expect-dialog`)
**When** a user runs `agentchrome man dialog`
**Then** the EXAMPLES section includes a worked multi-step flow demonstrating the cross-process use case end-to-end
**And** the worked flow is sourced from the shared examples data so `agentchrome examples dialog` renders the same flow

### AC14: Man generation stays deterministic in CI

**Given** the `cargo xtask man` pipeline
**When** the pipeline runs twice in CI against the same source tree
**Then** every generated `.1` file is byte-identical across runs
**And** no timestamp, hostname, path, or non-deterministic iteration order appears in the output

### AC15: No runtime overhead from enrichment

**Given** man-page enrichment happens at xtask build time, not at runtime
**When** a user runs `agentchrome man <cmd>`
**Then** command startup stays within the <50ms product target
**And** the command performs no extra I/O beyond reading the shipped man file (or rendering the in-memory clap tree, matching #27's current runtime path)

### Generated Gherkin Preview

```gherkin
Feature: Man Page Generation
  As a developer or automation engineer using agentchrome
  I want Unix man pages generated from the CLI definition
  So that I can access documentation through the standard man command or inline

  Scenario: Display top-level man page inline
    Given agentchrome is built
    When I run "agentchrome man"
    Then stdout should contain "agentchrome"
    And stdout should contain "SYNOPSIS"
    And the exit code should be 0

  Scenario Outline: Display subcommand man page inline
    Given agentchrome is built
    When I run "agentchrome man <subcommand>"
    Then stdout should contain "agentchrome-<subcommand>"
    And the exit code should be 0

    Examples:
      | subcommand |
      | connect    |
      | tabs       |
      | navigate   |
      | page       |
      | js         |

  Scenario: Invalid subcommand produces error
    Given agentchrome is built
    When I run "agentchrome man nonexistent"
    Then the exit code should be non-zero

  Scenario: Generate man pages via xtask
    Given the agentchrome source tree is available
    When I run "cargo xtask man"
    Then man page files exist in the man/ directory
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `agentchrome man [COMMAND]` subcommand to display man pages inline | Must | Runtime man page rendering via clap_mangen |
| FR2 | Generate man pages for all top-level commands | Must | connect, tabs, navigate, page, dom, js, console, network, interact, form, emulate, perf, dialog, config, completions, man |
| FR3 | Generate man pages for nested subcommands | Must | e.g., tabs list, tabs create, page screenshot |
| FR4 | `cargo xtask man` command to write man pages to `man/` directory | Must | For packaging and distribution |
| FR5 | Man pages in standard roff format (section 1) | Must | Compatible with `man` command |
| FR6 | Use `clap_mangen` crate for generation | Must | Keeps docs in sync with CLI definition |
| FR7 | Include help text, examples, and options in generated pages | Should | Leverages long_about and after_long_help from #26 |
| FR8 | Man pages included in release archives | Could | Under `man/` directory in release tarballs |
| FR9 | `cargo xtask man` pulls structured `capabilities` and `examples` content into each subcommand's generated man page at build time | Must | Added by #232 — closes the thin-man-page gap observed on 1.33.1 |
| FR10 | The single source of truth for each command's examples remains the `examples` subcommand's data; man pages are a derived artifact | Must | Added by #232 — prevents drift between `examples` and `man` outputs |
| FR11 | After #225 lands, the dialog man page includes a worked cross-process example wired into the shared examples data so both `examples dialog` and `man dialog` render it from one source | Must | Added by #232 — depends on #225 |
| FR12 | Investigation: evaluate whether a future `agentchrome man gotchas` topic-man page is worth building so the shipped Claude Code skill can defer to it | Could | Added by #232 — decision: defer; track as a separate follow-up issue |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | `agentchrome man` output is instant (< 50ms) — no Chrome connection needed |
| **Reliability** | Man pages stay in sync with CLI definition automatically (clap introspection) |
| **Platforms** | `agentchrome man` works on all platforms; `man` command integration is Unix-only |

---

## UI/UX Requirements

Reference `structure.md` and `product.md` for project-specific design standards.

| Element | Requirement |
|---------|-------------|
| **Interaction** | [Touch targets, gesture requirements] |
| **Typography** | [Minimum text sizes, font requirements] |
| **Contrast** | [Accessibility contrast requirements] |
| **Loading States** | [How loading should be displayed] |
| **Error States** | [How errors should be displayed] |
| **Empty States** | [How empty data should be displayed] |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| command | Optional string (subcommand name) | Must match a known subcommand or be empty for top-level | No |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| man page | String (stdout) | roff-format man page rendered to stdout |
| man page files | Files in `man/` | `.1` man page files for packaging |

---

## Dependencies

### Internal Dependencies
- [x] Issue #26 (comprehensive help text) — man pages are generated from it

### External Dependencies
- [ ] `clap_mangen` crate — man page generation library

### Blocked By
- None — Issue #26 is complete

---

## Out of Scope

- Automatic installation of man pages to system paths (e.g., `/usr/share/man`)
- HTML or Markdown generation from man pages
- Man pages for section other than 1 (user commands)
- Colored/styled output for `agentchrome man` (plain roff rendering)
- Building the `agentchrome man gotchas` topic-man page (deferred per #232 FR12; tracked as a future follow-up issue)
- Rewriting the `examples` or `capabilities` subcommands themselves (#232)
- Changing the packaged Claude Code skill (`~/.claude/skills/agentchrome/SKILL.md`) — it stays thin and keeps deferring to CLI self-documentation (#232)
- Internationalization of man pages (#232)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Command coverage | Man pages for all 16 top-level commands + nested subcommands | Count files in `man/` directory |
| Content completeness | NAME, SYNOPSIS, DESCRIPTION, OPTIONS in every man page | Grep man page content |
| Inline display | `agentchrome man [cmd]` works for all commands | BDD test pass |

---

## Open Questions

- None — the approach is well-defined via `clap_mangen`

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
