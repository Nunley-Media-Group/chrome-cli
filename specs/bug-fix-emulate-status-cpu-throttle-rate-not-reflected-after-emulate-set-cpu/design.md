# Root Cause Analysis: emulate status omits CPU and geolocation after emulate set

**Issue**: #253
**Date**: 2026-04-26
**Status**: Approved
**Author**: Codex

---

## Root Cause

The failing behavior can only occur when the state read by `execute_status` does not contain the CPU and geolocation values that `execute_set` reported as applied. The current source already has optional fields in both `EmulateStatusOutput` and `EmulateState`, and `execute_status` builds its output from `read_emulate_state()` for `cpu` and `geolocation`. Because both fields use `skip_serializing_if = "Option::is_none"`, the observed omission means the persisted read path is receiving `None` for both fields.

The implementation has a coverage gap around that exact contract. Existing tests prove that a hand-built `EmulateState` can serialize and deserialize, but they do not exercise the command-level set-to-status path that starts from `EmulateSetArgs`, merges values into existing persisted state, writes `~/.agentchrome/emulate-state.json`, and reads it back through the status assembly path. Existing BDD files document CPU and geolocation status scenarios, but those Chrome-dependent scenarios are not currently bound as executable CI regression gates. As a result, a broken or bypassed persistence merge can ship even though standalone serialization tests still pass.

The minimal fix is to isolate and correct the command-level persistence contract for CPU and geolocation, then add executable coverage that fails when the set path does not write those fields to the same state path consumed by status. During implementation, the first step must reproduce the state-file gap in a controlled test home and replace this root-cause hypothesis with the concrete failing branch if the code path differs.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/emulate.rs` | 18-35 | `EmulateStatusOutput` defines optional `cpu` and `geolocation` output fields. |
| `src/emulate.rs` | 82-101 | `EmulateState` defines optional persisted `cpu` and `geolocation` fields. |
| `src/emulate.rs` | 103-203 | Emulation state path, write, read, and delete helpers for `~/.agentchrome/emulate-state.json`. |
| `src/emulate.rs` | 541-798 | `execute_set` applies CPU/geolocation overrides and is responsible for merging them into persisted state. |
| `src/emulate.rs` | 812-900 | `execute_reset` clears CDP overrides and deletes persisted state. |
| `src/emulate.rs` | 915-995 | `execute_status` reads persisted CPU/geolocation state and serializes status output. |
| `tests/features/74-fix-emulate-status-inaccurate-state.feature` | 35-39 | Existing documented CPU status regression scenario. |
| `tests/features/85-emulate-overrides-persistence.feature` | 33-39, 53-59 | Existing documented geolocation and CPU persistence scenarios. |
| `tests/bdd.rs` | 6694-6703 | The active emulate BDD runner currently filters to CLI-only scenarios in `emulate.feature`, leaving Chrome-dependent persistence scenarios out of CI. |

### Triggering Conditions

- `agentchrome emulate set --cpu 4` or `agentchrome emulate set --geolocation 37.7749,-122.4194` succeeds and reports the value in that command's own JSON output.
- A later `agentchrome emulate status` invocation runs in a separate process and must rely on persisted state, not the previous command's in-memory `status` value.
- The persisted state file is missing the CPU/geolocation fields, or `execute_status` reads a different/empty state path.
- Serialization silently hides the failure because `None` optional fields are omitted from JSON.

---

## Fix Strategy

### Approach

Keep the fix inside the emulation command/state boundary. Do not change the public CLI schema, output field names, or absent-value behavior. First add a focused reproduction that uses a controlled AgentChrome home or injectable state path to prove the exact set-to-status persistence failure. Then fix the smallest failing path in `src/emulate.rs`.

If the failing branch is the command-level state merge, extract the merge into a small testable helper that updates an `EmulateState` from the same values `execute_set` reports to users. That helper should write CPU and geolocation from the parsed arguments and applied output, preserve unrelated existing fields, and clear geolocation only for `--no-geolocation`. If the failing branch is path selection, make `emulate set` and `emulate status` resolve the state path through the same helper and cover the behavior with a temp-home test.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/emulate.rs` | Fix the exact state merge or state-path bug that causes `cpu` and `geolocation` to be absent after `emulate set`. | Restores the core persistence contract without changing CLI behavior. |
| `src/emulate.rs` | Add or expose a narrow internal helper for set-state merging and/or status assembly if needed for deterministic non-Chrome tests. | Makes the persistence contract directly testable without depending on a live CDP session. |
| `src/emulate.rs` tests | Add a regression test that starts with default state, applies CPU and geolocation through the same merge/path used by `execute_set`, reads status state back, and asserts values. | Catches the defect even when Chrome-dependent BDD scenarios are skipped in CI. |
| `tests/features/253-fix-emulate-status-cpu-throttle-rate-not-reflected-after-emulate-set-cpu.feature` | Add BDD scenarios for CPU visibility, geolocation visibility, and reset clearing. | Preserves the acceptance criteria as executable/manual regression documentation. |
| `tests/bdd.rs` | Bind the new feature file according to the repo's BDD convention. If live Chrome remains required, explicitly filter those scenarios out of default CI and rely on the focused unit test for automated coverage. | Keeps `tests/features/` discoverable and prevents silent orphaning. |

### Blast Radius

- **Direct impact**: `src/emulate.rs` state write/read behavior for `emulate set`, `emulate status`, and `emulate reset`.
- **Test impact**: `tests/features/253-...feature`, `tests/bdd.rs`, and focused Rust tests in `src/emulate.rs`.
- **Risk level**: Medium. The fix touches shared emulation state used by CPU, geolocation, network, viewport, user-agent, device scale, color scheme, and mobile status reporting.
- **Compatibility**: Existing `emulate-state.json` files must continue to deserialize. The fix must not require migration or emit optional absent fields as `null`.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Fixing CPU/geolocation overwrites unrelated persisted fields | Medium | Add assertions that existing network, user-agent, viewport, color-scheme, device-scale, and mobile fields are preserved when only CPU or geolocation is set. |
| Reset behavior changes from omitted fields to `null` fields | Medium | Add explicit reset assertions for field absence after `emulate reset`. |
| Tests pass only because they construct `EmulateState` directly | Medium | Test the same merge/path helper used by `execute_set`, not only `serde_json::to_value` on a handcrafted state. |
| Chrome-dependent BDD scenarios remain skipped in CI | Medium | Add non-Chrome executable Rust coverage for the persistence contract and bind the BDD feature with an explicit comment if scenarios need live Chrome. |
| State path differs between set and status in temp-home/manual runs | Low | Use a controlled AgentChrome home in tests and assert the expected state file receives CPU/geolocation before status reads it. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Query CPU/geolocation live from CDP during status | Ask Chrome for the current emulation state instead of using the state file. | CDP emulation setters do not provide reliable read APIs for these values; AgentChrome already uses persisted state for non-queryable overrides. |
| Emit `null` for absent CPU/geolocation | Make missing optional fields visible as `null`. | This changes the existing JSON contract and conflicts with the issue's reset expectation that absent means omitted. |
| Refactor all emulation persistence | Redesign `EmulateState` and all state application logic. | Broader than the defect. The issue only requires restoring CPU/geolocation write/read correctness. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references and the implementation must verify the exact failing branch
- [x] Fix is minimal and avoids unrelated emulation refactors
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Design follows existing command-module and BDD patterns from `steering/structure.md`
