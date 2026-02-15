# Defect Report: form fill-many panics due to 'json' arg name collision with global --json flag

**Issue**: #84
**Date**: 2026-02-15
**Status**: Approved
**Author**: Claude
**Severity**: Critical
**Related Spec**: N/A

---

## Reproduction

### Steps to Reproduce

1. Launch Chrome: `chrome-cli connect --launch`
2. Navigate to a page: `chrome-cli navigate "https://www.google.com"`
3. Run: `chrome-cli form fill-many '[{"selector":"css:textarea","value":"test"}]' --pretty`

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS (Darwin 25.2.0) |
| **Version / Commit** | chrome-cli 0.1.0 (commit 01989d5) |
| **Browser / Runtime** | Chrome via CDP |
| **Configuration** | Default |

### Frequency

Always — 100% reproducible when inline JSON is passed as a positional argument.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | The command fills the specified form fields and returns a JSON result with exit code 0. |
| **Actual** | The process panics with exit code 101 (Rust panic). |

### Error Output

```
thread 'main' panicked at src/cli/mod.rs:127:15:
Mismatch between definition and access of `json`. Could not downcast to bool, need to downcast to alloc::string::String
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Bug Is Fixed

**Given** chrome-cli is built
**When** I run `chrome-cli form fill-many '[{"uid":"s1","value":"test"}]' --pretty`
**Then** the command does not panic
**And** the exit code is not 101

### AC2: --json output flag still works with fill-many

**Given** chrome-cli is built
**When** I run `chrome-cli form fill-many --help`
**Then** the help text shows the positional argument for inline JSON input
**And** the help text does not show a conflicting `--json` positional argument

### AC3: File-based input is not regressed

**Given** chrome-cli is built
**When** I run `chrome-cli form fill-many --help`
**Then** the help text shows `--file` as an option
**And** the help text shows `--include-snapshot` as an option

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Rename the positional `json` field in `FormFillManyArgs` to `input` to avoid collision with the global `--json` flag | Must |
| FR2 | Update all references to `args.json` in the form fill-many handler to use `args.input` | Must |

---

## Out of Scope

- Changing the global `--json` flag name or behavior
- Auditing other subcommands for similar naming collisions
- Refactoring beyond the minimal rename fix

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
