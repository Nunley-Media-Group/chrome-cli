use std::collections::HashMap;
use std::fmt::Write;
use std::time::Duration;

use serde::Serialize;

use chrome_cli::cdp::{CdpClient, CdpConfig};
use chrome_cli::connection::{ManagedSession, resolve_connection, resolve_target};
use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{
    DomArgs, DomCommand, DomGetAttributeArgs, DomGetStyleArgs, DomNodeIdArgs, DomSelectArgs,
    DomSetAttributeArgs, DomSetStyleArgs, DomSetTextArgs, DomTreeArgs, GlobalOpts,
};
use crate::emulate::apply_emulate_state;
use crate::snapshot;

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct DomElement {
    #[serde(rename = "nodeId")]
    node_id: i64,
    tag: String,
    attributes: HashMap<String, String>,
    #[serde(rename = "textContent")]
    text_content: String,
}

#[derive(Serialize)]
struct AttributeResult {
    attribute: String,
    value: String,
}

#[derive(Serialize)]
struct TextResult {
    #[serde(rename = "textContent")]
    text_content: String,
}

#[derive(Serialize)]
struct HtmlResult {
    #[serde(rename = "outerHTML")]
    outer_html: String,
}

#[derive(Serialize)]
struct MutationResult {
    success: bool,
    #[serde(rename = "nodeId")]
    node_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    attribute: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

/// set-text result: AC7 requires `{"success":true,"nodeId":<id>,"textContent":"..."}`
#[derive(Serialize)]
struct SetTextResult {
    success: bool,
    #[serde(rename = "nodeId")]
    node_id: i64,
    #[serde(rename = "textContent")]
    text_content: String,
}

/// remove result: AC8 requires `{"success":true,"nodeId":<id>,"removed":true}`
#[derive(Serialize)]
struct RemoveResult {
    success: bool,
    #[serde(rename = "nodeId")]
    node_id: i64,
    removed: bool,
}

/// set-style result: AC15 requires `{"success":true,"nodeId":<id>,"style":"..."}`
#[derive(Serialize)]
struct SetStyleResult {
    success: bool,
    #[serde(rename = "nodeId")]
    node_id: i64,
    style: String,
}

#[derive(Serialize)]
struct StyleResult {
    styles: HashMap<String, String>,
}

#[derive(Serialize)]
struct StylePropertyResult {
    property: String,
    value: String,
}

#[derive(Serialize)]
struct TreeOutput {
    tree: String,
}

// =============================================================================
// Output formatting
// =============================================================================

fn print_output(value: &impl Serialize, output: &crate::cli::OutputFormat) -> Result<(), AppError> {
    let json = if output.pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };
    let json = json.map_err(|e| AppError {
        message: format!("serialization error: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;
    println!("{json}");
    Ok(())
}

// =============================================================================
// Config helper
// =============================================================================

fn cdp_config(global: &GlobalOpts) -> CdpConfig {
    let mut config = CdpConfig::default();
    if let Some(timeout_ms) = global.timeout {
        config.command_timeout = Duration::from_millis(timeout_ms);
    }
    config
}

// =============================================================================
// Session setup
// =============================================================================

async fn setup_session(global: &GlobalOpts) -> Result<(CdpClient, ManagedSession), AppError> {
    let conn = resolve_connection(&global.host, global.port, global.ws_url.as_deref()).await?;
    let target = resolve_target(&conn.host, conn.port, global.tab.as_deref()).await?;

    let config = cdp_config(global);
    let client = CdpClient::connect(&conn.ws_url, config).await?;
    let session = client.create_session(&target.id).await?;
    let mut managed = ManagedSession::new(session);
    apply_emulate_state(&mut managed).await?;
    managed.install_dialog_interceptors().await;

    Ok((client, managed))
}

// =============================================================================
// Core helpers
// =============================================================================

/// Get the document root nodeId.
async fn get_document_root(session: &ManagedSession) -> Result<i64, AppError> {
    let doc = session
        .send_command("DOM.getDocument", None)
        .await
        .map_err(|e| AppError {
            message: format!("DOM.getDocument failed: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;
    doc["root"]["nodeId"]
        .as_i64()
        .ok_or_else(|| AppError::node_not_found("root"))
}

/// Check if a target string is a UID (matches pattern: 's' + digits).
fn is_uid(target: &str) -> bool {
    if !target.starts_with('s') {
        return false;
    }
    let rest = &target[1..];
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

/// Check if a target string is a CSS selector (starts with 'css:').
fn is_css_selector(target: &str) -> bool {
    target.starts_with("css:")
}

/// Resolved node with both session-scoped `nodeId` and stable `backendNodeId`.
struct ResolvedNode {
    /// Session-scoped nodeId for CDP method calls.
    node_id: i64,
    /// Stable backendNodeId for user-facing output.
    backend_node_id: i64,
}

/// Unified node resolution: integer (backendNodeId), UID, or CSS selector → CDP nodeId.
///
/// Integer targets are treated as `backendNodeId` values (stable across sessions).
async fn resolve_node(session: &ManagedSession, target: &str) -> Result<ResolvedNode, AppError> {
    // Try integer (backendNodeId) first
    if let Ok(backend_node_id) = target.parse::<i64>() {
        let node_id = push_backend_node_to_frontend(session, backend_node_id, target).await?;
        return Ok(ResolvedNode {
            node_id,
            backend_node_id,
        });
    }

    // UID resolution
    if is_uid(target) {
        let state = snapshot::read_snapshot_state()?.ok_or_else(AppError::no_snapshot_state)?;
        let backend_node_id = state
            .uid_map
            .get(target)
            .copied()
            .ok_or_else(|| AppError::uid_not_found(target))?;

        let node_id = push_backend_node_to_frontend(session, backend_node_id, target)
            .await
            .map_err(|_| AppError::stale_uid(target))?;
        return Ok(ResolvedNode {
            node_id,
            backend_node_id,
        });
    }

    // CSS selector resolution
    if is_css_selector(target) {
        let selector = &target[4..];
        let root_id = get_document_root(session).await?;

        let query = session
            .send_command(
                "DOM.querySelector",
                Some(serde_json::json!({
                    "nodeId": root_id,
                    "selector": selector,
                })),
            )
            .await
            .map_err(|e| AppError {
                message: format!("CSS selector query failed: {e}"),
                code: ExitCode::ProtocolError,
                custom_json: None,
            })?;

        let node_id = query["nodeId"]
            .as_i64()
            .filter(|&id| id > 0)
            .ok_or_else(|| AppError::element_not_found(selector))?;
        // Get backendNodeId for the resolved node
        let backend_node_id = get_backend_node_id(session, node_id)
            .await
            .unwrap_or(node_id);
        return Ok(ResolvedNode {
            node_id,
            backend_node_id,
        });
    }

    Err(AppError::node_not_found(target))
}

/// Resolve a `backendNodeId` to a DOM-tracked session `nodeId`.
///
/// Uses `DOM.resolveNode` to get a Runtime object, then `DOM.requestNode` to
/// push it into the DOM agent's node map so it can be used with methods like
/// `DOM.getAttributes`, `DOM.getOuterHTML`, etc.
async fn push_backend_node_to_frontend(
    session: &ManagedSession,
    backend_node_id: i64,
    label: &str,
) -> Result<i64, AppError> {
    // Ensure DOM domain is aware of the document tree
    let _ = get_document_root(session).await?;

    // Resolve backendNodeId → Runtime.RemoteObject
    let resolve = session
        .send_command(
            "DOM.resolveNode",
            Some(serde_json::json!({ "backendNodeId": backend_node_id })),
        )
        .await
        .map_err(|_| AppError::node_not_found(label))?;

    let object_id = resolve["object"]["objectId"]
        .as_str()
        .ok_or_else(|| AppError::node_not_found(label))?;

    // Push the Runtime object into the DOM agent's tracked node map
    let request = session
        .send_command(
            "DOM.requestNode",
            Some(serde_json::json!({ "objectId": object_id })),
        )
        .await
        .map_err(|_| AppError::node_not_found(label))?;

    request["nodeId"]
        .as_i64()
        .filter(|&id| id > 0)
        .ok_or_else(|| AppError::node_not_found(label))
}

/// Get the backendNodeId for a session-scoped nodeId.
async fn get_backend_node_id(session: &ManagedSession, node_id: i64) -> Result<i64, AppError> {
    let describe = session
        .send_command(
            "DOM.describeNode",
            Some(serde_json::json!({ "nodeId": node_id })),
        )
        .await
        .map_err(|_| AppError::node_not_found(&node_id.to_string()))?;
    describe["node"]["backendNodeId"]
        .as_i64()
        .ok_or_else(|| AppError::node_not_found(&node_id.to_string()))
}

/// Describe a DOM element: tag, attributes, textContent.
///
/// Returns a `DomElement` whose `node_id` field is the **backendNodeId** (stable
/// across sessions), so that callers can use it in subsequent commands.
async fn describe_element(session: &ManagedSession, node_id: i64) -> Result<DomElement, AppError> {
    let describe = session
        .send_command(
            "DOM.describeNode",
            Some(serde_json::json!({ "nodeId": node_id })),
        )
        .await
        .map_err(|_| AppError::node_not_found(&node_id.to_string()))?;

    let node = &describe["node"];
    let tag = node["nodeName"].as_str().unwrap_or("").to_lowercase();
    let backend_node_id = node["backendNodeId"].as_i64().unwrap_or(node_id);

    // Parse attributes from flat [name, value, name, value, ...] array
    let mut attributes = HashMap::new();
    if let Some(attrs) = node["attributes"].as_array() {
        let mut i = 0;
        while i + 1 < attrs.len() {
            let name = attrs[i].as_str().unwrap_or("").to_string();
            let value = attrs[i + 1].as_str().unwrap_or("").to_string();
            attributes.insert(name, value);
            i += 2;
        }
    }

    // Get textContent via Runtime
    let text_content = get_text_content(session, node_id).await.unwrap_or_default();

    Ok(DomElement {
        node_id: backend_node_id,
        tag,
        attributes,
        text_content,
    })
}

/// Get the textContent of a node via Runtime.callFunctionOn.
async fn get_text_content(session: &ManagedSession, node_id: i64) -> Result<String, AppError> {
    let resolve = session
        .send_command(
            "DOM.resolveNode",
            Some(serde_json::json!({ "nodeId": node_id })),
        )
        .await?;

    let object_id = resolve["object"]["objectId"]
        .as_str()
        .ok_or_else(|| AppError::node_not_found(&node_id.to_string()))?;

    let call = session
        .send_command(
            "Runtime.callFunctionOn",
            Some(serde_json::json!({
                "objectId": object_id,
                "functionDeclaration": "function() { return this.textContent || ''; }",
                "returnByValue": true,
            })),
        )
        .await?;

    Ok(call["result"]["value"].as_str().unwrap_or("").to_string())
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Execute the `dom` subcommand group.
pub async fn execute_dom(global: &GlobalOpts, args: &DomArgs) -> Result<(), AppError> {
    match &args.command {
        DomCommand::Select(select_args) => execute_select(global, select_args).await,
        DomCommand::GetAttribute(attr_args) => execute_get_attribute(global, attr_args).await,
        DomCommand::GetText(node_args) => execute_get_text(global, node_args).await,
        DomCommand::GetHtml(node_args) => execute_get_html(global, node_args).await,
        DomCommand::SetAttribute(attr_args) => execute_set_attribute(global, attr_args).await,
        DomCommand::SetText(text_args) => execute_set_text(global, text_args).await,
        DomCommand::Remove(node_args) => execute_remove(global, node_args).await,
        DomCommand::GetStyle(style_args) => execute_get_style(global, style_args).await,
        DomCommand::SetStyle(style_args) => execute_set_style(global, style_args).await,
        DomCommand::Parent(node_args) => execute_parent(global, node_args).await,
        DomCommand::Children(node_args) => execute_children(global, node_args).await,
        DomCommand::Siblings(node_args) => execute_siblings(global, node_args).await,
        DomCommand::Tree(tree_args) => execute_tree(global, tree_args).await,
    }
}

// =============================================================================
// dom select
// =============================================================================

#[allow(clippy::too_many_lines)]
async fn execute_select(global: &GlobalOpts, args: &DomSelectArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;
    managed.ensure_domain("Runtime").await?;

    let root_id = get_document_root(&managed).await?;

    let node_ids = if args.xpath {
        // XPath via DOM.performSearch
        let search = managed
            .send_command(
                "DOM.performSearch",
                Some(serde_json::json!({ "query": args.selector })),
            )
            .await
            .map_err(|e| AppError {
                message: format!("XPath search failed: {e}"),
                code: ExitCode::ProtocolError,
                custom_json: None,
            })?;

        let search_id = search["searchId"].as_str().unwrap_or("").to_string();
        let count = search["resultCount"].as_i64().unwrap_or(0);

        let ids = if count > 0 {
            let results = managed
                .send_command(
                    "DOM.getSearchResults",
                    Some(serde_json::json!({
                        "searchId": search_id,
                        "fromIndex": 0,
                        "toIndex": count,
                    })),
                )
                .await?;

            results["nodeIds"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(serde_json::Value::as_i64)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            vec![]
        };

        // Clean up search
        let _ = managed
            .send_command(
                "DOM.discardSearchResults",
                Some(serde_json::json!({ "searchId": search_id })),
            )
            .await;

        ids
    } else {
        // CSS via DOM.querySelectorAll
        let query = managed
            .send_command(
                "DOM.querySelectorAll",
                Some(serde_json::json!({
                    "nodeId": root_id,
                    "selector": args.selector,
                })),
            )
            .await
            .map_err(|e| AppError {
                message: format!("CSS selector query failed: {e}"),
                code: ExitCode::ProtocolError,
                custom_json: None,
            })?;

        query["nodeIds"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(serde_json::Value::as_i64)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };

    // Describe each node
    let mut elements = Vec::with_capacity(node_ids.len());
    for nid in node_ids {
        if let Ok(el) = describe_element(&managed, nid).await {
            elements.push(el);
        }
    }

    if global.output.plain {
        for el in &elements {
            let attrs: Vec<String> = el
                .attributes
                .iter()
                .map(|(k, v)| format!("{k}=\"{v}\""))
                .collect();
            let attr_str = if attrs.is_empty() {
                String::new()
            } else {
                format!(" {}", attrs.join(" "))
            };
            let text = truncate_text(&el.text_content, 60);
            println!("[{}] <{}{}> \"{}\"", el.node_id, el.tag, attr_str, text);
        }
        return Ok(());
    }

    print_output(&elements, &global.output)
}

// =============================================================================
// dom get-attribute
// =============================================================================

async fn execute_get_attribute(
    global: &GlobalOpts,
    args: &DomGetAttributeArgs,
) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;

    let resolved = resolve_node(&managed, &args.node_id).await?;

    let attrs = managed
        .send_command(
            "DOM.getAttributes",
            Some(serde_json::json!({ "nodeId": resolved.node_id })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("Failed to get attributes: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let attr_array = attrs["attributes"]
        .as_array()
        .ok_or_else(|| AppError::node_not_found(&args.node_id))?;

    // Search flat [name, value, name, value, ...] array
    let mut i = 0;
    while i + 1 < attr_array.len() {
        let name = attr_array[i].as_str().unwrap_or("");
        if name == args.attribute {
            let value = attr_array[i + 1].as_str().unwrap_or("").to_string();
            let result = AttributeResult {
                attribute: args.attribute.clone(),
                value: value.clone(),
            };

            if global.output.plain {
                println!("{value}");
                return Ok(());
            }
            return print_output(&result, &global.output);
        }
        i += 2;
    }

    Err(AppError::attribute_not_found(
        &args.attribute,
        &args.node_id,
    ))
}

// =============================================================================
// dom get-text
// =============================================================================

async fn execute_get_text(global: &GlobalOpts, args: &DomNodeIdArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;
    managed.ensure_domain("Runtime").await?;

    let node_id = resolve_node(&managed, &args.node_id).await?.node_id;
    let text = get_text_content(&managed, node_id).await?;

    if global.output.plain {
        print!("{text}");
        return Ok(());
    }

    let result = TextResult { text_content: text };
    print_output(&result, &global.output)
}

// =============================================================================
// dom get-html
// =============================================================================

async fn execute_get_html(global: &GlobalOpts, args: &DomNodeIdArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;

    let node_id = resolve_node(&managed, &args.node_id).await?.node_id;

    let html = managed
        .send_command(
            "DOM.getOuterHTML",
            Some(serde_json::json!({ "nodeId": node_id })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("Failed to get outerHTML: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let outer_html = html["outerHTML"].as_str().unwrap_or("").to_string();

    if global.output.plain {
        print!("{outer_html}");
        return Ok(());
    }

    let result = HtmlResult { outer_html };
    print_output(&result, &global.output)
}

// =============================================================================
// dom set-attribute
// =============================================================================

async fn execute_set_attribute(
    global: &GlobalOpts,
    args: &DomSetAttributeArgs,
) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;

    let resolved = resolve_node(&managed, &args.node_id).await?;

    managed
        .send_command(
            "DOM.setAttributeValue",
            Some(serde_json::json!({
                "nodeId": resolved.node_id,
                "name": args.attribute,
                "value": args.value,
            })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("Failed to set attribute: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let result = MutationResult {
        success: true,
        node_id: resolved.backend_node_id,
        attribute: Some(args.attribute.clone()),
        value: Some(args.value.clone()),
    };

    if global.output.plain {
        println!(
            "Set {}=\"{}\" on node {}",
            args.attribute, args.value, resolved.backend_node_id
        );
        return Ok(());
    }

    print_output(&result, &global.output)
}

// =============================================================================
// dom set-text
// =============================================================================

async fn execute_set_text(global: &GlobalOpts, args: &DomSetTextArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;
    managed.ensure_domain("Runtime").await?;

    let resolved = resolve_node(&managed, &args.node_id).await?;

    // Resolve to JS object and set textContent
    let resolve = managed
        .send_command(
            "DOM.resolveNode",
            Some(serde_json::json!({ "nodeId": resolved.node_id })),
        )
        .await
        .map_err(|_| AppError::node_not_found(&args.node_id))?;

    let object_id = resolve["object"]["objectId"]
        .as_str()
        .ok_or_else(|| AppError::node_not_found(&args.node_id))?;

    managed
        .send_command(
            "Runtime.callFunctionOn",
            Some(serde_json::json!({
                "objectId": object_id,
                "functionDeclaration": "function(text) { this.textContent = text; }",
                "arguments": [{ "value": args.text }],
            })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("Failed to set text: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let result = SetTextResult {
        success: true,
        node_id: resolved.backend_node_id,
        text_content: args.text.clone(),
    };

    if global.output.plain {
        println!("Set text on node {}", resolved.backend_node_id);
        return Ok(());
    }

    print_output(&result, &global.output)
}

// =============================================================================
// dom remove
// =============================================================================

async fn execute_remove(global: &GlobalOpts, args: &DomNodeIdArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;

    let resolved = resolve_node(&managed, &args.node_id).await?;

    managed
        .send_command(
            "DOM.removeNode",
            Some(serde_json::json!({ "nodeId": resolved.node_id })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("Failed to remove node: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let result = RemoveResult {
        success: true,
        node_id: resolved.backend_node_id,
        removed: true,
    };

    if global.output.plain {
        println!("Removed node {}", resolved.backend_node_id);
        return Ok(());
    }

    print_output(&result, &global.output)
}

// =============================================================================
// dom get-style
// =============================================================================

async fn execute_get_style(global: &GlobalOpts, args: &DomGetStyleArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;
    managed.ensure_domain("CSS").await?;

    let node_id = resolve_node(&managed, &args.node_id).await?.node_id;

    let computed = managed
        .send_command(
            "CSS.getComputedStyleForNode",
            Some(serde_json::json!({ "nodeId": node_id })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("Failed to get computed style: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let style_entries = computed["computedStyle"]
        .as_array()
        .ok_or_else(|| AppError {
            message: "Failed to get computed style: missing computedStyle array".to_string(),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    if let Some(ref prop) = args.property {
        // Return single property
        for entry in style_entries {
            let name = entry["name"].as_str().unwrap_or("");
            if name == prop {
                let value = entry["value"].as_str().unwrap_or("").to_string();
                let result = StylePropertyResult {
                    property: prop.clone(),
                    value: value.clone(),
                };

                if global.output.plain {
                    println!("{value}");
                    return Ok(());
                }
                return print_output(&result, &global.output);
            }
        }
        return Err(AppError {
            message: format!("CSS property '{prop}' not found in computed styles"),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }

    // Return all styles
    let mut styles = HashMap::new();
    for entry in style_entries {
        let name = entry["name"].as_str().unwrap_or("").to_string();
        let value = entry["value"].as_str().unwrap_or("").to_string();
        if !value.is_empty() {
            styles.insert(name, value);
        }
    }

    if global.output.plain {
        let mut keys: Vec<&String> = styles.keys().collect();
        keys.sort();
        for k in keys {
            println!("{k}: {}", styles[k]);
        }
        return Ok(());
    }

    let result = StyleResult { styles };
    print_output(&result, &global.output)
}

// =============================================================================
// dom set-style
// =============================================================================

async fn execute_set_style(global: &GlobalOpts, args: &DomSetStyleArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;

    let resolved = resolve_node(&managed, &args.node_id).await?;

    managed
        .send_command(
            "DOM.setAttributeValue",
            Some(serde_json::json!({
                "nodeId": resolved.node_id,
                "name": "style",
                "value": args.style,
            })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("Failed to set style: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let result = SetStyleResult {
        success: true,
        node_id: resolved.backend_node_id,
        style: args.style.clone(),
    };

    if global.output.plain {
        println!("Set style on node {}", resolved.backend_node_id);
        return Ok(());
    }

    print_output(&result, &global.output)
}

// =============================================================================
// dom parent
// =============================================================================

async fn execute_parent(global: &GlobalOpts, args: &DomNodeIdArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;
    managed.ensure_domain("Runtime").await?;

    let node_id = resolve_node(&managed, &args.node_id).await?.node_id;

    // Use Runtime to get parentElement info since DOM.describeNode doesn't
    // always return parentId reliably for all node types
    let resolve = managed
        .send_command(
            "DOM.resolveNode",
            Some(serde_json::json!({ "nodeId": node_id })),
        )
        .await
        .map_err(|_| AppError::node_not_found(&args.node_id))?;

    let object_id = resolve["object"]["objectId"]
        .as_str()
        .ok_or_else(|| AppError::node_not_found(&args.node_id))?;

    // Check if parent exists
    let parent_check = managed
        .send_command(
            "Runtime.callFunctionOn",
            Some(serde_json::json!({
                "objectId": object_id,
                "functionDeclaration": "function() { return this.parentElement !== null; }",
                "returnByValue": true,
            })),
        )
        .await?;

    let has_parent = parent_check["result"]["value"].as_bool().unwrap_or(false);

    if !has_parent {
        return Err(AppError::no_parent());
    }

    // Get parent node via DOM.requestNode on the parent object
    let parent_obj = managed
        .send_command(
            "Runtime.callFunctionOn",
            Some(serde_json::json!({
                "objectId": object_id,
                "functionDeclaration": "function() { return this.parentElement; }",
            })),
        )
        .await?;

    let parent_object_id = parent_obj["result"]["objectId"]
        .as_str()
        .ok_or_else(AppError::no_parent)?;

    let parent_node = managed
        .send_command(
            "DOM.requestNode",
            Some(serde_json::json!({ "objectId": parent_object_id })),
        )
        .await?;

    let parent_node_id = parent_node["nodeId"]
        .as_i64()
        .ok_or_else(AppError::no_parent)?;

    let parent = describe_element(&managed, parent_node_id).await?;

    if global.output.plain {
        println!(
            "[{}] <{}> \"{}\"",
            parent.node_id,
            parent.tag,
            truncate_text(&parent.text_content, 60)
        );
        return Ok(());
    }

    print_output(&parent, &global.output)
}

// =============================================================================
// dom children
// =============================================================================

async fn execute_children(global: &GlobalOpts, args: &DomNodeIdArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;
    managed.ensure_domain("Runtime").await?;

    let node_id = resolve_node(&managed, &args.node_id).await?.node_id;

    // Request child nodes
    managed
        .send_command(
            "DOM.requestChildNodes",
            Some(serde_json::json!({ "nodeId": node_id, "depth": 1 })),
        )
        .await?;

    // Get children via describeNode with depth 1
    let describe = managed
        .send_command(
            "DOM.describeNode",
            Some(serde_json::json!({ "nodeId": node_id, "depth": 1 })),
        )
        .await
        .map_err(|_| AppError::node_not_found(&args.node_id))?;

    let children = describe["node"]["children"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    // Filter to element nodes (nodeType 1) and describe each
    let mut elements = Vec::new();
    for child in &children {
        let node_type = child["nodeType"].as_i64().unwrap_or(0);
        if node_type == 1 {
            let child_id = child["nodeId"].as_i64().unwrap_or(0);
            if child_id > 0 {
                if let Ok(el) = describe_element(&managed, child_id).await {
                    elements.push(el);
                }
            }
        }
    }

    if global.output.plain {
        for el in &elements {
            println!(
                "[{}] <{}> \"{}\"",
                el.node_id,
                el.tag,
                truncate_text(&el.text_content, 60)
            );
        }
        return Ok(());
    }

    print_output(&elements, &global.output)
}

// =============================================================================
// dom siblings
// =============================================================================

async fn execute_siblings(global: &GlobalOpts, args: &DomNodeIdArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;
    managed.ensure_domain("Runtime").await?;

    let node_id = resolve_node(&managed, &args.node_id).await?.node_id;

    // Get parent nodeId via Runtime
    let resolve = managed
        .send_command(
            "DOM.resolveNode",
            Some(serde_json::json!({ "nodeId": node_id })),
        )
        .await
        .map_err(|_| AppError::node_not_found(&args.node_id))?;

    let object_id = resolve["object"]["objectId"]
        .as_str()
        .ok_or_else(|| AppError::node_not_found(&args.node_id))?;

    let parent_obj = managed
        .send_command(
            "Runtime.callFunctionOn",
            Some(serde_json::json!({
                "objectId": object_id,
                "functionDeclaration": "function() { return this.parentElement; }",
            })),
        )
        .await?;

    let parent_object_id = parent_obj["result"]["objectId"]
        .as_str()
        .ok_or_else(AppError::no_parent)?;

    let parent_node = managed
        .send_command(
            "DOM.requestNode",
            Some(serde_json::json!({ "objectId": parent_object_id })),
        )
        .await?;

    let parent_node_id = parent_node["nodeId"]
        .as_i64()
        .ok_or_else(AppError::no_parent)?;

    // Get parent's children
    managed
        .send_command(
            "DOM.requestChildNodes",
            Some(serde_json::json!({ "nodeId": parent_node_id, "depth": 1 })),
        )
        .await?;

    let describe = managed
        .send_command(
            "DOM.describeNode",
            Some(serde_json::json!({ "nodeId": parent_node_id, "depth": 1 })),
        )
        .await
        .map_err(|_| AppError::node_not_found(&args.node_id))?;

    let children = describe["node"]["children"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    // Filter to element nodes, excluding self
    let mut elements = Vec::new();
    for child in &children {
        let node_type = child["nodeType"].as_i64().unwrap_or(0);
        let child_id = child["nodeId"].as_i64().unwrap_or(0);
        if node_type == 1 && child_id > 0 && child_id != node_id {
            if let Ok(el) = describe_element(&managed, child_id).await {
                elements.push(el);
            }
        }
    }

    if global.output.plain {
        for el in &elements {
            println!(
                "[{}] <{}> \"{}\"",
                el.node_id,
                el.tag,
                truncate_text(&el.text_content, 60)
            );
        }
        return Ok(());
    }

    print_output(&elements, &global.output)
}

// =============================================================================
// dom tree
// =============================================================================

/// Maximum text content length shown per node in tree output.
const TREE_TEXT_MAX: usize = 60;

async fn execute_tree(global: &GlobalOpts, args: &DomTreeArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;

    // Determine the root and depth for DOM.getDocument
    let depth = args.depth.map_or(-1, i64::from);

    let root_node = if let Some(ref root_target) = args.root {
        // Resolve root target and get its subtree
        let node_id = resolve_node(&managed, root_target).await?.node_id;
        managed
            .send_command(
                "DOM.describeNode",
                Some(serde_json::json!({ "nodeId": node_id, "depth": depth })),
            )
            .await
            .map_err(|_| AppError::node_not_found(root_target))?
    } else {
        managed
            .send_command(
                "DOM.getDocument",
                Some(serde_json::json!({ "depth": depth })),
            )
            .await
            .map_err(|e| AppError {
                message: format!("DOM.getDocument failed: {e}"),
                code: ExitCode::ProtocolError,
                custom_json: None,
            })?
    };

    let node = if args.root.is_some() {
        &root_node["node"]
    } else {
        &root_node["root"]
    };

    let mut output = String::new();
    format_tree_node(&mut output, node, 0, args.depth);

    if global.output.plain || (!global.output.json && !global.output.pretty) {
        print!("{output}");
        return Ok(());
    }

    print_output(&TreeOutput { tree: output }, &global.output)
}

/// Recursively format a DOM node into indented text.
fn format_tree_node(
    out: &mut String,
    node: &serde_json::Value,
    indent: usize,
    max_depth: Option<u32>,
) {
    let node_type = node["nodeType"].as_i64().unwrap_or(0);
    let node_name = node["nodeName"].as_str().unwrap_or("");

    // Only show element nodes (1), document (9), and text nodes (3)
    match node_type {
        1 => {
            // Element node
            let tag = node_name.to_lowercase();
            let indent_str = "  ".repeat(indent);

            // Collect key attributes to show inline
            let mut attr_hints = Vec::new();
            if let Some(attrs) = node["attributes"].as_array() {
                let mut i = 0;
                while i + 1 < attrs.len() {
                    let name = attrs[i].as_str().unwrap_or("");
                    if matches!(name, "id" | "class" | "href" | "src" | "type" | "name") {
                        attr_hints.push(format!("[{name}]"));
                    }
                    i += 2;
                }
            }
            let attr_str = attr_hints.join("");

            // Check for direct text content (from child text nodes)
            let text = extract_direct_text(node);
            let text_str = if text.is_empty() {
                String::new()
            } else {
                format!(" \"{}\"", truncate_text(&text, TREE_TEXT_MAX))
            };

            let _ = writeln!(out, "{indent_str}{tag}{attr_str}{text_str}");
        }
        3 => {
            // Text node — skip standalone text nodes (shown inline on parent)
            return;
        }
        9 => {
            // Document node — show children only
        }
        _ => return,
    }

    // Check depth limit — show ellipsis if truncated children exist
    if let Some(max) = max_depth {
        #[allow(clippy::cast_possible_truncation)]
        if indent as u32 >= max {
            if node["children"]
                .as_array()
                .is_some_and(|c| c.iter().any(|ch| ch["nodeType"].as_i64() == Some(1)))
            {
                let child_indent = "  ".repeat(indent + 1);
                let _ = writeln!(out, "{child_indent}...");
            }
            return;
        }
    }

    // Recurse into children
    if let Some(children) = node["children"].as_array() {
        let child_indent = if node_type == 9 { indent } else { indent + 1 };
        for child in children {
            format_tree_node(out, child, child_indent, max_depth);
        }
    }
}

/// Extract direct text content from child text nodes.
fn extract_direct_text(node: &serde_json::Value) -> String {
    let mut text = String::new();
    if let Some(children) = node["children"].as_array() {
        for child in children {
            if child["nodeType"].as_i64() == Some(3) {
                if let Some(value) = child["nodeValue"].as_str() {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(trimmed);
                    }
                }
            }
        }
    }
    text
}

/// Truncate text to `max` characters, adding "..." if truncated.
/// Uses char boundaries to avoid panicking on multi-byte UTF-8.
fn truncate_text(text: &str, max: usize) -> String {
    let trimmed = text.trim().replace('\n', " ");
    if trimmed.chars().count() > max {
        let end = trimmed
            .char_indices()
            .nth(max)
            .map_or(trimmed.len(), |(i, _)| i);
        format!("{}...", &trimmed[..end])
    } else {
        trimmed
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_uid_valid() {
        assert!(is_uid("s1"));
        assert!(is_uid("s42"));
        assert!(is_uid("s999"));
    }

    #[test]
    fn is_uid_invalid() {
        assert!(!is_uid("s"));
        assert!(!is_uid("s0a"));
        assert!(!is_uid("css:button"));
        assert!(!is_uid("button"));
        assert!(!is_uid("42"));
    }

    #[test]
    fn is_css_selector_valid() {
        assert!(is_css_selector("css:#button"));
        assert!(is_css_selector("css:.class"));
    }

    #[test]
    fn is_css_selector_invalid() {
        assert!(!is_css_selector("#button"));
        assert!(!is_css_selector("s1"));
    }

    #[test]
    fn truncate_text_short() {
        assert_eq!(truncate_text("Hello", 10), "Hello");
    }

    #[test]
    fn truncate_text_long() {
        let long = "a".repeat(100);
        let result = truncate_text(&long, 10);
        assert_eq!(result.len(), 13); // 10 + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn truncate_text_with_newlines() {
        assert_eq!(truncate_text("hello\nworld", 20), "hello world");
    }

    #[test]
    fn truncate_text_multibyte_utf8() {
        // Each emoji is a multi-byte char — must not panic
        let text = "🎉🎊🎈🎁🎂🎃🎄🎅🎆🎇🎋🎍";
        let result = truncate_text(text, 5);
        assert!(result.ends_with("..."));
        // Should contain exactly 5 emoji chars + "..."
        assert_eq!(result.chars().count(), 8);
    }

    #[test]
    fn extract_direct_text_basic() {
        let node = serde_json::json!({
            "children": [
                { "nodeType": 3, "nodeValue": "Hello World" }
            ]
        });
        assert_eq!(extract_direct_text(&node), "Hello World");
    }

    #[test]
    fn extract_direct_text_multiple() {
        let node = serde_json::json!({
            "children": [
                { "nodeType": 3, "nodeValue": "Hello" },
                { "nodeType": 1, "nodeName": "SPAN" },
                { "nodeType": 3, "nodeValue": "World" }
            ]
        });
        assert_eq!(extract_direct_text(&node), "Hello World");
    }

    #[test]
    fn extract_direct_text_empty() {
        let node = serde_json::json!({
            "children": [
                { "nodeType": 1, "nodeName": "SPAN" }
            ]
        });
        assert_eq!(extract_direct_text(&node), "");
    }

    #[test]
    fn format_tree_simple() {
        let doc = serde_json::json!({
            "nodeType": 9,
            "nodeName": "#document",
            "children": [{
                "nodeType": 1,
                "nodeName": "HTML",
                "children": [{
                    "nodeType": 1,
                    "nodeName": "BODY",
                    "children": [{
                        "nodeType": 1,
                        "nodeName": "H1",
                        "children": [{
                            "nodeType": 3,
                            "nodeValue": "Hello"
                        }]
                    }]
                }]
            }]
        });
        let mut out = String::new();
        format_tree_node(&mut out, &doc, 0, None);
        assert!(out.contains("html"));
        assert!(out.contains("  body"));
        assert!(out.contains("    h1 \"Hello\""));
    }

    #[test]
    fn format_tree_with_depth_limit() {
        let doc = serde_json::json!({
            "nodeType": 9,
            "nodeName": "#document",
            "children": [{
                "nodeType": 1,
                "nodeName": "HTML",
                "children": [{
                    "nodeType": 1,
                    "nodeName": "BODY",
                    "children": [{
                        "nodeType": 1,
                        "nodeName": "H1",
                        "children": [{
                            "nodeType": 3,
                            "nodeValue": "Hello"
                        }]
                    }]
                }]
            }]
        });
        let mut out = String::new();
        format_tree_node(&mut out, &doc, 0, Some(1));
        assert!(out.contains("html"));
        // body is at indent 1 = max, so it should appear with an ellipsis
        assert!(out.contains("  body"));
        assert!(out.contains("..."));
        assert!(!out.contains("h1"));
    }

    #[test]
    fn format_tree_with_attributes() {
        let node = serde_json::json!({
            "nodeType": 1,
            "nodeName": "A",
            "attributes": ["href", "https://example.com", "class", "link"],
            "children": [{
                "nodeType": 3,
                "nodeValue": "Click me"
            }]
        });
        let mut out = String::new();
        format_tree_node(&mut out, &node, 0, None);
        assert!(out.contains("a[href][class]"));
        assert!(out.contains("\"Click me\""));
    }

    #[test]
    fn dom_element_serialization() {
        let el = DomElement {
            node_id: 42,
            tag: "h1".to_string(),
            attributes: HashMap::from([("class".to_string(), "title".to_string())]),
            text_content: "Hello".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&el).unwrap();
        assert_eq!(json["nodeId"], 42);
        assert_eq!(json["tag"], "h1");
        assert_eq!(json["attributes"]["class"], "title");
        assert_eq!(json["textContent"], "Hello");
    }

    #[test]
    fn attribute_result_serialization() {
        let result = AttributeResult {
            attribute: "href".to_string(),
            value: "https://example.com".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["attribute"], "href");
        assert_eq!(json["value"], "https://example.com");
    }

    #[test]
    fn text_result_serialization() {
        let result = TextResult {
            text_content: "Hello World".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["textContent"], "Hello World");
    }

    #[test]
    fn html_result_serialization() {
        let result = HtmlResult {
            outer_html: "<h1>Hello</h1>".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["outerHTML"], "<h1>Hello</h1>");
    }

    #[test]
    fn mutation_result_serialization_with_attribute() {
        let result = MutationResult {
            success: true,
            node_id: 42,
            attribute: Some("class".to_string()),
            value: Some("highlight".to_string()),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["nodeId"], 42);
        assert_eq!(json["attribute"], "class");
        assert_eq!(json["value"], "highlight");
    }

    #[test]
    fn mutation_result_serialization_without_attribute() {
        let result = MutationResult {
            success: true,
            node_id: 42,
            attribute: None,
            value: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["nodeId"], 42);
        assert!(json.get("attribute").is_none());
        assert!(json.get("value").is_none());
    }

    #[test]
    fn style_result_serialization() {
        let result = StyleResult {
            styles: HashMap::from([
                ("display".to_string(), "block".to_string()),
                ("color".to_string(), "rgb(0, 0, 0)".to_string()),
            ]),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["styles"]["display"], "block");
        assert_eq!(json["styles"]["color"], "rgb(0, 0, 0)");
    }

    #[test]
    fn style_property_result_serialization() {
        let result = StylePropertyResult {
            property: "display".to_string(),
            value: "block".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["property"], "display");
        assert_eq!(json["value"], "block");
    }
}
