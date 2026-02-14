# Requirements: README with Quick-Start, Examples, and Architecture Overview

**Issue**: #28
**Date**: 2026-02-14
**Status**: Approved
**Author**: Claude (writing-specs)

---

## User Story

**As a** developer or automation engineer discovering chrome-cli for the first time
**I want** a comprehensive README with installation instructions, quick-start guide, usage examples, and architecture overview
**So that** I can quickly understand what chrome-cli does, install it, and start using it without reading source code

---

## Background

The chrome-cli project has grown into a feature-rich CLI tool with 15+ major commands, multi-platform release pipelines, and extensive help text. However, the README is still a placeholder stub with only a project name, one-line description, and license section. As the primary entry point for new users and contributors, the README must communicate the tool's capabilities, installation options, and usage patterns clearly and concisely.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Header section with badges and description

**Given** the README.md file exists in the repository root
**When** a user views the README
**Then** it displays the project name "chrome-cli" and a one-line description
**And** it displays badges for CI status, license, and crates.io (when published)

**Example**:
- Given: README.md at repository root
- When: User opens the file
- Then: First line is a heading with "chrome-cli", followed by badges and a description matching Cargo.toml

### AC2: Features section with capabilities list

**Given** the README.md file exists
**When** a user reads the Features section
**Then** it contains a bullet list of key capabilities covering: tab management, navigation, page inspection, screenshots, JavaScript execution, form filling, network monitoring, performance tracing, and device emulation
**And** it includes a comparison table showing advantages over alternatives (standalone binary, no Node.js/Python dependency)

### AC3: Installation section with multiple methods

**Given** the README.md file exists
**When** a user reads the Installation section
**Then** it provides installation instructions for:
  - Pre-built binaries from GitHub Releases (with curl one-liner for macOS and Linux)
  - Cargo install: `cargo install chrome-cli`
  - Building from source
**And** it lists supported platforms (macOS ARM, macOS Intel, Linux x64, Linux ARM, Windows)

### AC4: Quick Start section with step-by-step guide

**Given** the README.md file exists
**When** a user reads the Quick Start section
**Then** it provides a numbered getting-started guide with at least 5 steps covering:
  1. Install chrome-cli
  2. Start Chrome with remote debugging
  3. Connect to Chrome
  4. Navigate to a URL
  5. Inspect the page
**And** each step includes the actual CLI command to run

### AC5: Usage Examples section with common workflows

**Given** the README.md file exists
**When** a user reads the Usage Examples section
**Then** it provides copy-pasteable command sequences for common workflows including:
  - Taking a screenshot
  - Extracting page text
  - Executing JavaScript
  - Form filling
  - Network monitoring
**And** each example includes the command and expected output format

### AC6: Command Reference section with all commands

**Given** the README.md file exists
**When** a user reads the Command Reference section
**Then** it contains a table listing all top-level commands (connect, tabs, navigate, page, js, console, network, interact, form, emulate, perf, dialog, config, completions, man) with brief descriptions
**And** it links to detailed help via `chrome-cli <command> --help` or man pages

### AC7: Architecture section with CDP communication overview

**Given** the README.md file exists
**When** a user reads the Architecture section
**Then** it includes a diagram showing how chrome-cli communicates with Chrome via CDP over WebSocket
**And** it describes the session management model
**And** it mentions key performance characteristics (native Rust binary, fast startup)

### AC8: Claude Code Integration section

**Given** the README.md file exists
**When** a user reads the Claude Code Integration section
**Then** it explains how to use chrome-cli with Claude Code for AI-assisted browser automation
**And** it provides an example CLAUDE.md snippet

### AC9: Contributing section with development setup

**Given** the README.md file exists
**When** a user reads the Contributing section
**Then** it includes development setup instructions (clone, build, test)
**And** it mentions code style guidelines (Clippy, rustfmt)
**And** it links to or describes how to run the test suite

### AC10: License section

**Given** the README.md file exists
**When** a user reads the License section
**Then** it states the dual MIT/Apache-2.0 license
**And** it links to both LICENSE-MIT and LICENSE-APACHE files

### AC11: Concise with collapsible sections

**Given** the README.md file exists
**When** a user scans the README
**Then** lengthy examples use collapsible `<details>` sections to keep the page scannable
**And** detailed documentation is linked to rather than inlined

### Generated Gherkin Preview

```gherkin
Feature: README documentation
  As a developer discovering chrome-cli
  I want a comprehensive README
  So that I can quickly install and start using the tool

  Scenario: Header with badges and description
    Given the README.md file exists in the repository root
    When I read the file content
    Then it starts with a heading containing "chrome-cli"
    And it contains badge image links for CI and license

  Scenario: Features section lists capabilities
    Given the README.md file exists
    When I read the Features section
    Then it lists key capabilities as bullet points
    And it contains a comparison table

  Scenario: Installation section with multiple methods
    Given the README.md file exists
    When I read the Installation section
    Then it contains "cargo install chrome-cli"
    And it contains curl commands for binary downloads
    And it lists supported platforms

  Scenario: Quick Start guide
    Given the README.md file exists
    When I read the Quick Start section
    Then it contains numbered steps
    And it includes "chrome-cli connect"
    And it includes "chrome-cli navigate"

  Scenario: Usage examples are copy-pasteable
    Given the README.md file exists
    When I read the Usage Examples section
    Then it contains code blocks with CLI commands
    And it covers screenshot, text extraction, and JS execution

  Scenario: Command Reference table
    Given the README.md file exists
    When I read the Command Reference section
    Then it contains a table with all top-level commands
    And each command has a description

  Scenario: Architecture overview with diagram
    Given the README.md file exists
    When I read the Architecture section
    Then it contains a diagram showing CDP communication
    And it describes session management

  Scenario: Claude Code integration guide
    Given the README.md file exists
    When I read the Claude Code Integration section
    Then it explains usage with Claude Code
    And it provides a CLAUDE.md example snippet

  Scenario: Contributing guide
    Given the README.md file exists
    When I read the Contributing section
    Then it includes build and test commands
    And it mentions code style requirements

  Scenario: License information
    Given the README.md file exists
    When I read the License section
    Then it states "MIT" and "Apache-2.0"
    And it links to license files

  Scenario: Collapsible sections for lengthy content
    Given the README.md file exists
    When I scan the full document
    Then lengthy examples use details/summary HTML tags
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Header with project name, description, and badges | Must | Badges: CI, license, crates.io |
| FR2 | Features section with capabilities list and comparison table | Must | |
| FR3 | Installation section with cargo install, binary downloads, source build | Must | |
| FR4 | Quick Start section with 5-step guide | Must | |
| FR5 | Usage Examples with common workflows | Must | Screenshot, text, JS, forms, network |
| FR6 | Command Reference table covering all 15+ commands | Must | |
| FR7 | Architecture section with CDP diagram | Must | |
| FR8 | Claude Code Integration section | Should | |
| FR9 | Contributing section with dev setup | Should | |
| FR10 | License section | Must | |
| FR11 | Collapsible sections for lengthy content | Should | `<details>` tags |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Readability** | Scannable in under 2 minutes; key info visible without scrolling past first screen |
| **Accuracy** | All commands and examples must match current CLI implementation |
| **Maintainability** | Structured so sections can be updated independently as features evolve |
| **Platforms** | Renders correctly on GitHub, crates.io, and in terminal Markdown viewers |

---

## Data Requirements

### Input Data

This feature has no runtime input data — it is a static documentation file.

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| README.md | Markdown file | Comprehensive project documentation at repository root |

---

## Dependencies

### Internal Dependencies
- [x] Core CLI commands implemented (connect, tabs, navigate, page, js, etc.)
- [x] CI workflow exists (.github/workflows/ci.yml)
- [x] Release workflow exists (.github/workflows/release.yml)
- [x] Man page generation via xtask
- [x] License files exist (LICENSE-MIT, LICENSE-APACHE)

### External Dependencies
- None

### Blocked By
- None — all prerequisite features are implemented

---

## Out of Scope

- Hosted documentation site (e.g., mdBook, GitHub Pages) — future issue
- Changelog generation — separate concern
- API/library documentation (rustdoc) — separate concern
- Homebrew formula — not yet set up
- Terminal screenshots/recordings — can be added later
- Translations / i18n — English only for now

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Section completeness | All 10 sections present | Manual review |
| Command coverage | All 15+ top-level commands listed | Diff against `chrome-cli --help` |
| Example accuracy | All examples runnable | Manual testing |
| Badge rendering | All badges display correctly on GitHub | Visual check |

---

## Open Questions

- None — requirements are well-defined by the issue.

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
