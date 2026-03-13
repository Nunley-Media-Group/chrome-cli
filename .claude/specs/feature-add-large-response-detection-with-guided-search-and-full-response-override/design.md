# Design: Large Response Detection with Guided Search and Full-Response Override

**Issues**: #168
**Date**: 2026-03-12
**Status**: Draft
**Author**: AI (nmg-sdlc)

---

## Overview

This feature introduces a unified output gate that intercepts serialized JSON before it reaches stdout, compares its byte length against a configurable threshold (default 16 KB), and either passes it through or replaces it with a structured guidance object. Two escape hatches — a per-command `--search` flag and a global `--full-response` flag — let agents filter or bypass the gate.

The design introduces a new `src/output.rs` module that centralizes the size-check-and-emit logic. Each command module continues to produce its normal typed output, but delegates final printing to the new module. The `--search` flag is handled per-command before the output stage, since each command's data has different structure. The `--full-response` and `--large-response-threshold` flags are added to the global `OutputFormat` struct.

The key architectural principle is **serialize once, check once, print once**: the output is serialized to a JSON string, the byte length is checked against the threshold, and either the original string or a guidance object is printed. No double-serialization occurs.

---

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI Layer                                │
│  GlobalOpts { OutputFormat { full_response, threshold, ... } }  │
│  Per-command Args { search: Option<String>, ... }               │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│                     Command Modules                              │
│  page/snapshot.rs, page/text.rs, js.rs, network.rs              │
│                                                                  │
│  1. Execute CDP commands, produce typed result                   │
│  2. If --search: filter result, return filtered data             │
│  3. If --plain: print plain text, return (no gate)               │
│  4. Call output::emit() with typed result + summary generator    │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│                    output.rs (NEW)                                │
│                                                                  │
│  emit(value, output_format, command_name, summary_fn)            │
│    1. Serialize value to JSON string                             │
│    2. If full_response: print JSON string, return                │
│    3. If len <= threshold: print JSON string, return             │
│    4. Generate summary via summary_fn(value)                     │
│    5. Build LargeResponseGuidance                                │
│    6. Serialize and print guidance object                        │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. CLI parses args → GlobalOpts (with threshold + full_response) + command Args (with search)
2. Config loaded → threshold merged (CLI > config > default 16384)
3. Command module executes CDP calls → produces typed result (e.g., SnapshotNode)
4. If --search present: command module filters result in-place → uses filtered result
5. If --plain: command prints plain text directly → exits (no output gate)
6. Command calls output::emit(&result, &output_format, "page snapshot", summary_fn)
7. output::emit() serializes result to JSON string
8. If --full-response OR byte_len <= threshold: print JSON string
9. Else: call summary_fn(&result) → build LargeResponseGuidance → print guidance JSON
```

---

## API / Interface Changes

### New Global CLI Flags

| Flag | Type | Default | Purpose |
|------|------|---------|---------|
| `--full-response` | bool | `false` | Bypass large-response gate, return complete output |
| `--large-response-threshold` | usize | `16384` | Byte threshold for triggering guidance object |

Added to `OutputFormat` struct in `src/cli/mod.rs`. The `OutputFormat` group constraint changes from `multiple = false` to allow these flags alongside `--json`/`--pretty`/`--plain`.

### New Per-Command Flag

| Flag | Type | Commands | Purpose |
|------|------|----------|---------|
| `--search <query>` | String | `page snapshot`, `page text`, `js exec`, `network list`, `network get` | Filter output to matching content |

Added to each command's Args struct (e.g., `PageSnapshotArgs`, `PageTextArgs`, `JsExecArgs`, `NetworkListArgs`, `NetworkGetArgs`).

### New Config File Key

```toml
[output]
large_response_threshold = 16384
```

Added to `OutputConfig` in `src/config.rs`.

### CLI Struct Changes

```rust
// src/cli/mod.rs — OutputFormat (modified)
#[derive(Args)]
pub struct OutputFormat {
    /// Output as compact JSON (mutually exclusive with --pretty, --plain)
    #[arg(long, global = true)]
    pub json: bool,

    /// Output as pretty-printed JSON (mutually exclusive with --json, --plain)
    #[arg(long, global = true)]
    pub pretty: bool,

    /// Output as human-readable plain text (mutually exclusive with --json, --pretty)
    #[arg(long, global = true)]
    pub plain: bool,

    /// Return complete output even when it exceeds the large-response threshold
    #[arg(long, global = true)]
    pub full_response: bool,

    /// Byte threshold for large-response detection (default: 16384)
    #[arg(long, global = true)]
    pub large_response_threshold: Option<usize>,
}
```

Note: The mutual-exclusivity group `#[group(multiple = false)]` must be restructured. The format flags (`--json`, `--pretty`, `--plain`) remain mutually exclusive via a named group, while `--full-response` and `--large-response-threshold` are independent.

### Guidance Object Schema

```rust
// src/output.rs
#[derive(Serialize)]
pub struct LargeResponseGuidance {
    pub large_response: bool,          // always true
    pub size_bytes: u64,
    pub command: String,               // e.g., "page snapshot"
    pub summary: serde_json::Value,    // command-specific metadata
    pub guidance: String,              // human-readable instructions
}
```

**Example output:**

```json
{
  "large_response": true,
  "size_bytes": 536576,
  "command": "page snapshot",
  "summary": {
    "total_nodes": 8500,
    "top_roles": ["main", "navigation", "complementary"]
  },
  "guidance": "Response is 524 KB (above 16 KB threshold). Summary: accessibility tree with 8,500 nodes (top roles: main, navigation, complementary). Options: (1) Use --search \"<query>\" to retrieve matching nodes only. Example: page snapshot --search \"login\". (2) Use --full-response to retrieve the complete tree. Use --full-response when: you need to inspect all interactive elements, --search doesn't narrow results sufficiently, or you are performing a comprehensive page audit."
}
```

### Command-Specific Summary Schemas

| Command | Summary Fields | Example |
|---------|---------------|---------|
| `page snapshot` | `total_nodes`, `top_roles` | `{"total_nodes": 8500, "top_roles": ["main", "navigation"]}` |
| `page text` | `character_count`, `line_count` | `{"character_count": 45000, "line_count": 1200}` |
| `js exec` | `result_type`, `size_bytes` | `{"result_type": "object", "size_bytes": 32000}` |
| `network list` | `request_count`, `methods`, `domains` | `{"request_count": 150, "methods": ["GET", "POST"], "domains": ["api.example.com"]}` |
| `network get` | `url`, `status`, `content_type`, `body_size_bytes` | `{"url": "https://...", "status": 200, "content_type": "application/json", "body_size_bytes": 50000}` |

---

## New Module: `src/output.rs`

### Public API

```rust
/// Default large-response threshold in bytes (16 KB).
pub const DEFAULT_THRESHOLD: usize = 16_384;

/// Emit a serializable value through the large-response gate.
///
/// If `--plain` mode is active, this function is NOT called (plain mode
/// is handled by the command module before reaching this point).
///
/// Returns Ok(()) on success.
pub fn emit<T: Serialize, F>(
    value: &T,
    output: &OutputFormat,
    command_name: &str,
    summary_fn: F,
) -> Result<(), AppError>
where
    F: FnOnce(&T) -> serde_json::Value;
```

### Internal Logic

```rust
pub fn emit<T, F>(value: &T, output: &OutputFormat, command_name: &str, summary_fn: F) -> Result<(), AppError>
where
    T: Serialize,
    F: FnOnce(&T) -> serde_json::Value,
{
    // 1. Serialize to JSON string (once)
    let json_string = if output.pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }.map_err(serialization_error)?;

    // 2. If full_response, print and return
    if output.full_response {
        println!("{json_string}");
        return Ok(());
    }

    // 3. Determine effective threshold
    let threshold = output.large_response_threshold.unwrap_or(DEFAULT_THRESHOLD);

    // 4. If under threshold, print and return
    if json_string.len() <= threshold {
        println!("{json_string}");
        return Ok(());
    }

    // 5. Build guidance object
    let summary = summary_fn(value);
    let size_bytes = json_string.len() as u64;
    let guidance_text = build_guidance_text(command_name, size_bytes, &summary);

    let guidance = LargeResponseGuidance {
        large_response: true,
        size_bytes,
        command: command_name.to_string(),
        summary,
        guidance: guidance_text,
    };

    // 6. Serialize and print guidance (always compact JSON, even with --pretty)
    let guidance_json = serde_json::to_string(&guidance).map_err(serialization_error)?;
    println!("{guidance_json}");
    Ok(())
}
```

### Guidance Text Builder

```rust
fn build_guidance_text(
    command_name: &str,
    size_bytes: u64,
    summary: &serde_json::Value,
) -> String {
    let human_size = format_human_size(size_bytes);
    let threshold_str = format_human_size(DEFAULT_THRESHOLD as u64);

    // Command-specific summary sentence + search example
    let (summary_sentence, search_example, full_response_reasons) =
        command_specific_guidance(command_name, summary);

    format!(
        "Response is {human_size} (above {threshold_str} threshold). \
         {summary_sentence} \
         Options: (1) Use --search \"<query>\" to retrieve matching content only. \
         Example: {search_example}. \
         (2) Use --full-response to retrieve the complete response. \
         Use --full-response when: {full_response_reasons}."
    )
}

fn format_human_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{} KB", bytes / 1024)
    } else {
        format!("{bytes} bytes")
    }
}
```

---

## Search Implementation Per Command

### `page snapshot --search <query>`

**Location**: `src/page/snapshot.rs`

**Algorithm**:
1. Build the full accessibility tree as normal
2. Walk the tree, mark nodes where `name`, `role`, or any text property contains the query (case-insensitive)
3. For each matching node, include all ancestors up to root (for tree context)
4. Prune branches with no matching descendants
5. Return the filtered tree in the normal `SnapshotNode` schema

```rust
// New function in snapshot.rs
pub fn filter_tree(root: &SnapshotNode, query: &str) -> Option<SnapshotNode> {
    let query_lower = query.to_lowercase();
    filter_node(root, &query_lower)
}

fn filter_node(node: &SnapshotNode, query: &str) -> Option<SnapshotNode> {
    let self_matches = node.name.to_lowercase().contains(query)
        || node.role.to_lowercase().contains(query);

    let filtered_children: Vec<SnapshotNode> = node.children.iter()
        .filter_map(|child| filter_node(child, query))
        .collect();

    if self_matches || !filtered_children.is_empty() {
        Some(SnapshotNode {
            role: node.role.clone(),
            name: node.name.clone(),
            uid: node.uid.clone(),
            properties: node.properties.clone(),
            backend_dom_node_id: node.backend_dom_node_id,
            children: filtered_children,
        })
    } else {
        None
    }
}
```

### `page text --search <query>`

**Location**: `src/page/text.rs`

**Algorithm**:
1. Extract full page text as normal
2. Split text into paragraphs (double-newline separated)
3. Return only paragraphs containing the query (case-insensitive)
4. Rejoin with double newlines

### `js exec --search <query>`

**Location**: `src/js.rs`

**Algorithm**:
1. Execute JS and get result as `serde_json::Value`
2. If result is object: retain only key-value pairs where key or serialized value contains query
3. If result is array: retain only elements where serialized element contains query
4. If result is string: return the string only if it contains query, else empty string
5. Other types: return as-is (no filtering for numbers/bools/null)

### `network list --search <query>`

**Location**: `src/network.rs`

**Algorithm**:
1. Collect network requests as normal
2. Filter to requests where URL or method contains query (case-insensitive)
3. Existing `--url` and `--method` filters still apply (search is additive)

### `network get --search <query>`

**Location**: `src/network.rs`

**Algorithm**:
1. Fetch full request detail as normal
2. If response body is a string: check if it contains query; if not, set body to `null`
3. Filter response headers to those whose name or value contains query
4. Return the filtered `NetworkRequestDetail`

---

## Config Changes

### `src/config.rs`

```rust
// OutputConfig (modified)
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct OutputConfig {
    pub format: Option<String>,
    pub large_response_threshold: Option<usize>,  // NEW
}

// ResolvedOutput (modified)
#[derive(Debug, Serialize)]
pub struct ResolvedOutput {
    pub format: String,
    pub large_response_threshold: usize,  // NEW — resolved with default
}
```

### Resolution in `resolve_config()`:

```rust
output: ResolvedOutput {
    format: file.output.format.clone().unwrap_or_else(|| "json".to_string()),
    large_response_threshold: file.output.large_response_threshold
        .unwrap_or(output::DEFAULT_THRESHOLD),
},
```

### Merging in `apply_config_defaults()` (main.rs):

```rust
output: cli::OutputFormat {
    json: cli_global.output.json,
    pretty: cli_global.output.pretty,
    plain: cli_global.output.plain,
    full_response: cli_global.output.full_response,
    large_response_threshold: cli_global.output.large_response_threshold
        .or(config.output.large_response_threshold),
},
```

---

## Command Module Changes

### Pattern for Each Affected Command

Each command module currently ends with:

```rust
// Before (e.g., page/text.rs)
if global.output.plain {
    print!("{text}");
    return Ok(());
}
let output = PageTextResult { text, url, title };
print_output(&output, &global.output)
```

Changes to:

```rust
// After
if global.output.plain {
    // --search in plain mode: filter text, then print
    let text = if let Some(ref query) = args.search {
        filter_text_by_query(&text, query)
    } else {
        text
    };
    print!("{text}");
    return Ok(());
}

let result = if let Some(ref query) = args.search {
    PageTextResult { text: filter_text_by_query(&text, query), url, title }
} else {
    PageTextResult { text, url, title }
};

output::emit(&result, &global.output, "page text", |r| {
    serde_json::json!({
        "character_count": r.text.len(),
        "line_count": r.text.lines().count(),
    })
})
```

### Snapshot Special Case

`page/snapshot.rs` has custom serialization logic (adding `truncated` and `total_nodes` fields to the JSON value). This needs to be adapted:

1. Build tree and produce `serde_json::Value` as before
2. If `--search`: filter tree nodes, then proceed
3. Pass the `serde_json::Value` to `output::emit()` with a snapshot-specific summary function

The `output::emit()` function accepts `&impl Serialize`, so `serde_json::Value` works directly.

### Summary Functions

Each command provides a closure to `output::emit()`:

```rust
// page/snapshot.rs
output::emit(&json_value, &global.output, "page snapshot", |v| {
    let total_nodes = count_nodes(v);
    let top_roles = extract_top_roles(v, 5);
    serde_json::json!({
        "total_nodes": total_nodes,
        "top_roles": top_roles,
    })
})

// page/text.rs
output::emit(&result, &global.output, "page text", |r| {
    serde_json::json!({
        "character_count": r.text.len(),
        "line_count": r.text.lines().count(),
    })
})

// js.rs
output::emit(&result, &global.output, "js exec", |r| {
    let type_str = &r.r#type;
    let size = serde_json::to_string(&r.result).map(|s| s.len()).unwrap_or(0);
    serde_json::json!({
        "result_type": type_str,
        "size_bytes": size,
    })
})

// network.rs (list)
output::emit(&requests, &global.output, "network list", |reqs| {
    let methods: HashSet<&str> = reqs.iter().map(|r| r.method.as_str()).collect();
    let domains: HashSet<String> = reqs.iter().filter_map(|r| extract_domain(&r.url)).collect();
    serde_json::json!({
        "request_count": reqs.len(),
        "methods": methods.into_iter().collect::<Vec<_>>(),
        "domains": domains.into_iter().take(10).collect::<Vec<_>>(),
    })
})

// network.rs (get)
output::emit(&detail, &global.output, "network get", |d| {
    serde_json::json!({
        "url": d.request.url,
        "status": d.response.status,
        "content_type": d.response.mime_type,
        "body_size_bytes": d.response.body.as_ref().map(|b| b.len()),
    })
})
```

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Middleware/trait-based** | Define a `CommandOutput` trait with `size_hint()`, `summarize()`, `search()` methods; implement for each output type | Clean abstraction, composable | Requires refactoring all output types to implement trait; over-engineered for 5 commands | Rejected — too much refactoring |
| **B: Centralized emit function** | New `output::emit()` function with summary closure; commands call it instead of `print_output()` | Minimal refactoring, summary logic stays close to data, single serialization pass | Summary closures are ad-hoc (no compile-time guarantee of schema) | **Selected** |
| **C: Post-serialization wrapper in main.rs** | Intercept all stdout in `main.rs` after command returns | Zero changes to command modules | Cannot generate command-specific summaries; can't handle `--search` which needs pre-serialization filtering | Rejected — can't support search or summaries |

---

## Security Considerations

- [x] **Input Validation**: `--search` query is used only for string matching (no regex, no injection vector). `--large-response-threshold` validated as > 0 by clap.
- [x] **No new external communication**: All processing is local.
- [x] **No sensitive data in guidance**: Summary contains only structural metadata (counts, roles, types), never actual page content.

---

## Performance Considerations

- [x] **Single serialization pass**: Output is serialized once to a JSON string; byte length check is O(1) on the string length.
- [x] **Summary generation is lazy**: Summary closure is only called when the threshold is exceeded.
- [x] **Search filtering**: Tree filtering is O(n) where n = number of nodes. Text filtering is O(n) where n = text length. Both are bounded by existing per-command truncation limits.
- [x] **No memory duplication**: The serialized JSON string is owned once; if guidance is emitted, the original string is dropped.

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| `output::emit()` | Unit | Threshold logic, guidance object schema, format_human_size, below/above threshold paths |
| Search functions | Unit | Per-command filtering: snapshot tree filtering, text paragraph filtering, JSON key/value filtering, network URL filtering |
| Summary functions | Unit | Per-command summary generation with known inputs |
| CLI flags | BDD | `--full-response`, `--large-response-threshold`, `--search` integration with real commands |
| Config file | BDD | `large_response_threshold` loaded from config, CLI override |
| Cross-command consistency | BDD | Guidance object schema is identical across all commands that trigger it |
| Plain mode exemption | BDD | `--plain` never produces guidance object |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Guidance object breaks existing scripts | Low | High | Only affects above-threshold responses; `--full-response` provides opt-out; below-threshold is unchanged |
| `--search` returns empty results | Medium | Low | Return empty array/object in normal schema (not an error); exit code remains 0 |
| Summary closure panics | Low | High | Summary closures use safe operations only (counting, collecting); no unwrap() |
| Double serialization for snapshot (custom JSON value manipulation) | Medium | Low | Refactor snapshot to produce a serializable struct, or pass pre-serialized `serde_json::Value` to `emit()` |

---

## Open Questions

- [x] Should `output::emit()` support `serde_json::Value` directly (for snapshot's custom serialization)? — **Yes, `Value` implements `Serialize`**
- [x] Should the guidance object be pretty-printed when `--pretty` is active? — **No, guidance is always compact JSON for machine readability**

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #168 | 2026-03-12 | Initial feature spec |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] N/A — No database/storage changes
- [x] N/A — No state management (CLI tool, no persistent state beyond session)
- [x] N/A — No UI components (CLI tool)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
