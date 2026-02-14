# Requirements: Shell Completions Generation

**Issue**: #25
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude (writing-specs)

---

## User Story

**As a** developer or automation engineer using chrome-cli
**I want** tab-completion for all commands, flags, and enum values in my shell
**So that** I can discover and use chrome-cli features faster without consulting documentation

---

## Background

chrome-cli has a rich command hierarchy with 12+ top-level commands, nested subcommands, numerous flags, and type-safe enum values (e.g., `--format`, `--wait-until`, `--direction`). Shell completions make this discoverable at the command line.

clap v4 has built-in support for generating shell completion scripts via the `clap_complete` crate. This leverages the existing `Cli` parser definition to produce accurate completions for all major shells without manual maintenance.

---

## Acceptance Criteria

### AC1: Generate bash completion script

**Given** the chrome-cli binary is installed
**When** the user runs `chrome-cli completions bash`
**Then** a valid bash completion script is printed to stdout
**And** the exit code is 0

### AC2: Generate zsh completion script

**Given** the chrome-cli binary is installed
**When** the user runs `chrome-cli completions zsh`
**Then** a valid zsh completion script is printed to stdout
**And** the exit code is 0

### AC3: Generate fish completion script

**Given** the chrome-cli binary is installed
**When** the user runs `chrome-cli completions fish`
**Then** a valid fish completion script is printed to stdout
**And** the exit code is 0

### AC4: Generate powershell completion script

**Given** the chrome-cli binary is installed
**When** the user runs `chrome-cli completions powershell`
**Then** a valid powershell completion script is printed to stdout
**And** the exit code is 0

### AC5: Generate elvish completion script

**Given** the chrome-cli binary is installed
**When** the user runs `chrome-cli completions elvish`
**Then** a valid elvish completion script is printed to stdout
**And** the exit code is 0

### AC6: Completions include all subcommands

**Given** a generated completion script for any supported shell
**When** the user types `chrome-cli <TAB>`
**Then** all top-level subcommands are suggested (connect, tabs, navigate, page, dom, js, console, network, interact, form, emulate, perf, dialog, config, completions)

### AC7: Completions include nested subcommands

**Given** a generated completion script for any supported shell
**When** the user types `chrome-cli tabs <TAB>`
**Then** nested subcommands are suggested (list, create, close, activate)

### AC8: Completions include flags with descriptions

**Given** a generated completion script for any supported shell
**When** the user types `chrome-cli navigate --<TAB>`
**Then** available flags are suggested with their descriptions

### AC9: Completions include enum values

**Given** a generated completion script for any supported shell
**When** the user types `chrome-cli navigate url --wait-until <TAB>`
**Then** the valid enum values are suggested (load, domcontentloaded, networkidle, none)

### AC10: Invalid shell argument produces error

**Given** the chrome-cli binary is installed
**When** the user runs `chrome-cli completions invalid-shell`
**Then** an error message is displayed listing the valid shell options
**And** the exit code is non-zero

### AC11: Help text includes installation instructions

**Given** the chrome-cli binary is installed
**When** the user runs `chrome-cli completions --help`
**Then** the help text includes installation instructions for each supported shell

### Generated Gherkin Preview

```gherkin
Feature: Shell Completions Generation
  As a developer or automation engineer using chrome-cli
  I want tab-completion for all commands, flags, and enum values in my shell
  So that I can discover and use chrome-cli features faster without consulting documentation

  Scenario Outline: Generate completion script for supported shells
    Given the chrome-cli binary is built
    When I run "chrome-cli completions <shell>"
    Then the output should be a non-empty completion script
    And the exit code should be 0

    Examples:
      | shell      |
      | bash       |
      | zsh        |
      | fish       |
      | powershell |
      | elvish     |

  Scenario: Completions contain all top-level subcommands
    Given the chrome-cli binary is built
    When I run "chrome-cli completions bash"
    Then the output should contain "navigate"
    And the output should contain "tabs"
    And the output should contain "completions"

  Scenario: Invalid shell argument produces error
    Given the chrome-cli binary is built
    When I run "chrome-cli completions invalid-shell"
    Then the exit code should be non-zero
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `chrome-cli completions <SHELL>` subcommand | Must | Positional arg for shell name |
| FR2 | Support bash, zsh, fish, powershell, elvish | Must | All major shells |
| FR3 | Output completion script to stdout | Must | Allows piping to file |
| FR4 | Include all subcommands and nested subcommands | Must | Derived from clap `Cli` struct |
| FR5 | Include all flags with descriptions | Must | Derived from clap `Cli` struct |
| FR6 | Include enum values for ValueEnum flags | Must | e.g., --wait-until, --format |
| FR7 | Include installation instructions in help text | Should | Per-shell instructions |
| FR8 | CI verification that completions generate without errors | Should | All shells tested in CI |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Completion generation is instant (< 50ms) — no Chrome connection needed |
| **Reliability** | Completions stay in sync with CLI definition automatically (clap introspection) |
| **Platforms** | Works on macOS, Linux, and Windows (generation only, not installation) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| shell | Shell enum (bash, zsh, fish, powershell, elvish) | Must be valid shell name | Yes |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| completion script | String (stdout) | Shell-specific completion script text |

---

## Dependencies

### Internal Dependencies
- [x] Issue #3 (CLI skeleton) — all commands must be defined

### External Dependencies
- [ ] `clap_complete` crate — shell completion generation library

### Blocked By
- None — all prerequisites are met (CLI skeleton with all commands is complete)

---

## Out of Scope

- Man page generation (may be a separate issue)
- Dynamic/runtime completions (e.g., completing tab IDs from a running Chrome)
- Auto-installation of completions (user must redirect stdout to appropriate file)
- Completions for nushell or other non-standard shells

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Shell coverage | 5 shells (bash, zsh, fish, powershell, elvish) | Count of supported shells |
| Subcommand coverage | 100% of defined subcommands appear in completions | Grep output for command names |
| CI passing | Completions generate without errors for all shells | CI test step |

---

## Open Questions

- None — the approach is well-defined via `clap_complete`

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
