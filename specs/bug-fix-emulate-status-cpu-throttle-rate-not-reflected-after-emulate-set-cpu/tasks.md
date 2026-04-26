# Tasks: emulate status omits CPU and geolocation after emulate set

**Issue**: #253
**Date**: 2026-04-26
**Status**: Planning
**Author**: Codex

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Reproduce and isolate the persistence gap | [ ] |
| T002 | Fix CPU and geolocation state persistence/readback | [ ] |
| T003 | Add regression coverage | [ ] |
| T004 | Verify no regressions | [ ] |

---

### T001: Reproduce and Isolate the Persistence Gap

**File(s)**: `src/emulate.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] A focused reproduction proves whether the failure is in state merging, state path resolution, state file writing, or status readback.
- [ ] The reproduction uses a controlled AgentChrome home or state path so `emulate set` and `emulate status` read/write the same `emulate-state.json`.
- [ ] The exact root cause is reflected in the implementation notes or test name before applying the fix.
- [ ] No public CLI behavior changes are introduced by the reproduction work.

**Notes**: The issue evidence shows `emulate set` can populate the transient output object, while `emulate status` sees `None`. Inspect the persisted file immediately after set and before status when isolating the failing branch.

### T002: Fix CPU and Geolocation State Persistence/Readback

**File(s)**: `src/emulate.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `emulate set --cpu 4` writes `cpu: 4` into persisted emulation state before returning success.
- [ ] `emulate set --geolocation 37.7749,-122.4194` writes latitude and longitude into persisted emulation state before returning success.
- [ ] `emulate status` reads `cpu` and `geolocation` from the same persisted state and includes them in JSON output when present.
- [ ] `emulate reset` clears persisted CPU and geolocation so the status output omits both fields afterward.
- [ ] Updating CPU or geolocation does not erase unrelated persisted fields such as network, user-agent, viewport, color scheme, device scale, or mobile.
- [ ] Existing state files without CPU or geolocation fields still deserialize without error.

**Notes**: Prefer the smallest fix inside the existing `EmulateState` helpers and `execute_set`/`execute_status` paths. If a helper is extracted, keep it private unless another module already needs it.

### T003: Add Regression Coverage

**File(s)**: `src/emulate.rs`, `tests/features/253-fix-emulate-status-cpu-throttle-rate-not-reflected-after-emulate-set-cpu.feature`, `tests/bdd.rs`
**Type**: Create/Modify
**Depends**: T002
**Acceptance**:
- [ ] Rust tests cover the set-to-status persistence contract for CPU and geolocation without requiring a live Chrome instance.
- [ ] The regression test fails if CPU or geolocation are not written to persisted state.
- [ ] The regression test fails if reset leaves CPU or geolocation visible in status state.
- [ ] The BDD feature maps all requirements acceptance criteria to `@regression` scenarios.
- [ ] The new feature file is bound in `tests/bdd.rs`; if scenarios require live Chrome, the binding documents the skip and the Rust tests provide CI coverage.
- [ ] Test wording uses existing BDD step conventions where possible, or adds shared step definitions for missing JSON field assertions.

### T004: Verify No Regressions

**File(s)**: existing test suite
**Type**: Verify
**Depends**: T003
**Acceptance**:
- [ ] `cargo fmt --check` passes.
- [ ] Focused emulate tests pass.
- [ ] `cargo test` passes, or any Chrome-dependent skips are documented in the verification report.
- [ ] Existing emulation behavior still passes for network, viewport, user-agent, color scheme, device scale, and mobile state.
- [ ] Existing JSON output contracts are unchanged.

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the defect
- [x] Dependencies are linear and explicit
- [x] Acceptance criteria are verifiable
- [x] BDD testing and executable regression coverage are included
- [x] File paths reference actual project structure
- [x] No unrelated version, release, or CLI schema work is included
