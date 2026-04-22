# Requirements: Improve Error Output Consistency on All Failure Paths

**Issues**: #197
**Date**: 2026-04-21
**Status**: Draft
**Author**: Claude (spec-driven)

---

## User Story

**As an** AI agent or browser-automation engineer debugging an AgentChrome command that failed
**I want** every non-zero exit to emit a descriptive JSON error object on stderr
**So that** I can programmatically identify what went wrong without guessing at silent failures

---

## Background

AgentChrome already defines a structured error contract: `src/error.rs` exposes `AppError` and `ExitCode` (0=success, 1=general, 2=connection, 3=target, 4=timeout, 5=protocol), and `src/main.rs` serialises errors to stderr as `{"error": "...", "code": N}`. `AppError.custom_json` allows richer structured payloads (already used by `js exec` and structured connection-loss errors).

However, several command paths exit with code 1 with no stderr output at all. `form fill` against an element whose type is not fillable (e.g., `div`, `canvas`, a `role="combobox"` without editable input) is the most frequently reported case — the command fails silently. Other paths observed to sometimes skip the error system include certain CDP protocol failures in `interact.rs` and page-wait timeouts that short-circuit before hitting `AppError`. AI agents parsing stderr cannot distinguish between "exited without message" and "empty-string error" — both look identical.

This feature audits every command module for silent failure paths and routes every failure through `AppError::print_json_stderr`, ensuring the structured-output contract documented in `steering/product.md` and `steering/tech.md` is honoured uniformly. The retrospective learning about global output contracts being silently violated by individual commands (see #96, #98, #114) directly motivates this work.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Every non-zero exit produces a structured JSON error on stderr

**Given** any AgentChrome subcommand is invoked
**When** the command exits with a non-zero exit code
**Then** stderr contains exactly one line that parses as a JSON object
**And** the object has a string `error` field with a non-empty human-readable message
**And** the object has an integer `code` field matching the process exit code (1–5)

**Example**:
- Given: `agentchrome form fill s99 "hello"` where s99 is not a snapshot UID
- When: the command runs and exits with code 1
- Then: stderr contains `{"error":"UID 's99' not found. Run 'agentchrome page snapshot' first.","code":1}`

### AC2: `form fill` on an incompatible element emits a descriptive error

**Given** an accessibility snapshot has been taken and UID `sN` refers to an element whose tag/role is not fillable (e.g., `<div>`, `<canvas>`, `<button>`, a non-editable `combobox`)
**When** the user runs `agentchrome form fill sN "value"`
**Then** the command exits with code 1 (general error)
**And** stderr contains a JSON error whose `error` field names the observed element type (tag or ARIA role)
**And** the message explains the element is not fillable and points to a compatible alternative (`interact click`, `js exec`, or a role-appropriate command)

**Example**:
- Given: `sN` is a `<div>` with no `contenteditable` attribute
- When: `agentchrome form fill sN "hello"`
- Then: `{"error":"Element 'sN' (tag=div) is not fillable. Use 'agentchrome interact click' or 'agentchrome js exec' for non-input elements.","code":1}`

### AC3: Syntax mistakes on common commands produce a suggestion

**Given** a user runs a command with a syntactically valid but semantically wrong argument shape — specifically `agentchrome interact click --uid s6` (flag syntax where `click` expects a positional UID/selector)
**When** the command fails
**Then** stderr contains a JSON error that either (a) is emitted directly by clap parsing (exit code 1, JSON format per AC1) or (b) is augmented with a "Did you mean: `agentchrome interact click s6`" suggestion in the `error` field
**And** the suggestion mentions the corrected invocation shape

### AC4: Silent failure audit is exhaustive for the flagged modules

**Given** the audit scope `src/form.rs`, `src/interact.rs`, and `src/page.rs` (wait/screenshot/snapshot paths)
**When** the audit is complete
**Then** every `Result`-returning public command entry point in those modules has been traced
**And** every leaf error path in those modules constructs an `AppError` (directly or via `?` on a typed error)
**And** no path in those modules returns `Err` via `anyhow::anyhow!`, bare-string `Err("...".into())`, or `Err(io::Error::...)` that would bypass `AppError::print_json_stderr`
**And** the audit findings (paths reviewed, paths fixed, paths confirmed OK) are recorded in `design.md` § Audit Findings

### AC5: Help documentation describes the error-output contract

**Given** a user runs `agentchrome --help` (long form) or `agentchrome help error-handling` (or equivalent after_long_help entry on the top-level command)
**When** the help text is rendered
**Then** the output describes the stderr JSON schema (`{error, code, [kind, recoverable, ...custom]}`)
**And** lists the exit-code meanings (0=success, 1=general, 2=connection, 3=target, 4=timeout, 5=protocol)
**And** notes that every non-zero exit emits exactly one JSON object on stderr

### AC6: Exactly one error object per invocation

**Given** any AgentChrome command that fails for any reason (clap parse error, runtime CDP failure, timeout, internal panic-to-error conversion)
**When** the command exits
**Then** stderr contains exactly one JSON error line (no duplicate, no partial fragment, no trailing prose)
**And** stdout is empty or valid JSON (never both a partial payload and an error)

*Guards against the regression pattern documented in #96 (double JSON stderr) and #98 (clap JSON stderr).*

### AC7: Structured context fields are preserved when present

**Given** an error path already populates `AppError.custom_json` (e.g., `chrome_terminated`, `js_execution_failed_with_json`, or a new `form_fill_not_fillable` variant with element-type context)
**When** the error is emitted
**Then** the `custom_json` payload is printed verbatim in place of the default `{error, code}` form
**And** the payload still contains at minimum the `error` (string) and `code` (integer) fields required by AC1

### Generated Gherkin Preview

```gherkin
Feature: Consistent error output on all failure paths
  As an AI agent debugging failed AgentChrome commands
  I want every failure to emit a structured JSON error on stderr
  So that I can programmatically react without guessing at silent failures

  Scenario: Non-zero exits always emit JSON on stderr
    Given an AgentChrome command is about to fail
    When it exits with a non-zero code
    Then stderr contains one JSON object with fields "error" (non-empty string) and "code" (integer 1..5)

  Scenario: form fill on a non-fillable element names the element type
    Given a snapshot UID refers to a <div> element
    When I run "agentchrome form fill <uid> value"
    Then stderr contains a JSON error whose "error" field names tag=div and suggests an alternative command

  # ... all remaining ACs become scenarios
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Audit `src/form.rs`, `src/interact.rs`, `src/page.rs` for silent-failure paths | Must | Record findings in design.md |
| FR2 | Every non-zero exit path routes through `AppError::print_json_stderr` | Must | Guards AC1, AC6 |
| FR3 | Add a `form_fill_not_fillable(target, tag, role)` `AppError` constructor with `custom_json` carrying `{element_type, suggested_alternatives}` | Must | Satisfies AC2, AC7 |
| FR4 | Detect the `--uid <val>` anti-pattern on positional commands and emit a suggestion in the error message | Should | Satisfies AC3 |
| FR5 | Add element-type context (tag, ARIA role) to form-fill errors via `custom_json` | Should | Supports AC2 agent-parseability |
| FR6 | Update `after_long_help` on the top-level `Cli` to describe the error schema and exit codes | Must | Satisfies AC5 |
| FR7 | Gherkin BDD scenarios in `tests/features/improve-error-output-consistency.feature` covering every AC | Must | Run under `cargo test --test bdd` |
| FR8 | No code path emits more than one JSON error per invocation | Must | Regression guard per AC6 |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Error formatting overhead < 1ms; no impact on success-path latency |
| **Security** | Error messages must not leak filesystem paths or CDP internals beyond what existing `AppError` constructors already surface |
| **Platforms** | macOS, Linux, Windows (all tier-1 platforms per `tech.md`) |
| **Compatibility** | Existing JSON error shape (`{error, code}`) MUST be preserved — added fields may appear only inside `custom_json` and only on paths that opt into it |

---

## UI/UX Requirements

CLI-only; see `steering/product.md` principle "Structured output". No UI components.

| Element | Requirement |
|---------|-------------|
| **Error States** | One JSON object per failure on stderr; plain-text error prose is prohibited |
| **Suggestion tone** | Short, imperative ("Use X"), pointing to a concrete command |

---

## Data Requirements

### Output Data — Error Schema

| Field | Type | Description | Required |
|-------|------|-------------|----------|
| `error` | string | Human-readable message | Yes |
| `code` | integer (1–5) | Matches process exit code | Yes |
| `kind` | string | Sub-category (e.g., `"chrome_terminated"`, `"not_fillable"`) | No — only when `custom_json` is set |
| `recoverable` | boolean | Hint for auto-retry | No |
| `element_type` | object `{tag, role}` | Form-fill errors only | No |
| `suggested_alternatives` | array\<string\> | Form-fill errors only | No |

---

## Dependencies

### Internal Dependencies
- [x] `src/error.rs` — existing `AppError` / `ExitCode` / `custom_json` infrastructure
- [x] `src/main.rs` — existing top-level `AppError::print_json_stderr` dispatch

### Blocked By
- None

---

## Out of Scope

- Changing the existing exit-code values or adding new ones (still 0–5)
- Warning-level (non-fatal) stderr output
- Verbose/debug logging modes
- Retry logic for transient failures
- Migration of per-subsystem error types (`cdp::error`, `chrome::error`) to a new shape — they already convert into `AppError` at module boundaries
- Changing stdout JSON shape for success paths

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Commands exiting non-zero with empty stderr | 0 | `tests/bdd` sweep over all command surfaces |
| `form fill` silent-failure reports | 0 | Post-release issue search for "form fill" + "no error output" |
| Audit-coverage ratio | 100% of error paths in `form.rs` + `interact.rs` + `page.rs` | Documented in `design.md` § Audit Findings |

---

## Open Questions

- [ ] Should we add a new `ExitCode::UnsupportedElement` (reserving code 6), or continue using `GeneralError (1)` with richer `custom_json`? **Proposed answer**: stay on `GeneralError` — out-of-scope says no new exit codes.
- [ ] Should the syntax-suggestion logic (`--uid` → positional) live in clap's error handler in `main.rs` or in a post-parse hook per subcommand? **Proposed answer**: centralise in `main.rs` clap-error branch (same place that already reformats clap messages).

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #197 | 2026-04-21 | Initial feature spec |

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements (only output contract)
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases (AC6 exactly-once, AC7 custom_json preservation) specified
- [x] Dependencies identified
- [x] Out of scope defined
- [x] Open questions documented with proposed answers
