# Root Cause Analysis: Fix same-document URL navigation waits for fragment-only navigations

**Issue**: #277
**Date**: 2026-04-27
**Status**: Draft
**Author**: Codex (write-spec)

---

## Root Cause

`src/navigate.rs::execute_url` subscribes to one load-style CDP event before sending `Page.navigate`: `Page.loadEventFired` for `--wait-until load` or `Page.domContentEventFired` for `--wait-until domcontentloaded`. After `Page.navigate` succeeds, the command waits exclusively on that event receiver. This is correct for cross-document navigations, but Chrome treats fragment-only URL changes on the current page as same-document navigations.

For a same-document fragment navigation, Chrome updates `location.href` and emits `Page.navigatedWithinDocument`; it does not emit a fresh `Page.loadEventFired` or `Page.domContentEventFired`. The command therefore reports a timeout even though the browser state has already reached the requested URL. The existing history navigation code already solved the equivalent same-document problem for `navigate back` and `navigate forward` by waiting for either `Page.frameNavigated` or `Page.navigatedWithinDocument`.

The same load-style assumption also exists in `navigate_and_wait`, the shared helper used by diagnose URL mode and the script-runner navigate adapter. Fixing only the top-level `execute_url` path would leave an adjacent public command path with the same defect pattern.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/navigate.rs` | 72-147 | `execute_url` implements top-level `agentchrome navigate <url>` and waits only for the selected load-style event for `load` / `domcontentloaded`. |
| `src/navigate.rs` | 212-281 | `navigate_and_wait` provides shared URL navigation behavior for diagnose URL mode and script-runner navigate commands, with the same load-style event wait. |
| `src/navigate.rs` | 456-477 | `wait_for_event` waits for a single event receiver or times out. |
| `src/navigate.rs` | 479-514 | `wait_for_history_navigation` is the existing same-document-aware wait helper for history navigation. |
| `src/diagnose/mod.rs` | 49 | URL-mode diagnose calls `navigate_and_wait`, so it inherits URL navigation wait semantics. |
| `src/script/dispatch.rs` | 84 | Script-runner `navigate` dispatch reaches `src/navigate.rs::run_from_session`, which delegates URL mode to `navigate_and_wait`. |

### Triggering Conditions

- The active tab is already loaded at a document URL.
- The user runs direct URL navigation to the same document with a different fragment/hash.
- The wait strategy is `load` or `domcontentloaded`.
- `Page.navigate` succeeds and Chrome emits `Page.navigatedWithinDocument` instead of a new load-style event.
- AgentChrome waits only for `Page.loadEventFired` or `Page.domContentEventFired` and times out.

---

## Fix Strategy

### Approach

Add same-document completion handling to URL navigation waits for `load` and `domcontentloaded`. Before sending `Page.navigate`, subscribe to `Page.navigatedWithinDocument` in addition to the existing wait-strategy event. After `Page.navigate` succeeds, wait for whichever relevant completion signal arrives first: the selected load-style event for cross-document navigation, or `Page.navigatedWithinDocument` for fragment-only same-document navigation.

Keep `networkidle` and `none` behavior unchanged. `networkidle` already uses network request tracking and should still resolve through the existing idle timer; `none` should still return after initiating navigation. Preserve the existing timeout error path when no completion event arrives.

Apply the change through the shared URL navigation helper or a small URL-specific wait helper so the direct CLI path, diagnose URL mode, and script-runner navigate adapter share the same semantics. Avoid widening the history navigation helper unless doing so reduces duplication without changing `navigate back` / `forward` behavior.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/navigate.rs` | Add a URL-navigation wait helper that can wait for either the selected load-style event or `Page.navigatedWithinDocument`. | Models Chrome's two completion modes for direct URL navigation without changing public CLI syntax. |
| `src/navigate.rs` | Subscribe to `Page.navigatedWithinDocument` before `Page.navigate` for `load` and `domcontentloaded` URL waits. | Prevents missing the same-document event, which is ephemeral. |
| `src/navigate.rs` | Route both `execute_url` and `navigate_and_wait` through the same fixed wait path, or refactor `execute_url` to use `navigate_and_wait` while preserving `--ignore-cache` and `--wait-for-selector`. | Prevents duplicated URL navigation logic from diverging and leaves diagnose/script paths fixed too. |
| `tests/features/277-fix-same-document-url-navigation-waits-for-fragment-only-navigations.feature` | Add regression scenarios for fragment-only URL navigation with `load`, fragment-only URL navigation with `domcontentloaded`, and cross-document load preservation. | Ensures every acceptance criterion has BDD coverage. |
| `tests/bdd.rs` | Register the new feature file using the existing Chrome-dependent BDD convention. | Keeps the regression feature discoverable while allowing Chrome-dependent scenarios to be filtered if needed. |
| `tests/fixtures/same-document-url-navigation.html` | Add a deterministic fixture with stable fragment targets, unless existing fixture coverage already provides the same behavior. | Supports the Feature Exercise Gate without depending solely on a public website. |

### Blast Radius

- **Direct impact**: URL navigation wait handling in `src/navigate.rs`.
- **Indirect impact**: `agentchrome diagnose <url>` URL mode and script-runner `navigate` commands if they share `navigate_and_wait`.
- **Risk level**: Medium - the change touches central navigation waiting logic, but it is constrained to direct URL navigation and preserves existing wait strategies.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Cross-document `--wait-until load` returns early on an unrelated same-document event | Low | Subscribe immediately before `Page.navigate` and, if inspecting event params is practical, only treat `Page.navigatedWithinDocument` as completion when its URL matches the requested same-document destination or the final `location.href` has reached the requested URL. AC3 verifies cross-document load preservation. |
| `domcontentloaded` returns before the requested cross-document DOMContentLoaded event | Low | Keep the existing `Page.domContentEventFired` path for cross-document navigations and use the same-document event only as the alternate completion signal. |
| `navigate_and_wait` callers lose `status` behavior | Low | Preserve `Network.responseReceived` buffering and `extract_http_status`; same-document navigations may continue to omit `status` when no document response exists. |
| `--wait-for-selector` timeout budget changes | Low | Keep selector polling after the primary wait and continue using the original elapsed-time calculation from `execute_url`. |
| `networkidle` behavior changes unintentionally | Low | Do not alter `wait_for_network_idle` or request-tracking subscriptions for this defect. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Detect same-document URL before navigating and bypass waiting entirely | Compare the current URL and target URL, then skip load-style waits when only the fragment differs. | Risks semantic drift for URL normalization, redirects, and same-document changes that are not simple fragment edits; it also duplicates browser logic instead of using CDP's completion event. |
| Change `load` / `domcontentloaded` to always return immediately after `Page.navigate` | Avoids the timeout for fragments. | Breaks the documented cross-document wait contract from `specs/feature-url-navigation/`. |
| Reuse `wait_for_history_navigation` directly for URL navigation | Waits for `Page.frameNavigated` or `Page.navigatedWithinDocument`. | Direct URL navigation already has wait-strategy-specific events; a URL-specific helper can preserve `load` vs `domcontentloaded` semantics more clearly. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal - focused on direct URL navigation wait completion
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #277 | 2026-04-27 | Initial defect report |
