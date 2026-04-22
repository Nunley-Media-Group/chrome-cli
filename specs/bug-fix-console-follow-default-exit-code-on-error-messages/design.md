# Root Cause Analysis: console follow default exit code on error messages

**Issue**: #228
**Date**: 2026-04-22
**Status**: Draft
**Author**: Rich Nunley

---

## Root Cause

`execute_follow` in `src/console.rs` tracks `saw_errors = true` whenever a console event with an error-level type arrives (`src/console.rs:471-474`). After the follow loop breaks — whether due to `--timeout` elapsing or Ctrl+C — the function unconditionally converts `saw_errors` into a non-zero exit via `AppError { message: "Error-level console messages were seen", code: GeneralError }` at `src/console.rs:528-536`.

This behavior was introduced for the CI-assertion use case documented in `specs/feature-console-message-reading-with-filtering/requirements.md` FR11 ("console follow non-zero exit on error messages — Useful for CI"). The original spec made that the sole behavior rather than an opt-in, so a user running `console follow` as a tail-style monitor gets an unexpected exit 1 whenever any page script calls `console.error`.

The fix flips the default to monitoring (exit 0 regardless of log levels) and gates the assertion behavior behind a new opt-in `--fail-on-error` flag on `ConsoleFollowArgs`.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/console.rs` | 441–537 | `execute_follow` — tracks `saw_errors` and returns the `AppError` after the loop ends |
| `src/console.rs` | 528–536 | Post-loop exit-code decision based on `saw_errors` only |
| `src/cli/mod.rs` | 2980–2994 | `ConsoleFollowArgs` struct — currently has `--type`, `--errors-only`, `--timeout` only |
| `src/cli/mod.rs` | 2933–2950 | `Follow` subcommand `long_about` and `after_long_help` — documents only the current behavior |
| `src/examples/commands.rs` | 356–366 | `examples` subcommand entries for `console follow` — no mention of `--fail-on-error` |

### Triggering Conditions

- `console follow` is invoked without `--fail-on-error` (i.e., every current invocation).
- At least one `console.error` (or other error-level — `assert`) message is observed during the window.
- The loop exits via either the `--timeout` deadline or Ctrl+C (Ctrl+C currently has the same bug, but issue #228 scopes the fix to the timeout case; the fix naturally covers both since the decision is centralized post-loop).

The condition was not caught before because the original feature spec treated CI-assertion as the intended default rather than an opt-in. No prior AC exercised the "timeout elapses with errors seen, default mode" scenario expecting exit 0.

---

## Fix Strategy

### Approach

Add a new `--fail-on-error` boolean flag to `ConsoleFollowArgs` (defaulting to `false`). In `execute_follow`, continue to track `saw_errors` unchanged, but gate the post-loop `AppError` return on `args.fail_on_error && saw_errors` instead of `saw_errors` alone. Update the `Follow` subcommand's `long_about`/`after_long_help`, and the `examples` command entry, to document both modes.

This is the minimal correct fix:

- No change to streaming output, filtering, or CDP interaction.
- No change to the `--fail-on-error` error contract — same message, same exit code, same JSON shape as today. This preserves the CI use case exactly for users who opt in.
- No change to Ctrl+C handling (both modes treat Ctrl+C as a clean exit 0 in practice — the Ctrl+C branch already breaks out of the loop and the post-loop decision now requires `--fail-on-error` to return non-zero).

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/cli/mod.rs` (`ConsoleFollowArgs`) | Add `pub fail_on_error: bool` with `#[arg(long)]` | New opt-in flag |
| `src/cli/mod.rs` (`Follow` subcommand doc attributes) | Expand `long_about` and `after_long_help` to describe default monitoring behavior and `--fail-on-error` assertion mode, with at least one worked example for each | FR3 / AC3 |
| `src/console.rs` (`execute_follow` post-loop decision) | Change the `if saw_errors` branch to `if args.fail_on_error && saw_errors` | FR1 / FR2 — flip default, preserve opt-in |
| `src/examples/commands.rs` | Add an `ExampleEntry` for `agentchrome console follow --fail-on-error --timeout 10000` with a short description | FR3 / AC3 |
| `README.md`, `docs/claude-code.md` | Update any narrative text that implies `console follow` exits non-zero on errors by default | FR3 — keep user-facing docs consistent |

### Blast Radius

- **Direct impact**: `src/console.rs` `execute_follow` and `src/cli/mod.rs` `ConsoleFollowArgs` + `Follow` doc strings; `src/examples/commands.rs`.
- **Indirect impact**: Any user or CI script that currently relies on `console follow --timeout` to fail when `console.error` occurs will silently shift to exit 0. This is a **behavior change for CI callers** and is the whole point of the fix per the issue — callers must add `--fail-on-error` to preserve the old semantics. Called out explicitly in release notes.
- **Test-side impact**: Existing BDD scenarios under `tests/features/console.feature` exercise only help/flag validation for `follow`; none run the full follow loop under Chrome and so no existing scenarios depend on the exit-1 behavior. The `103-` and `146-` regression features only assert `console follow --timeout 2000` completes — they do not assert exit code on error, so they remain correct under the new default. The new feature file adds explicit regression coverage for both modes.
- **Risk level**: Low.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| CI pipelines relying on the old exit-1 default silently start passing when they should fail | Medium | Fix intentionally requires opt-in via `--fail-on-error`. Call out in release notes / CHANGELOG. Issue #228 explicitly confirms this is the desired behavior change. |
| `--fail-on-error` opt-in emits a different error message or exit code than the previous default | Low | Reuse the exact `"Error-level console messages were seen"` message and `ExitCode::GeneralError` code — AC2 pins the contract byte-for-byte. |
| `--errors-only --fail-on-error` or `--type error --fail-on-error` combinations behave unexpectedly | Low | `--fail-on-error` operates on `saw_errors` which is tracked *before* the type filter is applied (see `src/console.rs:471-482`) — so the assertion fires on any observed error-level event regardless of type filtering. This matches the current behavior and does not regress. |
| Ctrl+C during `--fail-on-error` mode returns non-zero unexpectedly | Low | Existing Ctrl+C branch breaks out of the loop; the post-loop decision still checks `fail_on_error && saw_errors`. If the user opted in and errors were seen, exit 1 is consistent. Not covered by #228 scope; no change from current Ctrl+C behavior under the old code either. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Invert the flag (`--no-fail-on-error` to opt OUT) | Keep current default (exit 1 on errors) and let monitoring users opt out | Issue explicitly requests monitoring as the default ("behaves like tail -f by default"). Preserving the current default defeats the purpose of the fix. |
| Separate subcommand (`console monitor` vs `console follow`) | Split the two modes into distinct commands | Heavier change, divergent help/docs, no clear user benefit over a single flag. The two modes share identical streaming semantics — only the exit decision differs. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (clap `#[arg(long)]` boolean flag; centralized post-loop exit decision)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #228 | 2026-04-22 | Initial defect report |
