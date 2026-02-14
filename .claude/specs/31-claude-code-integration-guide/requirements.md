# Requirements: Claude Code Integration Guide

**Issue**: #31
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude (nmg-sdlc)

---

## User Story

**As a** developer using Claude Code for browser automation
**I want** a comprehensive integration guide and CLAUDE.md template for chrome-cli
**So that** Claude Code can immediately discover and effectively use chrome-cli to automate Chrome in my projects

---

## Background

chrome-cli is built primarily for AI agent consumption. Claude Code reads CLAUDE.md files to understand project tools. Currently the README has a minimal "Claude Code Integration" section (~20 lines), but there is no dedicated guide explaining discovery patterns, recommended workflows, error handling best practices, or a drop-in CLAUDE.md template.

A well-crafted integration guide will make chrome-cli immediately useful to Claude Code users — this is a key differentiator for the project. The guide should leverage chrome-cli's existing AI-friendly features: the `capabilities` manifest, the `examples` subcommand, JSON output, and accessibility-tree-based interaction model.

---

## Acceptance Criteria

### AC1: Integration guide covers discovery mechanisms

**Given** a developer has installed chrome-cli
**When** they read the integration guide
**Then** it explains how Claude Code discovers chrome-cli (PATH, `--help`, `capabilities` command)
**And** it provides a step-by-step setup checklist

**Example**:
- Given: chrome-cli is installed at `/usr/local/bin/chrome-cli`
- When: the developer reads `docs/claude-code.md`
- Then: they see instructions for verifying PATH availability, using `chrome-cli capabilities` for machine-readable discovery, and `chrome-cli examples` for learning commands

### AC2: CLAUDE.md template is provided as a drop-in example

**Given** a developer wants to enable Claude Code browser automation in their project
**When** they copy the CLAUDE.md template into their project
**Then** the template contains correct chrome-cli commands for common operations
**And** the template includes the recommended workflow loop (snapshot → interact → verify)
**And** the template references key commands: `capabilities`, `examples`, `page snapshot`, `interact`, `form fill`

**Example**:
- Given: the file `examples/CLAUDE.md.example` exists
- When: the developer copies it to their project as `CLAUDE.md` (or appends to existing)
- Then: Claude Code can read it and understand how to use chrome-cli for browser automation

### AC3: Common workflow patterns are documented

**Given** a developer is reading the integration guide
**When** they look for workflow examples
**Then** the guide documents at least four workflows: testing web apps, scraping data, debugging UI issues, and form automation
**And** each workflow shows the complete command sequence

**Example**:
- Given: the integration guide section "Common Workflows"
- When: the developer reads the "Testing Web Apps" workflow
- Then: they see: connect → navigate → snapshot → interact → verify with concrete chrome-cli commands

### AC4: Efficient usage tips minimize round-trips

**Given** a developer wants Claude Code to use chrome-cli efficiently
**When** they read the tips section
**Then** the guide explains batch commands (`form fill-many`), minimizing round-trips, using `--wait-until` to avoid race conditions, and using `--timeout` to prevent hangs
**And** the guide explains when to use `page snapshot` vs `page text` vs `page screenshot`

### AC5: Error handling patterns for AI agents are documented

**Given** an AI agent using chrome-cli encounters an error
**When** the developer reads the error handling section
**Then** the guide explains exit code conventions, stderr error parsing, common failure modes (connection refused, element not found, timeout), and recovery strategies
**And** it recommends checking exit codes and using `--timeout` flags

### AC6: Example conversation demonstrates real-world usage

**Given** a developer wants to see Claude Code using chrome-cli in practice
**When** they read the example conversation section
**Then** they see a realistic multi-turn example showing Claude Code debugging a web app
**And** the example uses actual chrome-cli commands with realistic output

### AC7: Best practices for AI agents are documented

**Given** an AI agent developer wants to optimize chrome-cli usage
**When** they read the best practices section
**Then** they find: always snapshot before interacting, use `--json` for reliable parsing, check exit codes, use `--timeout`, prefer `form fill` over `interact type`, use `console follow`/`network follow` for debugging
**And** the recommended workflow loop is clearly documented: snapshot → identify → interact → snapshot (verify)

### AC8: Recommended workflow loop is documented

**Given** a developer or AI agent needs to interact with a web page
**When** they follow the documented workflow loop
**Then** they use: (1) `page snapshot` to get the current accessibility tree, (2) identify the target element by UID, (3) `interact click <uid>` or `form fill <uid>` to act, (4) `page snapshot` again to verify the result
**And** a data extraction loop is also documented: navigate → wait → snapshot → extract

### Generated Gherkin Preview

```gherkin
Feature: Claude Code Integration Guide
  As a developer using Claude Code for browser automation
  I want a comprehensive integration guide and CLAUDE.md template
  So that Claude Code can immediately discover and use chrome-cli

  Scenario: Integration guide covers discovery mechanisms
    Given a developer has installed chrome-cli
    When they read the integration guide at "docs/claude-code.md"
    Then it explains how Claude Code discovers chrome-cli via PATH and capabilities
    And it provides a step-by-step setup checklist

  Scenario: CLAUDE.md template is provided as a drop-in example
    Given the file "examples/CLAUDE.md.example" exists in the repository
    When a developer copies it into their project
    Then it contains correct chrome-cli commands for common operations
    And it includes the recommended workflow loop

  Scenario: Common workflow patterns are documented
    Given the integration guide contains a "Common Workflows" section
    When a developer reads the workflows
    Then at least four workflows are documented with complete command sequences

  Scenario: Efficient usage tips minimize round-trips
    Given the integration guide contains a "Tips" section
    When a developer reads the efficiency tips
    Then it explains batch commands and strategies to minimize round-trips

  Scenario: Error handling patterns for AI agents
    Given the integration guide contains an "Error Handling" section
    When a developer reads the error handling patterns
    Then it documents exit codes, common failures, and recovery strategies

  Scenario: Example conversation demonstrates real-world usage
    Given the integration guide contains an "Example Conversation" section
    When a developer reads the example
    Then it shows a realistic multi-turn Claude Code session with chrome-cli

  Scenario: Best practices for AI agents are documented
    Given the integration guide contains a "Best Practices" section
    When a developer reads the best practices
    Then it covers snapshot-first interaction, JSON output, timeouts, and form fill

  Scenario: Recommended workflow loop is documented
    Given the integration guide contains a "Workflow Loop" section
    When a developer follows the documented loop
    Then they can reliably interact with web pages using snapshot-interact-verify
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Integration guide document at `docs/claude-code.md` | Must | Comprehensive guide with all sections from AC1-AC8 |
| FR2 | CLAUDE.md template at `examples/CLAUDE.md.example` | Must | Drop-in template with key commands and workflow loop |
| FR3 | Discovery section: PATH, --help, capabilities, examples | Must | How Claude Code finds and learns chrome-cli |
| FR4 | Common workflows: test, scrape, debug, form automation | Must | Complete command sequences for each |
| FR5 | Efficiency tips: batch commands, minimize round-trips | Must | Practical optimization advice |
| FR6 | Error handling patterns: exit codes, recovery strategies | Must | AI-agent-focused error handling |
| FR7 | Example conversation showing Claude Code + chrome-cli | Should | Realistic multi-turn debugging session |
| FR8 | Best practices checklist for AI agent usage | Must | Consolidated do/don't list |
| FR9 | Update README.md to link to the integration guide | Should | Replace minimal existing section with link |
| FR10 | Workflow loop diagrams (snapshot → interact → verify) | Should | Visual representation of recommended loops |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Accuracy** | All commands shown must match the current chrome-cli CLI interface |
| **Readability** | Written for both human developers and AI agents to parse |
| **Maintainability** | Commands should reference `chrome-cli capabilities` and `chrome-cli examples` so the guide stays up-to-date |
| **Completeness** | Cover all acceptance criteria from issue #31 |

---

## UI/UX Requirements

Not applicable — this is a documentation-only feature. No UI changes.

---

## Data Requirements

Not applicable — no data storage or API changes.

---

## Dependencies

### Internal Dependencies
- [x] Core chrome-cli commands implemented (connect, navigate, page, interact, form, etc.)
- [x] `capabilities` subcommand (#30) — provides machine-readable discovery
- [x] `examples` subcommand (#29) — provides built-in usage examples
- [x] Man pages generated for all commands

### External Dependencies
- None

### Blocked By
- None — all dependent features are already merged

---

## Out of Scope

- Video/GIF demos of Claude Code using chrome-cli (noted in issue as aspirational)
- Automated testing of the CLAUDE.md template with Claude Code itself
- Changes to chrome-cli binary or commands
- Integration guides for other AI agents (Copilot, Cursor, etc.)
- Tutorials for non-AI-agent users (covered by README and man pages)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| All acceptance criteria from issue #31 covered | 100% | Checklist review |
| Commands in guide are accurate and runnable | 100% | Manual verification against `chrome-cli capabilities` output |
| CLAUDE.md template contains all key commands | Yes | Template includes capabilities, examples, snapshot, interact, form fill |
| Guide is parseable by AI agents | Yes | Structured markdown with code blocks |

---

## Open Questions

- None — issue #31 is well-defined with clear acceptance criteria

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified (AC5, AC6)
- [x] Dependencies are identified (all resolved)
- [x] Out of scope is defined
- [x] Open questions are documented (none)
