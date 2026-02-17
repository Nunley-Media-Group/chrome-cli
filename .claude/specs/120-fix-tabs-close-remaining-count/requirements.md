# Defect Report: tabs close reports incorrect remaining count (off-by-one race condition)

**Issue**: #120
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude
**Severity**: Medium
**Related Spec**: `.claude/specs/7-tab-management/`

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. Create 3 additional tabs: `chrome-cli tabs create` (x3, for 4 total)
3. `chrome-cli tabs close <TAB_ID>` — observe `remaining: 4` (should be 3)
4. `chrome-cli tabs close <TAB_ID>` — observe `remaining: 3` (should be 2)

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 |
| **Version / Commit** | 0.1.0 (commit 112e231) |
| **Browser / Runtime** | HeadlessChrome/144.0.0.0 |
| **Configuration** | Default |

### Frequency

Intermittent (~80%+ of the time) — race condition depends on Chrome's HTTP endpoint propagation timing.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | After closing a tab, the `remaining` field in JSON output reflects the correct post-close tab count (e.g., 4 tabs → close 1 → remaining: 3) |
| **Actual** | The `remaining` field is off by 1 (too high), including the just-closed tab in the count because Chrome's `/json/list` HTTP endpoint hasn't propagated the closure yet |

### Error Output

```json
// After closing 1 of 4 tabs:
{"closed":["TARGET_ID"],"remaining":4}
// Expected:
{"closed":["TARGET_ID"],"remaining":3}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Remaining count is accurate after single close

**Given** a headless Chrome instance with 4 open tabs
**When** I close one tab via `chrome-cli tabs close <ID>`
**Then** the `remaining` field in the JSON output is `3`

**Example**:
- Given: 4 page-type tabs open in Chrome
- When: Close tab with a specific target ID
- Then: JSON output contains `"remaining": 3`

### AC2: Multiple sequential closes report correct counts

**Given** a headless Chrome instance with 4 open tabs
**When** I close 2 tabs sequentially via separate `chrome-cli tabs close` invocations
**Then** the remaining counts are `3` then `2` respectively

**Example**:
- Given: 4 page-type tabs open in Chrome
- When: Close first tab → output shows remaining: 3; Close second tab → output shows remaining: 2
- Then: Both counts accurately reflect post-close state

### AC3: Existing tab close behavior is preserved

**Given** a headless Chrome instance with 2 open tabs
**When** I close one tab via `chrome-cli tabs close <ID>`
**Then** the `closed` field contains the target ID and `remaining` is `1`
**And** the closed tab is no longer in `tabs list` output

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Add polling/retry after `Target.closeTarget` to wait for Chrome's `/json/list` HTTP endpoint to reflect the tab closure before querying the remaining count | Must |
| FR2 | The polling mechanism must match the existing pattern used in `execute_create()` (up to 10 retries, 10ms delay) for consistency | Should |

---

## Out of Scope

- Changes to tab creation logic (`execute_create`)
- Changes to the `Target.closeTarget` CDP command itself
- Refactoring the polling pattern into a shared utility (could be a follow-up)
- Changes to how the `closed` field is populated

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
