# Defect Report: Chrome launched via connect --launch missing --enable-automation flag

**Issue**: #70
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude
**Severity**: Medium

---

## Reproduction

### Steps to Reproduce

1. Run `chrome-cli connect --launch` (headed mode, no `--headless`)
2. Observe the Chrome window that opens

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (Darwin 25.2.0) |
| **Version / Commit** | chrome-cli v0.1.0, commit 02373b8 |
| **Browser / Runtime** | Google Chrome 144.x |
| **Configuration** | Default (no config file overrides) |

### Frequency

Always

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | Chrome displays the yellow "Chrome is being controlled by automated test software" infobar, confirming the CDP connection is active. The `--enable-automation` flag is present in Chrome's command-line arguments. |
| **Actual** | Chrome opens with no automation infobar. The window looks like a normal Chrome session. `--enable-automation` is absent from the launch arguments. |

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Automation flag is included on launch

**Given** a user runs `chrome-cli connect --launch` in headed mode
**When** Chrome is spawned
**Then** the `--enable-automation` flag is included in the Chrome command-line arguments

### AC2: Headless mode is unaffected

**Given** a user runs `chrome-cli connect --launch --headless`
**When** Chrome is spawned in headless mode
**Then** the `--enable-automation` flag is still passed and headless operation is unaffected

### AC3: Extra args do not duplicate the automation flag

**Given** a user passes `--chrome-arg=--enable-automation` explicitly
**When** Chrome is spawned
**Then** the flag appears in the arguments (no error from duplication, Chrome tolerates repeated flags)

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Pass `--enable-automation` when spawning Chrome via `launch_chrome()` | Must |
| FR2 | Existing launch behavior (port, user-data-dir, no-first-run, headless) remains unchanged | Must |

---

## Out of Scope

- Adding other automation flags (e.g., `--disable-popup-blocking`, `--disable-translate`)
- Making the `--enable-automation` flag configurable or optional
- Changes to the `connect` command's attach-to-existing-Chrome path

---

## Validation Checklist

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC2, AC3)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
