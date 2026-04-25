# Defect Report: Fix connect --launch deleting temporary Chrome profile on detach

**Issue**: #265
**Date**: 2026-04-24
**Status**: Investigating
**Author**: Codex
**Severity**: High
**Related Spec**: `specs/feature-chrome-instance-discovery-and-launch/`

---

## Reproduction

### Steps to Reproduce

1. Run `agentchrome connect --launch`.
2. Run a follow-up command that reuses the persisted session, such as `agentchrome tabs list` or `agentchrome navigate https://duckduckgo.com`.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS |
| **Version / Commit** | agentchrome 1.51.1 / `62ba187` |
| **Browser / Runtime** | Google Chrome launched by AgentChrome |
| **Configuration** | Default launch path with internally managed temporary user data directory |

### Frequency

Always in the observed local workflow.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `connect --launch` returns connection info and the launched Chrome remains reachable for subsequent AgentChrome commands. |
| **Actual** | `connect --launch` returns a PID and WebSocket URL, but Chrome exits immediately. The next command reports a stale session / `chrome_terminated` error. |

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Detached launch preserves temporary profile

**Given** Chrome was launched with an internally managed temporary user data directory
**When** AgentChrome detaches the launch handle after Chrome becomes ready
**Then** the temporary user data directory is not deleted by the launcher process

### AC2: Follow-up commands can reuse launched session

**Given** `agentchrome connect --launch --headless` succeeds
**When** the user runs a follow-up command in a new AgentChrome invocation
**Then** the persisted session remains reachable
**And** the follow-up command exits successfully

### AC3: Error cleanup still removes temporary profile

**Given** AgentChrome creates a temporary user data directory while launching Chrome
**When** the launch process is not detached
**Then** dropping the temporary directory owner still removes the directory

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `ChromeProcess::detach()` must prevent cleanup of the temporary user data directory that the detached Chrome process still needs. | Must |
| FR2 | Chrome must launch in an independent process session / process group so process-group cleanup by the caller does not terminate the detached browser. | Must |
| FR3 | Existing cleanup-on-drop behavior must remain for non-detached launch handles and startup failure paths. | Must |
| FR4 | The fix must not change launch arguments, session JSON output, or typed exit-code behavior. | Must |

---

## Out of Scope

- Adding a user-facing option to preserve or delete temporary profiles.
- Changing session file schema or reconnect behavior.
- Changing Chrome executable discovery or launch argument ordering.

---

## Validation Checklist

- [x] Reproduction steps are repeatable and specific.
- [x] Expected vs actual behavior is clearly stated.
- [x] Severity is assessed.
- [x] Acceptance criteria use Given/When/Then format.
- [x] At least one regression scenario is included.
- [x] Fix scope is minimal.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #265 | 2026-04-24 | Initial defect report |
