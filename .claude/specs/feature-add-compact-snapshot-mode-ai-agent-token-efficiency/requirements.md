# Requirements: Compact Snapshot Mode for AI Agent Token Efficiency

**Issues**: #162
**Date**: 2026-03-16
**Status**: Draft
**Author**: Claude

---

## User Story

**As an** AI agent consuming agentchrome output
**I want** a compact snapshot mode that shows only interactive and semantically meaningful elements
**So that** I can understand page structure without consuming excessive context window tokens

---

## Background

The current `page snapshot` output includes every node in the accessibility tree: `InlineTextBox`, `LineBreak`, nested `generic` containers, and `StaticText` wrappers around text that's already shown in parent node names. For the SauceDemo inventory page, this produces ~120 lines of output where only ~30 lines contain actionable information (buttons, links, inputs, headings).

For an AI agent with a finite context window, this 4x overhead is significant. Every snapshot consumes tokens that could be used for reasoning. When automating a multi-page workflow (login -> browse -> add to cart -> checkout), the cumulative snapshot output can dominate the agent's context.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Compact snapshot returns only interactive and landmark elements

**Given** a connected Chrome session on a page with mixed interactive and decorative elements
**When** I run `agentchrome page snapshot --compact`
**Then** the output includes only interactive elements (those with UIDs), landmark/structural elements (headings, navigation, main, form, list), and their direct text content — excluding InlineTextBox, LineBreak, and purely decorative generic containers

**Example**:
- Given: Chrome connected to a page with buttons, links, headings, and many InlineTextBox/generic nodes
- When: `agentchrome page snapshot --compact`
- Then: Output contains button, link, heading, navigation nodes but not InlineTextBox, LineBreak, or generic containers with no interactive descendants

### AC2: Compact snapshot preserves hierarchy context

**Given** a connected Chrome session on a page with nested interactive elements
**When** I run `agentchrome page snapshot --compact`
**Then** interactive elements still show their nesting relationship (indentation) but intermediate generic containers are collapsed — ancestors that are kept (landmarks, structural) maintain correct depth

**Example**:
- Given: A page where a button is nested inside `main > generic > generic > form > generic > button`
- When: `agentchrome page snapshot --compact`
- Then: The button appears under `main > form > button` with correct indentation; intermediate generic containers are removed

### AC3: Compact snapshot reduces output size significantly

**Given** a connected Chrome session on the SauceDemo inventory page (a typical content-heavy page)
**When** I run `agentchrome page snapshot --compact`
**Then** the output is at least 50% smaller than the full `page snapshot` output while retaining all actionable information (all UIDs present in compact output match those in full output)

**Example**:
- Given: Chrome connected to `https://www.saucedemo.com/inventory.html` after login
- When: Full snapshot has ~120 lines; compact snapshot is run
- Then: Compact snapshot has <= 60 lines and every `[sN]` UID from the full snapshot appears in the compact output

### AC4: Full snapshot remains default — backward compatibility

**Given** a connected Chrome session
**When** I run `agentchrome page snapshot` without `--compact`
**Then** the output is identical to current behavior (full accessibility tree)

### AC5: Compact mode works with --include-snapshot on other commands

**Given** a connected Chrome session on a page with form fields
**When** I run a command with both `--include-snapshot` and `--compact` (e.g., `agentchrome form fill s5 "value" --include-snapshot --compact`)
**Then** the included snapshot in the response uses compact filtering

**Example**:
- Given: Chrome connected, form field s5 exists
- When: `agentchrome form fill s5 "hello" --include-snapshot --compact`
- Then: The `snapshot` field in JSON output contains only interactive and landmark elements, same as `page snapshot --compact`

### AC6: Compact mode preserves all interactive UIDs

**Given** a connected Chrome session on a page with interactive elements
**When** I run `agentchrome page snapshot --compact`
**Then** every UID that appears in the full snapshot also appears in the compact snapshot — no interactive elements are lost

**Example**:
- Given: Full snapshot assigns UIDs s1 through s15 to interactive elements
- When: Compact snapshot is run
- Then: All of s1 through s15 appear in the compact output

### AC7: Compact mode works with --verbose flag

**Given** a connected Chrome session
**When** I run `agentchrome page snapshot --compact --verbose`
**Then** the output includes compact-filtered nodes with their additional properties (checked, disabled, level, etc.)

### AC8: Compact mode works with JSON output formats

**Given** a connected Chrome session
**When** I run `agentchrome page snapshot --compact --json` or `--pretty`
**Then** the JSON output contains the filtered tree structure, with the same compact filtering applied

### Generated Gherkin Preview

```gherkin
Feature: Compact Snapshot Mode
  As an AI agent consuming agentchrome output
  I want a compact snapshot mode
  So that I can understand page structure without excessive token consumption

  Scenario: Compact snapshot returns only interactive and landmark elements
    Given a Chrome session connected to a page with mixed interactive and decorative elements
    When I run page snapshot with the compact flag
    Then the output includes only interactive and landmark elements
    And InlineTextBox, LineBreak, and decorative generic containers are excluded

  Scenario: Compact snapshot preserves hierarchy context
    Given a Chrome session connected to a page with nested interactive elements
    When I run page snapshot with the compact flag
    Then interactive elements show correct nesting
    And intermediate generic containers are collapsed

  Scenario: Compact snapshot reduces output size significantly
    Given a Chrome session connected to a content-heavy page
    When I run page snapshot with the compact flag
    Then the output is at least 50% smaller than the full snapshot
    And all UIDs from the full snapshot are present

  Scenario: Full snapshot remains default
    Given a Chrome session connected to a page
    When I run page snapshot without the compact flag
    Then the output is the full accessibility tree

  Scenario: Compact mode with --include-snapshot on other commands
    Given a Chrome session with form fields on the page
    When I run a form command with include-snapshot and compact flags
    Then the included snapshot uses compact filtering

  Scenario: Compact mode preserves all interactive UIDs
    Given a Chrome session on a page with interactive elements
    When I run page snapshot with and without the compact flag
    Then both outputs contain the same set of UIDs

  Scenario: Compact mode with verbose flag
    Given a Chrome session connected to a page
    When I run page snapshot with compact and verbose flags
    Then compact-filtered nodes include additional properties

  Scenario: Compact mode with JSON output
    Given a Chrome session connected to a page
    When I run page snapshot with compact and json flags
    Then the JSON output contains the compact-filtered tree
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Add `--compact` flag to `page snapshot` command via `PageSnapshotArgs` | Must | Boolean flag, defaults to false |
| FR2 | Compact mode includes: interactive roles (from `INTERACTIVE_ROLES` with UIDs), headings, landmarks (navigation, main, complementary, contentinfo, banner, form, region), list/table structures (list, listitem, table, row, cell) | Must | Leverage existing `INTERACTIVE_ROLES` constant |
| FR3 | Compact mode excludes: `InlineTextBox`, `LineBreak`, `StaticText` (when text already in parent name), purely decorative `generic` containers with no interactive descendants | Must | Tree-pruning approach |
| FR4 | Compact mode preserves text content by inlining it into parent node names when a `StaticText` child is the sole text source for a parent | Should | Prevents information loss |
| FR5 | Add `--compact` flag to all commands that support `--include-snapshot` (interact click, click-at, hover, drag, type, key, scroll; form fill, fill-many, clear, upload, submit) | Should | Consistent compact behavior across the CLI |
| FR6 | Default behavior (no `--compact` flag) is unchanged for all commands | Must | Backward compatibility |
| FR7 | Compact filtering is applied as a post-processing step on the built `SnapshotNode` tree, after UID assignment | Must | UIDs must be assigned from the full tree to maintain stable mapping |
| FR8 | Compact mode works correctly with `--verbose`, `--json`, `--pretty`, and `--file` flags | Must | All existing output modes supported |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Compact filtering adds < 5ms to snapshot processing time for trees up to 10,000 nodes |
| **Compatibility** | No changes to exit codes, JSON error format, or session state behavior |
| **Platforms** | Works on macOS, Linux, and Windows (same as existing snapshot) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--compact` | bool flag | No value needed | No (defaults to false) |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| Compact tree (plain) | Text | Hierarchical text with only interactive/landmark nodes |
| Compact tree (JSON) | JSON | `SnapshotNode` structure with pruned children arrays |
| UIDs | String (`sN`) | All interactive element UIDs preserved from full tree |

---

## Dependencies

### Internal Dependencies
- [x] `snapshot.rs` — `build_tree()`, `format_text()`, `SnapshotNode`, `INTERACTIVE_ROLES`
- [x] `cli/mod.rs` — `PageSnapshotArgs`, all `*Args` structs with `include_snapshot`
- [x] `page/snapshot.rs` — `execute_snapshot()`
- [x] `interact.rs`, `form.rs` — `take_snapshot()` and `--include-snapshot` handling

### External Dependencies
- None

### Blocked By
- None

---

## Out of Scope

- Changing the default snapshot format (this is opt-in via `--compact`)
- Adding `--depth` or `--max-nodes` truncation (could be a follow-up)
- Changing the snapshot format from tree text to JSON (separate concern)
- Making compact mode the default for any command
- Adding compact-specific JSON schema changes (the JSON structure remains `SnapshotNode`)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Output reduction | >= 50% fewer lines on SauceDemo inventory page | Compare line counts of `page snapshot` vs `page snapshot --compact` |
| UID preservation | 100% of UIDs retained | Compare UID sets between full and compact output |
| Performance overhead | < 5ms added latency | Benchmark compact filtering on 10K-node tree |

---

## Open Questions

- None — the issue provides clear direction on filtering strategy and expected output.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #162 | 2026-03-16 | Initial feature spec |

---

## Validation Checklist

Before moving to PLAN phase:

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
