# Design: Accessibility Tree Snapshot

**Issue**: #10
**Date**: 2026-02-12
**Status**: Draft
**Author**: Claude (writing-specs)

---

## Overview

This feature adds the `page snapshot` subcommand to capture the page's accessibility tree. It follows the established command pattern: resolve connection, resolve target, create CDP session, call CDP domain methods, format output. The key new capabilities are:

1. **Accessibility tree capture** via CDP `Accessibility.getFullAXTree`, which returns a flat list of accessibility nodes that we reconstruct into a tree.
2. **UID assignment** for interactive elements — short sequential IDs (`s1`, `s2`, ...) that AI agents use to reference elements in subsequent interaction commands.
3. **UID-to-backend-node mapping persistence** in a snapshot state file (`~/.chrome-cli/snapshot.json`), enabling interaction commands (#14-#17) to resolve UIDs back to DOM elements.
4. **Dual output formatting** — hierarchical text (default) and JSON tree.

The implementation builds directly on the CDP infrastructure from Issue #6 and follows the module patterns established by `page.rs` (text extraction) and `navigate.rs`.

---

## Architecture

### Component Diagram

```
CLI Layer (cli/mod.rs)
  └── PageArgs → PageCommand::Snapshot(PageSnapshotArgs)
        ↓
Command Layer (page.rs)
  └── execute_page() → execute_snapshot()
        ↓
Snapshot Module (snapshot.rs)        ← NEW FILE
  └── build_ax_tree() → assign_uids() → format_output()
        ↓
Connection Layer (connection.rs)     ← existing
  └── resolve_connection() → resolve_target() → ManagedSession
        ↓
CDP Layer (cdp/client.rs)            ← existing
  └── Accessibility.getFullAXTree
        ↓
Chrome Browser
  └── Returns accessibility node list

Session Layer (session.rs + snapshot.rs)
  └── Write uid mapping to ~/.chrome-cli/snapshot.json
```

### Data Flow

```
1. User runs: chrome-cli page snapshot [--verbose] [--file PATH] [--json] [--tab ID]
2. CLI layer parses args into PageSnapshotArgs
3. Command layer resolves connection and target tab (existing pattern)
4. Creates CdpSession via Target.attachToTarget
5. Enables Accessibility domain (via ManagedSession.ensure_domain)
6. Sends Accessibility.getFullAXTree → flat list of AXNode objects
7. Reconstructs tree from childIds / parentId relationships
8. Filters out ignored nodes (ignored: true)
9. Assigns UIDs to interactive elements in tree-traversal order:
   - Roles: link, button, textbox, checkbox, radio, combobox, menuitem,
     tab, switch, slider, spinbutton, searchbox, option, treeitem
   - Format: s1, s2, s3, ...
10. Persists {uid → backendDOMNodeId} mapping to ~/.chrome-cli/snapshot.json
11. Formats output:
    - Default/--plain: hierarchical text with "- role "name" [uid]"
    - --json/--pretty: JSON tree with uid, role, name, children
12. Writes to stdout or --file path
```

---

## API / Interface Changes

### New CLI Commands

| Command | Purpose |
|---------|---------|
| `chrome-cli page snapshot` | Capture accessibility tree of current page |

### CLI Arguments (PageSnapshotArgs)

| Argument | Type | Required | Default | Description |
|----------|------|----------|---------|-------------|
| `--verbose` | `bool` | No | false | Include additional element properties |
| `--file <PATH>` | `Option<PathBuf>` | No | None (stdout) | Save output to file instead of stdout |

Global flags `--tab`, `--json`, `--pretty`, `--plain`, `--timeout` all apply as usual.

Note: `--plain` and default (no format flag) both produce the structured text format. `--json` and `--pretty` produce JSON. This differs slightly from other commands where default is JSON — for `snapshot`, text is the natural default since the output is a tree visualization.

### Output Schema (Text mode — default)

```
- document "Example Domain"
  - heading "Example Domain" [s1]
  - paragraph ""
    - text "This domain is for use in..."
  - link "More information..." [s2]
```

With `--verbose`:
```
- document "Example Domain"
  - heading "Example Domain" [s1] level=1
  - paragraph ""
    - text "This domain is for use in..."
  - link "More information..." [s2] url="https://www.iana.org/domains/example"
```

### Output Schema (JSON mode)

```json
{
  "role": "document",
  "name": "Example Domain",
  "uid": null,
  "children": [
    {
      "role": "heading",
      "name": "Example Domain",
      "uid": "s1",
      "children": []
    },
    {
      "role": "link",
      "name": "More information...",
      "uid": "s2",
      "children": []
    }
  ]
}
```

With `--verbose`, each node gains a `properties` object:
```json
{
  "role": "heading",
  "name": "Example Domain",
  "uid": "s1",
  "properties": {"level": 1},
  "children": []
}
```

### Errors

| Condition | Error Message | Exit Code |
|-----------|---------------|-----------|
| Snapshot failed | `Accessibility tree capture failed: {description}` | `GeneralError` (1) |
| File write error | `Failed to write snapshot to file: {path}: {error}` | `GeneralError` (1) |
| Tree too large (truncated) | Not an error — appends truncation note to output | N/A |
| No connection | Existing `no_session` / `no_chrome_found` | `ConnectionError` (2) |
| Tab not found | Existing `target_not_found` | `TargetError` (3) |
| Timeout | Existing `command_timeout` from CDP layer | `TimeoutError` (4) |

---

## New Files and Modifications

### New Files

| File | Purpose |
|------|---------|
| `src/snapshot.rs` | Accessibility tree types, UID assignment, tree building, text/JSON formatting, snapshot state persistence |

### Modified Files

| File | Change |
|------|--------|
| `src/cli/mod.rs` | Add `PageSnapshotArgs` struct; add `Snapshot(PageSnapshotArgs)` variant to `PageCommand` |
| `src/page.rs` | Add `execute_snapshot()` handler; add dispatch arm in `execute_page()` |
| `src/error.rs` | Add `snapshot_failed()` and `file_write_failed()` error constructors |
| `src/main.rs` | Add `mod snapshot;` declaration |

### No Changes Needed

| Component | Why |
|-----------|-----|
| `src/cdp/*` | `Accessibility.getFullAXTree` is a standard CDP command; no new transport features needed |
| `src/connection.rs` | `ManagedSession` and connection resolution reusable as-is |
| `src/session.rs` | Session file is for connection info; snapshot state uses its own file |
| `src/lib.rs` | `snapshot.rs` is a binary-only module (not part of the library) |

---

## CDP Domain: Accessibility.getFullAXTree

### Request

```json
{
  "method": "Accessibility.getFullAXTree",
  "params": {}
}
```

No parameters needed — returns the entire tree for the current page.

### Response Structure

```json
{
  "nodes": [
    {
      "nodeId": "1",
      "ignored": false,
      "role": {"type": "role", "value": "document"},
      "name": {"type": "computedString", "value": "Example Domain", "sources": [...]},
      "properties": [
        {"name": "level", "value": {"type": "integer", "value": 1}}
      ],
      "childIds": ["2", "3", "4"],
      "backendDOMNodeId": 1
    },
    ...
  ]
}
```

### Key Fields per AXNode

| Field | Type | Description |
|-------|------|-------------|
| `nodeId` | string | Accessibility node ID (internal to AX tree) |
| `ignored` | bool | Whether node is ignored by accessibility |
| `role.value` | string | ARIA role (document, heading, link, button, etc.) |
| `name.value` | string | Computed accessible name |
| `properties` | array | Array of `{name, value}` pairs |
| `childIds` | array of strings | Child node IDs |
| `backendDOMNodeId` | integer | Stable DOM node ID (for interaction commands) |
| `parentId` | string (optional) | Parent node ID |

### Domain Enabling

```
Accessibility.enable
```

Required before `getFullAXTree`. Handled by `ManagedSession.ensure_domain("Accessibility")`.

---

## Internal Data Types

### AXNode (parsed from CDP response)

```rust
struct AxNode {
    node_id: String,
    ignored: bool,
    role: String,
    name: String,
    properties: Vec<(String, serde_json::Value)>,
    child_ids: Vec<String>,
    backend_dom_node_id: Option<i64>,
}
```

### SnapshotNode (output tree)

```rust
#[derive(Serialize)]
struct SnapshotNode {
    role: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    uid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<HashMap<String, serde_json::Value>>,
    children: Vec<SnapshotNode>,
}
```

### SnapshotState (persisted)

```rust
#[derive(Serialize, Deserialize)]
struct SnapshotState {
    url: String,
    timestamp: String,
    uid_map: HashMap<String, i64>,  // uid → backendDOMNodeId
}
```

File location: `~/.chrome-cli/snapshot.json`

---

## UID Assignment Strategy

### Interactive Roles

UIDs are assigned to elements with these roles:

| Role | Example |
|------|---------|
| `link` | `<a href="...">` |
| `button` | `<button>`, `role="button"` |
| `textbox` | `<input type="text">`, `<textarea>` |
| `checkbox` | `<input type="checkbox">` |
| `radio` | `<input type="radio">` |
| `combobox` | `<select>`, `role="combobox"` |
| `menuitem` | `role="menuitem"` |
| `tab` | `role="tab"` |
| `switch` | `role="switch"` |
| `slider` | `<input type="range">` |
| `spinbutton` | `<input type="number">` |
| `searchbox` | `<input type="search">` |
| `option` | `<option>` in a select |
| `treeitem` | `role="treeitem"` |

### Assignment Order

1. Depth-first tree traversal
2. For each non-ignored node with an interactive role, assign the next sequential UID
3. Format: `s{N}` starting from `s1`
4. Store `{ uid: backendDOMNodeId }` in the mapping

### Stability Guarantee

UIDs are assigned in deterministic tree-traversal order. For unchanged pages, `Accessibility.getFullAXTree` returns the same node order, so elements receive the same UIDs across consecutive snapshots. Navigation or DOM changes invalidate the mapping (detected by URL mismatch in `snapshot.json`).

---

## Snapshot State Persistence

### File: `~/.chrome-cli/snapshot.json`

```json
{
  "url": "https://example.com/",
  "timestamp": "2026-02-12T10:30:00Z",
  "uid_map": {
    "s1": 42,
    "s2": 87,
    "s3": 156
  }
}
```

### Read/Write Pattern

Follow the same atomic-write pattern as `session.rs`:
1. Write to `snapshot.json.tmp`
2. Set permissions `0o600` on Unix
3. Rename to `snapshot.json`

### Consumption by Interaction Commands

Future interaction commands (#14-#17) will:
1. Read `snapshot.json`
2. Look up uid → `backendDOMNodeId`
3. Use `DOM.describeNode({backendNodeId})` to get the current `nodeId`/`objectId`
4. Perform the interaction (click, fill, etc.)

---

## Output Formatting

### Text Format (default)

```
fn format_text_tree(node: &SnapshotNode, depth: usize, verbose: bool) -> String
```

Rules:
- Each line: `{indent}- {role} "{name}" [{uid}]`
- Indentation: 2 spaces per depth level
- `[uid]` only present for interactive elements
- Verbose mode appends: `key=value key=value` after the uid bracket
- Empty names still show quotes: `- paragraph ""`
- Text nodes with empty names are hidden (they clutter the output)

### JSON Format

Direct serialization of the `SnapshotNode` tree via `serde_json::to_string` (compact) or `serde_json::to_string_pretty` (pretty).

### File Output

When `--file <PATH>` is specified:
1. Format the output as usual (text or JSON based on flags)
2. Write to the file path instead of stdout
3. Use `std::fs::write` (not atomic — snapshot files are not critical like session)
4. Return success with no stdout output

---

## Large Page Handling

### Truncation Strategy

| Metric | Limit | Behavior |
|--------|-------|----------|
| Node count | 10,000 | Truncate tree, append `[... truncated: {total} nodes, showing first 10,000]` |

Implementation:
- After parsing all AX nodes, if count exceeds limit, truncate during tree-building
- Track node count during tree traversal
- When limit reached, stop adding children and note truncation
- For JSON output: add `"truncated": true, "total_nodes": N` to root object

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: `Accessibility.getFullAXTree`** | Full AX tree in one CDP call | Direct accessibility semantics, includes `backendDOMNodeId`, single round-trip | Flat node list requires tree reconstruction | **Selected** |
| **B: `DOMSnapshot.captureSnapshot`** | DOM + layout + computed styles | Rich data | Not accessibility-focused, much more data than needed, no ARIA roles | Rejected — wrong abstraction level |
| **C: `Accessibility.getPartialAXTree`** | Subtree of AX tree | Can target specific elements | Requires knowing nodeId upfront, multiple calls for full tree | Rejected — more complex for full-page snapshot |
| **D: JavaScript `window.getComputedAccessibleNode`** | JS API for accessibility | Runs in page context | Non-standard, not widely supported, would need DOM walking | Rejected — CDP native approach is better |

### UID Format Alternatives

| Option | Format | Pros | Cons | Decision |
|--------|--------|------|------|----------|
| **A: Sequential** | `s1`, `s2`, `s3` | Short, simple, matches MCP reference | Not hierarchical | **Selected** |
| **B: Hierarchical** | `1.2.3` | Shows tree position | Verbose, changes when siblings change | Rejected |
| **C: Hash-based** | `a3f2` | Stable even when siblings change | Not sequential, harder to read | Rejected |

### Snapshot State Location

| Option | Location | Pros | Cons | Decision |
|--------|----------|------|------|----------|
| **A: Separate file** | `~/.chrome-cli/snapshot.json` | Clean separation, doesn't bloat session file | Extra file | **Selected** |
| **B: In session file** | `~/.chrome-cli/session.json` | Single file | Mixes connection and snapshot state, larger file | Rejected |
| **C: In-memory only** | Not persisted | Simple | Can't be used by subsequent CLI invocations | Rejected — breaks the use case |

---

## Security Considerations

- [x] **Input Validation**: `--file` path is used directly with `std::fs::write` — Rust's type system prevents injection
- [x] **No arbitrary JS**: Snapshot uses CDP domain method, not JavaScript evaluation
- [x] **Sensitive Data**: Accessibility tree may expose page content — expected for a localhost CLI tool
- [x] **File Permissions**: Snapshot state file gets `0o600` on Unix (same as session file)

---

## Performance Considerations

- [x] **Single CDP round-trip**: `Accessibility.getFullAXTree` fetches the entire tree in one call
- [x] **No DOM domain needed**: Only `Accessibility` domain (and implicit `DOM` dependency handled by Chrome)
- [x] **Truncation**: Pages with 10,000+ nodes are truncated to prevent memory/output issues
- [x] **Tree building**: O(n) reconstruction from flat node list using HashMap lookup

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Tree building | Unit | Parse CDP response → SnapshotNode tree reconstruction |
| UID assignment | Unit | Interactive roles get UIDs, non-interactive don't |
| Text formatting | Unit | Indentation, uid brackets, verbose properties |
| JSON output | Unit | Serialization of SnapshotNode tree |
| Snapshot state | Unit | Write/read round-trip of snapshot.json |
| Error constructors | Unit | `snapshot_failed()`, `file_write_failed()` messages and codes |
| CLI args | Unit | Parsing of `--verbose`, `--file` |
| Feature | BDD (Gherkin) | All 12 acceptance criteria as scenarios |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `Accessibility.getFullAXTree` slow on complex pages | Med | Med | Bounded by `--timeout`; truncation for very large trees |
| Some Chrome versions may return different AX node formats | Low | Med | Defensive parsing with defaults for missing fields |
| `backendDOMNodeId` may not be present for all nodes | Low | Low | Only assign UIDs to nodes that have `backendDOMNodeId` |
| Snapshot state file conflicts if multiple CLI instances run concurrently | Low | Low | Atomic write pattern; last-write-wins is acceptable |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed (file-based state only)
- [x] State management approach is clear (snapshot.json)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
