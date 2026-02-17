# Root Cause Analysis: connect --status ignores --pretty and --plain output format flags

**Issue**: #114
**Date**: 2026-02-16
**Status**: Draft
**Author**: Claude

---

## Root Cause

The `execute_status()` function in `src/main.rs` (line 461) calls `print_json()` (line 284) to serialize the `StatusInfo` struct. The `print_json()` helper unconditionally uses `serde_json::to_string()`, which always produces compact single-line JSON. It accepts only the value to serialize and has no awareness of the `OutputFormat` flags (`--pretty`, `--plain`) from `GlobalOpts`.

In contrast, `src/tabs.rs` defines its own `print_output()` function (line 67) that accepts a reference to `OutputFormat` and conditionally uses `serde_json::to_string_pretty()` when `output.pretty` is true. The `execute_list()` function in `tabs.rs` (line 149) also checks `global.output.plain` to produce a human-readable table instead of JSON.

The bug exists because `execute_status()` was written before the output format flags were added (or was not updated when they were), and it uses the format-unaware `print_json()` instead of a format-aware output path.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/main.rs` | 284–292 | `print_json()` — always serializes to compact JSON, ignoring output flags |
| `src/main.rs` | 461–477 | `execute_status()` — calls `print_json()` instead of respecting `global.output` |

### Triggering Conditions

- User passes `--pretty` or `--plain` flag with `connect --status`
- `execute_status()` receives `global` which contains the flags, but never inspects `global.output`
- The flags are parsed correctly by clap; they are simply not consulted during output

---

## Fix Strategy

### Approach

Modify `execute_status()` to inspect `global.output` and produce the appropriate output format:

1. When `global.output.plain` is true, format the `StatusInfo` as human-readable key-value text (similar to how `tabs.rs` uses `format_plain_table()` for plain output).
2. When `global.output.pretty` is true, use `serde_json::to_string_pretty()` for indented JSON.
3. When neither flag is set (default), use `serde_json::to_string()` for compact JSON (preserving current behavior).

The fix reuses the existing `print_output()` pattern from `tabs.rs` — either by calling a similar local helper or by extracting `print_output()` to a shared location. Given the defect-only scope, the simplest approach is to inline the format-aware logic directly in `execute_status()` or add a local `print_output()` to `main.rs` (matching the `tabs.rs` pattern). Extracting a shared helper is out of scope for this fix.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/main.rs` | Modify `execute_status()` to check `global.output.plain` and `global.output.pretty`, producing the correct output format | Directly addresses the root cause — the function now respects the output flags |
| `src/main.rs` | Add a `format_plain_status()` helper for the `--plain` text format | Provides human-readable key-value output for the `StatusInfo` struct, analogous to `format_plain_table()` in `tabs.rs` |

### Blast Radius

- **Direct impact**: Only `execute_status()` in `src/main.rs` is modified; no other callers of `print_json()` are affected
- **Indirect impact**: None — `execute_status()` is called only from `execute_connect()` when `args.status` is true (line 342), and no other code depends on its output format
- **Risk level**: Low

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Default (no-flag) output format changes | Low | AC3 regression test explicitly verifies compact JSON output is preserved |
| Plain text format is inconsistent with tabs list --plain | Low | Follow the same key-value formatting pattern used in `tabs.rs` |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Extract shared `print_output()` to a common module | Move `tabs.rs`'s `print_output()` to a shared utility and use it everywhere | Larger refactor, out of scope for a defect fix. Can be done in a separate cleanup issue. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
