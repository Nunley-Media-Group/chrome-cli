# Design: Page Element Command

**Issues**: #165
**Date**: 2026-03-11
**Status**: Draft
**Author**: Claude

---

## Overview

The `page element` subcommand adds a targeted element query to the existing `page` command group. Given a UID (from a prior `page snapshot`) or CSS selector, it retrieves the element's accessibility role, name, HTML tag name, bounding box, accessibility properties, and viewport visibility — all in a single CDP round-trip-optimized call sequence.

The implementation follows the existing `page` subcommand pattern: a new `Element` variant in `PageCommand`, a `PageElementArgs` struct for CLI parsing, and an `execute_element()` async function in `page.rs`. Target resolution reuses the established UID-to-backendNodeId and CSS-to-backendNodeId patterns already present in `page.rs` (for screenshots) and `form.rs` (for fill/submit).

Three CDP calls retrieve the element data: `Accessibility.getPartialAXTree` for role/name/properties, `DOM.getBoxModel` for bounding box, and `DOM.describeNode` for tag name. A fourth call (`Runtime.evaluate`) fetches viewport dimensions for the `inViewport` computation.

---

## Architecture

### Component Diagram

```
CLI Layer (cli/mod.rs)
    │
    │  PageCommand::Element(PageElementArgs { target })
    ↓
Command Dispatch (page.rs :: execute_page)
    │
    │  match PageCommand::Element → execute_element()
    ↓
execute_element (page.rs)
    │
    ├─ 1. Resolve target → backendNodeId
    │     ├─ UID path: read_snapshot_state() → uid_map.get(target)
    │     └─ CSS path: DOM.getDocument → DOM.querySelector → DOM.describeNode
    │
    ├─ 2. Accessibility.getPartialAXTree(backendNodeId) → role, name, properties
    │
    ├─ 3. DOM.getBoxModel(backendNodeId) → bounding box
    │
    ├─ 4. DOM.describeNode(backendNodeId) → tagName
    │
    ├─ 5. Runtime.evaluate("window.innerWidth/innerHeight") → viewport dims
    │
    ├─ 6. Compute inViewport from bbox vs viewport
    │
    └─ 7. Serialize → JSON stdout (or plain text if --plain)
```

### Data Flow

```
1. User runs: agentchrome page element s10
2. CLI layer parses "s10" as PageElementArgs.target
3. execute_element() sets up CDP session (resolve_connection → create_session)
4. Target "s10" recognized as UID → read snapshot state from ~/.agentchrome/snapshot.json
5. uid_map["s10"] → backendNodeId (e.g., 42)
6. Three CDP calls with backendNodeId: getPartialAXTree, getBoxModel, describeNode
7. One CDP call for viewport: Runtime.evaluate
8. Response assembled into ElementInfo struct
9. Serialized as JSON to stdout, exit code 0
```

---

## API / Interface Changes

### New CLI Subcommand

| Command | Type | Purpose |
|---------|------|---------|
| `agentchrome page element <target>` | CLI subcommand | Query single element's state by UID or CSS selector |

### Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `target` | positional String | Yes | UID (`s1`, `s2`, ...) or CSS selector (`css:#id`, `css:.class`) |

### Output Schema

**Success (stdout, exit 0):**
```json
{
  "role": "button",
  "name": "Submit",
  "tagName": "BUTTON",
  "boundingBox": {
    "x": 100.0,
    "y": 200.0,
    "width": 150.0,
    "height": 40.0
  },
  "properties": {
    "enabled": true,
    "focused": false,
    "checked": null,
    "expanded": null,
    "required": false,
    "readonly": false
  },
  "inViewport": true
}
```

**Error (stderr, exit 3 — target not found):**
```json
{"error": "Element 's10' not found in DOM", "code": 3}
```

**Error (stderr, exit 1 — no snapshot state, UID target only):**
```json
{"error": "No snapshot state available. Run 'page snapshot' first.", "code": 1}
```

**Error (stderr, exit 3 — CSS selector no match):**
```json
{"error": "No element matches selector '#nonexistent'", "code": 3}
```

### Plain Text Output (--plain)

```
Role:       button
Name:       Submit
Tag:        BUTTON
Bounds:     100, 200, 150x40
In Viewport: yes
Enabled:    yes
Focused:    no
Checked:    n/a
Expanded:   n/a
Required:   no
Read-only:  no
```

---

## Implementation Details

### Target Resolution

Reuse the established UID/CSS resolution pattern. The `page.rs` module already contains UID resolution for `page screenshot` (resolving UIDs to `backendNodeId` via `snapshot::read_snapshot_state()`), and CSS selector resolution via `DOM.querySelector`. The `form.rs` module has `resolve_target_to_backend_node_id()` which handles both paths.

**Approach:** Implement a local `resolve_element_target()` in `page.rs` that follows the same two-path logic:
- **UID path**: `snapshot::read_snapshot_state()` → `uid_map.get(target)` → `backendNodeId`
- **CSS path**: `DOM.getDocument` → `DOM.querySelector(selector)` → `DOM.describeNode(nodeId)` → `backendNodeId`

This is preferred over importing from `form.rs` because:
1. The function in `form.rs` is private and tightly coupled to form-specific error messages
2. `page.rs` already has inline UID resolution for screenshots
3. The resolution logic is ~15 lines — not worth a cross-module dependency

### Accessibility Properties Extraction

`Accessibility.getPartialAXTree` returns an array of AX nodes. The first node is the target element. Properties are in `nodes[0].properties`, an array of `{ name, value }` objects.

Property mapping:
| AX Property Name | Output Field | Default (if absent) |
|------------------|-------------|---------------------|
| `disabled` | `enabled` (inverted) | `true` (enabled) |
| `focused` | `focused` | `false` |
| `checked` | `checked` | `null` (if role lacks checkable semantics) |
| `expanded` | `expanded` | `null` (if role lacks expandable semantics) |
| `required` | `required` | `false` |
| `readonly` | `readonly` | `false` |

**Null vs false logic for `checked` and `expanded`:** These properties are only semantically meaningful for certain roles. If the AX tree includes a `checked` or `expanded` property for the node (even if its value is `false`), output the value. If the property is entirely absent from the AX node's properties array, output `null`.

### Bounding Box Computation

Use `DOM.getBoxModel` with `backendNodeId` (not `nodeId` — per the lesson documented at `page.rs:711-713` about intermediate nodeIds not being anchored in the document tree).

The `content` quad from `DOM.getBoxModel` is `[x1, y1, x2, y2, x3, y3, x4, y4]` representing four corners. Compute:
```
x = content[0]
y = content[1]
width = content[4] - content[0]   // x3 - x1
height = content[5] - content[1]  // y3 - y1
```

### Viewport Visibility

Fetch viewport dimensions via `Runtime.evaluate`:
```javascript
JSON.stringify({ width: window.innerWidth, height: window.innerHeight })
```

Compute `inViewport`:
```
inViewport = (x + width > 0) && (x < viewportWidth) &&
             (y + height > 0) && (y < viewportHeight)
```

An element is considered in-viewport if any part of its bounding box overlaps the viewport rectangle `(0, 0, viewportWidth, viewportHeight)`.

### Tag Name Retrieval

Use `DOM.describeNode` with `backendNodeId` to get `node.nodeName` (the HTML tag name in uppercase, e.g., "BUTTON", "A", "INPUT").

### Error Handling

| Condition | Error Constructor | Exit Code |
|-----------|-------------------|-----------|
| No snapshot state (UID target) | `AppError::no_snapshot_state()` | 1 (GeneralError) |
| UID not in uid_map | New: `AppError::target_not_found(target)` | 3 (TargetError) |
| CSS selector matches nothing | New: `AppError::target_not_found(selector)` | 3 (TargetError) |
| backendNodeId exists but DOM node gone | `AppError::target_not_found(target)` | 3 (TargetError) |
| CDP protocol failure | `AppError::interaction_failed(action, reason)` | 5 (ProtocolError) |

**Note:** The existing `AppError::uid_not_found()` uses exit code 1. For this command, element-not-found scenarios should use exit code 3 (TargetError) since "target not found" is the more precise semantic. A new error variant or constructor may be needed to map to exit code 3.

### Output Struct

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ElementInfo {
    role: String,
    name: String,
    tag_name: String,
    bounding_box: BoundingBoxInfo,
    properties: ElementProperties,
    in_viewport: bool,
}

#[derive(Serialize)]
struct BoundingBoxInfo {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Serialize)]
struct ElementProperties {
    enabled: bool,
    focused: bool,
    checked: Option<bool>,
    expanded: Option<bool>,
    required: bool,
    readonly: bool,
}
```

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Full snapshot + filter** | Run `Accessibility.getFullAXTree` then filter to target | Reuses existing snapshot code | Defeats the purpose — still fetches full tree. No performance benefit. | Rejected — no token/latency savings |
| **B: Extract shared target resolution module** | Create `target.rs` with shared `resolve_target()` | DRY, consolidates form.rs/interact.rs/page.rs resolution | Adds new file, refactors existing modules, scope creep beyond #165 | Rejected — over-engineering for this issue |
| **C: Inline resolution in page.rs** | Add `resolve_element_target()` directly in page.rs | Minimal change, follows existing pattern (page.rs already has inline UID resolution for screenshots) | Some code similarity with form.rs | **Selected** — simplest approach, avoids cross-module coupling |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `DOM.getBoxModel` fails for elements with no layout (e.g., `display: none`) | Medium | Medium | Catch the CDP error and return a null/zero bounding box with `inViewport: false` |
| `Accessibility.getPartialAXTree` returns empty nodes array | Low | Medium | Fall back to role="none", name="" if AX tree returns no nodes |
| Stale snapshot state: UID maps to a backendNodeId that Chrome has invalidated | Medium | Low | The `DOM.getBoxModel`/`describeNode` call will fail; catch and return TargetError (exit 3) |

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #165 | 2026-03-11 | Initial feature spec |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] N/A — No database/storage changes
- [x] N/A — No state management changes (reads existing snapshot state)
- [x] N/A — No UI components (CLI tool)
- [x] Security considerations: N/A — local-only CDP, no user data stored
- [x] Performance impact analyzed: single-element query vs full tree fetch
- [x] Testing strategy: BDD scenarios for all ACs
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
