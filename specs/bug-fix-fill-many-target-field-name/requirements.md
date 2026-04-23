# Defect Report: form fill-many uses `uid` key while rest of form API uses `target`

**Issues**: #246
**Date**: 2026-04-23
**Status**: Draft
**Author**: Rich Nunley
**Severity**: Medium
**Related Spec**: `specs/feature-form-input-and-filling/`

---

## Reproduction

### Steps to Reproduce

1. Build agentchrome.
2. Run `form fill-many` with the same key name used by every other `form` subcommand:
   ```
   agentchrome form fill-many '[{"target":"css:#email","value":"a@b.com"}]'
   ```
3. Observe the command fails with an error that references a key (`uid`) that does not appear in the CLI help text or examples.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS / Linux / Windows |
| **Version / Commit** | branch `246-fix-fill-many-target-field-name` |
| **Configuration** | Any |

### Frequency

Always.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `fill-many` entries use the same `target` key used everywhere else in the `form` command family (e.g., `form fill <target> <value>`). |
| **Actual** | `fill-many` entries require a `uid` key. The deserialization error message also references `{uid, value}`, a key name that appears nowhere in the `--help` text or `agentchrome examples form` output. |

### Error Output

```
Invalid JSON: expected array of {uid, value} objects: missing field `uid` ...
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Bug Is Fixed — `target` is accepted

**Given** an array entry `{"target":"s5","value":"Alice"}`
**When** `form fill-many` parses the JSON payload
**Then** it deserializes successfully and treats the `target` value as the element identifier (same semantics as `form fill <target>`).

### AC2: No Regression — `uid` remains accepted

**Given** an array entry using the legacy `{"uid":"s5","value":"Alice"}` shape
**When** `form fill-many` parses the JSON payload
**Then** it still deserializes successfully, with no deprecation warning, so existing scripts keep working.

### AC3: Error Message and Help Updated

**Given** a malformed `fill-many` payload (e.g., missing the identifier field)
**When** the command errors, **or** a developer reads `form fill-many --help` or `examples form`
**Then** the error message, `--help` long_about, inline example, and the examples-data strategies reference `target` (not `uid`).

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `FillEntry` accepts `target` as the primary JSON field name used by `form fill-many`. | Must |
| FR2 | `FillEntry` continues to accept `uid` as a silent alias for backward compatibility (no deprecation warning). | Should |
| FR3 | The `Invalid JSON` error in `src/form.rs:783`, the `long_about` and `after_long_help` text for `FillMany` in `src/cli/mod.rs:2760-2771`, and any `fill-many` guidance in `src/examples/strategies.rs` use `target` consistently. | Must |

---

## Out of Scope

- Changing the semantics of what values the identifier accepts (UID like `s5` or `css:`-prefixed selector — unchanged).
- Any other `form` subcommand changes.
- Emitting a deprecation warning when `uid` is used.

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
