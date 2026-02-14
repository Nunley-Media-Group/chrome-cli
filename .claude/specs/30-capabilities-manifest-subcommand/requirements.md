# Requirements: Machine-Readable Capabilities Manifest Subcommand

**Issue**: #30
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude (spec generation)

---

## User Story

**As a** developer or AI agent integrating with chrome-cli
**I want** a `capabilities` subcommand that outputs a complete, machine-readable JSON manifest of all commands, parameters, and their types
**So that** I can programmatically discover the full CLI surface and build correct commands without parsing `--help` output

---

## Background

AI agents like Claude Code need to understand what commands are available and what parameters they accept. While `--help` is human-readable and the `examples` subcommand shows usage patterns, neither provides a structured, complete schema of the CLI surface. A `chrome-cli capabilities` command outputs a JSON manifest describing every command, subcommand, flag, argument, and return type — essentially an OpenAPI spec for the CLI.

This differs from the `examples` command (issue #29) in a key way: `examples` shows *how* to use commands with sample invocations, while `capabilities` describes *what* the commands are — their full parameter signatures, types, defaults, and return schemas. Together they give AI agents complete programmatic access to the CLI.

The manifest must be generated at runtime from the clap command tree (not static data), so it stays automatically in sync as commands are added or modified.

---

## Acceptance Criteria

### AC1: Full capabilities manifest output

**Given** chrome-cli is installed
**When** I run `chrome-cli capabilities`
**Then** the output is a valid JSON object containing `name`, `version`, and `commands` fields
**And** every command and subcommand in the CLI is represented
**And** the exit code is 0

**Example**:
- Given: chrome-cli binary is on PATH
- When: `chrome-cli capabilities`
- Then: JSON output with `name: "chrome-cli"`, `version` matching binary version, and `commands` array covering all command groups

### AC2: Command entries include full metadata

**Given** chrome-cli is installed
**When** I run `chrome-cli capabilities`
**Then** each command entry includes `name`, `description`, and `subcommands` (if applicable)
**And** each subcommand entry includes `name`, `description`, `args`, and `flags`
**And** each arg/flag includes `name`, `type`, `required`, and `description`
**And** optional fields include `default` and `values` (for enums)

### AC3: Filter by specific command

**Given** chrome-cli is installed
**When** I run `chrome-cli capabilities --command navigate`
**Then** the output is a valid JSON object describing only the `navigate` command and its subcommands
**And** the exit code is 0

### AC4: Compact output mode

**Given** chrome-cli is installed
**When** I run `chrome-cli capabilities --compact`
**Then** the output is a minimal JSON object containing only command names and brief descriptions
**And** args, flags, and return types are omitted
**And** the exit code is 0

### AC5: Global flags are included

**Given** chrome-cli is installed
**When** I run `chrome-cli capabilities`
**Then** the output includes a `global_flags` array
**And** the array contains entries for `--port`, `--host`, `--ws-url`, `--timeout`, `--tab`, `--auto-dismiss-dialogs`, `--config`, `--json`, `--pretty`, and `--plain`

### AC6: Generated from clap definition at runtime

**Given** the CLI has a new command added to the `Command` enum
**When** I build and run `chrome-cli capabilities`
**Then** the new command appears automatically in the manifest without manual updates
**And** all its args and flags are correctly reflected

### AC7: Exit codes are documented

**Given** chrome-cli is installed
**When** I run `chrome-cli capabilities`
**Then** the output includes an `exit_codes` section documenting all exit code values and their meanings

### AC8: Error on unknown command filter

**Given** chrome-cli is installed
**When** I run `chrome-cli capabilities --command nonexistent`
**Then** stderr contains an error message indicating the command is not recognized
**And** the exit code is 1

### AC9: Pretty-printed JSON output

**Given** chrome-cli is installed
**When** I run `chrome-cli capabilities --pretty`
**Then** the output is a pretty-printed (indented) JSON object
**And** the exit code is 0

### AC10: Enum values are listed

**Given** chrome-cli is installed
**When** I run `chrome-cli capabilities`
**Then** flags with enum types (e.g., `--wait-until`, `--format`) include a `values` array listing all possible values

### Generated Gherkin Preview

```gherkin
Feature: Capabilities Manifest Subcommand
  As a developer or AI agent
  I want a capabilities subcommand that outputs a machine-readable manifest
  So that I can programmatically discover the full CLI surface

  Scenario: Full capabilities manifest output
    Given chrome-cli is installed
    When I run "chrome-cli capabilities"
    Then the output is a valid JSON object
    And the output has "name", "version", and "commands" fields
    And every command group is represented
    And the exit code is 0

  Scenario: Filter by specific command
    Given chrome-cli is installed
    When I run "chrome-cli capabilities --command navigate"
    Then the output describes only the "navigate" command
    And the exit code is 0

  Scenario: Compact output mode
    Given chrome-cli is installed
    When I run "chrome-cli capabilities --compact"
    Then the output contains only command names and descriptions
    And args and flags are omitted

  Scenario: Error on unknown command
    Given chrome-cli is installed
    When I run "chrome-cli capabilities --command nonexistent"
    Then stderr contains an error message
    And the exit code is 1
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `chrome-cli capabilities` outputs a complete JSON manifest of all commands, subcommands, args, flags, and types | Must | Generated from clap at runtime |
| FR2 | `--command <CMD>` filters output to a single command group | Must | |
| FR3 | `--compact` produces minimal output (names + descriptions only) | Must | |
| FR4 | `--pretty` produces indented JSON output | Must | Follows existing OutputFormat pattern |
| FR5 | Manifest includes `global_flags` array | Must | |
| FR6 | Manifest includes `exit_codes` section | Must | |
| FR7 | Enum-typed flags list their possible values | Must | |
| FR8 | Args include `type`, `required`, `description`, and optional `default` | Must | |
| FR9 | Manifest includes `name` and `version` at the root level | Must | |
| FR10 | Error on unknown command name with `--command` flag | Must | Non-zero exit code |
| FR11 | Manifest is auto-generated from clap introspection — not a static data file | Must | Key differentiator from `examples` |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Command should respond in < 50ms (no Chrome connection needed) |
| **Security** | No security implications — purely informational, no network/CDP calls |
| **Platforms** | Same as chrome-cli: macOS, Linux, Windows |
| **Reliability** | Deterministic output — always reflects the current binary's command tree |
| **Maintainability** | Zero manual maintenance — adding commands to the `Command` enum is sufficient |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| --command | String (flag) | Must match a known command name | No |
| --compact | Bool (flag) | N/A | No |
| --pretty | Bool (flag) | N/A | No |

### Output Data (JSON)

| Field | Type | Description |
|-------|------|-------------|
| name | String | CLI binary name ("chrome-cli") |
| version | String | Binary version from Cargo.toml |
| commands | Array | Command group descriptors |
| commands[].name | String | Command name (e.g., "navigate") |
| commands[].description | String | Brief description |
| commands[].subcommands | Array | Nested subcommands (if any) |
| commands[].subcommands[].name | String | Subcommand name |
| commands[].subcommands[].description | String | Subcommand description |
| commands[].subcommands[].args | Array | Positional arguments |
| commands[].subcommands[].flags | Array | Optional flags |
| commands[].subcommands[].args[].name | String | Argument name |
| commands[].subcommands[].args[].type | String | Value type (string, integer, bool, enum, etc.) |
| commands[].subcommands[].args[].required | Bool | Whether the arg is required |
| commands[].subcommands[].args[].description | String | Argument description |
| commands[].subcommands[].flags[].name | String | Flag name (e.g., "--timeout") |
| commands[].subcommands[].flags[].type | String | Value type |
| commands[].subcommands[].flags[].default | Any | Default value (if applicable) |
| commands[].subcommands[].flags[].values | Array | Possible values (for enum types) |
| commands[].subcommands[].flags[].description | String | Flag description |
| global_flags | Array | Global flag descriptors (same shape as flags) |
| exit_codes | Array | Exit code descriptors |
| exit_codes[].code | Integer | Exit code value |
| exit_codes[].name | String | Exit code name (e.g., "ConnectionError") |
| exit_codes[].description | String | When this code is returned |

---

## Dependencies

### Internal Dependencies
- [x] All CLI commands defined in `src/cli/mod.rs` with clap derive macros
- [x] `OutputFormat` global flags (--json, --pretty, --plain) already exist
- [x] `Cli::command()` (clap's `CommandFactory`) already used by `completions` and `man` commands
- [x] `ExitCode` enum defined in `src/error.rs`

### External Dependencies
- None new (clap introspection is built-in)

### Blocked By
- None

---

## Out of Scope

- Return type schemas for each command (future enhancement — requires annotating each command's output struct)
- Generating client libraries from the manifest
- OpenAPI or JSON Schema `$ref` format compliance
- Non-JSON output formats (YAML, TOML, etc.)
- Interactive exploration mode
- Versioning/diffing of the manifest between releases

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Command coverage | 100% of commands/subcommands in manifest | Compare manifest entries to `Command` enum variants |
| Flag/arg coverage | 100% of flags and args in manifest | Compare to clap-derived structs |
| Auto-sync verification | New commands appear without code changes to capabilities module | Add test that walks clap tree and compares |
| Response time | < 50ms | No CDP connection required |

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
