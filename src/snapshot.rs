use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use agentchrome::error::AppError;

// =============================================================================
// Constants
// =============================================================================

/// Maximum number of nodes before truncation.
pub const MAX_NODES: usize = 10_000;

/// Roles that receive a UID for interaction commands.
const INTERACTIVE_ROLES: &[&str] = &[
    "link",
    "button",
    "textbox",
    "checkbox",
    "radio",
    "combobox",
    "menuitem",
    "tab",
    "switch",
    "slider",
    "spinbutton",
    "searchbox",
    "option",
    "treeitem",
];

/// Roles preserved in compact mode because they carry semantic meaning.
const COMPACT_KEPT_ROLES: &[&str] = &[
    "banner",
    "complementary",
    "contentinfo",
    "form",
    "main",
    "navigation",
    "region",
    "search",
    "heading",
    "list",
    "listitem",
    "table",
    "row",
    "cell",
    "columnheader",
    "rowheader",
    "RootWebArea",
    "document",
];

/// Roles unconditionally removed in compact mode (noise nodes).
const COMPACT_EXCLUDED_ROLES: &[&str] = &["InlineTextBox", "LineBreak"];

// =============================================================================
// Internal CDP node representation
// =============================================================================

struct AxNode {
    node_id: String,
    parent_id: Option<String>,
    ignored: bool,
    role: String,
    name: String,
    properties: Vec<(String, serde_json::Value)>,
    child_ids: Vec<String>,
    backend_dom_node_id: Option<i64>,
}

fn parse_ax_nodes(nodes: &[serde_json::Value]) -> Vec<AxNode> {
    nodes
        .iter()
        .map(|n| {
            let child_ids = n["childIds"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let properties = n["properties"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|p| {
                            let name = p["name"].as_str()?.to_string();
                            let value = p["value"]["value"].clone();
                            Some((name, value))
                        })
                        .collect()
                })
                .unwrap_or_default();

            AxNode {
                node_id: n["nodeId"].as_str().unwrap_or_default().to_string(),
                parent_id: n["parentId"].as_str().map(String::from),
                ignored: n["ignored"].as_bool().unwrap_or(false),
                role: n["role"]["value"].as_str().unwrap_or_default().to_string(),
                name: n["name"]["value"].as_str().unwrap_or_default().to_string(),
                properties,
                child_ids,
                backend_dom_node_id: n["backendDOMNodeId"].as_i64(),
            }
        })
        .collect()
}

// =============================================================================
// Output tree node
// =============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotNode {
    pub role: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip)]
    pub backend_dom_node_id: Option<i64>,
    pub children: Vec<SnapshotNode>,
}

// =============================================================================
// Tree building + UID assignment
// =============================================================================

/// Result of building the snapshot tree.
pub struct BuildResult {
    pub root: SnapshotNode,
    pub uid_map: HashMap<String, i64>,
    pub truncated: bool,
    pub total_nodes: usize,
}

/// Build a `SnapshotNode` tree from the flat CDP `Accessibility.getFullAXTree` response.
///
/// Assigns sequential UIDs (`s1`, `s2`, ...) to interactive elements in depth-first order.
/// Returns the root node and the uid-to-`backendDOMNodeId` mapping.
pub fn build_tree(nodes: &[serde_json::Value], verbose: bool) -> BuildResult {
    let mut ax_nodes = parse_ax_nodes(nodes);
    let total_nodes = ax_nodes.len();

    // Find root (first node, or first non-ignored node)
    let root_id = ax_nodes
        .iter()
        .find(|n| !n.ignored)
        .map(|n| n.node_id.clone())
        .unwrap_or_default();

    // Fallback: if root has empty child_ids but there are other nodes,
    // reconstruct child_ids from parentId references.
    let root_has_children = ax_nodes
        .iter()
        .any(|n| n.node_id == root_id && !n.child_ids.is_empty());

    if !root_has_children && total_nodes > 1 {
        // Build parent_id → Vec<child_id> map from parentId fields
        let mut parent_to_children: HashMap<String, Vec<String>> =
            HashMap::with_capacity(total_nodes);
        for node in &ax_nodes {
            if let Some(ref pid) = node.parent_id {
                parent_to_children
                    .entry(pid.clone())
                    .or_default()
                    .push(node.node_id.clone());
            }
        }
        // Inject computed child_ids into nodes that have none
        for node in &mut ax_nodes {
            if node.child_ids.is_empty() {
                if let Some(children) = parent_to_children.remove(&node.node_id) {
                    node.child_ids = children;
                }
            }
        }
    }

    // Build lookup: node_id → AxNode
    let mut lookup: HashMap<&str, &AxNode> = HashMap::with_capacity(ax_nodes.len());
    for node in &ax_nodes {
        lookup.insert(&node.node_id, node);
    }

    let mut uid_counter: usize = 0;
    let mut uid_map: HashMap<String, i64> = HashMap::new();
    let mut node_count: usize = 0;
    let truncated = total_nodes > MAX_NODES;

    let mut roots = build_subtree(
        &root_id,
        &lookup,
        verbose,
        &mut uid_counter,
        &mut uid_map,
        &mut node_count,
        truncated,
    );
    let root = if roots.len() == 1 {
        roots.remove(0)
    } else {
        SnapshotNode {
            role: "document".to_string(),
            name: String::new(),
            uid: None,
            properties: None,
            backend_dom_node_id: None,
            children: vec![],
        }
    };

    BuildResult {
        root,
        uid_map,
        truncated,
        total_nodes,
    }
}

fn build_subtree(
    node_id: &str,
    lookup: &HashMap<&str, &AxNode>,
    verbose: bool,
    uid_counter: &mut usize,
    uid_map: &mut HashMap<String, i64>,
    node_count: &mut usize,
    truncated: bool,
) -> Vec<SnapshotNode> {
    if truncated && *node_count >= MAX_NODES {
        return vec![];
    }

    let Some(ax) = lookup.get(node_id) else {
        return vec![];
    };

    // Ignored nodes are transparent: skip rendering them but promote their children
    if ax.ignored {
        return ax
            .child_ids
            .iter()
            .flat_map(|cid| {
                build_subtree(
                    cid,
                    lookup,
                    verbose,
                    uid_counter,
                    uid_map,
                    node_count,
                    truncated,
                )
            })
            .collect();
    }

    *node_count += 1;

    // Assign UID if interactive and has a backend node ID
    let uid = if INTERACTIVE_ROLES.contains(&ax.role.as_str()) {
        if let Some(backend_id) = ax.backend_dom_node_id {
            *uid_counter += 1;
            let uid = format!("s{uid_counter}");
            uid_map.insert(uid.clone(), backend_id);
            Some(uid)
        } else {
            None
        }
    } else {
        None
    };

    // Collect properties if verbose
    let properties = if verbose && !ax.properties.is_empty() {
        let map: HashMap<String, serde_json::Value> = ax.properties.iter().cloned().collect();
        if map.is_empty() { None } else { Some(map) }
    } else {
        None
    };

    // Recursively build children (flat_map to flatten promoted children from ignored nodes)
    let children: Vec<SnapshotNode> = ax
        .child_ids
        .iter()
        .flat_map(|cid| {
            build_subtree(
                cid,
                lookup,
                verbose,
                uid_counter,
                uid_map,
                node_count,
                truncated,
            )
        })
        .collect();

    vec![SnapshotNode {
        role: ax.role.clone(),
        name: ax.name.clone(),
        uid,
        properties,
        backend_dom_node_id: ax.backend_dom_node_id,
        children,
    }]
}

// =============================================================================
// Tree search
// =============================================================================

/// A single hit from searching the snapshot tree.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub uid: Option<String>,
    pub role: String,
    pub name: String,
    pub backend_dom_node_id: Option<i64>,
}

/// Search the snapshot tree for nodes matching the given criteria.
///
/// Walks the tree depth-first (document order) and returns up to `limit` matches.
///
/// - `query`: text to match against node names (required for text search)
/// - `role_filter`: only include nodes with this role
/// - `exact`: if true, match name exactly (case-sensitive); otherwise case-insensitive substring
pub fn search_tree(
    root: &SnapshotNode,
    query: &str,
    role_filter: Option<&str>,
    exact: bool,
    limit: usize,
) -> Vec<SearchHit> {
    let mut ctx = SearchContext {
        query,
        query_lower: query.to_lowercase(),
        role_filter,
        exact,
        limit,
        results: Vec::new(),
    };
    search_node(root, &mut ctx);
    ctx.results
}

struct SearchContext<'a> {
    query: &'a str,
    query_lower: String,
    role_filter: Option<&'a str>,
    exact: bool,
    limit: usize,
    results: Vec<SearchHit>,
}

fn search_node(node: &SnapshotNode, ctx: &mut SearchContext<'_>) {
    if ctx.results.len() >= ctx.limit {
        return;
    }

    // Check role filter
    let role_matches = ctx.role_filter.is_none_or(|r| node.role == r);

    // Check text match
    let text_matches = if ctx.query.is_empty() {
        true
    } else if ctx.exact {
        node.name == ctx.query
    } else {
        node.name.to_lowercase().contains(&ctx.query_lower)
    };

    if role_matches && text_matches {
        ctx.results.push(SearchHit {
            uid: node.uid.clone(),
            role: node.role.clone(),
            name: node.name.clone(),
            backend_dom_node_id: node.backend_dom_node_id,
        });
    }

    for child in &node.children {
        if ctx.results.len() >= ctx.limit {
            return;
        }
        search_node(child, ctx);
    }
}

// =============================================================================
// Text formatting
// =============================================================================

/// Format the snapshot tree as hierarchical text.
///
/// Each line: `{indent}- {role} "{name}" [{uid}]`
pub fn format_text(root: &SnapshotNode, verbose: bool) -> String {
    let mut output = String::new();
    format_text_node(root, 0, verbose, &mut output);
    output
}

fn format_text_node(node: &SnapshotNode, depth: usize, verbose: bool, output: &mut String) {
    use std::fmt::Write;

    let indent = "  ".repeat(depth);

    let uid_str = node
        .uid
        .as_ref()
        .map_or(String::new(), |uid| format!(" [{uid}]"));

    let props_str = if verbose {
        node.properties
            .as_ref()
            .map(|props| {
                let mut parts: Vec<String> = props
                    .iter()
                    .map(|(k, v)| {
                        if v.is_string() {
                            format!("{k}=\"{}\"", v.as_str().unwrap_or_default())
                        } else {
                            format!("{k}={v}")
                        }
                    })
                    .collect();
                parts.sort();
                if parts.is_empty() {
                    String::new()
                } else {
                    format!(" {}", parts.join(" "))
                }
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    let _ = writeln!(
        output,
        "{indent}- {} \"{}\"{uid_str}{props_str}",
        node.role, node.name
    );

    for child in &node.children {
        format_text_node(child, depth + 1, verbose, output);
    }
}

// =============================================================================
// Summary helpers (for large-response temp file output)
// =============================================================================

/// Count total nodes in a snapshot tree (as a `serde_json::Value`).
pub fn count_nodes(value: &serde_json::Value) -> u64 {
    let mut count = 1u64;
    if let Some(children) = value["children"].as_array() {
        for child in children {
            count += count_nodes(child);
        }
    }
    count
}

/// Extract the most common roles from a snapshot tree (as a `serde_json::Value`).
pub fn top_roles(value: &serde_json::Value, limit: usize) -> Vec<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    collect_roles(value, &mut counts);
    let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.into_iter().take(limit).map(|(r, _)| r).collect()
}

fn collect_roles(value: &serde_json::Value, counts: &mut HashMap<String, usize>) {
    if let Some(role) = value["role"].as_str() {
        if !role.is_empty() {
            *counts.entry(role.to_string()).or_insert(0) += 1;
        }
    }
    if let Some(children) = value["children"].as_array() {
        for child in children {
            collect_roles(child, counts);
        }
    }
}

// =============================================================================
// Compact tree (token-efficient mode)
// =============================================================================

/// Produce a pruned copy of the snapshot tree that keeps only interactive
/// elements (those with a UID) and semantically meaningful landmark/structure
/// nodes, while collapsing decorative noise like `InlineTextBox`, `LineBreak`,
/// and empty `generic` wrappers.
pub fn compact_tree(root: &SnapshotNode) -> SnapshotNode {
    // Root is always kept (it's RootWebArea/document, both in COMPACT_KEPT_ROLES).
    compact_node(root).unwrap_or_else(|| SnapshotNode {
        role: root.role.clone(),
        name: root.name.clone(),
        uid: root.uid.clone(),
        properties: root.properties.clone(),
        backend_dom_node_id: root.backend_dom_node_id,
        children: vec![],
    })
}

/// Recursively decide whether to keep a node and compact its children.
///
/// Returns `None` when the node (and entire subtree) should be removed.
fn compact_node(node: &SnapshotNode) -> Option<SnapshotNode> {
    // Rule 1: Always exclude noise roles.
    if COMPACT_EXCLUDED_ROLES.contains(&node.role.as_str()) {
        return None;
    }

    let has_uid = node.uid.is_some();
    let is_kept_role = COMPACT_KEPT_ROLES.contains(&node.role.as_str());
    let is_interactive = INTERACTIVE_ROLES.contains(&node.role.as_str());

    // Rule 2: Always keep nodes with a UID or in COMPACT_KEPT_ROLES.
    // Rule 3: Other nodes kept only if they have interactive descendants.
    let keep = has_uid || is_kept_role || is_interactive || has_interactive_in_subtree(node);

    if !keep {
        return None;
    }

    // Rule 4: Text inlining — if a kept node with an empty name has a single
    // StaticText child (before compacting), absorb the text into the parent's name.
    // This must be checked before recursive compacting since StaticText nodes
    // would otherwise be filtered out.
    if node.name.is_empty() && node.children.len() == 1 && node.children[0].role == "StaticText" {
        return Some(SnapshotNode {
            role: node.role.clone(),
            name: node.children[0].name.clone(),
            uid: node.uid.clone(),
            properties: node.properties.clone(),
            backend_dom_node_id: node.backend_dom_node_id,
            children: vec![],
        });
    }

    // Recursively compact children.
    let children: Vec<SnapshotNode> = node.children.iter().filter_map(compact_node).collect();

    Some(SnapshotNode {
        role: node.role.clone(),
        name: node.name.clone(),
        uid: node.uid.clone(),
        properties: node.properties.clone(),
        backend_dom_node_id: node.backend_dom_node_id,
        children,
    })
}

/// Returns `true` if any node in the subtree (excluding the node itself) has a UID.
fn has_interactive_in_subtree(node: &SnapshotNode) -> bool {
    for child in &node.children {
        if child.uid.is_some() || INTERACTIVE_ROLES.contains(&child.role.as_str()) {
            return true;
        }
        if has_interactive_in_subtree(child) {
            return true;
        }
    }
    false
}

// =============================================================================
// Snapshot state persistence
// =============================================================================

/// Persisted UID-to-backend-node mapping for use by interaction commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotState {
    pub url: String,
    pub timestamp: String,
    pub uid_map: HashMap<String, i64>,
}

/// Errors from snapshot state file operations.
#[derive(Debug)]
pub enum SnapshotStateError {
    NoHomeDir,
    Io(std::io::Error),
    InvalidFormat(String),
}

impl fmt::Display for SnapshotStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHomeDir => write!(f, "could not determine home directory"),
            Self::Io(e) => write!(f, "snapshot state file error: {e}"),
            Self::InvalidFormat(e) => write!(f, "invalid snapshot state file: {e}"),
        }
    }
}

impl std::error::Error for SnapshotStateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SnapshotStateError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<SnapshotStateError> for AppError {
    fn from(e: SnapshotStateError) -> Self {
        use agentchrome::error::ExitCode;
        Self {
            message: e.to_string(),
            code: ExitCode::GeneralError,
            custom_json: None,
        }
    }
}

/// Returns the path to `~/.agentchrome/snapshot.json`.
fn snapshot_state_path() -> Result<PathBuf, SnapshotStateError> {
    #[cfg(unix)]
    let key = "HOME";
    #[cfg(windows)]
    let key = "USERPROFILE";

    let home = std::env::var(key)
        .map(PathBuf::from)
        .map_err(|_| SnapshotStateError::NoHomeDir)?;
    Ok(home.join(".agentchrome").join("snapshot.json"))
}

/// Write snapshot state to `~/.agentchrome/snapshot.json` using atomic write.
pub fn write_snapshot_state(state: &SnapshotState) -> Result<(), SnapshotStateError> {
    let path = snapshot_state_path()?;
    write_snapshot_state_to(&path, state)
}

/// Write snapshot state to a specific path (testable variant).
pub fn write_snapshot_state_to(
    path: &std::path::Path,
    state: &SnapshotState,
) -> Result<(), SnapshotStateError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
    }

    let json = serde_json::to_string_pretty(state)
        .map_err(|e| SnapshotStateError::InvalidFormat(e.to_string()))?;

    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, &json)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600))?;
    }

    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Read snapshot state from `~/.agentchrome/snapshot.json`.
///
/// Returns `Ok(None)` if the file does not exist.
///
/// Used by interaction commands (#14-#17) to resolve UIDs to backend node IDs.
#[allow(dead_code)]
pub fn read_snapshot_state() -> Result<Option<SnapshotState>, SnapshotStateError> {
    let path = snapshot_state_path()?;
    read_snapshot_state_from(&path)
}

/// Read snapshot state from a specific path (testable variant).
#[allow(dead_code)]
pub fn read_snapshot_state_from(
    path: &std::path::Path,
) -> Result<Option<SnapshotState>, SnapshotStateError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let state: SnapshotState = serde_json::from_str(&contents)
                .map_err(|e| SnapshotStateError::InvalidFormat(e.to_string()))?;
            Ok(Some(state))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(SnapshotStateError::Io(e)),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_cdp_nodes() -> Vec<serde_json::Value> {
        vec![
            json!({
                "nodeId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "document"},
                "name": {"type": "computedString", "value": "Example Domain"},
                "properties": [],
                "childIds": ["2", "3", "4"],
                "backendDOMNodeId": 1
            }),
            json!({
                "nodeId": "2",
                "ignored": false,
                "role": {"type": "role", "value": "heading"},
                "name": {"type": "computedString", "value": "Example Domain"},
                "properties": [
                    {"name": "level", "value": {"type": "integer", "value": 1}}
                ],
                "childIds": [],
                "backendDOMNodeId": 10
            }),
            json!({
                "nodeId": "3",
                "ignored": false,
                "role": {"type": "role", "value": "paragraph"},
                "name": {"type": "computedString", "value": ""},
                "properties": [],
                "childIds": ["5"],
                "backendDOMNodeId": 20
            }),
            json!({
                "nodeId": "4",
                "ignored": false,
                "role": {"type": "role", "value": "link"},
                "name": {"type": "computedString", "value": "More information..."},
                "properties": [
                    {"name": "url", "value": {"type": "string", "value": "https://www.iana.org/domains/example"}}
                ],
                "childIds": [],
                "backendDOMNodeId": 30
            }),
            json!({
                "nodeId": "5",
                "ignored": false,
                "role": {"type": "role", "value": "text"},
                "name": {"type": "computedString", "value": "This domain is for use in..."},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 25
            }),
        ]
    }

    #[test]
    fn build_tree_produces_correct_hierarchy() {
        let nodes = sample_cdp_nodes();
        let result = build_tree(&nodes, false);
        assert_eq!(result.root.role, "document");
        assert_eq!(result.root.name, "Example Domain");
        assert_eq!(result.root.children.len(), 3);
        assert_eq!(result.root.children[0].role, "heading");
        assert_eq!(result.root.children[1].role, "paragraph");
        assert_eq!(result.root.children[2].role, "link");
    }

    #[test]
    fn build_tree_assigns_uids_to_interactive_only() {
        let nodes = sample_cdp_nodes();
        let result = build_tree(&nodes, false);

        // document — not interactive
        assert!(result.root.uid.is_none());
        // heading — not interactive
        assert!(result.root.children[0].uid.is_none());
        // paragraph — not interactive
        assert!(result.root.children[1].uid.is_none());
        // link — interactive
        assert_eq!(result.root.children[2].uid.as_deref(), Some("s1"));
    }

    #[test]
    fn build_tree_uid_map_contains_backend_ids() {
        let nodes = sample_cdp_nodes();
        let result = build_tree(&nodes, false);
        assert_eq!(result.uid_map.len(), 1);
        assert_eq!(result.uid_map.get("s1"), Some(&30));
    }

    #[test]
    fn build_tree_deterministic_uid_order() {
        // Two interactive elements: button then link
        let nodes = vec![
            json!({
                "nodeId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "document"},
                "name": {"type": "computedString", "value": "Test"},
                "properties": [],
                "childIds": ["2", "3"],
                "backendDOMNodeId": 1
            }),
            json!({
                "nodeId": "2",
                "ignored": false,
                "role": {"type": "role", "value": "button"},
                "name": {"type": "computedString", "value": "Click"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 10
            }),
            json!({
                "nodeId": "3",
                "ignored": false,
                "role": {"type": "role", "value": "link"},
                "name": {"type": "computedString", "value": "Go"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 20
            }),
        ];
        let result = build_tree(&nodes, false);
        assert_eq!(result.root.children[0].uid.as_deref(), Some("s1"));
        assert_eq!(result.root.children[1].uid.as_deref(), Some("s2"));
        assert_eq!(result.uid_map.get("s1"), Some(&10));
        assert_eq!(result.uid_map.get("s2"), Some(&20));
    }

    #[test]
    fn build_tree_filters_ignored_nodes() {
        let nodes = vec![
            json!({
                "nodeId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "document"},
                "name": {"type": "computedString", "value": "Doc"},
                "properties": [],
                "childIds": ["2", "3"],
                "backendDOMNodeId": 1
            }),
            json!({
                "nodeId": "2",
                "ignored": true,
                "role": {"type": "role", "value": "generic"},
                "name": {"type": "computedString", "value": ""},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 10
            }),
            json!({
                "nodeId": "3",
                "ignored": false,
                "role": {"type": "role", "value": "button"},
                "name": {"type": "computedString", "value": "OK"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 20
            }),
        ];
        let result = build_tree(&nodes, false);
        assert_eq!(result.root.children.len(), 1);
        assert_eq!(result.root.children[0].role, "button");
    }

    #[test]
    fn build_tree_promotes_children_of_ignored_ancestor() {
        // Root → ignored(id=2) → heading(id=3), link(id=4)
        // The heading and link should be promoted to root's children.
        let nodes = vec![
            json!({
                "nodeId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "RootWebArea"},
                "name": {"type": "computedString", "value": "Example"},
                "properties": [],
                "childIds": ["2"],
                "backendDOMNodeId": 1
            }),
            json!({
                "nodeId": "2",
                "ignored": true,
                "role": {"type": "role", "value": "none"},
                "name": {"type": "computedString", "value": ""},
                "properties": [],
                "childIds": ["3", "4"],
                "backendDOMNodeId": 10
            }),
            json!({
                "nodeId": "3",
                "ignored": false,
                "role": {"type": "role", "value": "heading"},
                "name": {"type": "computedString", "value": "Example Domain"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 20
            }),
            json!({
                "nodeId": "4",
                "ignored": false,
                "role": {"type": "role", "value": "link"},
                "name": {"type": "computedString", "value": "Learn more"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 30
            }),
        ];
        let result = build_tree(&nodes, false);
        assert_eq!(result.root.role, "RootWebArea");
        assert_eq!(result.root.children.len(), 2);
        assert_eq!(result.root.children[0].role, "heading");
        assert_eq!(result.root.children[0].name, "Example Domain");
        assert_eq!(result.root.children[1].role, "link");
        assert_eq!(result.root.children[1].name, "Learn more");
    }

    #[test]
    fn build_tree_deeply_nested_ignored_chain_promotes_through_all() {
        // Root → ignored(id=2) → ignored(id=3) → ignored(id=4) → heading(id=5), paragraph(id=6)
        let nodes = vec![
            json!({
                "nodeId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "RootWebArea"},
                "name": {"type": "computedString", "value": "Deep"},
                "properties": [],
                "childIds": ["2"],
                "backendDOMNodeId": 1
            }),
            json!({
                "nodeId": "2",
                "ignored": true,
                "role": {"type": "role", "value": "none"},
                "name": {"type": "computedString", "value": ""},
                "properties": [],
                "childIds": ["3"],
                "backendDOMNodeId": 10
            }),
            json!({
                "nodeId": "3",
                "ignored": true,
                "role": {"type": "role", "value": "none"},
                "name": {"type": "computedString", "value": ""},
                "properties": [],
                "childIds": ["4"],
                "backendDOMNodeId": 11
            }),
            json!({
                "nodeId": "4",
                "ignored": true,
                "role": {"type": "role", "value": "none"},
                "name": {"type": "computedString", "value": ""},
                "properties": [],
                "childIds": ["5", "6"],
                "backendDOMNodeId": 12
            }),
            json!({
                "nodeId": "5",
                "ignored": false,
                "role": {"type": "role", "value": "heading"},
                "name": {"type": "computedString", "value": "Title"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 20
            }),
            json!({
                "nodeId": "6",
                "ignored": false,
                "role": {"type": "role", "value": "paragraph"},
                "name": {"type": "computedString", "value": "Content"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 30
            }),
        ];
        let result = build_tree(&nodes, false);
        assert_eq!(result.root.role, "RootWebArea");
        assert_eq!(result.root.children.len(), 2);
        assert_eq!(result.root.children[0].role, "heading");
        assert_eq!(result.root.children[0].name, "Title");
        assert_eq!(result.root.children[1].role, "paragraph");
        assert_eq!(result.root.children[1].name, "Content");
    }

    #[test]
    fn build_tree_ignored_ancestor_interactive_children_get_uids() {
        // Root → ignored(id=2) → button(id=3), link(id=4)
        // Interactive children promoted through ignored ancestor should get UIDs.
        let nodes = vec![
            json!({
                "nodeId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "RootWebArea"},
                "name": {"type": "computedString", "value": "Page"},
                "properties": [],
                "childIds": ["2"],
                "backendDOMNodeId": 1
            }),
            json!({
                "nodeId": "2",
                "ignored": true,
                "role": {"type": "role", "value": "none"},
                "name": {"type": "computedString", "value": ""},
                "properties": [],
                "childIds": ["3", "4"],
                "backendDOMNodeId": 10
            }),
            json!({
                "nodeId": "3",
                "ignored": false,
                "role": {"type": "role", "value": "button"},
                "name": {"type": "computedString", "value": "Submit"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 20
            }),
            json!({
                "nodeId": "4",
                "ignored": false,
                "role": {"type": "role", "value": "link"},
                "name": {"type": "computedString", "value": "Cancel"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 30
            }),
        ];
        let result = build_tree(&nodes, false);
        assert_eq!(result.root.children.len(), 2);
        assert_eq!(result.root.children[0].uid.as_deref(), Some("s1"));
        assert_eq!(result.root.children[0].role, "button");
        assert_eq!(result.root.children[1].uid.as_deref(), Some("s2"));
        assert_eq!(result.root.children[1].role, "link");
        assert_eq!(result.uid_map.get("s1"), Some(&20));
        assert_eq!(result.uid_map.get("s2"), Some(&30));
    }

    #[test]
    fn build_tree_empty_nodes() {
        let result = build_tree(&[], false);
        assert_eq!(result.root.role, "document");
        assert_eq!(result.root.name, "");
        assert!(result.root.children.is_empty());
    }

    #[test]
    fn format_text_basic() {
        let nodes = sample_cdp_nodes();
        let result = build_tree(&nodes, false);
        let text = format_text(&result.root, false);
        assert!(text.contains("- document \"Example Domain\""));
        assert!(text.contains("  - heading \"Example Domain\""));
        assert!(text.contains("  - link \"More information...\" [s1]"));
        assert!(text.contains("    - text \"This domain is for use in...\""));
    }

    #[test]
    fn format_text_verbose_includes_properties() {
        let nodes = sample_cdp_nodes();
        let result = build_tree(&nodes, true);
        let text = format_text(&result.root, true);
        assert!(text.contains("level=1"), "text was: {text}");
        assert!(
            text.contains("url=\"https://www.iana.org/domains/example\""),
            "text was: {text}"
        );
    }

    #[test]
    fn format_text_empty_tree() {
        let result = build_tree(&[], false);
        let text = format_text(&result.root, false);
        assert!(text.contains("- document \"\""));
    }

    #[test]
    fn snapshot_node_serialization() {
        let node = SnapshotNode {
            role: "button".to_string(),
            name: "Submit".to_string(),
            uid: Some("s1".to_string()),
            properties: None,
            backend_dom_node_id: Some(10),
            children: vec![],
        };
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["role"], "button");
        assert_eq!(json["name"], "Submit");
        assert_eq!(json["uid"], "s1");
        assert!(json.get("properties").is_none());
        // backend_dom_node_id is #[serde(skip)] — must not appear in JSON
        assert!(json.get("backend_dom_node_id").is_none());
        assert!(json.get("backendDOMNodeId").is_none());
    }

    #[test]
    fn snapshot_node_serialization_no_uid() {
        let node = SnapshotNode {
            role: "paragraph".to_string(),
            name: "Hello".to_string(),
            uid: None,
            properties: None,
            backend_dom_node_id: Some(20),
            children: vec![],
        };
        let json = serde_json::to_value(&node).unwrap();
        assert!(json.get("uid").is_none());
        assert!(json.get("backend_dom_node_id").is_none());
    }

    #[test]
    fn snapshot_state_write_read_round_trip() {
        let dir = std::env::temp_dir().join("agentchrome-test-snapshot-rt");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("snapshot.json");

        let state = SnapshotState {
            url: "https://example.com/".to_string(),
            timestamp: "2026-02-12T10:00:00Z".to_string(),
            uid_map: HashMap::from([("s1".to_string(), 42), ("s2".to_string(), 87)]),
        };

        write_snapshot_state_to(&path, &state).unwrap();
        let read = read_snapshot_state_from(&path).unwrap().unwrap();

        assert_eq!(read.url, state.url);
        assert_eq!(read.timestamp, state.timestamp);
        assert_eq!(read.uid_map, state.uid_map);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_snapshot_state_nonexistent_returns_none() {
        let path = std::path::Path::new("/tmp/agentchrome-test-snap-nonexistent/snapshot.json");
        let result = read_snapshot_state_from(path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn truncation_large_tree() {
        // Create a flat tree with > MAX_NODES children
        let mut nodes = vec![json!({
            "nodeId": "root",
            "ignored": false,
            "role": {"type": "role", "value": "document"},
            "name": {"type": "computedString", "value": "Big"},
            "properties": [],
            "childIds": (1..=10_001).map(|i| serde_json::Value::String(format!("n{i}"))).collect::<Vec<_>>(),
            "backendDOMNodeId": 0
        })];
        for i in 1..=10_001 {
            nodes.push(json!({
                "nodeId": format!("n{i}"),
                "ignored": false,
                "role": {"type": "role", "value": "text"},
                "name": {"type": "computedString", "value": format!("Item {i}")},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": i
            }));
        }

        let result = build_tree(&nodes, false);
        assert!(result.truncated);
        assert_eq!(result.total_nodes, 10_002);
        // Should have fewer children than total
        assert!(result.root.children.len() < 10_001);
    }

    #[test]
    fn snapshot_state_error_display() {
        assert_eq!(
            SnapshotStateError::NoHomeDir.to_string(),
            "could not determine home directory"
        );
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        assert_eq!(
            SnapshotStateError::Io(io_err).to_string(),
            "snapshot state file error: denied"
        );
        assert_eq!(
            SnapshotStateError::InvalidFormat("bad".into()).to_string(),
            "invalid snapshot state file: bad"
        );
    }

    // =========================================================================
    // search_tree tests
    // =========================================================================

    fn search_test_nodes() -> Vec<serde_json::Value> {
        vec![
            json!({
                "nodeId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "document"},
                "name": {"type": "computedString", "value": "Test Page"},
                "properties": [],
                "childIds": ["2", "3", "4", "5"],
                "backendDOMNodeId": 1
            }),
            json!({
                "nodeId": "2",
                "ignored": false,
                "role": {"type": "role", "value": "button"},
                "name": {"type": "computedString", "value": "Submit"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 10
            }),
            json!({
                "nodeId": "3",
                "ignored": false,
                "role": {"type": "role", "value": "button"},
                "name": {"type": "computedString", "value": "Login"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 20
            }),
            json!({
                "nodeId": "4",
                "ignored": false,
                "role": {"type": "role", "value": "link"},
                "name": {"type": "computedString", "value": "Log out"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 30
            }),
            json!({
                "nodeId": "5",
                "ignored": false,
                "role": {"type": "role", "value": "heading"},
                "name": {"type": "computedString", "value": "Submit Your Application"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 40
            }),
        ]
    }

    #[test]
    fn search_tree_substring_match() {
        let nodes = search_test_nodes();
        let build = build_tree(&nodes, false);
        let hits = search_tree(&build.root, "Submit", None, false, 10);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].name, "Submit");
        assert_eq!(hits[0].role, "button");
        assert_eq!(hits[1].name, "Submit Your Application");
        assert_eq!(hits[1].role, "heading");
    }

    #[test]
    fn search_tree_case_insensitive() {
        let nodes = search_test_nodes();
        let build = build_tree(&nodes, false);
        let hits = search_tree(&build.root, "submit", None, false, 10);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn search_tree_exact_match() {
        let nodes = search_test_nodes();
        let build = build_tree(&nodes, false);
        let hits = search_tree(&build.root, "Submit", None, true, 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Submit");
    }

    #[test]
    fn search_tree_exact_match_case_sensitive() {
        let nodes = search_test_nodes();
        let build = build_tree(&nodes, false);
        let hits = search_tree(&build.root, "submit", None, true, 10);
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn search_tree_role_filter() {
        let nodes = search_test_nodes();
        let build = build_tree(&nodes, false);
        let hits = search_tree(&build.root, "Log", Some("button"), false, 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Login");
        assert_eq!(hits[0].role, "button");
    }

    #[test]
    fn search_tree_combined_role_and_text() {
        let nodes = search_test_nodes();
        let build = build_tree(&nodes, false);
        let hits = search_tree(&build.root, "Log", Some("link"), false, 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Log out");
        assert_eq!(hits[0].role, "link");
    }

    #[test]
    fn search_tree_limit() {
        let nodes = search_test_nodes();
        let build = build_tree(&nodes, false);
        // All nodes match empty query
        let hits = search_tree(&build.root, "", None, false, 2);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn search_tree_no_matches() {
        let nodes = search_test_nodes();
        let build = build_tree(&nodes, false);
        let hits = search_tree(&build.root, "nonexistent", None, false, 10);
        assert!(hits.is_empty());
    }

    #[test]
    fn search_tree_empty_tree() {
        let build = build_tree(&[], false);
        let hits = search_tree(&build.root, "anything", None, false, 10);
        assert!(hits.is_empty());
    }

    #[test]
    fn search_tree_includes_backend_dom_node_id() {
        let nodes = search_test_nodes();
        let build = build_tree(&nodes, false);
        let hits = search_tree(&build.root, "Submit", Some("button"), false, 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].uid.as_deref(), Some("s1"));
        assert_eq!(hits[0].backend_dom_node_id, Some(10));
    }

    #[test]
    fn search_tree_non_interactive_includes_backend_id() {
        let nodes = search_test_nodes();
        let build = build_tree(&nodes, false);
        // Heading is non-interactive (no uid) but should still have backend_dom_node_id
        let hits = search_tree(
            &build.root,
            "Submit Your Application",
            Some("heading"),
            false,
            10,
        );
        assert_eq!(hits.len(), 1);
        assert!(hits[0].uid.is_none()); // heading is not interactive
        assert_eq!(hits[0].backend_dom_node_id, Some(40)); // but backend ID is available
    }

    #[test]
    fn search_tree_document_order() {
        let nodes = search_test_nodes();
        let build = build_tree(&nodes, false);
        // Empty query matches all nodes, verifying depth-first order
        let hits = search_tree(&build.root, "", None, false, 100);
        let roles: Vec<&str> = hits.iter().map(|h| h.role.as_str()).collect();
        assert_eq!(roles, ["document", "button", "button", "link", "heading"]);
    }

    // =========================================================================
    // parentId fallback tests (issue #73)
    // =========================================================================

    /// CDP response with parentId but empty childIds — simulates the bug scenario
    /// where Chrome returns nodes without top-down child references.
    fn parent_id_only_nodes() -> Vec<serde_json::Value> {
        vec![
            json!({
                "nodeId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "RootWebArea"},
                "name": {"type": "computedString", "value": "Google"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 1
            }),
            json!({
                "nodeId": "2",
                "parentId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "textbox"},
                "name": {"type": "computedString", "value": "Search"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 10
            }),
            json!({
                "nodeId": "3",
                "parentId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "button"},
                "name": {"type": "computedString", "value": "Google Search"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 20
            }),
            json!({
                "nodeId": "4",
                "parentId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "link"},
                "name": {"type": "computedString", "value": "About"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 30
            }),
        ]
    }

    #[test]
    fn build_tree_parent_id_fallback_produces_populated_tree() {
        let nodes = parent_id_only_nodes();
        let result = build_tree(&nodes, false);
        assert_eq!(result.root.role, "RootWebArea");
        assert_eq!(result.root.name, "Google");
        assert_eq!(result.root.children.len(), 3);
        assert_eq!(result.root.children[0].role, "textbox");
        assert_eq!(result.root.children[1].role, "button");
        assert_eq!(result.root.children[2].role, "link");
    }

    #[test]
    fn build_tree_parent_id_fallback_assigns_uids() {
        let nodes = parent_id_only_nodes();
        let result = build_tree(&nodes, false);
        // All three children are interactive roles
        assert_eq!(result.root.children[0].uid.as_deref(), Some("s1")); // textbox
        assert_eq!(result.root.children[1].uid.as_deref(), Some("s2")); // button
        assert_eq!(result.root.children[2].uid.as_deref(), Some("s3")); // link
        assert_eq!(result.uid_map.len(), 3);
        assert_eq!(result.uid_map.get("s1"), Some(&10));
        assert_eq!(result.uid_map.get("s2"), Some(&20));
        assert_eq!(result.uid_map.get("s3"), Some(&30));
    }

    #[test]
    fn build_tree_parent_id_fallback_nested_hierarchy() {
        // Tests multi-level tree reconstruction from parentId
        let nodes = vec![
            json!({
                "nodeId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "RootWebArea"},
                "name": {"type": "computedString", "value": "Page"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 1
            }),
            json!({
                "nodeId": "2",
                "parentId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "navigation"},
                "name": {"type": "computedString", "value": "Nav"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 10
            }),
            json!({
                "nodeId": "3",
                "parentId": "2",
                "ignored": false,
                "role": {"type": "role", "value": "link"},
                "name": {"type": "computedString", "value": "Home"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 20
            }),
            json!({
                "nodeId": "4",
                "parentId": "2",
                "ignored": false,
                "role": {"type": "role", "value": "link"},
                "name": {"type": "computedString", "value": "About"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 30
            }),
        ];
        let result = build_tree(&nodes, false);
        assert_eq!(result.root.children.len(), 1); // navigation
        let nav = &result.root.children[0];
        assert_eq!(nav.role, "navigation");
        assert_eq!(nav.children.len(), 2); // two links
        assert_eq!(nav.children[0].role, "link");
        assert_eq!(nav.children[0].name, "Home");
        assert_eq!(nav.children[1].role, "link");
        assert_eq!(nav.children[1].name, "About");
    }

    #[test]
    fn build_tree_child_ids_take_precedence_over_parent_id() {
        // When childIds are present, parentId should NOT cause duplication
        let nodes = vec![
            json!({
                "nodeId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "document"},
                "name": {"type": "computedString", "value": "Test"},
                "properties": [],
                "childIds": ["2", "3"],
                "backendDOMNodeId": 1
            }),
            json!({
                "nodeId": "2",
                "parentId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "button"},
                "name": {"type": "computedString", "value": "OK"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 10
            }),
            json!({
                "nodeId": "3",
                "parentId": "1",
                "ignored": false,
                "role": {"type": "role", "value": "link"},
                "name": {"type": "computedString", "value": "More"},
                "properties": [],
                "childIds": [],
                "backendDOMNodeId": 20
            }),
        ];
        let result = build_tree(&nodes, false);
        // childIds path should be used — root has childIds so fallback does NOT activate
        assert_eq!(result.root.children.len(), 2);
        assert_eq!(result.root.children[0].role, "button");
        assert_eq!(result.root.children[1].role, "link");
    }

    // =========================================================================
    // count_nodes / top_roles tests
    // =========================================================================

    #[test]
    fn count_nodes_simple() {
        let nodes = sample_cdp_nodes();
        let build = build_tree(&nodes, false);
        let value = serde_json::to_value(&build.root).unwrap();
        let count = count_nodes(&value);
        // document + heading + paragraph + text + link = 5
        assert_eq!(count, 5);
    }

    #[test]
    fn top_roles_returns_sorted_by_frequency() {
        let nodes = search_test_nodes();
        let build = build_tree(&nodes, false);
        let value = serde_json::to_value(&build.root).unwrap();
        let roles = top_roles(&value, 3);
        // button appears twice, others once
        assert_eq!(roles[0], "button");
        assert!(roles.len() <= 3);
    }

    // =========================================================================
    // compact_tree tests
    // =========================================================================

    /// Helper to build a `SnapshotNode` literal for compact tests.
    fn sn(role: &str, name: &str, uid: Option<&str>, children: Vec<SnapshotNode>) -> SnapshotNode {
        SnapshotNode {
            role: role.to_string(),
            name: name.to_string(),
            uid: uid.map(String::from),
            properties: None,
            backend_dom_node_id: None,
            children,
        }
    }

    #[test]
    fn compact_tree_empty_tree() {
        let root = sn("document", "", None, vec![]);
        let result = compact_tree(&root);
        assert_eq!(result.role, "document");
        assert!(result.children.is_empty());
    }

    #[test]
    fn compact_tree_interactive_only_kept() {
        let root = sn(
            "document",
            "Page",
            None,
            vec![
                sn("button", "Submit", Some("s1"), vec![]),
                sn("link", "Home", Some("s2"), vec![]),
            ],
        );
        let result = compact_tree(&root);
        assert_eq!(result.children.len(), 2);
        assert_eq!(result.children[0].uid.as_deref(), Some("s1"));
        assert_eq!(result.children[1].uid.as_deref(), Some("s2"));
    }

    #[test]
    fn compact_tree_excluded_roles_removed() {
        let root = sn(
            "document",
            "Page",
            None,
            vec![
                sn("InlineTextBox", "text", None, vec![]),
                sn("LineBreak", "", None, vec![]),
                sn("button", "OK", Some("s1"), vec![]),
            ],
        );
        let result = compact_tree(&root);
        assert_eq!(result.children.len(), 1);
        assert_eq!(result.children[0].role, "button");
    }

    #[test]
    fn compact_tree_kept_roles_preserved() {
        let root = sn(
            "RootWebArea",
            "Page",
            None,
            vec![
                sn(
                    "navigation",
                    "Nav",
                    None,
                    vec![sn("link", "Home", Some("s1"), vec![])],
                ),
                sn("heading", "Title", None, vec![]),
                sn("main", "Content", None, vec![]),
            ],
        );
        let result = compact_tree(&root);
        assert_eq!(result.children.len(), 3);
        assert_eq!(result.children[0].role, "navigation");
        assert_eq!(result.children[1].role, "heading");
        assert_eq!(result.children[2].role, "main");
    }

    #[test]
    fn compact_tree_generic_with_interactive_descendants_kept() {
        let root = sn(
            "document",
            "Page",
            None,
            vec![sn(
                "generic",
                "",
                None,
                vec![sn("button", "Click", Some("s1"), vec![])],
            )],
        );
        let result = compact_tree(&root);
        assert_eq!(result.children.len(), 1);
        assert_eq!(result.children[0].role, "generic");
        assert_eq!(result.children[0].children[0].role, "button");
    }

    #[test]
    fn compact_tree_generic_without_interactive_descendants_removed() {
        let root = sn(
            "document",
            "Page",
            None,
            vec![sn(
                "generic",
                "",
                None,
                vec![sn("StaticText", "Just text", None, vec![])],
            )],
        );
        let result = compact_tree(&root);
        assert!(result.children.is_empty());
    }

    #[test]
    fn compact_tree_text_inlining() {
        // A heading with empty name and a single StaticText child should absorb the text.
        let root = sn(
            "document",
            "Page",
            None,
            vec![sn(
                "heading",
                "",
                None,
                vec![sn("StaticText", "Welcome", None, vec![])],
            )],
        );
        let result = compact_tree(&root);
        assert_eq!(result.children.len(), 1);
        assert_eq!(result.children[0].role, "heading");
        assert_eq!(result.children[0].name, "Welcome");
        assert!(result.children[0].children.is_empty());
    }

    #[test]
    fn compact_tree_text_inlining_skipped_when_name_present() {
        let root = sn(
            "document",
            "Page",
            None,
            vec![sn(
                "heading",
                "Already Named",
                None,
                vec![sn("StaticText", "Ignored", None, vec![])],
            )],
        );
        let result = compact_tree(&root);
        assert_eq!(result.children[0].name, "Already Named");
    }

    #[test]
    fn compact_tree_uid_preservation() {
        fn collect_uids(node: &SnapshotNode, uids: &mut Vec<String>) {
            if let Some(ref uid) = node.uid {
                uids.push(uid.clone());
            }
            for child in &node.children {
                collect_uids(child, uids);
            }
        }

        let root = sn(
            "document",
            "Page",
            None,
            vec![
                sn(
                    "generic",
                    "",
                    None,
                    vec![
                        sn("button", "A", Some("s1"), vec![]),
                        sn(
                            "generic",
                            "",
                            None,
                            vec![sn("textbox", "B", Some("s2"), vec![])],
                        ),
                    ],
                ),
                sn("link", "C", Some("s3"), vec![]),
            ],
        );

        let result = compact_tree(&root);
        let mut uids = vec![];
        collect_uids(&result, &mut uids);
        uids.sort();
        assert_eq!(uids, vec!["s1", "s2", "s3"]);
    }

    #[test]
    fn compact_tree_hierarchy_context_preserved() {
        // navigation > generic > button should keep the generic as hierarchy context
        let root = sn(
            "RootWebArea",
            "Page",
            None,
            vec![sn(
                "navigation",
                "Nav",
                None,
                vec![sn(
                    "list",
                    "",
                    None,
                    vec![sn(
                        "listitem",
                        "",
                        None,
                        vec![sn("link", "Home", Some("s1"), vec![])],
                    )],
                )],
            )],
        );
        let result = compact_tree(&root);
        // navigation > list > listitem > link hierarchy should be preserved
        assert_eq!(result.children[0].role, "navigation");
        assert_eq!(result.children[0].children[0].role, "list");
        assert_eq!(result.children[0].children[0].children[0].role, "listitem");
        assert_eq!(
            result.children[0].children[0].children[0].children[0].role,
            "link"
        );
    }

    #[test]
    fn compact_tree_deeply_nested_interactive_preserves_path() {
        let root = sn(
            "document",
            "Page",
            None,
            vec![sn(
                "generic",
                "",
                None,
                vec![sn(
                    "generic",
                    "",
                    None,
                    vec![sn("checkbox", "Agree", Some("s1"), vec![])],
                )],
            )],
        );
        let result = compact_tree(&root);
        // Both generic containers should be kept because they lead to an interactive node
        assert_eq!(result.children.len(), 1);
        assert_eq!(result.children[0].role, "generic");
        assert_eq!(result.children[0].children[0].role, "generic");
        assert_eq!(result.children[0].children[0].children[0].role, "checkbox");
    }
}
