# Defect Report: Iframe guidance advertises --frame command shapes the parser rejects

**Issue**: #286
**Date**: 2026-04-28
**Status**: Draft
**Author**: Codex (write-spec)
**Severity**: Medium
**Related Spec**: specs/feature-add-iframe-frame-targeting-support/

---

## Reproduction

### Steps to Reproduce

1. Build the debug binary with `cargo build`.
2. Launch Chrome and navigate to a page containing an iframe, such as `https://qaplayground.vercel.app/`.
3. Run `./target/debug/agentchrome examples strategies iframes --json`.
4. Observe that the iframe strategy advertises command strings such as `agentchrome page snapshot --frame N`.
5. Run `./target/debug/agentchrome --port <port> page snapshot --frame 1 --compact --pretty`.
6. Observe the parser error for the advertised `--frame` placement.
7. Run `./target/debug/agentchrome --port <port> page --frame 1 snapshot --compact --pretty`.
8. Observe that the group-scoped command shape succeeds.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS, local debug build |
| **Version / Commit** | Live debug build during April 28, 2026 regression |
| **Browser / Runtime** | AgentChrome-managed Chrome |
| **Configuration** | Iframe-capable page with a current AgentChrome session |

### Frequency

Always - the affected guidance strings advertise a static command shape that the live clap parser rejects.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | Built-in strategy guides, diagnose suggestions, command examples, help text, and generated man pages advertise only `--frame` command shapes that the current CLI parser accepts. |
| **Actual** | Some iframe guidance advertises subcommand-scoped forms such as `agentchrome page snapshot --frame N`, while `--frame` is parsed on the command group for `page`, causing `unexpected argument '--frame' found`. |

### Error Output

```json
{"error":"unexpected argument '--frame' found","code":1}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Strategy guide uses accepted command shapes

**Given** the AgentChrome binary is built
**When** `agentchrome examples strategies iframes --json` is run
**Then** every iframe strategy command that uses `--frame` parses successfully with the current CLI parser
**And** page-scoped commands use the accepted group-scoped form, for example `agentchrome page --frame 1 snapshot`

### AC2: Diagnose suggestions use accepted command shapes

**Given** the current page contains an iframe
**When** `agentchrome diagnose --current` is run
**Then** iframe-related challenge and pattern suggestions show concrete command examples that parse successfully
**And** no suggestion sends users to a rejected `page <subcommand> --frame <index>` command shape

### AC3: Help, examples, and man pages are consistent

**Given** the AgentChrome binary and generated man pages are available
**When** help text, built-in examples, and man pages are inspected for `page`, `dom`, `js`, `interact`, `form`, `media`, and `network` frame targeting guidance
**Then** each guidance surface uses the accepted `--frame` placement for that command surface
**And** the accepted placement is consistent within each surface

### AC4: Regression coverage validates parser acceptance

**Given** the fix is implemented
**When** BDD regression tests run for iframe strategy and diagnose guidance
**Then** the tests validate advertised frame command strings by parsing them
**And** existing successful frame targeting commands such as `page --frame 1 snapshot`, `form --frame 1 fill`, and `dom --frame 1 select` remain covered

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Correct iframe strategy guide command strings in `examples strategies iframes` to match the current parser. | Must |
| FR2 | Correct diagnose iframe challenge and pattern suggestions to use concrete accepted command shapes. | Must |
| FR3 | Correct built-in command examples and clap help examples that advertise rejected `--frame` placement; regenerate man pages from the corrected sources. | Must |
| FR4 | Add parser-validation regression coverage for advertised frame examples instead of relying only on text-presence assertions. | Must |
| FR5 | Preserve the existing accepted frame-targeting behavior and public parser contract unless the implementation intentionally supports both placements. | Must |
| FR6 | Document and test the canonical placement map: group-scoped `--frame` for `page`, `dom`, `js`, `interact`, `form`, and `media`; subcommand-scoped `--frame` for `network list`. | Should |

---

## Out of Scope

- Reworking frame targeting internals.
- Adding new iframe interaction capabilities.
- Changing non-frame command guidance except where it shares the same rejected `--frame` placement pattern.
- Broadening the parser to accept duplicate flag placements unless that is chosen deliberately during implementation and covered consistently.

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included
- [x] Fix scope is minimal - no feature work mixed in
- [x] Out of scope is defined

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #286 | 2026-04-28 | Initial defect report |
