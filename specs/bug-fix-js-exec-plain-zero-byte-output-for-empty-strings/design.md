# Root Cause Analysis: Fix js exec --plain zero-byte output for empty strings

**Issue**: #229
**Date**: 2026-04-22
**Status**: Draft
**Author**: Rich Nunley

---

## Root Cause

The `js exec --plain` output path in `src/js.rs` converts a CDP result `Value` to a raw `String` and passes it straight to `crate::output::emit_plain`. For a `Value::String(s)` result, the conversion is `s.clone()` — no quoting, no trailing newline — and `emit_plain` calls `print!("{text}")`. When `s == ""`, zero bytes reach stdout.

The bug sits in two mirrored code blocks (`execute_exec` at `src/js.rs:404-413` and `execute_in_worker` at `src/js.rs:569-577`). Both branch on `value` with the same `match`:

```rust
let text = match &value {
    serde_json::Value::String(s) => s.clone(),
    serde_json::Value::Null => "undefined".to_string(),
    other => serde_json::to_string(other).unwrap_or_default(),
};
crate::output::emit_plain(&text, &global.output)?;
```

The `String("")` arm collapses to an empty text, which `emit_plain` dutifully prints as zero bytes. All other arms are fine: numbers/booleans/objects go through `serde_json::to_string` (which yields `42`, `true`, `null`, `{"k":"v"}` — always non-empty), and `Value::Null` is mapped to the literal `"undefined"`.

`emit_plain` itself is not the right layer to fix this — it is a generic large-response gate used by several commands and should not embed JS-specific semantics. The fix belongs at the call site where the JS result type is known.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/js.rs` | 404-413 | `execute_exec` — `--plain` branch for the primary (non-worker) evaluation path |
| `src/js.rs` | 569-577 | `execute_in_worker` — `--plain` branch for the `--worker` evaluation path |

### Triggering Conditions

- `--plain` flag is set on the `js exec` invocation.
- The evaluated JavaScript expression returns a value whose CDP `type` is `"string"` and whose value is the empty string `""`.
- The feature as originally specified in `feature-javascript-execution` did not distinguish "empty-string result" from "no output" in the plain formatter — the JSON modes carry the type alongside the value, so the ambiguity is invisible there, which is why it went unnoticed until a user scripted against the plain path.

---

## Fix Strategy

### Approach

Add a single special-case at the two `--plain` emission sites: when the result `Value` is `String("")`, emit the two-byte JSON literal `""` instead of the raw empty string. This is the smallest change that satisfies AC1 (distinguishable empty-string output) without touching the generic `emit_plain` helper, the JSON/pretty paths, or the behavior of any other result type. The literal `""` was chosen over "empty string plus newline" because it is self-describing — a caller who reads the stdout as text sees the two-character token `""`, which is the conventional JS/JSON notation for an empty string and also happens to be what `serde_json::to_string(&Value::String(""))` produces, so the fix stays consistent with how non-string types are already serialized for plain output.

The fix is applied in both `execute_exec` and `execute_in_worker` because the two `--plain` branches are exact duplicates; patching only one would leave the `--worker` path still broken.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/js.rs` (≈ line 404-413) | In `execute_exec`, change the `Value::String(s) => s.clone()` arm to emit `"\"\"".to_string()` when `s.is_empty()`, otherwise `s.clone()`. | Guarantees non-empty stdout for empty-string results in the primary evaluation path without altering any other type's formatting. |
| `src/js.rs` (≈ line 569-577) | Apply the identical change in `execute_in_worker`. | Keeps the worker path consistent with the primary path; both paths share the defect. |

### Blast Radius

- **Direct impact**: the two `match` blocks inside `src/js.rs` that format `--plain` output for `js exec`. No other call sites consume those blocks.
- **Indirect impact**: callers that pipe `agentchrome js exec … --plain` stdout into shell scripts. Scripts that currently special-case "empty stdout" as "empty string" will now see `""` — this is the intended observable change and is the point of the fix.
- **Out of scope**: `emit_plain` in `src/output.rs` is not modified, so every other plain-mode command (`page text`, etc.) is untouched. JSON and `--pretty` modes in `js exec` are untouched. Non-empty strings, numbers, booleans, `null`, `undefined`, objects, and arrays in `--plain` mode are untouched.
- **Risk level**: Low. The change is a two-line conditional in two mirrored blocks, bounded by a type check and an emptiness check.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| A caller relied on "zero bytes ⇒ empty string" semantics and will see `""` instead. | Low | This is the bug being fixed; AC1 codifies the new contract. The two-byte payload is the minimal non-zero signal, and it is the JSON form the JSON modes already use — so consumers that parse output as JSON already handle it. |
| A non-empty string `"\"\""` (a two-character string containing only two quotes) becomes indistinguishable from the new empty-string sentinel in plain mode. | Low | Plain mode is inherently lossy for this edge case today — non-empty strings are emitted raw with no quoting. This fix does not make the ambiguity worse; callers that need unambiguous JS-value semantics should use `--pretty` or default JSON mode (AC4 preserves those paths). |
| The change leaks into the `--worker` path in unexpected ways. | Low | The `--worker` emission block in `execute_in_worker` is a literal copy of the primary block and receives the same patch. Regression test (AC1) will run against the primary path; a follow-up smoke test during verification can exercise the worker path manually if a worker expression returning `""` is easy to construct. |
| Block-scope wrapping of the expression (per `execute_expression_with_context`) changes what CDP returns as `type: "string"`. | Low | The wrapping is identical before and after the fix; the fix is downstream of CDP result extraction. Existing unit tests for `extract_result_value` and `js_type_string` continue to pass. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Always append a trailing newline in `--plain` mode | Use `println!` instead of `print!` (or add `\n`) inside `emit_plain`. | Changes behavior for every plain-mode consumer across the CLI, not just `js exec`. Violates the "minimal fix" constraint and would cascade into unrelated command output. |
| JSON-stringify every string in `--plain` mode | Emit `"hello"` (quoted) for non-empty strings as well. | Breaks AC2 — `hello` → `"hello"` is a visible regression for every existing script that pipes a non-empty string result. |
| Emit a configurable sentinel (e.g., `--empty-as=quoted`) | Add a flag to opt into the new behavior. | Overkill for a bug fix; adds surface area and documentation burden. The issue asks for a default behavior change, not a knob. |
| Fix in `emit_plain` | Special-case empty input in the generic helper. | `emit_plain` has no type information and is shared across commands. A command-level empty-string contract doesn't belong there. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #229 | 2026-04-22 | Initial defect design |
