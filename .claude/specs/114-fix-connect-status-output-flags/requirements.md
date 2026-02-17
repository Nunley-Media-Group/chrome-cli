# Defect Report: connect --status ignores --pretty and --plain output format flags

**Issue**: #114
**Date**: 2026-02-16
**Status**: Draft
**Author**: Claude
**Severity**: Medium
**Related Spec**: `.claude/specs/6-session-and-connection-management/`

---

## Reproduction

### Steps to Reproduce

1. Launch headless Chrome: `chrome-cli connect --launch --headless`
2. Run `chrome-cli connect --status` — outputs compact JSON (correct)
3. Run `chrome-cli connect --status --pretty` — outputs compact JSON (incorrect)
4. Run `chrome-cli connect --status --plain` — outputs compact JSON (incorrect)

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 |
| **Version / Commit** | 0.1.0 (commit 112e231) |
| **Browser / Runtime** | HeadlessChrome/144.0.0.0 |
| **Configuration** | Default |

### Frequency

Always

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `--pretty` produces indented/pretty-printed JSON with newlines and spaces; `--plain` produces human-readable key-value text (not JSON); default (no flag) produces compact single-line JSON |
| **Actual** | All three invocations produce byte-identical compact single-line JSON output |

### Error Output

```
No error output — the command succeeds (exit 0) in all cases, but
the output format is always compact JSON regardless of the flag.
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Pretty flag produces indented JSON

**Given** a connected Chrome session
**When** I run `chrome-cli connect --status --pretty`
**Then** the output is valid indented JSON with newlines and spaces

### AC2: Plain flag produces human-readable text

**Given** a connected Chrome session
**When** I run `chrome-cli connect --status --plain`
**Then** the output is human-readable text (not JSON) showing connection details as key-value pairs

### AC3: Default output unchanged

**Given** a connected Chrome session
**When** I run `chrome-cli connect --status`
**Then** the output is compact single-line JSON (existing behavior preserved)

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `execute_status()` must respect the `OutputFormat` flags (`--pretty`, `--plain`) from `GlobalOpts` | Must |
| FR2 | Default output (no flag) must remain compact single-line JSON | Must |

---

## Out of Scope

- Changing the JSON schema of the status output
- Adding new fields to the status response
- Modifying output formatting for other `connect` subcommands (e.g., `connect`, `disconnect`)
- Refactoring `print_json()` callers beyond `execute_status()`

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
