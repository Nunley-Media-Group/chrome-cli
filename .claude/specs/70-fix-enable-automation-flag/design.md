# Root Cause Analysis: Chrome launched via connect --launch missing --enable-automation flag

**Issue**: #70
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude

---

## Root Cause

The `launch_chrome()` function in `src/chrome/launcher.rs` builds the Chrome command with four flags: `--remote-debugging-port`, `--user-data-dir`, `--no-first-run`, and `--no-default-browser-check`. The `--enable-automation` flag was simply never included in this list.

This flag is what tells Chrome to display the "Chrome is being controlled by automated test software" infobar and to enable certain automation-specific behaviors (e.g., suppressing certain security prompts that interfere with automated testing). Without it, Chrome appears as a normal browser session, giving users no visual indication that CDP is active.

The omission is a straightforward oversight — the flag was not part of the original launch implementation. There is no conditional logic or configuration that intentionally excludes it.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/chrome/launcher.rs` | 148–152 | Builds the Chrome `Command` with launch flags |

### Triggering Conditions

- User runs `chrome-cli connect --launch` (any mode — headed or headless)
- `launch_chrome()` is called, which builds the Chrome command without `--enable-automation`
- This occurs on every launch — it is not intermittent

---

## Fix Strategy

### Approach

Add `--enable-automation` as a hardcoded argument in the Chrome command builder, immediately after the existing flags (`--no-first-run`, `--no-default-browser-check`). This is the minimal correct fix — a single `.arg("--enable-automation")` call in the command builder chain.

The flag should be unconditional (not gated on headed/headless mode) because:
1. In headed mode, it displays the automation infobar
2. In headless mode, it has no visible effect but enables automation behaviors
3. Chrome tolerates the flag in all modes

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/chrome/launcher.rs` | Add `.arg("--enable-automation")` at line ~152 | Includes the missing flag in all Chrome launches |
| `src/chrome/launcher.rs` | Add unit test verifying the flag is present | Prevents future regressions |

### Blast Radius

- **Direct impact**: `launch_chrome()` in `src/chrome/launcher.rs` — the only function modified
- **Indirect impact**: All code paths that call `launch_chrome()` will now pass the flag. This is the `connect --launch` command path. No other callers exist.
- **Risk level**: Low — adding an argument to a command builder is additive and cannot break existing arguments

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Headless mode breaks with the flag | Very Low | Chrome documents `--enable-automation` as compatible with headless; AC2 regression test covers this |
| Extra args duplication causes Chrome error | Very Low | Chrome silently deduplicates repeated flags; AC3 covers this |
| Existing integration tests fail | Very Low | The flag is additive; existing tests don't assert absence of the flag |

---

## Validation Checklist

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
