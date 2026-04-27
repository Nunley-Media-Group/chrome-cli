# Defect Report: Fix same-document URL navigation waits for fragment-only navigations

**Issue**: #277
**Date**: 2026-04-27
**Status**: Draft
**Author**: Codex (write-spec)
**Severity**: High
**Related Spec**: specs/feature-url-navigation/

---

## Reproduction

### Steps to Reproduce

1. Build the debug binary with `cargo build`.
2. Launch headless Chrome with `./target/debug/agentchrome connect --launch --headless`.
3. Navigate to the base page: `./target/debug/agentchrome navigate https://qaplayground.vercel.app/ --wait-until load`.
4. Navigate to a fragment on the same document: `./target/debug/agentchrome navigate https://qaplayground.vercel.app/#S06 --wait-until load --timeout 5000`.
5. Compare with `./target/debug/agentchrome navigate https://qaplayground.vercel.app/#S07 --wait-until none`, which succeeds and returns the fragment URL.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 arm64 |
| **Version / Commit** | `agentchrome 1.56.1` / `756db61` |
| **Browser / Runtime** | AgentChrome-managed headless Chrome via CDP |
| **Configuration** | `--wait-until load` or `--wait-until domcontentloaded` on a same-document fragment navigation |

### Frequency

Always - direct URL navigation to a fragment on the current document updates the browser URL, but `load` and `domcontentloaded` wait strategies time out because Chrome does not emit new load-style events for fragment-only navigations.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `agentchrome navigate <same-document-fragment> --wait-until load` and `--wait-until domcontentloaded` complete successfully, exit 0, and return JSON containing the final fragment URL and title. |
| **Actual** | The browser completes the same-document navigation and updates `location.href`, but AgentChrome waits for `Page.loadEventFired` or `Page.domContentEventFired` until the timeout and exits 4. |

### Error Output

```json
{"error":"Navigation timed out after 5000ms waiting for Load","code":4}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Same-document URL navigate succeeds with load wait

**Given** Chrome is connected and the active tab is loaded at a same-document navigation page whose base URL exposes a `#S06` fragment target
**When** I run `agentchrome navigate <base-url>#S06 --wait-until load`
**Then** the command exits 0
**And** stdout contains JSON with `url` ending in `#S06`
**And** stdout contains a `title` field

### AC2: Same-document URL navigate succeeds with DOMContentLoaded wait

**Given** Chrome is connected and the active tab is loaded at a same-document navigation page whose base URL exposes a `#S07` fragment target
**When** I run `agentchrome navigate <base-url>#S07 --wait-until domcontentloaded`
**Then** the command exits 0
**And** stdout contains JSON with `url` ending in `#S07`
**And** stdout contains a `title` field

### AC3: Cross-document URL navigate still waits for load completion

**Given** Chrome is connected and the active tab can reach `https://example.com/`
**When** I run `agentchrome navigate https://example.com/ --wait-until load`
**Then** the command still waits for a cross-document load completion
**And** the command exits 0
**And** stdout contains JSON with the final URL, title, and status

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Direct URL navigation must recognize same-document completion via `Page.navigatedWithinDocument` instead of waiting only for load-style events. | Must |
| FR2 | `--wait-until load` and `--wait-until domcontentloaded` must complete successfully for fragment-only navigations after `Page.navigate` reports no error. | Must |
| FR3 | The shared URL navigation helper used by diagnose URL mode and script-runner navigate adapters must follow the same same-document completion semantics as the top-level `agentchrome navigate <url>` command. | Must |
| FR4 | Existing cross-document `load`, `domcontentloaded`, `networkidle`, and `none` behavior must be preserved. | Must |
| FR5 | Timeout behavior must remain unchanged when neither a load-style event nor a same-document navigation event arrives. | Must |

---

## Out of Scope

- Changing the default navigation timeout.
- Changing `navigate back` or `navigate forward`, which already subscribe to `Page.navigatedWithinDocument`.
- Adding a new wait strategy.
- Changing `--wait-for-selector` semantics after the primary navigation wait completes.
- Refactoring unrelated CDP event waiting in `interact`, `perf`, or `network` commands.

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
| #277 | 2026-04-27 | Initial defect report |
