# Defect Report: tabs activate not reflected in subsequent tabs list

**Issue**: #122
**Date**: 2026-02-16
**Status**: Draft
**Author**: Claude
**Severity**: Medium
**Related Spec**: `.claude/specs/7-tab-management/`

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli tabs create https://google.com`
3. `chrome-cli tabs create https://example.com` — example.com is now active
4. `chrome-cli tabs activate <google_tab_id>` — reports success with google.com info
5. `chrome-cli tabs list --plain` — example.com still shows as active (`*`)

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 |
| **Version / Commit** | 0.1.0 (commit 112e231) |
| **Browser / Runtime** | HeadlessChrome/144.0.0.0 |
| **Configuration** | Default (headless mode) |

### Frequency

Intermittent (~80%+ of the time) — race condition depends on Chrome's `/json/list` HTTP endpoint propagation timing relative to the CDP `Target.activateTarget` command.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | After `tabs activate <ID>`, a subsequent `tabs list` shows the activated tab as active (first page-type target in `/json/list`) |
| **Actual** | `tabs activate` reports success, but `tabs list` still shows the previously active tab because Chrome's `/json/list` endpoint hasn't propagated the activation state change |

### Error Output

```
No error output — the command exits with code 0 but Chrome's HTTP endpoint
has not yet reflected the Target.activateTarget CDP command.
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Activate is reflected in tabs list

**Given** a headless Chrome instance with multiple open tabs
**When** I run `chrome-cli tabs activate <TAB_ID>` targeting a non-active tab
**And** I run `chrome-cli tabs list`
**Then** the activated tab shows as active in the list

**Example**:
- Given: 3 tabs open — about:blank, google.com, example.com (example.com is active)
- When: `chrome-cli tabs activate <google_tab_id>`
- Then: `chrome-cli tabs list` shows google.com as the active tab

### AC2: Activate returns correct tab info

**Given** a headless Chrome instance with multiple open tabs
**When** I run `chrome-cli tabs activate <TAB_ID>` targeting a specific tab
**Then** the JSON output includes the correct `activated`, `url`, and `title` fields for that tab

### AC3: Existing tab activation behavior is preserved

**Given** a headless Chrome instance with 2 open tabs
**When** I activate a tab that is already active
**Then** the command succeeds and `tabs list` still shows it as active

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Add polling/retry after `Target.activateTarget` to wait for Chrome's `/json/list` HTTP endpoint to reflect the activation before returning, matching the established pattern in `execute_create` and `execute_close` | Must |
| FR2 | The polling mechanism should use the same parameters as the `execute_create` background verification: up to 50 iterations with 10ms sleep, for a 500ms maximum budget | Should |

---

## Out of Scope

- Changes to tab creation or close polling logic
- Changes to the CDP `Target.activateTarget` command itself
- Refactoring the polling pattern into a shared utility (could be a follow-up)
- Changes to the `ActivateResult` output struct

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC3)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
