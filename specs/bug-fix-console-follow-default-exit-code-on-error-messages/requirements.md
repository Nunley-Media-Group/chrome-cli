# Defect Report: console follow default exit code on error messages

**Issue**: #228
**Date**: 2026-04-22
**Status**: Draft
**Author**: Rich Nunley
**Severity**: Medium
**Related Spec**: `specs/feature-console-message-reading-with-filtering/`

---

## Reproduction

### Steps to Reproduce

1. `agentchrome connect --launch --headless --port <P>`
2. `agentchrome --port <P> navigate https://example.com`
3. In one process: `agentchrome --port <P> console follow --timeout 3000`
4. In another process: `agentchrome --port <P> js exec "console.log('hello'); console.warn('warn-msg'); console.error('err-msg')"`
5. All three messages stream correctly on the follower's stdout.
6. The follower exits once the 3000 ms timeout elapses.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | Windows 11 |
| **Version / Commit** | agentchrome 1.33.1 |
| **Browser / Runtime** | Chrome via `connect --launch --headless` |
| **Configuration** | default |

### Frequency

Always (deterministic — any error-level message observed before timeout triggers the non-zero exit).

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `console follow --timeout 3000` exits with code 0 when the timeout elapses, even if `console.error` messages were seen. Exit code 1 on error-level messages should be opt-in via `--fail-on-error`. |
| **Actual** | `console follow --timeout 3000` exits with code 1 whenever any error-level message is observed during the window, emitting `{"error":"Error-level console messages were seen","code":1}` on stderr. |

### Error Output

```
{"error":"Error-level console messages were seen","code":1}
```

Emitted by `src/console.rs:528-533` whenever `saw_errors` is true at timeout.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Default exit 0 after timeout despite console.error

**Given** `agentchrome console follow --timeout <ms>` is running without `--fail-on-error`
**And** the target page emits at least one `console.error` before the timeout elapses
**When** the `<ms>` timeout elapses
**Then** `console follow` exits with code 0
**And** no `Error-level console messages were seen` JSON error is written to stderr

### AC2: `--fail-on-error` opt-in preserves assertion behavior

**Given** `agentchrome console follow --timeout <ms> --fail-on-error` is running
**And** the target page emits at least one `console.error` before the timeout elapses
**When** the `<ms>` timeout elapses
**Then** `console follow` exits with code 1
**And** stderr contains the JSON error `{"error":"Error-level console messages were seen","code":1}` (existing contract preserved)

### AC3: Help documents both modes

**Given** agentchrome is built
**When** I run `agentchrome console follow --help`
**Then** the exit code is 0
**And** stdout contains `--fail-on-error`
**And** stdout describes the default monitoring behavior (exit 0) and the `--fail-on-error` assertion mode (exit 1) with at least one worked example for each

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Default `console follow --timeout <ms>` exits 0 when the timeout elapses, regardless of log levels observed | Must |
| FR2 | Add `--fail-on-error` flag on `console follow` that restores the current "exit 1 when any error-level message is seen" behavior | Must |
| FR3 | Update `console follow` long-form help (`--help`), the `examples` subcommand entry, and any capabilities/docs references to document both modes | Must |

---

## Out of Scope

- Changing the streaming JSON output format for individual messages.
- Adding log-level filtering semantics to `--fail-on-error` (e.g., "fail on warn") — only error-level (error/assert) triggers the opt-in failure, matching the current contract.
- Changing behavior of `console read` / one-shot console reads.
- Changing exit behavior when Ctrl+C interrupts (remains exit 0 regardless of mode).

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC2 preserves the prior contract)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #228 | 2026-04-22 | Initial defect report |
