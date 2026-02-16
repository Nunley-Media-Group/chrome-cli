# Defect Report: JS execution errors emit two JSON objects on stderr

**Issue**: #96
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude
**Severity**: Medium
**Related Spec**: `.claude/specs/13-javascript-execution/`

---

## Reproduction

### Steps to Reproduce

1. Connect to Chrome: `chrome-cli connect --launch --headless`
2. Execute failing JS: `chrome-cli js exec "throw new Error('test')" 2>/tmp/stderr.txt`
3. Inspect stderr: `cat /tmp/stderr.txt`
4. Observe two JSON objects on stderr instead of one

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (Darwin 25.3.0) |
| **Version / Commit** | `c584d2d` (main) |

### Frequency

Always — any JavaScript runtime error triggers this.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | Exactly one JSON error object on stderr containing `error`, `stack`, and `code` fields |
| **Actual** | Two JSON error objects on stderr: (1) `JsExecError` with `error`, `stack`, `code` from `js.rs:326`, then (2) `ErrorOutput` with `error`, `code` from `main.rs:40` via `AppError::print_json_stderr()` |

### Error Output

```
{"error":"Error: test\n    at <anonymous>:1:7","stack":"Error: test\n    at <anonymous>:1:7","code":1}
{"error":"JavaScript execution failed: Error: test\n    at <anonymous>:1:7","code":1}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Single JSON error on throw

**Given** Chrome is connected and a page is loaded
**When** I execute `chrome-cli js exec "throw new Error('test')"`
**Then** exactly one JSON object is written to stderr
**And** it contains the `error`, `stack`, and `code` fields
**And** the exit code is non-zero

### AC2: Single JSON error on ReferenceError

**Given** Chrome is connected and a page is loaded
**When** I execute `chrome-cli js exec "nonExistentVar"`
**Then** exactly one JSON object is written to stderr
**And** it contains the `error` and `code` fields

### AC3: Stdout remains empty on error

**Given** Chrome is connected and a page is loaded
**When** I execute JS that throws an error
**Then** stdout is empty
**And** only stderr contains the error JSON

### AC4: Successful execution unchanged

**Given** Chrome is connected and a page is loaded
**When** I execute `chrome-cli js exec "document.title"`
**Then** stdout contains the result JSON
**And** stderr is empty
**And** the exit code is 0

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | JS execution errors must emit exactly one JSON error object on stderr | Must |
| FR2 | The single error JSON must include JS error details (`error`, `stack`, `code`) — not the generic wrapper message | Must |
| FR3 | Successful JS execution must remain unaffected by this fix | Must |

---

## Out of Scope

- Changing the error JSON schema (fields, structure)
- Changing error handling for non-JS commands
- Refactoring the global `AppError::print_json_stderr()` mechanism

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC4)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
