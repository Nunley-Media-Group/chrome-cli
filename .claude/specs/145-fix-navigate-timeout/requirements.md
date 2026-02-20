# Defect Report: navigate back/forward/reload ignores global --timeout option

**Issue**: #145
**Date**: 2026-02-19
**Status**: Draft
**Author**: Claude
**Severity**: Medium
**Related Spec**: `.claude/specs/144-fix-spa-same-document-navigate-timeout/`

---

## Reproduction

### Steps to Reproduce

1. Launch Chrome: `chrome-cli connect --launch`
2. Navigate to any page: `chrome-cli navigate https://example.com`
3. Navigate to another: `chrome-cli navigate https://example.com/about`
4. Run with explicit timeout: `CHROME_CLI_TIMEOUT=5000 chrome-cli navigate back`
5. If the navigation event is missed (e.g., SPA), observe the command still waits 30 seconds instead of 5 seconds

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 |
| **Version / Commit** | `dbbbfac` (v1.0.5) |
| **Browser / Runtime** | Chrome via CDP |
| **Configuration** | Default |

### Frequency

Always

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `navigate back`, `navigate forward`, and `navigate reload` should respect the `--timeout` flag and `CHROME_CLI_TIMEOUT` environment variable, using the specified timeout instead of the hardcoded 30 seconds |
| **Actual** | All three commands hardcode `DEFAULT_NAVIGATE_TIMEOUT_MS` (30,000ms) and ignore `global.timeout` entirely |

### Error Output

```
# No error — the command simply waits 30 seconds before timing out,
# ignoring the user-specified timeout value.
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: navigate back respects --timeout

**Given** a tab with navigation history
**When** I run `chrome-cli --timeout 5000 navigate back` and the navigation event is not detected
**Then** the command times out after approximately 5 seconds (not 30 seconds)

### AC2: navigate forward respects --timeout

**Given** a tab with forward navigation history
**When** I run `chrome-cli --timeout 5000 navigate forward` and the navigation event is not detected
**Then** the command times out after approximately 5 seconds (not 30 seconds)

### AC3: navigate reload respects --timeout

**Given** a tab with a loaded page
**When** I run `chrome-cli --timeout 5000 navigate reload` and the load event is not detected
**Then** the command times out after approximately 5 seconds (not 30 seconds)

### AC4: CHROME_CLI_TIMEOUT environment variable works for history navigation

**Given** `CHROME_CLI_TIMEOUT=5000` is set
**When** I run `chrome-cli navigate back` and the navigation event is not detected
**Then** the command times out after approximately 5 seconds

### AC5: Default timeout preserved when no override specified

**Given** no `--timeout` flag or `CHROME_CLI_TIMEOUT` is set
**When** I run `chrome-cli navigate back`
**Then** the command uses the default 30-second timeout

### AC6: navigate URL still respects per-command --timeout

**Given** the `navigate <URL>` subcommand has its own `--timeout` flag
**When** I run `chrome-cli navigate https://example.com --timeout 5000`
**Then** the per-command timeout is used (existing behavior preserved)

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `execute_back`, `execute_forward`, and `execute_reload` must use `global.timeout` when specified, falling back to `DEFAULT_NAVIGATE_TIMEOUT_MS` (30,000ms) otherwise | Must |
| FR2 | Existing default behavior (30s timeout) must be preserved when no timeout is specified | Must |
| FR3 | `execute_url` existing behavior (per-command `--timeout` flag) must be preserved without regression | Must |

---

## Out of Scope

- Changing the default timeout value (30 seconds)
- Adding per-subcommand `--timeout` flags to `navigate back`, `navigate forward`, or `navigate reload` (using the global `--timeout` is sufficient)
- Fixing the SPA same-document navigation timeout (covered in #144, already merged)

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC5, AC6)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
