use std::time::Duration;

use serde::Serialize;

use chrome_cli::cdp::{CdpClient, CdpConfig};
use chrome_cli::connection::{ManagedSession, resolve_connection, resolve_target};
use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, PageArgs, PageCommand, PageFindArgs, PageSnapshotArgs, PageTextArgs};

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct PageTextResult {
    text: String,
    url: String,
    title: String,
}

/// A single element match from `page find`.
#[derive(Debug, Clone, Serialize)]
struct FindMatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    uid: Option<String>,
    role: String,
    name: String,
    #[serde(rename = "boundingBox", skip_serializing_if = "Option::is_none")]
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
// Dispatcher
// =============================================================================

/// Execute the `page` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_page(global: &GlobalOpts, args: &PageArgs) -> Result<(), AppError> {
    match &args.command {
        PageCommand::Text(text_args) => execute_text(global, text_args).await,
        PageCommand::Snapshot(snap_args) => execute_snapshot(global, snap_args).await,
        PageCommand::Find(find_args) => execute_find(global, find_args).await,
    }
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
    let managed = ManagedSession::new(session);

    Ok((client, managed))
}

// =============================================================================
// Page info helper
// =============================================================================

async fn get_page_info(managed: &ManagedSession) -> Result<(String, String), AppError> {
    let url_result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": "location.href" })),
        )
        .await?;

    let title_result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": "document.title" })),
        )
        .await?;

    let url = url_result["result"]["value"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let title = title_result["result"]["value"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    Ok((url, title))
}

// =============================================================================
// Text extraction
// =============================================================================

/// Escape a CSS selector for embedding in a JavaScript double-quoted string.
fn escape_selector(selector: &str) -> String {
    selector.replace('\\', "\\\\").replace('"', "\\\"")
}

async fn execute_text(global: &GlobalOpts, args: &PageTextArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;

    // Enable Runtime domain
    managed.ensure_domain("Runtime").await?;

    // Build JS expression
    let expression = match &args.selector {
        None => "document.body?.innerText ?? ''".to_string(),
        Some(selector) => {
            let escaped = escape_selector(selector);
            format!(
                r#"(() => {{ const el = document.querySelector("{escaped}"); if (!el) return {{ __error: "not_found" }}; return el.innerText; }})()"#
            )
        }
    };

    let params = serde_json::json!({
        "expression": expression,
        "returnByValue": true,
    });

    let result = managed
        .send_command("Runtime.evaluate", Some(params))
        .await?;

    // Check for exception
    if let Some(exception) = result.get("exceptionDetails") {
        let description = exception["exception"]["description"]
            .as_str()
            .or_else(|| exception["text"].as_str())
            .unwrap_or("unknown error");
        return Err(AppError::evaluation_failed(description));
    }

    let value = &result["result"]["value"];

    // Check for sentinel error object
    if let Some(error) = value.get("__error") {
        if error.as_str() == Some("not_found") {
            let selector = args.selector.as_deref().unwrap_or("unknown");
            return Err(AppError::element_not_found(selector));
        }
    }

    let text = value.as_str().unwrap_or_default().to_string();

    // Get page info
    let (url, title) = get_page_info(&managed).await?;

    // Output
    if global.output.plain {
        print!("{text}");
        return Ok(());
    }

    let output = PageTextResult { text, url, title };
    print_output(&output, &global.output)
}

// =============================================================================
// Accessibility tree snapshot
// =============================================================================

async fn execute_snapshot(global: &GlobalOpts, args: &PageSnapshotArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;

    // Enable required domains
    managed.ensure_domain("Accessibility").await?;
    managed.ensure_domain("Runtime").await?;

    // Capture the accessibility tree
    let result = managed
        .send_command("Accessibility.getFullAXTree", None)
        .await
        .map_err(|e| AppError::snapshot_failed(&e.to_string()))?;

    let nodes = result["nodes"]
        .as_array()
        .ok_or_else(|| AppError::snapshot_failed("response missing 'nodes' array"))?;

    // Build tree and assign UIDs
    let build = crate::snapshot::build_tree(nodes, args.verbose);

    // Get page URL for snapshot state
    let (url, _title) = get_page_info(&managed).await?;

    // Persist UID mapping
    let state = crate::snapshot::SnapshotState {
        url,
        timestamp: chrome_cli::session::now_iso8601(),
        uid_map: build.uid_map,
    };
    if let Err(e) = crate::snapshot::write_snapshot_state(&state) {
        eprintln!("warning: could not save snapshot state: {e}");
    }

    // Format output
    let formatted = if global.output.json || global.output.pretty {
        // JSON output — add truncation info to root if applicable
        let mut json_value = serde_json::to_value(&build.root).map_err(|e| AppError {
            message: format!("serialization error: {e}"),
            code: ExitCode::GeneralError,
        })?;
        if build.truncated {
            if let Some(obj) = json_value.as_object_mut() {
                obj.insert("truncated".to_string(), serde_json::Value::Bool(true));
                obj.insert(
                    "total_nodes".to_string(),
                    serde_json::Value::Number(build.total_nodes.into()),
                );
            }
        }
        let serializer = if global.output.pretty {
            serde_json::to_string_pretty(&json_value)
        } else {
            serde_json::to_string(&json_value)
        };
        serializer.map_err(|e| AppError {
            message: format!("serialization error: {e}"),
            code: ExitCode::GeneralError,
        })?
    } else {
        // Text output (default and --plain)
        let mut text = crate::snapshot::format_text(&build.root, args.verbose);
        if build.truncated {
            text.push_str(&format!(
                "[... truncated: {} nodes, showing first {}]\n",
                build.total_nodes,
                crate::snapshot::MAX_NODES
            ));
        }
        text
    };

    // Write to file or stdout
    if let Some(ref file_path) = args.file {
        std::fs::write(file_path, &formatted).map_err(|e| {
            AppError::file_write_failed(&file_path.display().to_string(), &e.to_string())
        })?;
    } else {
        print!("{formatted}");
    }

    Ok(())
}

// =============================================================================
// Element finding
// =============================================================================

/// Resolve bounding box for a DOM node by its `backendDOMNodeId`.
///
/// Returns `None` if the element is invisible or has been removed from the DOM.
async fn resolve_bounding_box(
    managed: &ManagedSession,
    backend_dom_node_id: i64,
) -> Option<BoundingBox> {
    // Resolve backendNodeId → nodeId via DOM.describeNode
    let describe = managed
        .send_command(
            "DOM.describeNode",
            Some(serde_json::json!({ "backendNodeId": backend_dom_node_id })),
        )
        .await
        .ok()?;

    let node_id = describe["node"]["nodeId"].as_i64()?;

    // Get box model for the resolved nodeId
    let box_result = managed
        .send_command(
            "DOM.getBoxModel",
            Some(serde_json::json!({ "nodeId": node_id })),
        )
        .await
        .ok()?;

    let content = box_result["model"]["content"].as_array()?;
    if content.len() < 8 {
        return None;
    }

    // Content quad: [x1,y1, x2,y2, x3,y3, x4,y4] — top-left and bottom-right
    let x1 = content[0].as_f64()?;
    let y1 = content[1].as_f64()?;
    let x3 = content[4].as_f64()?;
    let y3 = content[5].as_f64()?;

    Some(BoundingBox {
        x: x1,
        y: y1,
        width: x3 - x1,
        height: y3 - y1,
    })
}

/// Find elements by CSS selector using CDP DOM methods.
///
/// Returns up to `limit` matches with role, name, and bounding box.
async fn find_by_selector(
    managed: &ManagedSession,
    selector: &str,
    limit: usize,
) -> Result<Vec<FindMatch>, AppError> {
    // Get the document root
    let doc = managed
        .send_command("DOM.getDocument", None)
        .await
        .map_err(|e| AppError::snapshot_failed(&e.to_string()))?;
    let root_node_id = doc["root"]["nodeId"]
        .as_i64()
        .ok_or_else(|| AppError::snapshot_failed("DOM.getDocument missing root nodeId"))?;

    // Query all matching nodes
    let query_result = managed
        .send_command(
            "DOM.querySelectorAll",
            Some(serde_json::json!({
                "nodeId": root_node_id,
                "selector": selector,
            })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("CSS selector query failed: {e}"),
            code: ExitCode::ProtocolError,
        })?;

    let node_ids = query_result["nodeIds"]
        .as_array()
        .ok_or_else(|| AppError::snapshot_failed("DOM.querySelectorAll missing nodeIds"))?;

    let mut matches = Vec::with_capacity(limit.min(node_ids.len()));
    for node_id_val in node_ids.iter().take(limit) {
        let Some(node_id) = node_id_val.as_i64() else {
            continue;
        };

        // Get backendDOMNodeId via DOM.describeNode
        let describe = managed
            .send_command(
                "DOM.describeNode",
                Some(serde_json::json!({ "nodeId": node_id })),
            )
            .await;

        let backend_dom_node_id = describe
            .as_ref()
            .ok()
            .and_then(|d| d["node"]["backendNodeId"].as_i64());

        // Get accessibility info for this node
        let (role, name) = if let Some(backend_id) = backend_dom_node_id {
            let ax_result = managed
                .send_command(
                    "Accessibility.getPartialAXTree",
                    Some(serde_json::json!({
                        "backendNodeId": backend_id,
                        "fetchRelatives": false,
                    })),
                )
                .await;

            if let Ok(ax) = ax_result {
                let nodes = ax["nodes"].as_array();
                let first = nodes.and_then(|arr| arr.first());
                let role = first
                    .and_then(|n| n["role"]["value"].as_str())
                    .unwrap_or("none")
                    .to_string();
                let name = first
                    .and_then(|n| n["name"]["value"].as_str())
                    .unwrap_or("")
                    .to_string();
                (role, name)
            } else {
                ("none".to_string(), String::new())
            }
        } else {
            ("none".to_string(), String::new())
        };

        // Get bounding box
        let bounding_box = if let Some(backend_id) = backend_dom_node_id {
            resolve_bounding_box(managed, backend_id).await
        } else {
            None
        };

        matches.push(FindMatch {
            uid: None, // UIDs assigned after full snapshot
            role,
            name,
            bounding_box,
        });
    }

    Ok(matches)
}

/// Capture the accessibility tree, build it, and persist snapshot state.
async fn capture_snapshot(
    managed: &ManagedSession,
) -> Result<crate::snapshot::BuildResult, AppError> {
    let ax_result = managed
        .send_command("Accessibility.getFullAXTree", None)
        .await
        .map_err(|e| AppError::snapshot_failed(&e.to_string()))?;
    let nodes = ax_result["nodes"]
        .as_array()
        .ok_or_else(|| AppError::snapshot_failed("response missing 'nodes' array"))?;
    let build = crate::snapshot::build_tree(nodes, false);

    let (url, _title) = get_page_info(managed).await?;
    let state = crate::snapshot::SnapshotState {
        url,
        timestamp: chrome_cli::session::now_iso8601(),
        uid_map: build.uid_map.clone(),
    };
    if let Err(e) = crate::snapshot::write_snapshot_state(&state) {
        eprintln!("warning: could not save snapshot state: {e}");
    }

    Ok(build)
}

async fn execute_find(global: &GlobalOpts, args: &PageFindArgs) -> Result<(), AppError> {
    // Validate: at least one of query or selector must be provided
    if args.query.is_none() && args.selector.is_none() {
        return Err(AppError {
            message: "either a text query or --selector is required".to_string(),
            code: ExitCode::GeneralError,
        });
    }

    let (_client, mut managed) = setup_session(global).await?;

    // Enable required domains
    managed.ensure_domain("Accessibility").await?;
    managed.ensure_domain("DOM").await?;
    managed.ensure_domain("Runtime").await?;

    // Capture snapshot (used by both search paths for UID assignment)
    let build = capture_snapshot(&managed).await?;

    let matches = if let Some(ref selector) = args.selector {
        // CSS selector path
        let mut css_matches = find_by_selector(&managed, selector, args.limit).await?;

        // Enrich CSS matches with UIDs from the snapshot tree
        for m in &mut css_matches {
            assign_uid_from_snapshot(&build.root, m);
        }

        css_matches
    } else {
        // Accessibility text search path
        let query = args.query.as_deref().unwrap_or("");

        let hits = crate::snapshot::search_tree(
            &build.root,
            query,
            args.role.as_deref(),
            args.exact,
            args.limit,
        );

        // Resolve bounding boxes for each hit
        let mut matches = Vec::with_capacity(hits.len());
        for hit in hits {
            let bounding_box = if let Some(backend_id) = hit.backend_dom_node_id {
                resolve_bounding_box(&managed, backend_id).await
            } else {
                None
            };
            matches.push(FindMatch {
                uid: hit.uid,
                role: hit.role,
                name: hit.name,
                bounding_box,
            });
        }
        matches
    };

    // Output
    if global.output.plain {
        for m in &matches {
            let uid_str = m.uid.as_ref().map_or(String::new(), |u| format!("[{u}] "));
            let bb_str = m.bounding_box.as_ref().map_or(String::new(), |bb| {
                format!(" ({},{} {}x{})", bb.x, bb.y, bb.width, bb.height)
            });
            println!("{uid_str}{} \"{}\"{bb_str}", m.role, m.name);
        }
        return Ok(());
    }

    print_output(&matches, &global.output)
}

/// Try to assign a UID from the snapshot tree to a CSS-matched element.
fn assign_uid_from_snapshot(node: &crate::snapshot::SnapshotNode, m: &mut FindMatch) {
    if node.role == m.role && node.name == m.name {
        if let Some(ref uid) = node.uid {
            m.uid = Some(uid.clone());
            return;
        }
    }
    for child in &node.children {
        if m.uid.is_some() {
            return;
        }
        assign_uid_from_snapshot(child, m);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_text_result_serialization() {
        let result = PageTextResult {
            text: "Hello, world!".to_string(),
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["text"], "Hello, world!");
        assert_eq!(json["url"], "https://example.com");
        assert_eq!(json["title"], "Example");
    }

    #[test]
    fn page_text_result_empty_text() {
        let result = PageTextResult {
            text: String::new(),
            url: "about:blank".to_string(),
            title: String::new(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["text"], "");
        assert_eq!(json["url"], "about:blank");
    }

    #[test]
    fn escape_selector_no_special_chars() {
        assert_eq!(escape_selector("#content"), "#content");
    }

    #[test]
    fn escape_selector_with_quotes() {
        assert_eq!(
            escape_selector(r#"div[data-name="test"]"#),
            r#"div[data-name=\"test\"]"#
        );
    }

    #[test]
    fn escape_selector_with_backslash() {
        assert_eq!(escape_selector(r"div\.class"), r"div\\.class");
    }

    // =========================================================================
    // FindMatch / BoundingBox serialization tests
    // =========================================================================

    #[test]
    fn find_match_serialization_with_all_fields() {
        let m = FindMatch {
            uid: Some("s3".to_string()),
            role: "button".to_string(),
            name: "Submit".to_string(),
            bounding_box: Some(BoundingBox {
                x: 120.0,
                y: 340.0,
                width: 80.0,
                height: 36.0,
            }),
        };
        let json: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(json["uid"], "s3");
        assert_eq!(json["role"], "button");
        assert_eq!(json["name"], "Submit");
        assert_eq!(json["boundingBox"]["x"], 120.0);
        assert_eq!(json["boundingBox"]["y"], 340.0);
        assert_eq!(json["boundingBox"]["width"], 80.0);
        assert_eq!(json["boundingBox"]["height"], 36.0);
    }

    #[test]
    fn find_match_serialization_without_uid() {
        let m = FindMatch {
            uid: None,
            role: "heading".to_string(),
            name: "Title".to_string(),
            bounding_box: Some(BoundingBox {
                x: 50.0,
                y: 10.0,
                width: 300.0,
                height: 32.0,
            }),
        };
        let json: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert!(json.get("uid").is_none());
        assert_eq!(json["role"], "heading");
        assert!(json.get("boundingBox").is_some());
    }

    #[test]
    fn find_match_serialization_without_bounding_box() {
        let m = FindMatch {
            uid: Some("s1".to_string()),
            role: "button".to_string(),
            name: "Hidden".to_string(),
            bounding_box: None,
        };
        let json: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(json["uid"], "s1");
        assert!(json.get("boundingBox").is_none());
    }

    #[test]
    fn find_match_serialization_minimal() {
        let m = FindMatch {
            uid: None,
            role: "text".to_string(),
            name: "Hello".to_string(),
            bounding_box: None,
        };
        let json: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert!(json.get("uid").is_none());
        assert!(json.get("boundingBox").is_none());
        assert_eq!(json["role"], "text");
        assert_eq!(json["name"], "Hello");
    }

    #[test]
    fn find_match_array_serialization() {
        let matches = vec![
            FindMatch {
                uid: Some("s1".to_string()),
                role: "button".to_string(),
                name: "OK".to_string(),
                bounding_box: Some(BoundingBox {
                    x: 10.0,
                    y: 20.0,
                    width: 50.0,
                    height: 30.0,
                }),
            },
            FindMatch {
                uid: None,
                role: "heading".to_string(),
                name: "Title".to_string(),
                bounding_box: None,
            },
        ];
        let json: serde_json::Value = serde_json::to_value(&matches).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["uid"], "s1");
        assert!(arr[1].get("uid").is_none());
    }

    #[test]
    fn find_match_empty_array_serialization() {
        let matches: Vec<FindMatch> = vec![];
        let json = serde_json::to_string(&matches).unwrap();
        assert_eq!(json, "[]");
    }

    #[test]
    fn bounding_box_camel_case_key() {
        let m = FindMatch {
            uid: None,
            role: "button".to_string(),
            name: "Test".to_string(),
            bounding_box: Some(BoundingBox {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            }),
        };
        let json_str = serde_json::to_string(&m).unwrap();
        assert!(json_str.contains("\"boundingBox\""));
        assert!(!json_str.contains("\"bounding_box\""));
    }

    // =========================================================================
    // assign_uid_from_snapshot tests
    // =========================================================================

    fn make_snapshot_node(
        role: &str,
        name: &str,
        uid: Option<&str>,
        children: Vec<crate::snapshot::SnapshotNode>,
    ) -> crate::snapshot::SnapshotNode {
        crate::snapshot::SnapshotNode {
            role: role.to_string(),
            name: name.to_string(),
            uid: uid.map(String::from),
            properties: None,
            backend_dom_node_id: None,
            children,
        }
    }

    #[test]
    fn assign_uid_matches_role_and_name() {
        let tree = make_snapshot_node(
            "document",
            "Page",
            None,
            vec![make_snapshot_node("button", "Submit", Some("s1"), vec![])],
        );
        let mut m = FindMatch {
            uid: None,
            role: "button".to_string(),
            name: "Submit".to_string(),
            bounding_box: None,
        };
        assign_uid_from_snapshot(&tree, &mut m);
        assert_eq!(m.uid.as_deref(), Some("s1"));
    }

    #[test]
    fn assign_uid_no_match_leaves_none() {
        let tree = make_snapshot_node(
            "document",
            "Page",
            None,
            vec![make_snapshot_node("button", "Submit", Some("s1"), vec![])],
        );
        let mut m = FindMatch {
            uid: None,
            role: "link".to_string(),
            name: "Other".to_string(),
            bounding_box: None,
        };
        assign_uid_from_snapshot(&tree, &mut m);
        assert!(m.uid.is_none());
    }

    #[test]
    fn assign_uid_first_match_wins() {
        let tree = make_snapshot_node(
            "document",
            "Page",
            None,
            vec![
                make_snapshot_node("button", "OK", Some("s1"), vec![]),
                make_snapshot_node("button", "OK", Some("s2"), vec![]),
            ],
        );
        let mut m = FindMatch {
            uid: None,
            role: "button".to_string(),
            name: "OK".to_string(),
            bounding_box: None,
        };
        assign_uid_from_snapshot(&tree, &mut m);
        assert_eq!(m.uid.as_deref(), Some("s1"));
    }
}
