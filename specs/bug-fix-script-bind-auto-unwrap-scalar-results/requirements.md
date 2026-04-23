# Defect Report: Script bind stores raw command output envelope for `js exec`

**Issues**: #248
**Date**: 2026-04-23
**Status**: Draft
**Author**: Rich Nunley
**Severity**: High
**Related Spec**: `specs/feature-add-batch-script-execution-for-command-chaining/`

---

## Reproduction

### Steps to Reproduce

1. Create a script with a `js exec` step that binds its result:
   ```json
   {
     "steps": [
       { "cmd": ["js", "exec", "document.title"], "bind": "t" },
       { "if": "$vars.t.includes('Internet')", "then": [{ "cmd": ["page", "snapshot"] }] }
     ]
   }
   ```
2. Run `agentchrome script run <file>` against any page whose title contains `"Internet"`.
3. Observe the `if` expression fails with a `TypeError`.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS 15.x (any) / Linux |
| **Version / Commit** | 1.45.0 (branch `248-script-bind-auto-unwrap-scalar-results`) |
| **Browser / Runtime** | Chrome/Chromium via CDP |
| **Configuration** | N/A — default script runner |

### Frequency

Always — deterministic.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `$vars.t` holds the scalar string `"The Internet"` so `$vars.t.includes('Internet')` returns `true`. |
| **Actual** | `$vars.t` holds the full envelope `{ "result": "The Internet", "truncated": false, "type": "string" }`, so `$vars.t.includes(...)` raises `TypeError: $vars.t.includes is not a function`. |

### Error Output

```
TypeError: $vars.t.includes is not a function
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Bug Is Fixed — scalar `js exec` result is auto-unwrapped

**Given** a script step `{ "cmd": ["js", "exec", "document.title"], "bind": "t" }` run against a page titled `"The Internet"`
**When** a subsequent `if` step evaluates `$vars.t.includes('Internet')`
**Then** the expression evaluates without a `TypeError` and the branch is taken (truthy).

### AC2: Non-scalar `js exec` results expose the underlying value directly

**Given** a `js exec` step that returns an object such as `{"a": 1, "b": 2}` and binds it as `obj`
**When** a later step references `$vars.obj.a`
**Then** `$vars.obj.a` resolves to `1` (i.e. `$vars.obj` holds the returned object, not the `{result,truncated,type}` envelope).

### AC3: No regression for other command binds

**Given** existing bind behaviour for `page find` (returns an array of matches directly)
**When** a script chains `{ "cmd": ["page", "find", ...], "bind": "match" }` and then references `$vars.match[0].uid` (as in the existing batch-script BDD fixture)
**Then** the reference continues to resolve to the first match's `uid` with no change in shape.

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | For `js exec` binds, store the `result` field of the output envelope in `$vars` instead of the full `{result, truncated, type}` envelope. | Must |
| FR2 | Leave bind shape for non-`js-exec` commands (`navigate`, `page find`, `page text`, `page screenshot`, …) unchanged — only `js exec` is modified by this fix. | Must |
| FR3 | The stdout JSON shape of `js exec` (outside the script context) is unchanged — the envelope remains the wire format for standalone invocations. | Must |

---

## Out of Scope

- Changing the stdout JSON shape of `agentchrome js exec` when invoked directly (not via `script run`).
- A general “auto-unwrap any envelope with a single scalar `result` field” rule across all scriptable commands (issue FR3 — deferred).
- Documenting bind shapes for every scriptable command in man/examples (issue FR2 — deferred to a follow-up doc issue).
- Refactoring the script runner’s dispatch or context types beyond the minimal change needed to unwrap `js exec`.

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC3)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
