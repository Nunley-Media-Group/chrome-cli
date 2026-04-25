# Tasks: Fix connect --launch deleting temporary Chrome profile on detach

**Issue**: #265
**Date**: 2026-04-24
**Status**: Investigating
**Author**: Codex
**Related Spec**: `specs/feature-chrome-instance-discovery-and-launch/`

---

## Implementation Tasks

- [x] **T001: Fix detach process and temporary profile lifetime**
  - In `src/chrome/launcher.rs`, update `ChromeProcess::detach()` to preserve the internally managed temporary user data directory after the Chrome process is detached.
  - In `src/chrome/launcher.rs`, spawn Chrome in an independent process session / process group so caller cleanup does not terminate the detached browser.
  - Keep non-detached `TempDir` cleanup behavior unchanged.

- [x] **T002: Add regression coverage**
  - Add a unit test proving `detach()` does not remove a temporary user data directory.
  - Keep the existing cleanup-on-drop test to prove non-detached cleanup still works.

- [x] **T003: Verify launched session reuse**
  - Run focused Rust tests for `chrome::launcher`.
  - Build the CLI and manually verify `connect --launch --headless` followed by a separate follow-up command succeeds.

- [ ] **T004: Release and local install verification**
  - Bump from `1.51.1` to `1.51.2`.
  - Tag and push the release.
  - Monitor the GitHub release workflow through completion.
  - Reinstall the published crate locally and verify `agentchrome connect --launch` plus a follow-up command succeeds with the installed binary.

---

## Acceptance Mapping

| Acceptance Criteria | Tasks |
|---------------------|-------|
| AC1 | T001, T002 |
| AC2 | T003, T004 |
| AC3 | T002, T003 |

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #265 | 2026-04-24 | Initial defect tasks |
