# Root Cause Analysis: form fill-many panics due to 'json' arg name collision

**Issue**: #84
**Date**: 2026-02-15
**Status**: Approved
**Author**: Claude

---

## Root Cause

The `OutputFormat` struct in `src/cli/mod.rs` (line 127) defines a global `--json` flag as a `bool` field named `json`, with `#[arg(long, global = true)]`. This flag is propagated to all subcommands by clap.

Separately, the `FormFillManyArgs` struct in `src/cli/mod.rs` (line 1582) defines a positional argument also named `json` as an `Option<String>`. This field accepts inline JSON input for the form fill-many command.

When clap builds the argument parser, both arguments are registered under the name `json`. The global flag registers it as a `bool`, and the positional registers it as a `String`. When the CLI is invoked with an inline JSON positional argument, clap attempts to resolve the `json` argument, finds the global `--json` flag definition first, and tries to downcast the value to `bool`. Since the actual value is a `String`, the downcast fails and the process panics.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/cli/mod.rs` | 121–136 | `OutputFormat` struct defines `json: bool` with `global = true` |
| `src/cli/mod.rs` | 1578–1591 | `FormFillManyArgs` struct defines `json: Option<String>` as positional |
| `src/form.rs` | 417–429 | `execute_fill_many` reads `args.json` to get inline JSON input |

### Triggering Conditions

- The user passes an inline JSON string as a positional argument to `form fill-many`
- The clap parser encounters both a global `--json` flag (bool) and a local positional `json` (String) under the same name
- Clap's argument resolution picks the global definition and attempts a type downcast that fails

---

## Fix Strategy

### Approach

Rename the positional field in `FormFillManyArgs` from `json` to `input`. This is the minimal correct fix: it eliminates the naming collision while preserving the exact same CLI behavior (the positional argument has no `--` flag prefix, so users won't see a name change). The `value_name = "JSON"` attribute already controls what appears in help text, so the rename is purely internal.

Then update the single reference site in `src/form.rs` where `args.json` is accessed, changing it to `args.input`.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/cli/mod.rs` | Rename `FormFillManyArgs::json` field to `input` | Eliminates the name collision with the global `--json` flag |
| `src/form.rs` | Change `args.json` to `args.input` on line 422 | Updates the handler to use the renamed field |

### Blast Radius

- **Direct impact**: `FormFillManyArgs` struct (field rename) and `execute_fill_many` function (field access)
- **Indirect impact**: None — the field is a private positional argument accessed only in `src/form.rs`. No external callers or serialization depend on the field name.
- **Risk level**: Low

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `--json` global output flag stops working for fill-many | Low | The global flag is on `OutputFormat`, not `FormFillManyArgs`; rename doesn't touch `OutputFormat` |
| `--file` input path breaks | Low | `--file` is a separate named argument unaffected by the rename |
| Help text changes unexpectedly | Low | `value_name = "JSON"` already controls the display name; the Rust field name is not user-visible |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Rename global `--json` flag | Change the global output format flag name | Much larger blast radius — affects every command, user-facing flag name change |
| Use `#[arg(name = "input")]` annotation | Keep field as `json` but set clap's internal name | Less clear — the Rust field name and clap name would diverge, making code harder to understand |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
