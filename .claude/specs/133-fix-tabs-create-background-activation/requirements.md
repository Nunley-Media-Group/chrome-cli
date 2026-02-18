# Defect Report: tabs create --background not preventing tab activation (regression)

**Issue**: #133
**Date**: 2026-02-17
**Status**: Implemented
**Author**: Claude
**Severity**: Medium
**Related Spec**: `.claude/specs/121-fix-tabs-create-background/` — third attempt at the same fix; `.claude/specs/82-fix-tabs-create-background/` — original fix; `.claude/specs/7-tab-management/` — AC6

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli tabs create https://www.google.com` — google.com becomes active
3. `chrome-cli tabs list --pretty` — confirms google.com is `active: true`
4. `chrome-cli tabs create https://example.com --background` — should keep google.com active
5. `chrome-cli tabs list --pretty` — **example.com shows as `active: true`**

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 |
| **Version / Commit** | 1.0.0 (commit e50f7b3) |
| **Browser / Runtime** | HeadlessChrome/144.0.0.0 |
| **Configuration** | Default (headless mode) |

### Frequency

Always — reproducible on every invocation, though the underlying race condition means the polling loop sometimes succeeds and sometimes does not.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | After `tabs create --background`, the previously active tab remains `active: true` in `tabs list`. The new tab has `active: false`. |
| **Actual** | The newly created background tab is shown as `active: true` and the previously active tab shows `active: false`. |

### Error Output

No error output — the command exits with code 0 but the activation state reported by `tabs list` is incorrect.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Background tab does not become active

**Given** tab A is the active tab
**When** I run `tabs create https://example.com --background`
**And** I run `tabs list` in a separate invocation
**Then** tab A is still shown as `active: true`
**And** the new tab is shown as `active: false`

**Example**:
- Given: Tab at `https://www.google.com` is active
- When: `chrome-cli tabs create https://example.com --background`
- Then: `chrome-cli tabs list` shows google.com tab with `active: true` and example.com with `active: false`

### AC2: Non-background create still activates the new tab

**Given** tab A is the active tab
**When** I run `tabs create https://example.com` (without `--background`)
**Then** the new tab becomes the active tab (`active: true`)
**And** tab A shows `active: false`

### AC3: Active state is determined by Chrome, not by list position

**Given** the implementation queries Chrome for the actual active tab state
**When** `tabs list` is called
**Then** the `active` field reflects Chrome's authoritative activation state, not the positional index in `/json/list`

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Replace position-based active detection (`i == 0` in `execute_list`) with a reliable Chrome state query that reflects the actual activated target | Must |
| FR2 | Ensure `--background` re-activation via HTTP `/json/activate` propagates before `execute_create` returns, verified via CDP `document.visibilityState` so that subsequent `tabs list` invocations observe the correct state | Must |
| FR3 | Non-background `tabs create` behavior must remain unchanged — the new tab should become active | Should |

---

## Out of Scope

- Changes to `tabs activate` (separate issue #122)
- Changes to `tabs close`
- Changes to Chrome's CDP behavior (we can only work around it)
- Refactoring polling mechanisms for other commands

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
