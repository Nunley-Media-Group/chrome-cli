# Design: Element Finding

**Issue**: #11
**Date**: 2026-02-12
**Status**: Draft
**Author**: Claude (writing-specs)

---

## Overview

The `page find` command adds element search to chrome-cli, enabling users and AI agents to locate elements by text query, CSS selector, or accessibility role. It builds on the existing snapshot infrastructure (issue #10), reusing `Accessibility.getFullAXTree` and the UID assignment system. For CSS selector searches, it uses CDP `DOM.querySelector`/`DOM.querySelectorAll`. Results include UIDs, roles, names, and bounding boxes for each matched element.

The command fits naturally into the existing `PageCommand` enum as a new `Find` variant, following the same session setup, domain enabling, and output formatting patterns used by `page text` and `page snapshot`.

---

## Architecture

### Component Diagram

```
CLI Layer (cli/mod.rs)
    │
    │  PageCommand::Find(PageFindArgs)
    ↓
Command Layer (page.rs)
    │
    │  execute_find()
    ↓
┌──────────────────────────────────────────────────────────────────┐
│                      Search Strategy                              │
│                                                                    │
│  ┌─────────────────────────┐   ┌──────────────────────────────┐  │
│  │  Accessibility Search   │   │   CSS Selector Search        │  │
│  │                         │   │                               │  │
│  │  Accessibility.          │   │  DOM.enable                  │  │
│  │    getFullAXTree        │   │  DOM.getDocument              │  │
│  │  build_tree() + filter  │   │  DOM.querySelectorAll         │  │
│  │  by name/role           │   │  Accessibility.               │  │
│  │                         │   │    getPartialAXTree (per node)│  │
│  └─────────────────────────┘   └──────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
    │
    │  Enrich with bounding boxes
    │  DOM.getBoxModel (per matched element)
    ↓
Output Layer (page.rs)
    │  FindResult → JSON / plain text
    ↓
stdout
```

### Data Flow

**Accessibility text search** (default path):
1. User runs `chrome-cli page find "Submit"`
2. CLI parses args into `PageFindArgs`
3. `execute_find()` sets up CDP session
4. Enables Accessibility, DOM, Runtime domains
5. Calls `Accessibility.getFullAXTree` → builds tree via `build_tree()`
6. Walks the `SnapshotNode` tree, filtering by name match (and optional role)
7. For each match, resolves `backendDOMNodeId` from uid_map
8. Calls `DOM.getBoxModel` for each match to get bounding box
9. Persists snapshot state (uid_map)
10. Serializes and outputs results

**CSS selector search** (when `--selector` is provided):
1. User runs `chrome-cli page find --selector "button.primary"`
2. Enables DOM domain
3. Calls `DOM.getDocument` to get root node
4. Calls `DOM.querySelectorAll` with the CSS selector
5. For each matched `nodeId`, calls `DOM.describeNode` to get `backendDOMNodeId`
6. Calls `Accessibility.getPartialAXTree` with the `backendDOMNodeId` to get role/name
7. Calls `DOM.getBoxModel` for bounding box
8. Also triggers full snapshot (for UID assignment) so UIDs are available
9. Serializes and outputs results

---

## API / Interface Changes

### New CLI Subcommand

```
chrome-cli page find [QUERY] [OPTIONS]

Arguments:
  [QUERY]    Text to search for (searches accessible names, text content, labels)

Options:
  --selector <CSS>   Find by CSS selector instead of text
  --role <ROLE>      Filter by accessibility role (button, link, textbox, etc.)
  --exact            Require exact text match (default: substring/case-insensitive)
  --limit <N>        Maximum results to return [default: 10]
```

Either `QUERY` or `--selector` must be provided (validated at runtime).

### Output Schema

**Success (JSON, default):**
```json
[
  {
    "uid": "s3",
    "role": "button",
    "name": "Submit",
    "boundingBox": {
      "x": 120,
      "y": 340,
      "width": 80,
      "height": 36
    }
  },
  {
    "uid": null,
    "role": "heading",
    "name": "Submit Your Application",
    "boundingBox": {
      "x": 50,
      "y": 100,
      "width": 300,
      "height": 32
    }
  }
]
```

**No matches:**
```json
[]
```

**Plain text output (`--plain`):**
```
[s3] button "Submit" (120,340 80x36)
heading "Submit Your Application" (50,100 300x36)
```

**Errors:**
```json
{"error": "CSS selector is invalid: unexpected token", "code": 5}
```

| Error Condition | Exit Code | Message |
|----------------|-----------|---------|
| Invalid CSS selector | 5 (ProtocolError) | CDP protocol error from DOM.querySelectorAll |
| Neither query nor --selector | 1 (GeneralError) | "either a text query or --selector is required" |
| Connection failure | 2 (ConnectionError) | Standard connection error |
| Snapshot failure | 1 (GeneralError) | "snapshot failed: ..." |

---

## State Management

### Snapshot State Reuse

The find command reuses the existing `SnapshotState` persistence from issue #10:

1. Every `page find` invocation triggers a fresh `Accessibility.getFullAXTree`
2. The snapshot tree is built via `build_tree()`, assigning UIDs
3. The `uid_map` is persisted to `~/.chrome-cli/snapshot.json`
4. This ensures subsequent interaction commands (future #14-#17) can reference UIDs from the find results

### New Types

```rust
/// A single element match from `page find`.
#[derive(Debug, Serialize)]
struct FindMatch {
    uid: Option<String>,
    role: String,
    name: String,
    #[serde(rename = "boundingBox")]
    bounding_box: Option<BoundingBox>,
}

/// Pixel-based bounding box of an element.
#[derive(Debug, Clone, Serialize)]
struct BoundingBox {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
```

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Search raw HTML** | Use DOM.performSearch to find text in HTML | Fast, handles raw content | Doesn't match accessible names, no role info | Rejected — doesn't align with accessibility-first approach |
| **B: Search accessibility tree** | Build snapshot, filter nodes by name/role | Consistent with snapshot UIDs, role filtering native | Requires full tree build per search | **Selected** — aligns with existing infrastructure |
| **C: Hybrid per-query** | Text search → AX tree, CSS → DOM methods | Best of both worlds | More complex, two code paths | **Selected for CSS** — CSS selectors need DOM methods |

---

## Security Considerations

- [x] **Input Validation**: CSS selectors are passed directly to CDP `DOM.querySelectorAll` — CDP handles validation and returns errors for invalid selectors
- [x] **No injection risk**: Selectors go via CDP JSON protocol, not embedded in JavaScript strings (unlike `page text --selector`)
- [x] **Local only**: All CDP communication is localhost

---

## Performance Considerations

- **Full tree build**: Each `page find` rebuilds the accessibility tree. This is acceptable since the tree is typically < 10,000 nodes and builds in < 100ms
- **Bounding box queries**: One `DOM.getBoxModel` call per matched element. With default limit of 10, this is at most 10 CDP round-trips
- **CSS selector path**: `DOM.querySelectorAll` is fast for typical selectors. The per-node `Accessibility.getPartialAXTree` calls add latency but are bounded by `--limit`
- **No caching**: Each invocation captures current page state, ensuring fresh results for dynamic content

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Snapshot search | Unit | Text matching logic (substring, exact, role filter) |
| Output formatting | Unit | FindMatch serialization, plain text formatting |
| BoundingBox | Unit | Serialization, null handling |
| CLI args | Unit | Argument validation (query vs selector requirement) |
| End-to-end | Integration (BDD) | Full command invocation with Chrome |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `DOM.getBoxModel` fails for invisible elements | Medium | Low | Return `null` bounding box, don't error |
| CSS selector search returns non-accessible elements | Low | Low | Still report role/name from AX tree, may be empty |
| Large pages slow down with many matches before limit | Low | Medium | Apply limit during tree walk, not after collecting all |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] N/A — No database/storage schema changes (reuses snapshot state)
- [x] State management approach is clear (reuse SnapshotState)
- [x] N/A — CLI tool, no UI components
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
