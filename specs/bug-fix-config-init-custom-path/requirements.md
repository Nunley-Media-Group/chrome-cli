# Defect Report: `config init` ignores `--config` path and exits non-zero while reporting success

**Issue**: #249
**Date**: 2026-04-23
**Status**: Draft
**Author**: Rich Nunley
**Severity**: High
**Related Spec**: `specs/feature-configuration-file-support/`

---

## Reproduction

### Steps to Reproduce

1. Run `agentchrome config init --config /tmp/my.toml` (any path other than the default XDG location).
2. Inspect stdout and `$?`.
3. Inspect the filesystem for the requested path versus the default XDG path.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS 15 (Darwin 25.3.0); reproduces on Linux/Windows because the same code path is used. |
| **Version / Commit** | 1.46.0 (`546bd96`) |
| **Browser / Runtime** | N/A — CLI command, no browser involved. |
| **Configuration** | Default — no pre-existing config file required to reproduce the path-mismatch. |

### Frequency

Always.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | The template config file is written to the path supplied via `--config`, the JSON output reports that path, and the process exits 0. |
| **Actual** | The `--config` flag is ignored. The file is written to the default XDG path, the JSON output reports the default path, and the process exits 1 (the global `--config` resolution treats the supplied path as a missing config file to *read*, not a destination to *write*). |

### Error Output

```
$ agentchrome config init --config /tmp/my.toml
{"created":"/Users/rnunley/Library/Application Support/agentchrome/config.toml"}
$ echo $?
1
$ ls /tmp/my.toml
ls: /tmp/my.toml: No such file or directory
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: `--config` Honored as Init Destination

**Given** a writable target path `/tmp/custom.toml` that does not yet exist
**When** the user runs `agentchrome config init --config /tmp/custom.toml`
**Then** the template config file is written to `/tmp/custom.toml`
**And** the JSON output's `created` field reports `/tmp/custom.toml`
**And** the process exits 0.

### AC2: Default Path Preserved When No Path Flag Given

**Given** no `--config` and no `--path` flag is supplied
**When** the user runs `agentchrome config init`
**Then** the template is written to the platform XDG default
**And** the JSON output's `created` field reports the XDG default
**And** the process exits 0.

### AC3: Clear Error On Unwritable Path (No Silent Default Fallback)

**Given** `--config /nonexistent/dir/file.toml` where the parent directory does not exist and cannot be created
**When** the user runs `agentchrome config init --config /nonexistent/dir/file.toml`
**Then** no file is written to the XDG default
**And** the process exits 1
**And** stderr contains a clear message identifying the unwritable path.

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `config init` MUST treat the global `--config <path>` flag as the destination path when no subcommand-level `--path` is given. | Must |
| FR2 | When `--config` (or `--path`) is supplied, the XDG default path MUST NOT be created and MUST NOT appear in the `created` field. | Must |
| FR3 | The JSON `created` field MUST always reflect the path actually written. | Must |
| FR4 | The process exit code MUST be 0 on success and non-zero only when no file was written. | Must |
| FR5 | If both `--config` and `--path` are supplied with conflicting values, `--path` MUST win (subcommand-level flag is more specific) and a one-line stderr note MUST acknowledge the override. | Should |

---

## Out of Scope

- Changing the template content emitted by `config init`.
- Changing `config show` or `config path` behaviour.
- Changing how `--config` is interpreted by *non-init* subcommands (it remains a read-from path everywhere else).
- Refactoring the global config resolution chain (`find_config_file`, `load_config`).

---

## Validation Checklist

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC2 guards default-path behavior)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
