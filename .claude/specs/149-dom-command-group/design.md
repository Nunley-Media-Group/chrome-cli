# Design: DOM Command Group

**Issue**: #149
**Date**: 2026-02-19
**Status**: Draft
**Author**: Claude (writing-specs)

---

## Overview

This design implements the `dom` command group — 14 subcommands for querying, navigating, styling, visualizing, and manipulating DOM elements via the Chrome DevTools Protocol. The implementation follows the established command module pattern: a `DomArgs`/`DomCommand` subcommand enum in `src/cli/mod.rs`, a new `src/dom.rs` command module with an `execute_dom` dispatcher, and CDP calls using the existing `CdpClient`/`ManagedSession` infrastructure.

The design reuses proven patterns from `src/page.rs` (session setup, DOM queries, output formatting), `src/form.rs` (UID resolution, `backendNodeId` targeting), and `src/snapshot.rs` (tree formatting). A new CDP domain (`CSS`) is introduced for `get-style`/`set-style` commands; all other subcommands use the existing `DOM` and `Runtime` domains.

Key architectural decisions: (1) node targeting supports raw `nodeId` integers, snapshot UIDs (`s1`), and CSS selectors — unified via a `resolve_node` helper; (2) `dom tree` outputs plain text (not JSON) by default, following the `page snapshot` precedent; (3) all JSON output uses the existing `print_output` helper with `--pretty`/`--plain` support.

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────────┐
│  CLI Layer (src/cli/mod.rs)                                  │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ DomArgs { command: DomCommand }                        │  │
│  │ DomCommand::Select | GetAttribute | GetText | GetHtml  │  │
│  │            SetAttribute | SetText | Remove             │  │
│  │            GetStyle | SetStyle                         │  │
│  │            Parent | Children | Siblings | Tree         │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────┬───────────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────────┐
│  Command Dispatch (src/main.rs)                              │
│  Command::Dom(args) => dom::execute_dom(&global, args).await │
└──────────────────────┬───────────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────────┐
│  Command Module (src/dom.rs)                                 │
│  ┌─────────────────┐  ┌─────────────────────────────────┐   │
│  │ execute_dom()    │  │ Helpers:                        │   │
│  │ ├─ select        │  │  resolve_node(target) → nodeId  │   │
│  │ ├─ get_attribute │  │  describe_element(nodeId) → El  │   │
│  │ ├─ get_text      │  │  get_document_root() → nodeId   │   │
│  │ ├─ get_html      │  │  format_tree(node, depth) → Str │   │
│  │ ├─ set_attribute │  │  setup_session(global)          │   │
│  │ ├─ set_text      │  └─────────────────────────────────┘   │
│  │ ├─ remove        │                                        │
│  │ ├─ get_style     │  ┌─────────────────────────────────┐   │
│  │ ├─ set_style     │  │ Output types:                   │   │
│  │ ├─ parent        │  │  DomElement (shared struct)     │   │
│  │ ├─ children      │  │  AttributeResult                │   │
│  │ ├─ siblings      │  │  TextResult, HtmlResult         │   │
│  │ └─ tree          │  │  MutationResult                 │   │
│  └─────────────────┘  │  StyleResult, StylePropertyResult│   │
│                        └─────────────────────────────────┘   │
└──────────────────────┬───────────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────────┐
│  CDP Client (src/cdp/client.rs)                              │
│  ManagedSession::send_command(method, params)                │
│  Domains: DOM, Runtime, CSS                                  │
└──────────────────────┬───────────────────────────────────────┘
                       │
                       ▼
                 Chrome Browser
```

### Data Flow

```
1. User runs: chrome-cli dom select "h1"
2. CLI layer parses args → DomArgs { command: Select(DomSelectArgs { selector: "h1" }) }
3. main.rs dispatches → dom::execute_dom(&global, args)
4. execute_dom matches Select → execute_select(global, select_args)
5. setup_session(global) → (CdpClient, ManagedSession)
6. ManagedSession.ensure_domain("DOM")
7. send_command("DOM.getDocument") → root nodeId
8. send_command("DOM.querySelectorAll", { nodeId, selector }) → nodeIds[]
9. For each nodeId: describe_element(nodeId) → DomElement struct
10. Serialize Vec<DomElement> → JSON stdout via print_output
```

---

## API / Interface Changes

### New Subcommands

| Subcommand | Arguments | Purpose |
|------------|-----------|---------|
| `dom select <selector>` | `--xpath` flag | Query elements by CSS selector or XPath |
| `dom get-attribute <node-id> <name>` | | Read a single attribute |
| `dom get-text <node-id>` | | Read textContent |
| `dom get-html <node-id>` | | Read outerHTML |
| `dom set-attribute <node-id> <name> <value>` | | Set an attribute |
| `dom set-text <node-id> <text>` | | Set textContent |
| `dom remove <node-id>` | | Remove element from DOM |
| `dom get-style <node-id> [property]` | | Read computed CSS styles (all or one) |
| `dom set-style <node-id> <css-text>` | | Set inline style attribute |
| `dom parent <node-id>` | | Navigate to parent element |
| `dom children <node-id>` | | List direct child elements |
| `dom siblings <node-id>` | | List sibling elements |
| `dom tree` | `--depth`, `--root` | Pretty-print DOM tree |

### Node ID Resolution

All `<node-id>` arguments accept three formats, unified by `resolve_node()`:

| Format | Example | Resolution |
|--------|---------|------------|
| Integer | `42` | Treated as stable `backendNodeId`; resolved via `DOM.resolveNode(backendNodeId)` → `DOM.requestNode(objectId)` → session `nodeId` |
| UID | `s3` | Resolved via snapshot state `uid_map` → `backendNodeId` → same `DOM.resolveNode` → `DOM.requestNode` pipeline |
| CSS selector | `css:h1.title` | Resolved via `DOM.getDocument` → `DOM.querySelector` → `nodeId` |

**Note:** All user-facing `nodeId` values in output are `backendNodeId` (stable across sessions). Integer inputs are also interpreted as `backendNodeId`. This ensures node IDs can be used across separate CLI invocations.

### Output Schemas

#### DomElement (shared across select, parent, children, siblings)

```json
{
  "nodeId": 42,
  "tag": "h1",
  "attributes": { "class": "title", "id": "main-heading" },
  "textContent": "Example Domain"
}
```

#### get-attribute

```json
{ "attribute": "href", "value": "https://example.com" }
```

#### get-text

```json
{ "textContent": "Example Domain" }
```

#### get-html

```json
{ "outerHTML": "<h1>Example Domain</h1>" }
```

#### set-attribute

```json
{ "success": true, "nodeId": 42, "attribute": "class", "value": "new-class" }
```

#### set-text

```json
{ "success": true, "nodeId": 42, "textContent": "new text" }
```

#### remove

```json
{ "success": true, "nodeId": 42, "removed": true }
```

#### set-style

```json
{ "success": true, "nodeId": 42, "style": "color: red; font-weight: bold" }
```

#### get-style (all)

```json
{ "styles": { "display": "block", "color": "rgb(0, 0, 0)", "font-size": "32px" } }
```

#### get-style (single property)

```json
{ "property": "display", "value": "block" }
```

#### tree (plain text, not JSON)

```
html
  head
    title "Example Domain"
  body
    div
      h1 "Example Domain"
      p "This domain is for use..."
      p
        a[href] "More information..."
```

### Error Responses

| Condition | Error Message | Exit Code |
|-----------|--------------|-----------|
| Invalid/stale nodeId | `"Node not found: <id>"` | 3 (TargetError) |
| UID not in snapshot | `"UID 's5' not found. Run 'chrome-cli page snapshot' first."` | 1 (GeneralError) |
| No snapshot state | `"No snapshot state found. Run 'chrome-cli page snapshot' first..."` | 1 (GeneralError) |
| CSS selector no match | `"Element not found for selector: <sel>"` | 1 (GeneralError) |
| Attribute not found | `"Attribute '<name>' not found on node <id>"` | 1 (GeneralError) |
| Parent of root | `"Element has no parent (document root)"` | 3 (TargetError) |
| CDP protocol error | Passthrough from CdpError | 5 (ProtocolError) |

---

## Implementation Details

### File: `src/dom.rs` (Create)

Primary command module. Follows the `page.rs` pattern.

**Session setup**: Reuse the `setup_session` pattern (resolve connection → resolve target → CDP connect → create session → apply emulate state).

**Core helpers**:

```
resolve_node(session, target: &str) → Result<ResolvedNode, AppError>
```
Unified node resolution: parses integer, checks `is_uid()`, checks `css:` prefix. Returns a `ResolvedNode` with both `node_id` (session-scoped for CDP calls) and `backend_node_id` (stable for output). For integers and UIDs, uses `push_backend_node_to_frontend()` which calls `DOM.resolveNode(backendNodeId)` → `DOM.requestNode(objectId)` to get a tracked session nodeId. For CSS selectors, uses `DOM.querySelector` and then `get_backend_node_id` for the stable output ID.

```
describe_element(session, node_id: i64) → Result<DomElement, AppError>
```
Calls `DOM.describeNode(nodeId)` to get tag, attributes. Calls `Runtime.callFunctionOn` with the resolved `objectId` to get `textContent`. Returns a `DomElement` output struct.

```
get_document_root(session) → Result<i64, AppError>
```
Calls `DOM.getDocument` and returns the root `nodeId`.

**CDP method mapping per subcommand**:

| Subcommand | CDP Method(s) | Notes |
|------------|---------------|-------|
| `select` (CSS) | `DOM.getDocument` → `DOM.querySelectorAll` → `DOM.describeNode` (per node) | Returns `nodeIds[]`, describe each |
| `select --xpath` | `DOM.performSearch` → `DOM.getSearchResults` → `DOM.describeNode` (per node) | XPath via search API; call `DOM.discardSearchResults` after |
| `get-attribute` | `DOM.getAttributes(nodeId)` | Returns flat `[name, value, name, value, ...]` array; find by name |
| `get-text` | `DOM.resolveNode(nodeId)` → `Runtime.callFunctionOn(objectId, "function() { return this.textContent; }")` | Resolve to JS object, read property |
| `get-html` | `DOM.getOuterHTML(nodeId)` | Direct CDP method |
| `set-attribute` | `DOM.setAttributeValue(nodeId, name, value)` | Direct CDP method |
| `set-text` | `DOM.resolveNode(nodeId)` → `Runtime.callFunctionOn(objectId, "function() { this.textContent = '<text>'; }")` | Set via JS on resolved object |
| `remove` | `DOM.removeNode(nodeId)` | Direct CDP method |
| `get-style` | `CSS.getComputedStyleForNode(nodeId)` | Enable CSS domain first; returns `computedStyle[]` with `{name, value}` entries |
| `set-style` | `DOM.setAttributeValue(nodeId, "style", css_text)` | Sets inline style via attribute |
| `parent` | `DOM.describeNode(nodeId)` → read `parentId` → `describe_element(parentId)` | Error if parentId is 0 or missing |
| `children` | `DOM.describeNode(nodeId, depth: 1)` → iterate `node.children[]` | Filter to element nodes (nodeType 1) |
| `siblings` | `DOM.describeNode(nodeId)` → get `parentId` → `DOM.describeNode(parentId, depth: 1)` → filter children excluding self | Compose parent + children |
| `tree` | `DOM.getDocument(depth: N)` → recursive walk | Custom text formatter; `--root` resolves via `resolve_node` first |

### File: `src/cli/mod.rs` (Modify)

Replace the bare `Dom` variant with `Dom(DomArgs)`. Add:

- `DomArgs` struct with `#[command(subcommand)] pub command: DomCommand`
- `DomCommand` enum with 13 variants (Select, GetAttribute, GetText, GetHtml, SetAttribute, SetText, Remove, GetStyle, SetStyle, Parent, Children, Siblings, Tree)
- Per-variant args structs: `DomSelectArgs`, `DomGetAttributeArgs`, `DomSetAttributeArgs`, `DomGetStyleArgs`, `DomSetStyleArgs`, `DomTreeArgs`, `DomNodeIdArgs` (shared by get-text, get-html, remove, parent, children, siblings)
- Update `Dom` variant help text: replace "not yet implemented" with full subcommand listing and examples

### File: `src/main.rs` (Modify)

- Add `mod dom;` declaration
- Change dispatch: `Command::Dom => Err(...)` → `Command::Dom(args) => dom::execute_dom(&global, args).await`

### File: `src/examples.rs` (Modify)

Replace the placeholder `dom` examples with working subcommand examples covering select, get-attribute, get-text, set-attribute, tree.

### File: `src/error.rs` (Modify)

Add error constructors:
- `node_not_found(id)` — for invalid/stale nodeId (exit code 3)
- `attribute_not_found(name, node_id)` — for missing attributes (exit code 1)
- `no_parent(node_id)` — for parent of root element (exit code 3)

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Raw JS execution wrapper** | Implement dom commands as thin wrappers around `Runtime.evaluate` with generated JS | Simple, fewer CDP methods | Fragile, no structured nodeId output, can't reuse node references | Rejected — doesn't provide stable node handles |
| **B: Direct CDP DOM methods** | Map each subcommand to native CDP DOM/CSS domain methods | Structured, type-safe, stable node IDs, reuses CDP session | More CDP methods to orchestrate, CSS domain enablement needed | **Selected** |
| **C: Hybrid (CDP for queries, JS for mutations)** | Use CDP for reads, JS for writes | Simpler mutation code | Inconsistent error handling, harder to validate mutations | Rejected — uniformity is worth the CDP complexity |

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Command module | BDD/Acceptance | All 22 ACs as Gherkin scenarios |
| Node resolution | Unit | `resolve_node` with integer, UID, CSS selector, and invalid inputs |
| Tree formatting | Unit | `format_tree` with depth limits, various DOM structures |
| Output structs | Unit | Serialization of `DomElement`, `AttributeResult`, etc. |
| CLI parsing | Unit | `DomCommand` parsing with all subcommands and flags |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `nodeId` values become stale between invocations (DOM updates invalidate IDs) | High | Medium | Document this behavior in help text; recommend re-running `dom select` to get fresh IDs. Each dom subcommand calls `DOM.getDocument` first which refreshes the DOM tree. |
| CSS domain not enabled by default, `get-style` fails | Medium | Low | Call `managed.ensure_domain("CSS")` before style commands, matching the `ensure_domain("Accessibility")` pattern in `page snapshot` |
| XPath via `DOM.performSearch` is deprecated in newer CDP versions | Low | Medium | Document as alternative to CSS selectors; primary path is CSS. Monitor CDP changelogs. |
| Large DOM trees cause slow/huge `dom tree` output | Medium | Low | `--depth` flag limits traversal; text content truncated to 60 chars per node |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] No state management changes (reuses existing snapshot state)
- [x] Security considerations: no new attack surface (local CDP only)
- [x] Performance impact analyzed (per-node describe calls may be slow for large result sets)
- [x] Testing strategy defined
- [x] Alternatives considered and documented
- [x] Risks identified with mitigations
