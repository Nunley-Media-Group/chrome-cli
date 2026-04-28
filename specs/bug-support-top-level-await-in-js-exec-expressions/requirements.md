# Defect Report: Support top-level await in js exec expressions

**Issue**: #279
**Date**: 2026-04-27
**Status**: Draft
**Author**: Codex (write-spec)
**Severity**: Medium
**Related Spec**: `specs/feature-javascript-execution/`

---

## Reproduction

### Steps to Reproduce

1. Build the debug binary with `cargo build`.
2. Launch headless Chrome with `./target/debug/agentchrome connect --launch --headless`.
3. Navigate to any page, for example `./target/debug/agentchrome navigate https://qaplayground.vercel.app/ --wait-until load`.
4. Run `./target/debug/agentchrome js exec 'await Promise.resolve("done")' --pretty`.
5. Compare with `./target/debug/agentchrome js exec 'new Promise(r => setTimeout(() => r("done"), 100))' --pretty`, which succeeds today.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 arm64 |
| **Version / Commit** | agentchrome 1.56.1, commit 756db61 |
| **Browser / Runtime** | Headless Chrome via CDP |
| **Configuration** | Default `js exec` behavior with promise awaiting enabled |

### Frequency

Always - deterministic whenever the expression path receives direct top-level `await` syntax.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `agentchrome js exec 'await Promise.resolve("done")'` exits 0 and writes JSON equivalent to `{"result":"done","type":"string"}`. |
| **Actual** | The command exits 1 before a promise result can be produced because the wrapped script body rejects direct `await` syntax. |

### Error Output

```json
{"error":"SyntaxError: await is only valid in async functions and the top level bodies of modules","stack":"SyntaxError: await is only valid in async functions and the top level bodies of modules","code":1}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Bug Is Fixed - Top-Level Await Succeeds

**Given** Chrome is connected and a page is loaded
**When** I run `agentchrome js exec 'await Promise.resolve("done")'`
**Then** the command exits 0
**And** stdout contains JSON with `result` equal to `"done"` and `type` equal to `"string"`

### AC2: No Regression - Promise Return Awaiting Still Works

**Given** Chrome is connected and a page is loaded
**When** I run `agentchrome js exec 'new Promise(r => setTimeout(() => r("done"), 100))'`
**Then** the command exits 0
**And** stdout contains JSON with `result` equal to `"done"` and `type` equal to `"string"`

### AC3: No Regression - Existing Scope Isolation Still Works

**Given** Chrome is connected and a page is loaded
**When** I run two consecutive `agentchrome js exec` commands that declare the same `let` or `const` variable name
**Then** both commands exit 0
**And** the second command does not fail due to redeclaration from the first command

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `js exec` expression evaluation must make direct top-level `await` syntax legal in the evaluated page context while preserving the existing default behavior of awaiting returned Promises. | Must |
| FR2 | The fix must preserve current `let` and `const` isolation behavior across consecutive expression invocations. | Must |
| FR3 | The fix must apply through the shared expression evaluation helper so primary page execution, frame execution, worker execution, and script-runner execution stay consistent unless a path is explicitly out of scope. | Must |
| FR4 | JavaScript execution failures must continue to emit the existing structured JSON error shape and typed exit-code behavior. | Must |

---

## Out of Scope

- Changing `--no-await` semantics for promise-returning expressions.
- Changing `--uid` function execution semantics unless required for consistency.
- Adding a separate JavaScript module execution mode.
- Refactoring JavaScript input resolution, console capture, truncation, or output formatting.

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included
- [x] Fix scope is minimal - no feature work mixed in
- [x] Out of scope is defined

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #279 | 2026-04-27 | Initial defect report |
