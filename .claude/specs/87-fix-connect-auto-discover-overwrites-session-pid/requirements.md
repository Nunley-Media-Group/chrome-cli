# Defect Report: Connect auto-discover overwrites session PID

**Issue**: #87
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude
**Severity**: High
**Related Spec**: `.claude/specs/session-connection-management/` — AC8

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --pretty` — output includes `"pid": 12345`
2. `chrome-cli --port <PORT> connect --pretty` — auto-discover overwrites session
3. `chrome-cli connect --disconnect --pretty` — output: `{"disconnected": true}` (no `killed_pid`)
4. Chrome is still running — the PID was lost in step 2

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (Darwin 25.2.0) |
| **Version / Commit** | chrome-cli 0.1.0 (commit 01989d5) |
| **Browser / Runtime** | Chrome via CDP |
| **Configuration** | Default (session stored in `~/.chrome-cli/session.json`) |

### Frequency

Always

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `connect` (auto-discover) preserves the existing PID in the session file when reconnecting to the same Chrome instance |
| **Actual** | `connect` (auto-discover) writes a new `SessionData` with `pid: None`, overwriting the PID stored by `--launch` |

### Error Output

```
# After step 3 — disconnect output is missing killed_pid:
{"disconnected": true}

# Expected:
{"disconnected": true, "killed_pid": 12345}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: PID is preserved across reconnections

**Given** Chrome was launched with `connect --launch` (which stores a PID in the session file)
**When** I run `connect` (auto-discover) on the same port
**Then** the session file retains the PID from the original launch

**Example**:
- Given: `chrome-cli connect --launch --headless` writes session with `pid: 54321`, `port: 9222`
- When: `chrome-cli --port 9222 connect` reconnects and overwrites session
- Then: session.json still contains `"pid": 54321`

### AC2: Disconnect kills launched Chrome after reconnection

**Given** Chrome was launched with `connect --launch` and subsequently reconnected with `connect` (auto-discover)
**When** I run `connect --disconnect`
**Then** the Chrome process is killed and `killed_pid` appears in the output

**Example**:
- Given: launch → reconnect (PID preserved)
- When: `chrome-cli connect --disconnect --pretty`
- Then: output includes `"killed_pid": 54321`

### AC3: PID is not injected when no prior session exists

**Given** no session file exists (`~/.chrome-cli/session.json` is absent)
**When** I run `connect` (auto-discover) to a running Chrome
**Then** the session file is written with `pid: null` (no spurious PID)

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | When writing a session file, read the existing session and preserve `pid` if the port matches | Must |
| FR2 | Do not inject a stale PID when the port has changed (different Chrome instance) | Must |

---

## Out of Scope

- PID tracking for externally launched Chrome instances (not launched via `--launch`)
- Refactoring session management beyond the minimal PID preservation fix
- Adding new CLI flags or options

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
