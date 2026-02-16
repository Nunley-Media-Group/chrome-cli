# Root Cause Analysis: JS execution errors emit two JSON objects on stderr

**Issue**: #96
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude

---

## Root Cause

When JavaScript execution fails (e.g., `throw new Error()` or `ReferenceError`), the error path in `src/js.rs` explicitly serializes a `JsExecError` struct to JSON and prints it to stderr via `eprintln!()` at line 326. It then returns `Err(AppError::js_execution_failed(&error_desc))` to the caller.

The global error handler in `src/main.rs` (lines 39-43) catches **all** `AppError` values and calls `e.print_json_stderr()`, which serializes the `AppError` into an `ErrorOutput` JSON object and prints it to stderr as well. This handler is correct for most commands — it is the standard way errors reach the user. However, the JS execution path has already printed its own richer error JSON (with `stack` trace) before returning the `AppError`, so the global handler produces a duplicate.

The result is two JSON objects on stderr for a single error: the first from `js.rs` (with `error`, `stack`, `code`) and the second from `main.rs` via `AppError::print_json_stderr()` (with `error`, `code`, and a "JavaScript execution failed: ..." wrapper message).

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/js.rs` | 319-327 | Constructs `JsExecError`, serializes to JSON, prints to stderr, then returns `Err(AppError)` |
| `src/error.rs` | 216-221 | `AppError::js_execution_failed()` constructor — creates the wrapper error message |
| `src/error.rs` | 397-399 | `AppError::print_json_stderr()` — serializes and prints the second JSON |
| `src/main.rs` | 39-43 | Global error handler — calls `print_json_stderr()` on every `Err(AppError)` |

### Triggering Conditions

- JavaScript execution via `js exec` encounters a runtime error (exception, reference error, syntax error, etc.)
- The CDP response contains `exceptionDetails`
- The code at `js.rs:326` prints the first JSON, then the code at `main.rs:40` prints the second

---

## Fix Strategy

### Approach

Remove the manual `eprintln!()` of the `JsExecError` JSON in `src/js.rs` (line 326). Instead, propagate the rich error information (including the stack trace) through the `AppError` so that the **single** global error handler in `main.rs` produces the one JSON output.

The cleanest minimal fix: instead of constructing a separate `JsExecError` struct and printing it, remove the `eprintln!` call and embed the JS error details (error description + stack trace) directly into the `AppError` that is returned. The `AppError::print_json_stderr()` call in `main.rs` will then be the sole source of stderr output.

To preserve the `stack` field in the JSON output (which `ErrorOutput` currently lacks), the `JsExecError` serialization should be moved into the `AppError` path. Concretely: instead of `eprintln!` + `return Err(...)`, just `return Err(...)` where the error carries enough information for `print_json_stderr()` to produce the combined output — or the JS error path overrides how its error is serialized.

Two sub-approaches:
1. **Option A**: Add a `js_error_json` field to `AppError` (or a variant) that, when present, is used by `print_json_stderr()` instead of the default `ErrorOutput` serialization. This preserves the exact `JsExecError` schema.
2. **Option B**: Remove `eprintln!` in `js.rs` and include the stack trace in the `AppError` message field. The `ErrorOutput` JSON would then contain the full error with stack in the `error` field. This changes the output schema slightly (no separate `stack` field).

**Selected: Option A** — it preserves the existing `JsExecError` JSON schema (with separate `error`, `stack`, `code` fields) which downstream consumers may depend on.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/js.rs` | Remove `eprintln!("{err_json}")` at line 326. Instead, pass the pre-serialized `JsExecError` JSON string into the `AppError` so the global handler can emit it. | Eliminates the first duplicate `eprintln!` |
| `src/error.rs` | Add an optional field (e.g., `custom_json: Option<String>`) to `AppError`. In `print_json_stderr()`, if `custom_json` is `Some`, print it instead of the default `ErrorOutput`. Add a constructor or builder method for this. | Allows JS errors to carry their own richer JSON through the standard error path |
| `src/main.rs` | No changes needed | The existing `e.print_json_stderr()` call now handles both standard and custom JSON errors |

### Blast Radius

- **Direct impact**: `src/js.rs` (JS error path), `src/error.rs` (`AppError` struct and `print_json_stderr`)
- **Indirect impact**: Any command that returns `AppError` and relies on `print_json_stderr()` — but since the new `custom_json` field defaults to `None`, existing behavior is unchanged for all other commands
- **Risk level**: Low

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Successful JS execution breaks | Low | AC4 regression test verifies success path is unaffected |
| Other commands' error output changes | Low | `custom_json` defaults to `None`; only JS errors opt in |
| Exit code changes | Low | Fix does not alter exit code logic — `AppError.code` is unchanged |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| **B: Flatten stack into error message** | Remove `eprintln!` in `js.rs`, include stack trace in `AppError.message` | Changes the JSON schema — `stack` would no longer be a separate field, breaking consumers that parse it |
| **C: Skip `print_json_stderr` for JS errors** | Add a flag to `AppError` to suppress global printing | More fragile — easy to forget to set the flag for future error types that also pre-print |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
