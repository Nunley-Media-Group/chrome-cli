# Design: Page Text Extraction

**Issue**: #9
**Date**: 2026-02-11
**Status**: Draft
**Author**: Claude (writing-specs)

---

## Overview

This feature adds the `page text` subcommand to extract visible text content from browser pages. It follows the established command patterns from `tabs.rs` and `navigate.rs`: resolve connection, resolve target, create CDP session, execute via `Runtime.evaluate`, and format output.

The implementation is straightforward — a single JavaScript expression (`document.body.innerText` or `querySelector(...).innerText`) executed in the page context via the existing CDP infrastructure. No new CDP domains or protocol features are needed beyond `Runtime.evaluate`, which is already used by `navigate.rs` for `get_page_info()`.

---

## Architecture

### Component Diagram

```
CLI Layer (cli/mod.rs)
  └── PageArgs → PageCommand::Text(PageTextArgs)
        ↓
Command Layer (page.rs)       ← NEW FILE
  └── execute_page() → execute_text()
        ↓
Connection Layer (connection.rs)  ← existing
  └── resolve_connection() → resolve_target() → ManagedSession
        ↓
CDP Layer (cdp/client.rs)     ← existing
  └── Runtime.evaluate({ expression: JS_SCRIPT })
        ↓
Chrome Browser
  └── Returns innerText result
```

### Data Flow

```
1. User runs: chrome-cli page text [--selector CSS] [--plain] [--tab ID]
2. CLI layer parses args into PageTextArgs
3. Command layer resolves connection and target tab
4. Creates CdpSession via Target.attachToTarget
5. Enables Runtime domain (via ManagedSession.ensure_domain)
6. Builds JavaScript expression:
   - No --selector:  document.body.innerText
   - With --selector: document.querySelector("CSS").innerText
7. Sends Runtime.evaluate with the expression
8. Also fetches URL and title (reuse get_page_info pattern)
9. Handles result:
   - Success → PageTextResult { text, url, title }
   - Null element → AppError (selector not found)
   - Exception → AppError (evaluation failed)
10. Formats output:
    - Default/--json → compact JSON
    - --pretty → pretty-printed JSON
    - --plain → raw text string only
```

---

## API / Interface Changes

### New CLI Commands

| Command | Purpose |
|---------|---------|
| `chrome-cli page text` | Extract visible text from current page |

### CLI Arguments (PageTextArgs)

| Argument | Type | Required | Default | Description |
|----------|------|----------|---------|-------------|
| `--selector <CSS>` | `Option<String>` | No | None (whole page) | CSS selector to target specific element |

Global flags `--tab`, `--json`, `--pretty`, `--plain`, `--timeout` all apply as usual.

### Output Schema

**JSON mode** (default, `--json`, `--pretty`):

```json
{
  "text": "Page heading\n\nParagraph text...",
  "url": "https://example.com/",
  "title": "Example Domain"
}
```

**Plain mode** (`--plain`):

```
Page heading

Paragraph text...
```

### Errors

| Condition | Error Message | Exit Code |
|-----------|---------------|-----------|
| Selector not found | `Element not found for selector: {selector}` | `GeneralError` (1) |
| JS evaluation error | `Text extraction failed: {description}` | `GeneralError` (1) |
| No connection | Existing `no_session` / `no_chrome_found` | `ConnectionError` (2) |
| Tab not found | Existing `target_not_found` | `TargetError` (3) |
| Timeout | Existing `command_timeout` from CDP layer | `TimeoutError` (4) |

---

## New Files and Modifications

### New Files

| File | Purpose |
|------|---------|
| `src/page.rs` | Page command implementation (execute_page, execute_text, output types, helpers) |

### Modified Files

| File | Change |
|------|--------|
| `src/cli/mod.rs` | Add `PageArgs`, `PageCommand`, `PageTextArgs` structs; change `Page` variant to `Page(PageArgs)` |
| `src/main.rs` | Add `mod page;` import; wire `Command::Page(args)` to `page::execute_page()` |
| `src/error.rs` | Add `element_not_found()` and `evaluation_failed()` helper constructors |

### No Changes Needed

| Component | Why |
|-----------|-----|
| `src/cdp/*` | `Runtime.evaluate` already works; no new CDP features needed |
| `src/connection.rs` | `resolve_connection`, `resolve_target`, `ManagedSession` all reusable as-is |
| `src/session.rs` | No session changes |
| `src/lib.rs` | No new public modules (page.rs is a binary-only module like tabs.rs and navigate.rs) |

---

## JavaScript Extraction Strategy

### Whole-page extraction (no --selector)

```javascript
document.body.innerText
```

`innerText` is the right choice because:
- It returns only **visible** text (excludes `display: none`, `<script>`, `<style>`)
- It preserves basic structure with newlines between block elements
- It's the standard DOM API, supported across all browsers
- It matches what a human would see on the page

### Selector-targeted extraction (--selector)

```javascript
(() => {
  const el = document.querySelector("CSS_SELECTOR");
  if (!el) return { __error: "not_found" };
  return el.innerText;
})()
```

Uses an IIFE to safely detect null elements and return a sentinel error object, distinguishing "element not found" from "element has empty text".

### Runtime.evaluate parameters

```json
{
  "expression": "<JS>",
  "returnByValue": true
}
```

Using `returnByValue: true` ensures we get the actual string value rather than a remote object reference. This is critical for large text content.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: `innerText`** | Use `element.innerText` | Excludes scripts/styles, preserves structure, simple | Slightly slower than textContent (triggers layout) | **Selected** |
| **B: `textContent`** | Use `element.textContent` | Fast, no layout needed | Includes script/style text, no structure | Rejected — violates AC6 |
| **C: Custom DOM walker** | Walk DOM tree, filter nodes | Full control over formatting | Complex JS, fragile, over-engineered | Rejected — unnecessary complexity |
| **D: Accessibility tree** | CDP `Accessibility.getFullAXTree` | Semantic structure | Separate issue (#10), different data shape | Rejected — out of scope |

---

## Security Considerations

- [x] **Input Validation**: CSS selector is passed into `querySelector()` which safely handles invalid selectors (returns null, no injection risk)
- [x] **No arbitrary JS**: The JavaScript executed is a fixed template, not user-provided code (the `js` command will handle arbitrary JS separately)
- [x] **Sensitive Data**: Text extraction may include sensitive page content — this is expected behavior for a CLI tool operating on localhost

---

## Performance Considerations

- [x] **Single CDP round-trip**: Text extraction + page info can be done in 2-3 `Runtime.evaluate` calls (text, URL, title)
- [x] **No DOM domain needed**: Only `Runtime` domain is required, which is lightweight
- [x] **`returnByValue: true`**: Avoids a second round-trip to fetch the remote object

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Output types | Unit | Serialization of `PageTextResult` (JSON fields, skip_serializing_if) |
| Error helpers | Unit | `element_not_found()`, `evaluation_failed()` produce correct messages/codes |
| Plain output | Unit | `--plain` outputs raw text without JSON |
| Selector IIFE | Unit | JavaScript expression construction with selector escaping |
| Feature | BDD (Gherkin) | All 10 acceptance criteria as scenarios |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `innerText` triggers layout, slow on complex pages | Low | Low | Bounded by `--timeout`; acceptable for CLI use |
| CSS selector with quotes breaks JS expression | Med | Med | Use IIFE with proper escaping (backslash-escape quotes in selector) |
| Page still loading when text extracted | Low | Med | User can combine with `navigate --wait-until load` first |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] No state management changes needed
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
