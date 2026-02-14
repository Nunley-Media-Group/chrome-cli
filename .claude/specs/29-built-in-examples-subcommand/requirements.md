# Requirements: Built-in Examples Subcommand

**Issue**: #29
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude (spec generation)

---

## User Story

**As a** developer or AI agent using chrome-cli
**I want** a dedicated `examples` subcommand that prints usage examples for each command group
**So that** I can quickly discover working CLI invocations without parsing `--help` output or reading external documentation

---

## Background

AI agents benefit from being able to query examples programmatically. The existing `--help` and `after_long_help` text in clap provides examples, but they are interleaved with flag descriptions and not easily machine-parseable. A dedicated `chrome-cli examples` command provides a focused, structured view of real usage patterns — in both plain text (human-readable) and JSON (machine-readable) formats.

This aligns with the M6 milestone (Documentation & AI Discoverability) and the product principle of scriptability.

---

## Acceptance Criteria

### AC1: List all command groups with summary examples

**Given** chrome-cli is installed
**When** I run `chrome-cli examples`
**Then** the output lists every command group with a brief description and one representative example each
**And** the exit code is 0

**Example**:
- Given: chrome-cli binary is on PATH
- When: `chrome-cli examples`
- Then: Output includes all command groups (connect, tabs, navigate, page, js, console, network, interact, form, emulate, perf, dialog, config) each with a one-line description and one example command

### AC2: Show detailed examples for a specific command group

**Given** chrome-cli is installed
**When** I run `chrome-cli examples <COMMAND>` (e.g., `chrome-cli examples navigate`)
**Then** the output shows 3–5 detailed examples for that command group
**And** each example includes the command string and a description comment
**And** the exit code is 0

**Example**:
- Given: chrome-cli binary is on PATH
- When: `chrome-cli examples navigate`
- Then: Output includes examples like "Navigate to a URL and wait for load" with `chrome-cli navigate https://example.com --wait-until load`

### AC3: JSON output for summary listing

**Given** chrome-cli is installed
**When** I run `chrome-cli examples --json`
**Then** the output is a valid JSON array of objects with `command`, `description`, and `examples` fields
**And** each example object contains `cmd` and `description` fields
**And** the exit code is 0

### AC4: JSON output for a specific command group

**Given** chrome-cli is installed
**When** I run `chrome-cli examples navigate --json`
**Then** the output is a valid JSON object with `command`, `description`, and `examples` fields
**And** each example includes `cmd`, `description`, and optional `flags` fields
**And** the exit code is 0

### AC5: Error on unknown command group

**Given** chrome-cli is installed
**When** I run `chrome-cli examples nonexistent`
**Then** the output is an error message indicating the command group is not recognized
**And** the exit code is non-zero

### AC6: All command groups have examples

**Given** chrome-cli is installed
**When** I run `chrome-cli examples --json`
**Then** the output contains entries for all command groups: connect, tabs, navigate, page, js, console, network, interact, form, emulate, perf, dialog, config
**And** each command group has at least 3 examples

### AC7: Plain text output is the default

**Given** chrome-cli is installed
**When** I run `chrome-cli examples` without any output flags
**Then** the output is human-readable plain text (not JSON)
**And** examples are formatted with comment-style descriptions (# prefix)

### AC8: Pretty-printed JSON output

**Given** chrome-cli is installed
**When** I run `chrome-cli examples --pretty`
**Then** the output is a pretty-printed (indented) JSON array
**And** the exit code is 0

### Generated Gherkin Preview

```gherkin
Feature: Built-in Examples Subcommand
  As a developer or AI agent
  I want a dedicated examples subcommand
  So that I can discover working CLI invocations without parsing --help output

  Scenario: List all command groups with summary examples
    Given chrome-cli is installed
    When I run "chrome-cli examples"
    Then the output lists every command group with a description and example
    And the exit code is 0

  Scenario: Show detailed examples for a specific command group
    Given chrome-cli is installed
    When I run "chrome-cli examples navigate"
    Then the output shows 3-5 detailed examples for "navigate"
    And each example includes a command and description
    And the exit code is 0

  Scenario: JSON output for summary listing
    Given chrome-cli is installed
    When I run "chrome-cli examples --json"
    Then the output is a valid JSON array
    And each entry has "command", "description", and "examples" fields

  Scenario: Error on unknown command group
    Given chrome-cli is installed
    When I run "chrome-cli examples nonexistent"
    Then the output contains an error message
    And the exit code is non-zero
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `chrome-cli examples` lists all command groups with one example each | Must | Default plain text output |
| FR2 | `chrome-cli examples <COMMAND>` shows detailed examples for one group | Must | 3–5 examples per group |
| FR3 | `--json` flag produces structured JSON output | Must | Follows existing OutputFormat pattern |
| FR4 | `--pretty` flag produces pretty-printed JSON | Must | Follows existing OutputFormat pattern |
| FR5 | `--plain` flag forces plain text output | Must | Follows existing OutputFormat pattern |
| FR6 | Every command group (connect, tabs, navigate, page, js, console, network, interact, form, emulate, perf, dialog, config) has examples | Must | At least 3 examples each |
| FR7 | Examples are syntactically valid chrome-cli commands | Should | Verified at test time |
| FR8 | Error on unknown command group names | Must | Non-zero exit code |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Command should respond in < 10ms (no Chrome connection needed) |
| **Security** | No security implications — purely informational, no network/CDP calls |
| **Platforms** | Same as chrome-cli: macOS, Linux, Windows |
| **Reliability** | Static data, no external dependencies — always works |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| command | String (positional arg) | Must match a known command group name | No |

### Output Data (JSON mode)

| Field | Type | Description |
|-------|------|-------------|
| command | String | Command group name (e.g., "navigate") |
| description | String | Brief description of the command group |
| examples | Array | List of example objects |
| examples[].cmd | String | Full command string |
| examples[].description | String | What the example demonstrates |
| examples[].flags | Array<String> | Optional: relevant flags used |

---

## Dependencies

### Internal Dependencies
- [x] All CLI command groups implemented (connect, tabs, navigate, page, js, console, network, interact, form, emulate, perf, dialog, config)
- [x] `OutputFormat` global flags (--json, --pretty, --plain) already exist

### External Dependencies
- None

### Blocked By
- None (all command groups already exist)

---

## Out of Scope

- Interactive / REPL mode for exploring examples
- Workflow/recipe examples that chain multiple commands (future enhancement)
- Generating examples from test fixtures or golden files (future enhancement)
- Markdown or HTML output formats
- Man page generation for examples (covered by `chrome-cli man`)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Command group coverage | 100% of groups have examples | Count groups in JSON output |
| Examples per group | ≥ 3 per group | Count examples in JSON output |
| Response time | < 10ms | No CDP connection required |

---

## Open Questions

- None — requirements are clear from the issue.

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
