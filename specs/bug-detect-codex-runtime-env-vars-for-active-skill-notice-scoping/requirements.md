# Defect Report: Detect Codex runtime env vars for active skill-notice scoping

**Issue**: #278
**Date**: 2026-04-27
**Status**: Draft
**Author**: Codex (AI-assisted)
**Severity**: Medium
**Related Spec**: `specs/feature-add-agentchrome-skill-command-group/`

---

## Reproduction

### Steps to Reproduce

1. Run AgentChrome from a Codex session where `CODEX_CI`, `CODEX_MANAGED_BY_NPM`, or `CODEX_THREAD_ID` is present and `CODEX_HOME` is not set.
2. Ensure the Codex AgentChrome skill is stale or current at the temp Codex skill path, and ensure at least one unrelated tool skill such as `claude-code` is stale.
3. Run a successful command, for example `agentchrome capabilities --json` or an equivalent no-Chrome command used by the BDD harness.
4. Inspect stderr for stale-skill notices.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 arm64 observed; affected code is cross-platform CLI environment detection. |
| **Version / Commit** | Reported against AgentChrome 1.56.1, commit `756db61`; current branch `278-detect-codex-runtime-env-vars-for-active-skill-notice-scoping`. |
| **Browser / Runtime** | Codex session with `CODEX_CI`, `CODEX_MANAGED_BY_NPM`, and `CODEX_THREAD_ID`; no Chrome/CDP connection required. |
| **Configuration** | `CODEX_HOME` may be unset; supported AgentChrome skill files may exist under temp-home fixtures or user-level skill paths. |

### Frequency

Always when a Codex session exposes Codex runtime environment variables other than `CODEX_HOME`, the active Codex skill check depends on active-tool detection, and an inactive supported tool has a stale AgentChrome skill.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | AgentChrome recognizes a real Codex runtime from `CODEX_CI`, `CODEX_MANAGED_BY_NPM`, or `CODEX_THREAD_ID`, scopes stale-skill notices to Codex, and suppresses inactive-tool stale notices when the active Codex skill is current. |
| **Actual** | `detect_active_tool_with` recognizes Codex only when `CODEX_HOME` is non-empty. With `CODEX_HOME` unset, active-tool detection returns `None`, so `stale_tools_for_notice` falls back to the registry-wide stale scan and emits notices for inactive tools such as `claude-code`. |

### Error Output

```text
note: installed agentchrome skills for claude-code, codex are stale (oldest v1.51.2, binary v1.56.1) -- run 'agentchrome skill update' to refresh
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Codex Runtime Environment Variables Mark Codex Active

**Given** an AgentChrome command runs with a Codex runtime environment variable such as `CODEX_CI`, `CODEX_MANAGED_BY_NPM`, or `CODEX_THREAD_ID`
**And** `CODEX_HOME` is not set
**When** active-tool detection runs
**Then** the active tool is classified as `codex`.

### AC2: Current Active Codex Skill Suppresses Inactive Stale Notices

**Given** the active tool is detected as Codex from a Codex runtime environment variable
**And** the Codex AgentChrome skill is current
**And** another installed AgentChrome skill is stale
**When** any successful AgentChrome command runs
**Then** no stale-skill notice is emitted for the inactive stale tool.

### AC3: Stale Active Codex Notice Names Only Codex

**Given** the active tool is detected as Codex from a Codex runtime environment variable
**And** the Codex AgentChrome skill is stale
**And** another installed AgentChrome skill is also stale
**When** any successful AgentChrome command runs
**Then** exactly one stale-skill notice is emitted
**And** the notice names `codex`
**And** the notice does not name inactive stale tools.

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Active-tool detection MUST classify Codex as active when any known Codex runtime environment signal is present, including `CODEX_CI`, `CODEX_MANAGED_BY_NPM`, or `CODEX_THREAD_ID`. | Must |
| FR2 | Existing non-empty `CODEX_HOME` active-tool detection MUST continue to classify Codex as active. | Must |
| FR3 | Passive config-directory detection for install targeting, including `~/.codex`, MUST remain separate from active-tool detection and MUST NOT by itself make Codex active for stale-notice scoping. | Must |
| FR4 | When no active runtime signal is present, stale-skill notices MUST preserve the existing registry-wide all-tools fallback and aggregation behavior. | Must |
| FR5 | Existing active-tool priority MUST be preserved so higher-priority explicit runtime signals such as `CLAUDE_CODE` still win over Codex runtime signals when both are present. | Must |

---

## Out of Scope

- Changing stale-skill notice wording except where needed for correct active-tool scoping.
- Changing `agentchrome skill install`, `agentchrome skill update`, or `agentchrome skill uninstall` semantics.
- Suppressing stale-skill notices globally.
- Treating passive `~/.codex` directory existence as an active Codex session signal.
- Adding new supported agentic tools.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #278 | 2026-04-27 | Initial defect report |

---
