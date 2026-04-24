# Tasks: Fix `config init --config <path>` ignored

**Issue**: #249
**Date**: 2026-04-23
**Status**: Planning
**Author**: Rich Nunley

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Honor global `--config` as init destination in dispatch | [ ] |
| T002 | Add regression Gherkin scenarios + step definitions | [ ] |
| T003 | Verify no regressions across `config show` / `config path` and existing CLI tests | [ ] |

---

### T001: Honor `--config` as Init Destination

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `execute_config` accepts the raw `cli.global.config` value (pre-`find_config_file`).
- [ ] `ConfigCommand::Init` arm computes destination as `args.path.clone().or_else(|| global_config_raw.clone())` and passes it to `config::init_config`.
- [ ] When both `--path` and `--config` are supplied with different values, `--path` wins and a one-line note is emitted on stderr identifying the override.
- [ ] When neither flag is supplied, the destination remains the XDG default (`config::default_init_path()`).
- [ ] The JSON `created` field reports the path actually written.
- [ ] On success the process exits 0; on writer failure (e.g., parent directory missing) the process exits 1 with a clear stderr error and no XDG-default file is created.
- [ ] No changes outside the `execute_config` dispatch and its caller in `main`.

**Notes**: Do not modify `src/config.rs` (`init_config`, `init_config_to`, `find_config_file`) or `src/cli/mod.rs` (`ConfigInitArgs`). The fix is purely a dispatch change. Follow the fix strategy in `design.md` § Fix Strategy.

### T002: Add Regression Test

**File(s)**: `tests/features/config-init-custom-path.feature`, `tests/bdd.rs`
**Type**: Create / Modify
**Depends**: T001
**Acceptance**:
- [ ] New feature file `tests/features/config-init-custom-path.feature` contains the three scenarios from `feature.gherkin` (AC1 / AC2 / AC3), each tagged `@regression`.
- [ ] Cucumber World and step definitions are added to `tests/bdd.rs` following the project's existing single-file pattern.
- [ ] Steps shell out to the built `agentchrome` binary (or invoke the library entry point) using a per-scenario tempdir for the destination path, so the tests do not collide with the user's real XDG config.
- [ ] All scenarios pass with the fix from T001 applied.
- [ ] AC1 scenario fails when the T001 dispatch change is reverted (confirms it catches the original bug).
- [ ] `cargo test --test bdd` runs cleanly.

**Notes**: Use `tempfile::TempDir` and override `XDG_CONFIG_HOME` (or set `HOME`) inside the test process so the "default-path" assertion in AC2 lands inside the tempdir rather than the developer's real config directory.

### T003: Verify No Regressions

**File(s)**: existing — no changes
**Type**: Verify
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo test --workspace` passes.
- [ ] `cargo test --test bdd` passes.
- [ ] Manual smoke: `agentchrome config show --config <existing.toml>` still loads and displays the supplied file (read-side semantics preserved).
- [ ] Manual smoke: `agentchrome config path` after a custom-path init reflects the search order documented in `find_config_file` (no behavior change).
- [ ] Manual smoke per `steering/tech.md`: run `agentchrome config init --config <tempdir>/custom.toml`, confirm exit 0, confirm file exists at the requested path with mode `0600` on Unix, and confirm no file appears at the XDG default.

---

## Validation Checklist

- [x] Tasks are focused on the fix — no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)
