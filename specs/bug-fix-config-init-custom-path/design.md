# Root Cause Analysis: `config init` ignores `--config` path

**Issue**: #249
**Date**: 2026-04-23
**Status**: Draft
**Author**: Rich Nunley

---

## Root Cause

The `--config <path>` flag is defined as a **global** option (`GlobalOpts.config`) whose intended semantics elsewhere in the CLI are *"read configuration from this file"*. The `config init` subcommand declares its own destination flag, `--path`, on `ConfigInitArgs`. When a user supplies only `--config` (which is the natural flag name for "the config file I want to create"), the global flag is consumed by `config::find_config_file` for read-resolution and never reaches `init_config`.

`find_config_file` (`src/config.rs:210`) silently skips the explicit `--config` value when the file does not exist (`if p.exists()` → `Some`, otherwise fall through). The init handler at `src/main.rs:308` then calls `config::init_config(args.path.as_deref())` with `args.path = None`, which writes to the XDG default via `default_init_path()` (`src/config.rs:489`).

The exit-code mismatch reported by the user is a separate symptom of the same misuse: `load_config` runs before `execute_config`; if the explicit `--config` path is non-existent the load returns an empty `ConfigFile`, the init succeeds and prints its JSON, and a downstream code path (likely `skill_check::emit_stale_notice_if_any` or a deferred error from the unread config) surfaces a non-zero exit. Once the destination flag is honored properly, the init handler short-circuits before that mismatch matters.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/main.rs` | 308–314 | `ConfigCommand::Init` arm — only consults `args.path`; ignores `cli.global.config`. |
| `src/main.rs` | 165–172 | Pre-dispatch `load_config` consumes `cli.global.config` for read-resolution before init runs. |
| `src/config.rs` | 481–535 | `init_config` / `init_config_to` — already correct; just never receives the user's path. |
| `src/cli/mod.rs` | 3758–3764 | `ConfigInitArgs.path` — the only flag the init arm currently honors. |

### Triggering Conditions

- User invokes `agentchrome config init` with the global `--config <path>` flag (a natural choice — the flag name matches user intent for the init subcommand).
- The user did NOT also pass `--path`.
- The path supplied to `--config` does not yet exist (which is the normal case for `init`).

This was not caught before because every other subcommand treats `--config` as a *read* path, and the existing tests for `init_config_to` exercise the underlying writer directly rather than the CLI surface (`src/config.rs:700–760`).

---

## Fix Strategy

### Approach

Make `config init` accept the destination from either flag, with `--path` winning when both are present. The change is confined to the `ConfigCommand::Init` match arm in `src/main.rs`: when `args.path` is `None`, fall back to `cli.global.config.clone()` before calling `config::init_config`. To make this robust we need to thread `cli.global.config` (the **raw** user-supplied value, not the resolved path returned from `load_config`) into `execute_config` for the Init arm specifically — `find_config_file` discards the value when the file doesn't exist, so we cannot rely on `resolved.config_path`.

This is the minimal correct fix because:
1. `init_config_to` already does the right thing for any path it is given — no changes needed in `src/config.rs`.
2. The flag plumbing already exists on `GlobalOpts`; we are only changing dispatch logic.
3. We avoid altering the read-side semantics of `--config` for every other subcommand.

A one-line stderr note when both flags are supplied with different values (FR5) makes the precedence transparent without breaking either form.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/main.rs` | In `execute_config`, accept the raw global `--config` value. In the `Init` arm, choose destination as `args.path.or(global_config.clone())`. If both are set and differ, emit a stderr note and use `args.path`. | Honors FR1, FR2, FR3, FR5 without touching the writer. |
| `src/main.rs` | Pass `cli.global.config.as_deref()` (raw, pre-resolution) into `execute_config` alongside the existing `resolved`. | `find_config_file` drops non-existent paths, so the resolved value is unreliable for the init destination. |

No changes to `src/config.rs` or `src/cli/mod.rs` are required. The stale-notice / load-config error that produced the exit-code-1 symptom becomes unreachable once the init arm consumes its own path: the global `--config` no longer points at a file that load_config attempts to interpret as readable input — but as a defensive measure the init arm should run *before* any read-side validation that could itself signal failure. Reordering is achieved by the dispatch change: when init detects that the global `--config` is being used as a destination (i.e. supplied and `args.path` is None), the path is consumed locally and not re-exposed to read-side resolution for this invocation's exit code.

### Blast Radius

- **Direct impact**: `execute_config`'s Init arm and the call site in `main`.
- **Indirect impact**: None for `Show` and `Path` subcommands — they continue to consume the resolved read path. No external consumers (man pages, completions, examples) need updating because `--path` remains the documented init destination; honoring `--config` is additive.
- **Risk level**: Low.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `config init` (no flags) starts writing somewhere unexpected. | Low | AC2 / regression scenario verifies the XDG default path is still chosen when neither flag is supplied. |
| `config init --path X --config Y` produces ambiguous behavior. | Medium | FR5 defines `--path` as the winner with a stderr note; covered by a precedence regression scenario. |
| `config show --config /existing.toml` (read path) breaks. | Low | Read-side semantics of `--config` are untouched; the change is scoped to the Init arm. Covered by leaving existing `Show`/`Path` integration tests in place. |
| Existing CI scripts that pass `--config` to `init` expecting it to be silently ignored. | Very Low | Documented behavior was that `--path` is the destination; no contract is being broken — only the silent-failure case is being fixed. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Reject the combination with a hard error | Detect `--config` on `init` and exit 1 with "use --path instead". | The user story explicitly asks for `--config` to *work* as the destination. A hard error keeps the broken UX. |
| Rename / unify so `init` takes only `--config` | Drop `--path` from `ConfigInitArgs`. | Breaking change to a published CLI; violates Out of Scope and the project's flag-shape conventions. |
| Make `find_config_file` return the path even when missing | So `resolved.config_path` carries it through. | Changes read-side semantics for every subcommand; far larger blast radius than the init dispatch change. |

---

## Validation Checklist

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
