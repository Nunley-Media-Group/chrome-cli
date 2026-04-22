# Defect Report: Fix js exec --plain zero-byte output for empty strings

**Issue**: #229
**Date**: 2026-04-22
**Status**: Draft
**Author**: Rich Nunley
**Severity**: Medium
**Related Spec**: `specs/feature-javascript-execution/`

---

## Reproduction

### Steps to Reproduce

1. Launch headless Chrome: `agentchrome connect --launch --headless`.
2. Navigate to any page where `document.getElementById('result').innerText` evaluates to `""` (e.g., `https://the-internet.herokuapp.com/javascript_alerts` before any prompt is accepted).
3. Run: `agentchrome js exec "document.getElementById('result').innerText" --plain`.
4. Observe stdout: zero bytes are emitted, exit code is `0`.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | Windows 11 |
| **Version / Commit** | agentchrome 1.33.1 (reproducible on current `main`) |
| **Browser / Runtime** | Chrome (headless, `connect --launch --headless`) |
| **Configuration** | bash shell, default output settings |

### Frequency

Always — deterministic whenever the evaluated expression returns an empty JavaScript string.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `--plain` mode emits a non-empty, deterministic stdout payload so callers can distinguish an empty-string result from "no output". Preferred: JSON-quoted `""` (two bytes). |
| **Actual** | stdout is zero bytes. Exit code is `0`. Callers cannot tell whether the command produced an empty string or silently produced nothing. |

### Error Output

```
(none — command exits 0 with zero bytes on stdout)
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Bug Is Fixed — empty-string result

**Given** a page where expression `E` evaluates to `""`
**When** `agentchrome js exec "E" --plain` runs
**Then** stdout is non-empty
**And** the output allows the caller to distinguish "empty-string result" from "no output" (the emitted payload is the two-byte JSON literal `""`)

### AC2: No Regression — non-empty strings

**Given** a page where expression `E` evaluates to `"hello"`
**When** `agentchrome js exec "E" --plain` runs
**Then** stdout matches the current behavior exactly (the raw string `hello`, with no added quoting or trailing newline)

### AC3: No Regression — numbers, booleans, null, undefined

**Given** expressions that evaluate to `42`, `true`, or `null`, and a statement that evaluates to `undefined`
**When** `agentchrome js exec "E" --plain` runs for each
**Then** stdout matches the current behavior for each type (`42`, `true`, `null`, `undefined` respectively)

### AC4: No Regression — `--pretty` and default JSON modes

**Given** an expression that evaluates to `""`
**When** `agentchrome js exec "E" --pretty` or `agentchrome js exec "E"` runs
**Then** the JSON output still contains `"result": ""` and `"type": "string"` unchanged

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `--plain` mode for an empty JavaScript string result emits a deterministic non-empty stdout payload (the two-byte JSON literal `""`). | Must |
| FR2 | All other `--plain` result types (non-empty strings, numbers, booleans, null, undefined, objects/arrays) continue to emit exactly what they emit today. | Must |
| FR3 | `--pretty` and default JSON output paths remain unchanged for every result type. | Must |

---

## Out of Scope

- Changing `--pretty` or default JSON output formatting.
- Adding a trailing newline to non-empty-string `--plain` output.
- Adding a `--null-is-empty` or similar flag.
- Refactoring the `--plain` formatting layer beyond the empty-string special case.

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #229 | 2026-04-22 | Initial defect report |
