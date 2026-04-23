# Root Cause Analysis: form fill-many uses `uid` instead of `target`

**Issues**: #246
**Date**: 2026-04-23
**Status**: Draft
**Author**: Rich Nunley

---

## Root Cause

The `FillEntry` struct in `src/form.rs:63-68` was authored with a field named `uid`, mirroring the snapshot UID concept internally. Every other subcommand in the `form` family exposes this identifier through a CLI argument named `target` (see `FormFillArgs.target` at `src/cli/mod.rs:2830`, `FormClearArgs.target` at `src/cli/mod.rs:2872`), which accepts either a UID (`s5`) or a `css:`-prefixed selector — resolved uniformly by `resolve_target_to_backend_node_id` (`src/form.rs:108`).

Because `FillEntry` deserializes from JSON by field name, the documented positional/flag vocabulary (`target`) diverges from the batch-entry vocabulary (`uid`). The inconsistency is propagated by three surfaces that the user reads:

1. The deserialization error message in `src/form.rs:782-786` literally says `expected array of {uid, value} objects`.
2. The clap `long_about` and `after_long_help` for `FillMany` in `src/cli/mod.rs:2760-2773` documents `{uid, value}`.
3. Usage examples in `src/cli/mod.rs:448` and `src/cli/mod.rs:2768` use `"uid"`.

None of these three copies ever reference the word `target`, which is what a developer expects after reading `form fill --help`.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/form.rs` | 63–68 | `FillEntry` struct — the deserialization target |
| `src/form.rs` | 782–786 | Error message for malformed JSON |
| `src/form.rs` | 814–820 | Uses `entry.uid` when filling and when building the `FillResult.filled` field |
| `src/form.rs` | 1423–1450 | Unit tests for `FillEntry` deserialization |
| `src/cli/mod.rs` | 448 | `agentchrome --help` top-level example using `"uid"` |
| `src/cli/mod.rs` | 2760–2773 | `FillMany` clap `long_about` + `after_long_help` examples |
| `src/cli/mod.rs` | 2852 | `FormFillManyArgs.input` doc comment mentions `{uid, value}` |

### Triggering Conditions

- User writes a `fill-many` payload using the vocabulary they just learned from `form fill --help` (`target`).
- Serde rejects the payload because `uid` is the declared (and only) field name.
- `serde_json`'s "missing field `uid`" message reinforces the confusion rather than pointing at `target`.

---

## Fix Strategy

### Approach

Rename the primary JSON field on `FillEntry` from `uid` to `target`, keep `uid` working as a silent serde alias, and update every user-visible string that currently says `uid` in the `fill-many` context. This is a one-line serde attribute change in the struct plus string edits in the three doc surfaces. No behavior around what values the identifier accepts changes — `resolve_target_to_backend_node_id` already handles both UIDs and `css:` selectors.

Because serde aliases deserialize into the same Rust field, we also rename the Rust field itself from `uid` to `target` so call sites read naturally (`entry.target` instead of `entry.uid`). The public `FillResult.filled` JSON shape is unchanged — it already surfaces the resolved identifier, not the payload key.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/form.rs` (struct `FillEntry`) | Rename field `uid: String` → `target: String` and add `#[serde(alias = "uid")]`. Update the doc comment. | Accept `target` primarily; preserve `uid` silently for existing scripts. |
| `src/form.rs` (`execute_fill_many`) | Replace `&entry.uid` / `entry.uid.clone()` with `&entry.target` / `entry.target.clone()`. | Field rename follow-through. |
| `src/form.rs` (error at line 783) | `"expected array of {{uid, value}} objects"` → `"expected array of {{target, value}} objects"`. | Error text matches the documented key. |
| `src/form.rs` (tests at 1423–1450) | Add a `target`-keyed fixture, keep a `uid`-keyed fixture to prove alias compatibility, rename field accessors. | Lock in both keys going forward. |
| `src/cli/mod.rs:448` | Update top-level example from `{"uid":...}` to `{"target":...}`. | Surface consistency. |
| `src/cli/mod.rs:2760–2773` | Rewrite `long_about` and `after_long_help` example to use `target`. Mention that `uid` is still accepted for back-compat in a short trailing sentence. | Primary help copy uses `target`; preserves scripts reading old docs. |
| `src/cli/mod.rs:2852` | Doc comment on `FormFillManyArgs.input` → `{target, value}`. | Surface consistency. |

### Blast Radius

- **Direct impact**: `FillEntry` struct and its three tests in `src/form.rs`; `FillMany` clap metadata in `src/cli/mod.rs`.
- **Indirect impact**: None at the type boundary — `FillEntry` is private (pub-in-module) and only constructed by `execute_fill_many` via `serde_json::from_str`. No external callers.
- **Downstream docs**: `src/skill.rs:160` mentions `form fill-many` by name but does not show a payload; no change needed. README / generated man pages will refresh on next `chore(man)` regeneration — this is existing project practice, not extra work for this fix.
- **Risk level**: Low.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Existing scripts that post `{"uid": ..., "value": ...}` break. | Low | `#[serde(alias = "uid")]` keeps the old key accepted silently; AC2 regression test asserts this. |
| A payload contains *both* `uid` and `target` and the result is silently wrong. | Very Low | Serde's `alias` treats them as the same field — whichever appears last wins during parsing. Behavior is equivalent to a caller passing the same field twice; accepted as expected serde semantics. Out of scope to raise an explicit error. |
| Updated help copy drifts from the error message. | Low | Both strings are changed in the same commit; T002 asserts `--help` mentions `target` and the error message mentions `target`. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Rename only the struct field; don't alias `uid`. | Cleaner internal code but hard break for anyone scripting against the current CLI. | Issue body explicitly calls out the need for backward compatibility (FR2). |
| Accept both keys but emit a deprecation warning when `uid` is used. | Gives a migration signal. | Issue specifies "no deprecation warning"; AgentChrome's structured-output contract discourages stderr noise on the happy path. |
| Add a new `fill-many-v2` subcommand. | Keeps `fill-many` untouched. | Massive overkill for a field-rename; duplicates surface area permanently. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
