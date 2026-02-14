# Requirements: Man Page Generation

**Issue**: #27
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude (writing-specs)

---

## User Story

**As a** developer or automation engineer using chrome-cli
**I want** Unix man pages generated from the CLI definition
**So that** I can access documentation through the standard `man` command or inline via `chrome-cli man`

---

## Background

Man pages are the standard documentation format on Unix systems. Users expect to find documentation via `man chrome-cli` after installing a CLI tool. The `clap_mangen` crate can generate roff-format man pages directly from clap's `Command` definition, keeping documentation automatically in sync with the CLI structure.

Issue #26 (comprehensive help text) has been completed, meaning all commands now have detailed `about`, `long_about`, and `after_long_help` attributes. This rich help text will flow directly into the generated man pages, producing high-quality documentation without manual authoring.

---

## Acceptance Criteria

### AC1: Generate man page for the top-level command

**Given** the chrome-cli binary is built
**When** I run the man page generation process
**Then** a man page file `chrome-cli.1` is produced
**And** it is valid roff-format man page content

### AC2: Generate man pages for all subcommands

**Given** the chrome-cli binary is built
**When** I run the man page generation process
**Then** man page files are produced for each subcommand (e.g., `chrome-cli-connect.1`, `chrome-cli-tabs.1`, `chrome-cli-navigate.1`)
**And** nested subcommands also get man pages (e.g., `chrome-cli-tabs-list.1`)

### AC3: Man pages include standard sections

**Given** a generated man page for any command
**When** I inspect its content
**Then** it includes NAME, SYNOPSIS, and DESCRIPTION sections
**And** it includes an OPTIONS section listing all flags and arguments

### AC4: Build script generates man pages to man/ directory

**Given** the chrome-cli source tree
**When** I run the build script or xtask to generate man pages
**Then** all man pages are written to the `man/` directory at the project root
**And** the generation completes without errors

### AC5: chrome-cli man subcommand displays man page inline

**Given** the chrome-cli binary is built
**When** I run `chrome-cli man`
**Then** the top-level man page content is displayed to stdout
**And** the exit code is 0

### AC6: chrome-cli man subcommand displays subcommand man page

**Given** the chrome-cli binary is built
**When** I run `chrome-cli man connect`
**Then** the man page for the `connect` subcommand is displayed to stdout
**And** the exit code is 0

### AC7: chrome-cli man with invalid subcommand produces error

**Given** the chrome-cli binary is built
**When** I run `chrome-cli man nonexistent`
**Then** an error message indicates the subcommand is not found
**And** the exit code is non-zero

### AC8: Man page generation uses clap_mangen crate

**Given** the project dependencies
**When** I inspect the man page generation implementation
**Then** it uses the `clap_mangen` crate to generate from the clap `Command` definition
**And** man pages stay automatically in sync with CLI definition changes

### AC9: Man pages are generated via xtask command

**Given** the chrome-cli source tree
**When** I run `cargo xtask man`
**Then** man pages are generated for all commands and subcommands
**And** they are written to the `man/` directory

### AC10: chrome-cli man help text describes usage

**Given** the chrome-cli binary is built
**When** I run `chrome-cli man --help`
**Then** the help text explains how to display man pages for commands
**And** lists available subcommand names

### Generated Gherkin Preview

```gherkin
Feature: Man Page Generation
  As a developer or automation engineer using chrome-cli
  I want Unix man pages generated from the CLI definition
  So that I can access documentation through the standard man command or inline

  Scenario: Display top-level man page inline
    Given chrome-cli is built
    When I run "chrome-cli man"
    Then stdout should contain "chrome-cli"
    And stdout should contain "SYNOPSIS"
    And the exit code should be 0

  Scenario Outline: Display subcommand man page inline
    Given chrome-cli is built
    When I run "chrome-cli man <subcommand>"
    Then stdout should contain "chrome-cli-<subcommand>"
    And the exit code should be 0

    Examples:
      | subcommand |
      | connect    |
      | tabs       |
      | navigate   |
      | page       |
      | js         |

  Scenario: Invalid subcommand produces error
    Given chrome-cli is built
    When I run "chrome-cli man nonexistent"
    Then the exit code should be non-zero

  Scenario: Generate man pages via xtask
    Given the chrome-cli source tree is available
    When I run "cargo xtask man"
    Then man page files exist in the man/ directory
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `chrome-cli man [COMMAND]` subcommand to display man pages inline | Must | Runtime man page rendering via clap_mangen |
| FR2 | Generate man pages for all top-level commands | Must | connect, tabs, navigate, page, dom, js, console, network, interact, form, emulate, perf, dialog, config, completions, man |
| FR3 | Generate man pages for nested subcommands | Must | e.g., tabs list, tabs create, page screenshot |
| FR4 | `cargo xtask man` command to write man pages to `man/` directory | Must | For packaging and distribution |
| FR5 | Man pages in standard roff format (section 1) | Must | Compatible with `man` command |
| FR6 | Use `clap_mangen` crate for generation | Must | Keeps docs in sync with CLI definition |
| FR7 | Include help text, examples, and options in generated pages | Should | Leverages long_about and after_long_help from #26 |
| FR8 | Man pages included in release archives | Could | Under `man/` directory in release tarballs |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | `chrome-cli man` output is instant (< 50ms) — no Chrome connection needed |
| **Reliability** | Man pages stay in sync with CLI definition automatically (clap introspection) |
| **Platforms** | `chrome-cli man` works on all platforms; `man` command integration is Unix-only |

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
- Colored/styled output for `chrome-cli man` (plain roff rendering)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Command coverage | Man pages for all 16 top-level commands + nested subcommands | Count files in `man/` directory |
| Content completeness | NAME, SYNOPSIS, DESCRIPTION, OPTIONS in every man page | Grep man page content |
| Inline display | `chrome-cli man [cmd]` works for all commands | BDD test pass |

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
