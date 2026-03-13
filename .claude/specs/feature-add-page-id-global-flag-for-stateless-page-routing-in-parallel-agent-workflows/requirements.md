# Requirements: Add --page-id Global Flag for Stateless Page Routing

**Issues**: #170
**Date**: 2026-03-12
**Status**: Draft
**Author**: Claude (SDLC)

---

## User Story

**As an** AI agent orchestrator running multiple parallel browser automation agents
**I want** to specify an explicit page target ID per command via a `--page-id` flag
**So that** parallel agents can each operate on their own page without conflicting over the shared session's active tab state

---

## Background

agentchrome's session file stores a single `active_tab_id` used as the default page target for all commands. When multiple agents run in parallel -- each responsible for a different page -- they clobber each other's active tab: Agent A activates tab-1, Agent B activates tab-2, and Agent A's next command silently targets the wrong page.

The existing `--tab` flag accepts a tab index or target ID but still participates in the shared session fallback chain. A new `--page-id` flag that accepts only a CDP target ID and bypasses the session entirely gives each parallel agent a stable, stateless routing mechanism.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Explicit page routing bypasses session state

**Given** a Chrome session with multiple open pages and a persisted `active_tab_id`
**When** a command is run with `--page-id <target-id>`
**Then** the command operates on exactly that page, ignoring the session's `active_tab_id`

**Example**:
- Given: Chrome has pages P1 and P2; session `active_tab_id` is set to P1's target ID
- When: `agentchrome page text --page-id <P2-target-id>`
- Then: The command returns text from P2, not P1

### AC2: Fallback chain preserved when --page-id is absent

**Given** a Chrome session with a persisted `active_tab_id` and no `--page-id` flag
**When** any command is run without `--page-id`
**Then** the existing fallback behavior applies: `--tab` -> session `active_tab_id` -> first page target

**Example**:
- Given: Chrome has pages P1 and P2; session `active_tab_id` is set to P2's target ID
- When: `agentchrome page text` (no `--page-id`, no `--tab`)
- Then: The command returns text from P2 via the session fallback

### AC3: Unknown page ID returns a target error

**Given** a Chrome session where the specified target ID does not exist
**When** a command is run with `--page-id <nonexistent-id>`
**Then** the command exits with code 3 (TargetError) and a structured JSON error on stderr containing the nonexistent ID

**Example**:
- Given: Chrome has pages P1 and P2, neither with ID "DOESNOTEXIST"
- When: `agentchrome page text --page-id DOESNOTEXIST`
- Then: Exit code 3, stderr: `{"error":"Tab 'DOESNOTEXIST' not found. Run 'agentchrome tabs list' to see available tabs.","code":3}`

### AC4: Parallel agents operate without session interference

**Given** two agents running simultaneously, each with a different `--page-id`
**When** Agent A uses `--page-id <page-A-id>` and Agent B uses `--page-id <page-B-id>`
**Then** each agent's commands target their respective pages independently with no cross-agent interference

**Example**:
- Given: Chrome has pages PA (url: site-A) and PB (url: site-B)
- When: Agent A runs `agentchrome page text --page-id <PA-id>` concurrently with Agent B running `agentchrome page text --page-id <PB-id>`
- Then: Agent A receives text from site-A and Agent B receives text from site-B

### AC5: --page-id and --tab are mutually exclusive

**Given** a command invocation that specifies both `--page-id` and `--tab`
**When** the CLI parses the arguments
**Then** the command exits with exit code 1 and a clear error message on stderr before any CDP connection is made

**Example**:
- Given: Any Chrome session state
- When: `agentchrome page text --page-id ABC123 --tab 0`
- Then: Exit code 1, stderr JSON error indicating the conflict between `--page-id` and `--tab`

### AC6: --page-id does not write to session state

**Given** a Chrome session with an existing `active_tab_id` in the session file
**When** a command is run with `--page-id <target-id>`
**Then** the session file's `active_tab_id` is not modified by the command

**Example**:
- Given: Session file has `active_tab_id` = "OLD_TAB_ID"
- When: `agentchrome page text --page-id NEW_PAGE_ID`
- Then: Session file still has `active_tab_id` = "OLD_TAB_ID"

### AC7: --page-id works with all commands that use target resolution

**Given** a Chrome session with a known page target
**When** any command that resolves a target (navigate, page, js, form, interact, console, network, emulate, perf, dialog, dom, cookie) is run with `--page-id <target-id>`
**Then** the command operates on the specified page

### Generated Gherkin Preview

```gherkin
Feature: Page ID global flag for stateless page routing
  As an AI agent orchestrator running multiple parallel browser automation agents
  I want to specify an explicit page target ID per command via a --page-id flag
  So that parallel agents can each operate on their own page without conflicting

  Scenario: Explicit page routing bypasses session state
    Given a Chrome session with multiple pages and a persisted active_tab_id
    When a command is run with --page-id targeting a non-active page
    Then the command operates on exactly the specified page

  Scenario: Fallback chain preserved when --page-id is absent
    Given a Chrome session with a persisted active_tab_id
    When a command is run without --page-id
    Then the existing fallback behavior applies

  Scenario: Unknown page ID returns a target error
    Given a Chrome session where the specified target ID does not exist
    When a command is run with --page-id using a nonexistent ID
    Then the command exits with code 3 and a structured JSON error

  Scenario: Parallel agents operate without session interference
    Given two page IDs targeting different pages
    When two commands run concurrently with different --page-id values
    Then each command targets its respective page independently

  Scenario: --page-id and --tab are mutually exclusive
    Given a command invocation with both --page-id and --tab
    When the CLI parses the arguments
    Then the command exits with code 1 and a conflict error message

  Scenario: --page-id does not write to session state
    Given a session file with an existing active_tab_id
    When a command is run with --page-id
    Then the session file active_tab_id remains unchanged

  Scenario: --page-id works with all target-resolving commands
    Given a Chrome session with a known page target
    When any target-resolving command is run with --page-id
    Then the command operates on the specified page
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Add `--page-id <target-id>` as a new global flag in `GlobalOpts` accepting a CDP target ID string | Must | Same `global = true` pattern as `--tab` |
| FR2 | When `--page-id` is provided, `resolve_target()` must look up the target ID directly in the target list, bypassing the session `active_tab_id` fallback entirely | Must | No session read for target resolution |
| FR3 | When `--page-id` is provided and the target doesn't exist in Chrome's target list, return `AppError::target_not_found()` with exit code 3 | Must | Reuses existing error constructor |
| FR4 | `--page-id` and `--tab` must be declared as a clap conflict group (mutually exclusive) | Must | Use `conflicts_with` attribute |
| FR5 | `--tab` retains all existing behavior unchanged | Must | No changes to `--tab` code paths |
| FR6 | When neither `--page-id` nor `--tab` is supplied, current fallback behavior is unchanged: session `active_tab_id` -> first page target | Must | |
| FR7 | All command modules that call `resolve_target()` must pass the new `page_id` parameter | Must | 13 modules: navigate, page, js, form, interact, console, network, emulate, perf, dialog, dom, cookie, tabs |
| FR8 | `--page-id` must not trigger any session file writes (no `active_tab_id` update) | Must | Stateless by design |
| FR9 | `--page-id` is exposed in shell completions and man pages via clap metadata | Should | Automatic via clap derive |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | No additional overhead when `--page-id` is used -- target lookup is a single linear scan of the target list (same as `--tab` by-ID lookup) |
| **Backward Compatibility** | All existing `--tab` behavior, session fallback, and no-flag defaults remain identical |
| **Platforms** | macOS, Linux, Windows -- same as all global flags |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--page-id` | String (CDP target ID) | Non-empty string; must match an existing target ID in Chrome's target list | No (optional global flag) |

### Output Data

No new output fields. Existing command outputs are unchanged. Error output follows the standard `{"error": "...", "code": N}` schema.

---

## Dependencies

### Internal Dependencies
- [x] `GlobalOpts` struct (`src/cli/mod.rs`) -- add new field
- [x] `resolve_target()` (`src/connection.rs`) -- add `page_id` parameter
- [x] `select_target()` (`src/connection.rs`) -- may need new variant for direct ID lookup

### External Dependencies
- None

### Blocked By
- None

---

## Out of Scope

- Named/scoped agent sessions (multiple session files)
- Replacing `--tab` with `--page-id`
- URL-pattern-based page routing
- Any change to `tabs activate` or session `active_tab_id` write behavior
- Environment variable support for `--page-id` (e.g., `AGENTCHROME_PAGE_ID`)
- Config file support for `page_id` default

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Parallel agent isolation | Zero cross-agent target interference | AC4 BDD test passes |
| Backward compatibility | All existing tests pass without modification | CI green |
| Error clarity | Nonexistent page ID produces actionable error | AC3 BDD test passes |

---

## Open Questions

(None -- all requirements are clear from the issue.)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #170 | 2026-03-12 | Initial feature spec |

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
