# Root Cause Analysis: navigate back/forward timeout on cross-origin history navigation

**Issue**: #72
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude

---

## Root Cause

The `execute_back()` and `execute_forward()` functions in `src/navigate.rs` subscribe to the `Page.loadEventFired` CDP event to detect when a history navigation has completed. After calling `Page.navigateToHistoryEntry`, they wait up to 30 seconds for this event.

Chrome's DevTools Protocol does **not reliably fire `Page.loadEventFired`** for cross-origin back/forward navigations. When the browser navigates to a history entry on a different origin, the page process is replaced (site isolation), and the load event is either not emitted or emitted on a different target. This causes the `wait_for_event` call to block until the 30-second timeout expires, making cross-origin back/forward navigation completely broken.

The `Page.frameNavigated` event, by contrast, fires reliably for all navigation types — including cross-origin back/forward. This event is already used successfully elsewhere in the codebase (e.g., `src/interact.rs` line 1359 for click-based navigation detection, `src/network.rs` line 504, `src/console.rs` line 407).

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/navigate.rs` | 231, 242 | `execute_back()` subscribes to `Page.loadEventFired` and waits for it |
| `src/navigate.rs` | 289, 298 | `execute_forward()` subscribes to `Page.loadEventFired` and waits for it |

### Triggering Conditions

- The browser history contains entries from **different origins** (e.g., `https://example.com` and `https://about.google`)
- The user runs `navigate back` or `navigate forward` to cross the origin boundary
- Chrome's site isolation causes the `Page.loadEventFired` event to not fire on the current CDP session target

---

## Fix Strategy

### Approach

Replace `Page.loadEventFired` with `Page.frameNavigated` in both `execute_back()` and `execute_forward()`. This is a two-line change (one subscription per function) plus updating the strategy label string passed to `wait_for_event` for accurate error messages.

The `wait_for_event()` helper function itself needs no changes — it is event-agnostic and works with any `CdpEvent` receiver.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/navigate.rs` line 231 | Change `subscribe("Page.loadEventFired")` to `subscribe("Page.frameNavigated")` | `Page.frameNavigated` fires for both same-origin and cross-origin navigations |
| `src/navigate.rs` line 242 | Change strategy label `"load"` to `"navigation"` | Accurate error message if timeout still occurs |
| `src/navigate.rs` line 289 | Change `subscribe("Page.loadEventFired")` to `subscribe("Page.frameNavigated")` | Same fix for forward navigation |
| `src/navigate.rs` line 298 | Change strategy label `"load"` to `"navigation"` | Accurate error message if timeout still occurs |

### Blast Radius

- **Direct impact**: `execute_back()` and `execute_forward()` in `src/navigate.rs`
- **Indirect impact**: None — the `wait_for_event()` helper is unchanged, and no other callers depend on the event choice in these two functions
- **Risk level**: Low — `Page.frameNavigated` is a superset of `Page.loadEventFired` for navigation detection purposes and is already proven in the codebase

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Same-origin back/forward breaks | Low | `Page.frameNavigated` fires for same-origin navigations too; existing test scenarios AC9/AC11 in `url-navigation.feature` cover this |
| `Page.frameNavigated` fires too early (before page is usable) | Low | The function calls `get_page_info()` immediately after, which fetches the current URL/title — this works regardless of full load completion |
| Timeout error message changes | Low | Strategy label change is cosmetic; no downstream code parses timeout error strings |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Composite wait (either `loadEventFired` or `frameNavigated`) | Wait for whichever fires first using `tokio::select!` | Unnecessary complexity — `frameNavigated` alone is sufficient and is the proven pattern in the codebase |
| Add a short fixed delay instead of event-based waiting | Sleep for a fixed duration after `navigateToHistoryEntry` | Unreliable — slow pages might not finish, fast pages waste time |

---

## Validation Checklist

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
