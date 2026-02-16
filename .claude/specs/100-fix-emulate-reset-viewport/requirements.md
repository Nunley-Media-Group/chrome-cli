# Defect Report: emulate reset does not restore original viewport dimensions

**Issue**: #100
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude
**Severity**: Medium
**Related Spec**: `.claude/specs/21-device-network-viewport-emulation/`

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli emulate status` → observe baseline viewport (e.g., `{width: 756, height: 417}`)
3. `chrome-cli emulate set --viewport 375x667 --mobile`
4. `chrome-cli emulate status` → viewport is `{width: 375, height: 667}`, mobile is `true`
5. `chrome-cli emulate reset`
6. `chrome-cli emulate status` → viewport remains `{width: 375, height: 667}` instead of `{width: 756, height: 417}`

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (Darwin 25.3.0) |
| **Version / Commit** | `c584d2d` (main) |

### Frequency

Always

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | After `emulate reset`, the viewport returns to its original baseline dimensions (e.g., 756x417) — the dimensions the browser had before any emulation overrides were applied |
| **Actual** | After `emulate reset`, `mobile` is correctly reset to `false` but the viewport retains the previously overridden dimensions (375x667). The reset is incomplete. |

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Reset restores original viewport dimensions

**Given** a Chrome session is connected with an original viewport (e.g., 756x417)
**And** the viewport has been overridden via `emulate set --viewport 375x667`
**When** I run `emulate reset`
**Then** `emulate status` reports the original viewport dimensions (756x417)
**And** the exit code is 0

### AC2: Reset clears all emulation overrides completely

**Given** viewport, mobile, user-agent, and network overrides have been set via `emulate set`
**When** I run `emulate reset`
**Then** all values return to their original defaults
**And** the viewport dimensions match the pre-override baseline

### AC3: Reset is idempotent

**Given** no emulation overrides are active
**When** I run `emulate reset`
**Then** the command succeeds with exit code 0
**And** the viewport remains unchanged at its current dimensions

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `emulate reset` must restore the original viewport dimensions that were present before any emulation overrides were applied | Must |
| FR2 | `emulate reset` must clear `Emulation.setDeviceMetricsOverride` by re-applying the original baseline dimensions rather than relying solely on `Emulation.clearDeviceMetricsOverride` | Must |
| FR3 | The original baseline viewport dimensions must be captured and persisted before the first viewport override is applied | Should |

---

## Out of Scope

- Remembering viewport across Chrome restarts or session disconnects
- Restoring viewport on disconnect
- Changing the behavior of `emulate set` beyond what is needed for baseline capture
- Refactoring unrelated emulation code

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC2)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
