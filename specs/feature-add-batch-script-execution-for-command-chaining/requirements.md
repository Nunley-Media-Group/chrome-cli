# Requirements: Batch Script Execution

**Issues**: #199
**Date**: 2026-04-21
**Status**: Draft
**Author**: Rich Nunley

---

## User Story

**As a** browser automation engineer (developer or AI agent) running repetitive multi-step workflows
**I want** to define a sequence of agentchrome commands in a JSON script file and execute them as a single operation
**So that** I can dramatically reduce round-trips and context-window usage for repetitive patterns (e.g. advancing through a 92-slide SCORM course)

---

## Background

AI agents and shell users driving long automations (e.g. SCORM courses) currently must emit one CLI invocation per step. A 92-slide course requiring 5–8 commands per slide produces ~500 tool calls, blowing up latency and token usage. Every agentchrome command already emits structured JSON — chaining those commands with conditional branching, loops, and variable binding turns the long sequence into a concise loop driven by a single `script run` invocation.

The feature introduces a new `script` command group with a `run <file>` subcommand that reads a JSON script, executes its commands sequentially against an existing CDP session, and emits a structured JSON result array on stdout.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Execute a script file (happy path)

**Given** a JSON script file `slides.json` containing a `commands` array of three valid agentchrome invocations (`navigate`, `page snapshot`, `js exec`)
**And** an active CDP session to a headless Chrome instance
**When** the user runs `agentchrome script run slides.json`
**Then** the process exits with code `0`
**And** stdout contains a single JSON object `{ "results": [ ... ], "executed": 3, "skipped": 0, "failed": 0 }`
**And** `results[i]` contains the structured output of the i-th command along with `index`, `command`, `status: "ok"`, and `duration_ms`

**Example**:
- Script: `{ "commands": [ { "cmd": ["navigate", "https://example.com"] }, { "cmd": ["page", "snapshot"] }, { "cmd": ["js", "exec", "document.title"] } ] }`
- Expected `results[2].output.result == "Example Domain"`

### AC2: Script execution from stdin

**Given** a JSON script piped on stdin
**When** the user runs `agentchrome script run -`
**Then** the script is read from stdin and executed identically to a file-based script
**And** the exit code and stdout JSON shape match AC1

### AC3: Conditional branching (if / else)

**Given** a script containing a step of shape `{ "if": "<JS expression>", "then": [<commands>], "else": [<commands>] }`
**When** the condition evaluates to truthy against `$prev` (the previous step's output) and `$vars` (bound variables)
**Then** only the `then` branch executes and its results appear in order in `results`
**And** when the condition is falsy, only the `else` branch executes
**And** steps in the non-selected branch appear in `results` with `status: "skipped"` and no `output`

### AC4: Fail-fast mode

**Given** the `--fail-fast` flag is passed and step 2 of a 5-step script fails (non-zero exit from its underlying command module)
**When** `agentchrome script run --fail-fast broken.json` is executed
**Then** execution stops immediately after step 2
**And** the process exits with code `1`
**And** stderr contains exactly one JSON error object `{ "error": "script step 2 failed: <message>", "code": 1, "failing_index": 2, "failing_command": "<cmd string>" }`
**And** stdout contains the partial `results` array (steps 1 and 2 only; step 2 has `status: "error"`; subsequent steps absent)

### AC5: Continue-on-error mode (default)

**Given** `--fail-fast` is NOT set and step 2 of a 5-step script fails
**When** the script runs
**Then** execution continues through all 5 steps
**And** stdout's `results` array contains all 5 entries with the failing step carrying `status: "error"` and an `error` field
**And** the process exits with code `0` if any downstream step succeeded
**And** `executed`, `skipped`, and `failed` counters reflect the final counts

### AC6: Count-based loop

**Given** a script step of shape `{ "loop": { "count": 3 }, "body": [<commands>] }`
**When** the script runs
**Then** the body executes exactly 3 times in order
**And** each iteration exposes the current index as `$i` (0-based) inside expression contexts
**And** loop iterations are flattened into `results` with an additional `loop_index` field per entry

### AC7: Condition-based loop with max-iterations guard

**Given** a script step `{ "loop": { "while": "<JS expression>", "max": 100 }, "body": [<commands>] }`
**When** the script runs
**Then** the body repeats while the expression evaluates to truthy
**And** the loop aborts once `max` iterations have been reached regardless of the condition
**And** reaching `max` emits one JSON warning on stderr: `{ "warning": "loop max iterations reached", "max": 100 }`

### AC8: Variable binding

**Given** a step `{ "cmd": ["js", "exec", "document.title"], "bind": "title" }` followed by a later step that references `$vars.title` in its argument list
**When** the script runs
**Then** the first step's structured output is stored in `$vars.title`
**And** the later step receives the bound value substituted into its arguments
**And** variable references to undefined names produce a step-level error (or halt under `--fail-fast`)

### AC9: Dry-run validation

**Given** a script file and the `--dry-run` flag
**When** `agentchrome script run --dry-run plan.json` is executed
**Then** no commands are dispatched to Chrome
**And** the process exits with code `0` when the script parses, schema-validates, and references only known subcommands and declared variables
**And** the process exits with code `1` with a structured JSON error on stderr when parsing, schema validation, or reference resolution fails

### AC10: `--help` metadata (short form)

**Given** the new `script` surface
**When** the user runs `agentchrome script --help`
**Then** the short description mentions "script execution" / "command chaining"
**And** `script run --help` lists `--fail-fast`, `--dry-run`, and the `<file>` positional (with `-` documented as stdin)

### AC11: `--help` long form includes worked examples

**Given** the new `script run` subcommand
**When** the user runs `agentchrome script run --help` (long form)
**Then** the output includes at least one worked EXAMPLE block
**And** at least one example demonstrates reading a script from stdin
**And** at least one example shows `--fail-fast` use

### AC12: Capabilities manifest reflects the new surface

**Given** the `script` subcommand is registered via clap
**When** `agentchrome capabilities --json` is run
**Then** its output includes a `script` entry with `run` as a subcommand
**And** the entry documents the `--fail-fast` and `--dry-run` flags

### AC13: `examples script` built-in documentation

**Given** the built-in examples subcommand
**When** the user runs `agentchrome examples script`
**Then** at least one example script is printed
**And** at least one example covers conditional branching
**And** at least one example covers loops

### AC14: Command is blocked by inactive session

**Given** no active CDP session exists
**When** `agentchrome script run good.json` is executed
**Then** the process exits with code `2` (connection error)
**And** stderr contains one JSON error matching the global contract
**And** no partial `results` are emitted on stdout

### AC15: Name collision check (no clap-reserved identifiers)

**Given** the new `script` command group and its flags
**When** clap parses `agentchrome script run --help`
**Then** no flag or positional collides with any global flag (`--json`, `--pretty`, `--port`, `--host`, `--timeout`, `--config`, etc.)
**And** the `cmd` key inside a script step is internal JSON, not a CLI flag

### AC16: Stateful sub-commands work across script steps

**Given** a script whose first step creates a new tab and whose second step issues `page snapshot` against the active tab
**When** the script runs
**Then** the second step observes the tab created by the first step (state propagation across steps is visible)
**And** this holds regardless of whether the session is headed or headless (per the runtime-variant retrospective learning)

### Generated Gherkin Preview

```gherkin
Feature: Batch Script Execution
  As a browser automation engineer running repetitive workflows
  I want to run a sequence of agentchrome commands from a script
  So that I can cut round-trips and context consumption

  Scenario: Execute a script file
    Given a script file with three valid commands
    And an active CDP session
    When I run "agentchrome script run slides.json"
    Then the exit code is 0
    And stdout contains a results array with three ok entries

  # ... all ACs become scenarios
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | New `script run <file>` subcommand under a `script` command group | Must | `<file>` accepts `-` for stdin |
| FR2 | JSON script schema (v1): `{ commands: Step[] }` where `Step` is one of `{ cmd }`, `{ if, then, else }`, `{ loop, body }` | Must | JSON only in v1; YAML deferred |
| FR3 | Sequential command execution with result collection emitted as `{ results, executed, skipped, failed }` | Must | Stable JSON shape for agents |
| FR4 | `--fail-fast` flag — stop on first error, emit structured error with `failing_index` and `failing_command` | Must | Default is continue-on-error |
| FR5 | Conditional branching via `if`/`then`/`else` evaluated against `$prev` and `$vars` | Should | JS expression evaluated via Chrome `Runtime.evaluate` in a sandboxed context |
| FR6 | Count-based loops (`loop.count`) and condition-based loops (`loop.while`) with mandatory `max` guard for `while` | Should | `max` prevents runaway loops |
| FR7 | Variable binding (`bind: "<name>"` on any `cmd` step) + reference substitution `$vars.<name>` in later steps' arguments | Should | Substitution is string-level for positional args; whole-value for JSON args |
| FR8 | `--dry-run` flag — validate schema, subcommand names, and variable references without dispatching to Chrome | Could | Returns the same counters with `dispatched: false` |
| FR9 | `script --help` / `script run --help` surface help text with at least one `--json` worked example and stdin example | Must | Enforced by steering `tech.md` clap-help rule |
| FR10 | `examples script` entry with at least three script samples (sequential, conditional, loop) | Must | Per steering `tech.md` examples rule |
| FR11 | Capabilities manifest includes `script` entry | Must | clap-driven; free if FR9 is satisfied |
| FR12 | BDD scenarios covering AC1–AC16 | Must | cucumber-rs, `tests/features/batch-script-execution.feature` |
| FR13 | Per-step `duration_ms` and overall `total_ms` in the result JSON | Should | Useful for profiling agent scripts |
| FR14 | Script steps execute against the active session (connect/disconnect not embedded in the script language in v1) | Must | Avoids complex state machines; matches existing stateless-CLI model |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Per-step overhead (script-runner bookkeeping, excluding the underlying command) < 5 ms. Startup budget still < 50 ms total per project target. |
| **Security** | Conditional/loop JS expressions execute in Chrome's existing `Runtime.evaluate` — no new JS host. No arbitrary filesystem or shell access from scripts. |
| **Reliability** | A malformed step must never leak a partial action without a corresponding entry in `results`. Stdout JSON is always well-formed, even on partial failure. |
| **Platforms** | macOS, Linux, Windows (per steering). |
| **Output Contract** | JSON stdout / JSON stderr / exit codes per `tech.md`. Exactly one error object on stderr per invocation. |

---

## UI/UX Requirements

N/A — CLI-only surface.

---

## Data Requirements

### Input: Script v1 schema

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `commands` | `Step[]` | Non-empty array | Yes |
| `Step.cmd` | `string[]` | Argv-style; first element is a known agentchrome subcommand | One of `cmd` / `if` / `loop` required |
| `Step.bind` | `string` | Identifier-safe (`[a-zA-Z_][a-zA-Z0-9_]*`) | No (only on `cmd` steps) |
| `Step.if` | `string` | JS expression | One of `cmd` / `if` / `loop` required |
| `Step.then` | `Step[]` | — | Required when `if` present |
| `Step.else` | `Step[]` | — | No (defaults to `[]`) |
| `Step.loop.count` | `int` | `>= 0` | One of `count` / `while` required under `loop` |
| `Step.loop.while` | `string` | JS expression | — |
| `Step.loop.max` | `int` | `>= 1`; required when `while` is used | Yes (when `while` present) |
| `Step.body` | `Step[]` | Non-empty | Required when `loop` present |

### Output: Result JSON

| Field | Type | Description |
|-------|------|-------------|
| `results` | `Result[]` | Ordered entries per executed or skipped step |
| `results[].index` | `int` | Zero-based position in the flattened execution trace |
| `results[].command` | `string[]` or `null` | Echoed `cmd` argv (null for synthetic entries like loop summaries) |
| `results[].status` | `"ok"` / `"error"` / `"skipped"` | Step outcome |
| `results[].output` | `any` | Structured output from the underlying command (absent on `skipped`) |
| `results[].error` | `object` | Present only when `status == "error"`; matches global error shape |
| `results[].duration_ms` | `int` | Per-step wall-clock duration |
| `results[].loop_index` | `int` | Present only for iterations produced by a `loop` step |
| `executed` | `int` | Count of `ok` results |
| `skipped` | `int` | Count of `skipped` results |
| `failed` | `int` | Count of `error` results |
| `total_ms` | `int` | Overall wall-clock duration |

---

## Dependencies

### Internal Dependencies
- [x] Connection management (`connection.rs`) — script steps share the existing CDP session.
- [x] Each command module (`navigate.rs`, `page/`, `js.rs`, `form.rs`, etc.) — script dispatcher must invoke them as library functions (not re-shelling `agentchrome`).
- [x] Capabilities manifest (`capabilities.rs`) and examples module (`examples/`) — both must reflect the new surface.

### External Dependencies
- [x] Chrome `Runtime.evaluate` for `if` / `while` expression evaluation (already used by `js.rs`).

### Blocked By
- None.

---

## Out of Scope

- YAML / TOML / any non-JSON script format (JSON only in v1).
- Visual script editor or GUI.
- Parallel command execution within a script.
- External API calls or non-agentchrome commands.
- Script sharing, marketplace, or remote script URLs.
- Managing connect/disconnect from inside scripts (session is established externally).
- Arbitrary Rust evaluation or arbitrary Node scripting — `if` / `while` expressions are constrained to Chrome `Runtime.evaluate`.

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Tool-call reduction for a 92-slide SCORM loop | ≥ 50× | Count tool invocations driving the loop today vs. after (1 script) |
| Per-step overhead (runner only) | < 5 ms | `results[].duration_ms` minus the underlying command's internal timing |
| Script scenarios covered by BDD | 100 % of ACs | Cucumber scenario count |

---

## Open Questions

- [ ] Should `bind` permit nested path extraction (e.g. `bind.path: "result.title"`) in v1, or is whole-output binding sufficient?
- [ ] Do we guarantee `loop_index` is monotonically increasing across nested loops, or scope it to the innermost loop?
- [ ] For `if` / `while` expressions that throw in Chrome, do we treat the step as `error` (default) or coerce to falsy? Spec assumes `error`.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #199 | 2026-04-21 | Initial feature spec |

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified (AC4, AC5, AC7 max, AC14 no-session, AC15 flag collision)
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented
