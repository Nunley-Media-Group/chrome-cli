# Defect Report: Fix frame auto selector targeting in DOM commands

**Issue**: #275
**Date**: 2026-04-27
**Status**: Draft
**Author**: Codex (write-spec)
**Severity**: Medium
**Related Spec**: specs/feature-add-iframe-frame-targeting-support/

---

## Reproduction

### Steps to Reproduce

1. Build the debug binary with `cargo build`.
2. Launch an isolated headless session: `./target/debug/agentchrome --timeout 20000 connect --launch --headless`.
3. Navigate to the iframe practice page: `./target/debug/agentchrome --timeout 20000 navigate https://the-internet.herokuapp.com/iframe --wait-until load`.
4. Confirm explicit frame targeting works: `./target/debug/agentchrome --timeout 20000 dom --frame 1 select body`.
5. Run selector-based auto targeting: `./target/debug/agentchrome --timeout 20000 dom --frame auto select body`.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS, local debug build |
| **Version / Commit** | `agentchrome 1.56.0` / `267f52c` |
| **Browser / Runtime** | AgentChrome-managed headless Chrome |
| **Test Site** | `https://the-internet.herokuapp.com/iframe` |

### Frequency

Always - selector-based `dom --frame auto select <selector>` fails before the selector is evaluated in child frames.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `dom --frame auto select body` searches the main frame and child frames in document order, finds the first matching `body` in a child frame when the main frame path is not the target, exits 0, and includes frame context in the output. |
| **Actual** | `dom --frame auto select body` resolves frame auto with an empty target hint, searches the accessibility UID path instead of selector-bearing frames, and exits 3 before running the DOM selector in each frame. |

### Error Output

```json
{"error":"Element not found in any frame. Use 'agentchrome page frames' to list available frames.","code":3}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Selector-based DOM auto frame targeting finds child-frame elements

**Given** a page contains a child iframe with a `body` element
**When** I run `agentchrome dom --frame auto select body`
**Then** AgentChrome searches child frames by selector
**And** the command exits 0
**And** the output returns the first matching element from the child frame with frame context included

### AC2: Explicit DOM frame targeting still works

**Given** the same iframe page
**When** I run `agentchrome dom --frame 1 select body`
**Then** the command continues to return the child frame body successfully
**And** the exit code is 0

### AC3: UID-based auto frame targeting still works

**Given** a snapshot has exposed an iframe-owned UID
**When** I run `agentchrome interact --frame auto click <uid>`
**Then** the command continues to resolve and act in the frame that owns the UID
**And** the exit code is 0

### AC4: Missing selector preserves target-error contract

**Given** no frame contains an element matching `css:#missing-auto-selector`
**When** I run `agentchrome dom --frame auto select css:#missing-auto-selector`
**Then** stderr contains the JSON error `{"error":"Element not found in any frame. Use 'agentchrome page frames' to list available frames.","code":3}`
**And** the process exits with code 3

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | `--frame auto` must support selector-based searches for DOM commands whose target argument is a CSS selector. | Must |
| FR2 | `dom select --frame auto <selector>` must pass the selector target hint into frame resolution instead of resolving auto frame with an empty target. | Must |
| FR3 | Selector auto-search must scan frame contexts in document order using the same frame resolution semantics as explicit numeric/path frame targeting. | Must |
| FR4 | The fix must preserve existing UID-based `--frame auto` behavior for `interact`, `form`, and other UID-targeted command paths. | Must |
| FR5 | Failure to find a selector in any frame must keep the existing JSON error shape and exit code 3. | Must |

---

## Out of Scope

- Adding `--frame auto` behavior to commands that have no target UID or selector argument, such as `page --frame auto text`.
- Changing the public frame indexing scheme or `page frames` output schema.
- Reworking cross-origin frame attachment beyond the existing `FrameContext` implementation.
- Refactoring all DOM subcommands unless they have a target selector/UID path that shares this exact defect.

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
| #275 | 2026-04-27 | Initial defect report |
