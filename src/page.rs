use std::time::Duration;

use serde::Serialize;

use chrome_cli::cdp::{CdpClient, CdpConfig};
use chrome_cli::connection::{ManagedSession, resolve_connection, resolve_target};
use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{
    GlobalOpts, PageArgs, PageCommand, PageFindArgs, PageResizeArgs, PageScreenshotArgs,
    PageSnapshotArgs, PageTextArgs, ScreenshotFormat,
};
use crate::emulate::apply_emulate_state;

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

/// Output when screenshot is returned as base64 (no --file).
#[derive(Serialize)]
struct ScreenshotResult {
    format: String,
    data: String,
    width: u32,
    height: u32,
}

/// Output when screenshot is saved to a file (--file).
#[derive(Serialize)]
struct ScreenshotFileResult {
    format: String,
    file: String,
    width: u32,
    height: u32,
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
        PageCommand::Screenshot(ss_args) => execute_screenshot(global, ss_args).await,
        PageCommand::Resize(resize_args) => execute_page_resize(global, resize_args).await,
    }
}

async fn execute_page_resize(global: &GlobalOpts, args: &PageResizeArgs) -> Result<(), AppError> {
    crate::emulate::execute_resize(global, &args.size).await
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
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

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
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

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
            custom_json: None,
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
            custom_json: None,
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
            custom_json: None,
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
    // Validate: at least one of query, selector, or role must be provided
    if args.query.is_none() && args.selector.is_none() && args.role.is_none() {
        return Err(AppError {
            message: "a text query, --selector, or --role is required".to_string(),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }

    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

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
// Screenshot capture
// =============================================================================

/// A clip region for CDP's `Page.captureScreenshot` clip parameter.
#[derive(Debug)]
struct ClipRegion {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// Parse a "X,Y,WIDTH,HEIGHT" string into a [`ClipRegion`].
fn parse_clip(input: &str) -> Result<ClipRegion, AppError> {
    let parts: Vec<&str> = input.split(',').collect();
    if parts.len() != 4 {
        return Err(AppError::invalid_clip(input));
    }
    let values: Result<Vec<f64>, _> = parts.iter().map(|p| p.trim().parse::<f64>()).collect();
    let values = values.map_err(|_| AppError::invalid_clip(input))?;
    Ok(ClipRegion {
        x: values[0],
        y: values[1],
        width: values[2],
        height: values[3],
    })
}

/// Extract a [`ClipRegion`] from a `DOM.getBoxModel` response.
fn extract_clip_from_box_model(box_result: &serde_json::Value) -> Option<ClipRegion> {
    let content = box_result["model"]["content"].as_array()?;
    if content.len() < 8 {
        return None;
    }
    let x1 = content[0].as_f64()?;
    let y1 = content[1].as_f64()?;
    let x3 = content[4].as_f64()?;
    let y3 = content[5].as_f64()?;
    Some(ClipRegion {
        x: x1,
        y: y1,
        width: x3 - x1,
        height: y3 - y1,
    })
}

/// Resolve an element's bounding box as a clip region using a CSS selector.
async fn resolve_selector_clip(
    managed: &ManagedSession,
    selector: &str,
) -> Result<ClipRegion, AppError> {
    let doc = managed
        .send_command("DOM.getDocument", None)
        .await
        .map_err(|e| AppError::screenshot_failed(&e.to_string()))?;
    let root_node_id = doc["root"]["nodeId"]
        .as_i64()
        .ok_or_else(|| AppError::screenshot_failed("DOM.getDocument missing root nodeId"))?;

    let query_result = managed
        .send_command(
            "DOM.querySelector",
            Some(serde_json::json!({
                "nodeId": root_node_id,
                "selector": selector,
            })),
        )
        .await
        .map_err(|e| AppError::screenshot_failed(&format!("CSS selector query failed: {e}")))?;

    let node_id = query_result["nodeId"]
        .as_i64()
        .filter(|&id| id > 0)
        .ok_or_else(|| AppError::element_not_found(selector))?;

    let box_result = managed
        .send_command(
            "DOM.getBoxModel",
            Some(serde_json::json!({ "nodeId": node_id })),
        )
        .await
        .map_err(|e| {
            AppError::screenshot_failed(&format!("Failed to get element bounding box: {e}"))
        })?;

    extract_clip_from_box_model(&box_result)
        .ok_or_else(|| AppError::screenshot_failed("Element has no visible bounding box"))
}

/// Resolve an element's bounding box as a clip region using an accessibility UID.
async fn resolve_uid_clip(managed: &mut ManagedSession, uid: &str) -> Result<ClipRegion, AppError> {
    managed.ensure_domain("DOM").await?;

    let state = crate::snapshot::read_snapshot_state()
        .map_err(|e| AppError::screenshot_failed(&format!("Failed to read snapshot state: {e}")))?
        .ok_or_else(|| AppError {
            message: "No snapshot state found. Run 'chrome-cli page snapshot' first.".to_string(),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

    let backend_node_id = state
        .uid_map
        .get(uid)
        .ok_or_else(|| AppError::uid_not_found(uid))?;

    let describe = managed
        .send_command(
            "DOM.describeNode",
            Some(serde_json::json!({ "backendNodeId": backend_node_id })),
        )
        .await
        .map_err(|e| AppError::screenshot_failed(&format!("Failed to resolve UID '{uid}': {e}")))?;

    let node_id = describe["node"]["nodeId"].as_i64().ok_or_else(|| {
        AppError::screenshot_failed(&format!("UID '{uid}' could not be resolved to a DOM node"))
    })?;

    let box_result = managed
        .send_command(
            "DOM.getBoxModel",
            Some(serde_json::json!({ "nodeId": node_id })),
        )
        .await
        .map_err(|e| {
            AppError::screenshot_failed(&format!("Failed to get element bounding box: {e}"))
        })?;

    extract_clip_from_box_model(&box_result)
        .ok_or_else(|| AppError::screenshot_failed("Element has no visible bounding box"))
}

/// Get the full page dimensions (scroll width/height) via `Runtime.evaluate`.
async fn get_page_dimensions(managed: &ManagedSession) -> Result<(f64, f64), AppError> {
    let result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": "JSON.stringify({ width: Math.max(document.documentElement.scrollWidth, document.documentElement.clientWidth), height: Math.max(document.documentElement.scrollHeight, document.documentElement.clientHeight) })",
                "returnByValue": true,
            })),
        )
        .await
        .map_err(|e| AppError::screenshot_failed(&format!("Failed to get page dimensions: {e}")))?;

    let value_str = result["result"]["value"]
        .as_str()
        .ok_or_else(|| AppError::screenshot_failed("Failed to read page dimensions"))?;
    let dims: serde_json::Value = serde_json::from_str(value_str).map_err(|e| {
        AppError::screenshot_failed(&format!("Failed to parse page dimensions: {e}"))
    })?;

    let width = dims["width"].as_f64().unwrap_or(1280.0);
    let height = dims["height"].as_f64().unwrap_or(720.0);

    Ok((width, height))
}

/// Get the current viewport dimensions via `Runtime.evaluate`.
async fn get_viewport_dimensions(managed: &ManagedSession) -> Result<(u32, u32), AppError> {
    let result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": "JSON.stringify({ width: window.innerWidth, height: window.innerHeight })",
                "returnByValue": true,
            })),
        )
        .await
        .map_err(|e| {
            AppError::screenshot_failed(&format!("Failed to get viewport dimensions: {e}"))
        })?;

    let value_str = result["result"]["value"]
        .as_str()
        .ok_or_else(|| AppError::screenshot_failed("Failed to read viewport dimensions"))?;
    let dims: serde_json::Value = serde_json::from_str(value_str).map_err(|e| {
        AppError::screenshot_failed(&format!("Failed to parse viewport dimensions: {e}"))
    })?;

    #[allow(clippy::cast_possible_truncation)]
    let width = dims["width"].as_u64().unwrap_or(1280) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let height = dims["height"].as_u64().unwrap_or(720) as u32;

    Ok((width, height))
}

/// Set the viewport size via `Emulation.setDeviceMetricsOverride`.
async fn set_viewport_size(
    managed: &ManagedSession,
    width: u32,
    height: u32,
) -> Result<(), AppError> {
    managed
        .send_command(
            "Emulation.setDeviceMetricsOverride",
            Some(serde_json::json!({
                "width": width,
                "height": height,
                "deviceScaleFactor": 1,
                "mobile": false,
            })),
        )
        .await
        .map_err(|e| AppError::screenshot_failed(&format!("Failed to set viewport: {e}")))?;
    Ok(())
}

/// Clear viewport override via `Emulation.clearDeviceMetricsOverride`.
async fn clear_viewport_override(managed: &ManagedSession) -> Result<(), AppError> {
    managed
        .send_command("Emulation.clearDeviceMetricsOverride", None)
        .await
        .map_err(|e| AppError::screenshot_failed(&format!("Failed to restore viewport: {e}")))?;
    Ok(())
}

/// Map `ScreenshotFormat` to CDP format string.
fn screenshot_format_str(format: ScreenshotFormat) -> &'static str {
    match format {
        ScreenshotFormat::Png => "png",
        ScreenshotFormat::Jpeg => "jpeg",
        ScreenshotFormat::Webp => "webp",
    }
}

/// Large image threshold for base64 warnings (10MB).
const LARGE_IMAGE_THRESHOLD: usize = 10_000_000;

/// Convert a clip region's dimensions to `(width, height)` as `u32`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn clip_dimensions(clip: &ClipRegion) -> (u32, u32) {
    (clip.width as u32, clip.height as u32)
}

#[allow(clippy::too_many_lines)]
async fn execute_screenshot(
    global: &GlobalOpts,
    args: &PageScreenshotArgs,
) -> Result<(), AppError> {
    // Validate mutual exclusion: --full-page vs --selector/--uid
    if args.full_page && (args.selector.is_some() || args.uid.is_some()) {
        return Err(AppError::screenshot_failed(
            "Cannot combine --full-page with --selector or --uid",
        ));
    }

    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("Page").await?;
    managed.ensure_domain("Runtime").await?;

    let format_str = screenshot_format_str(args.format);

    // Determine capture strategy
    let (clip, capture_beyond_viewport, dimensions) = if let Some(ref selector) = args.selector {
        managed.ensure_domain("DOM").await?;
        let clip = resolve_selector_clip(&managed, selector).await?;
        let dims = clip_dimensions(&clip);
        (Some(clip), false, dims)
    } else if let Some(ref uid) = args.uid {
        let clip = resolve_uid_clip(&mut managed, uid).await?;
        let dims = clip_dimensions(&clip);
        (Some(clip), false, dims)
    } else if args.full_page {
        let (page_w, page_h) = get_page_dimensions(&managed).await?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let dims = (page_w as u32, page_h as u32);
        set_viewport_size(&managed, dims.0, dims.1).await?;
        (None, true, dims)
    } else if let Some(ref clip_str) = args.clip {
        let clip = parse_clip(clip_str)?;
        let dims = clip_dimensions(&clip);
        (Some(clip), false, dims)
    } else {
        let dims = get_viewport_dimensions(&managed).await?;
        (None, false, dims)
    };

    // Build CDP params
    let mut params = serde_json::json!({ "format": format_str });

    if !matches!(args.format, ScreenshotFormat::Png) {
        let quality = args.quality.unwrap_or(80);
        params["quality"] = serde_json::json!(quality);
    }

    if let Some(ref clip) = clip {
        params["clip"] = serde_json::json!({
            "x": clip.x,
            "y": clip.y,
            "width": clip.width,
            "height": clip.height,
            "scale": 1,
        });
    }

    if capture_beyond_viewport {
        params["captureBeyondViewport"] = serde_json::json!(true);
    }

    // Capture
    let result = managed
        .send_command("Page.captureScreenshot", Some(params))
        .await
        .map_err(|e| AppError::screenshot_failed(&e.to_string()))?;

    // Restore viewport if full-page
    if args.full_page {
        clear_viewport_override(&managed).await?;
    }

    let data = result["data"]
        .as_str()
        .ok_or_else(|| AppError::screenshot_failed("No image data in response"))?;

    let (width, height) = dimensions;

    if data.len() > LARGE_IMAGE_THRESHOLD {
        eprintln!(
            "warning: screenshot data is {}MB (base64)",
            data.len() / 1_000_000
        );
    }

    if let Some(ref file_path) = args.file {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| {
                AppError::screenshot_failed(&format!("Failed to decode image data: {e}"))
            })?;
        std::fs::write(file_path, &bytes).map_err(|e| {
            AppError::screenshot_failed(&format!(
                "Failed to write screenshot to file: {}: {e}",
                file_path.display()
            ))
        })?;

        let output = ScreenshotFileResult {
            format: format_str.to_string(),
            file: file_path.display().to_string(),
            width,
            height,
        };
        print_output(&output, &global.output)
    } else {
        let output = ScreenshotResult {
            format: format_str.to_string(),
            data: data.to_string(),
            width,
            height,
        };
        print_output(&output, &global.output)
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

    // =========================================================================
    // Screenshot output type serialization tests
    // =========================================================================

    #[test]
    fn screenshot_result_serialization() {
        let result = ScreenshotResult {
            format: "png".to_string(),
            data: "iVBORw0KGgo=".to_string(),
            width: 1280,
            height: 720,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["format"], "png");
        assert_eq!(json["data"], "iVBORw0KGgo=");
        assert_eq!(json["width"], 1280);
        assert_eq!(json["height"], 720);
        assert!(json.get("file").is_none());
    }

    #[test]
    fn screenshot_file_result_serialization() {
        let result = ScreenshotFileResult {
            format: "jpeg".to_string(),
            file: "/tmp/screenshot.jpg".to_string(),
            width: 800,
            height: 600,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["format"], "jpeg");
        assert_eq!(json["file"], "/tmp/screenshot.jpg");
        assert_eq!(json["width"], 800);
        assert_eq!(json["height"], 600);
        assert!(json.get("data").is_none());
    }

    #[test]
    fn screenshot_result_webp_format() {
        let result = ScreenshotResult {
            format: "webp".to_string(),
            data: "UklGR...".to_string(),
            width: 640,
            height: 480,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["format"], "webp");
    }

    // =========================================================================
    // parse_clip tests
    // =========================================================================

    #[test]
    fn parse_clip_valid() {
        let clip = parse_clip("10,20,200,100").unwrap();
        assert!((clip.x - 10.0).abs() < f64::EPSILON);
        assert!((clip.y - 20.0).abs() < f64::EPSILON);
        assert!((clip.width - 200.0).abs() < f64::EPSILON);
        assert!((clip.height - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_clip_decimal() {
        let clip = parse_clip("10.5,20.5,200.5,100.5").unwrap();
        assert!((clip.x - 10.5).abs() < f64::EPSILON);
        assert!((clip.y - 20.5).abs() < f64::EPSILON);
        assert!((clip.width - 200.5).abs() < f64::EPSILON);
        assert!((clip.height - 100.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_clip_with_spaces() {
        let clip = parse_clip("10 , 20 , 200 , 100").unwrap();
        assert!((clip.x - 10.0).abs() < f64::EPSILON);
        assert!((clip.y - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_clip_invalid_text() {
        let err = parse_clip("abc").unwrap_err();
        assert!(err.message.contains("Invalid clip format"));
    }

    #[test]
    fn parse_clip_too_few_values() {
        let err = parse_clip("10,20,200").unwrap_err();
        assert!(err.message.contains("Invalid clip format"));
    }

    #[test]
    fn parse_clip_non_numeric() {
        let err = parse_clip("10,abc,200,100").unwrap_err();
        assert!(err.message.contains("Invalid clip format"));
    }

    #[test]
    fn parse_clip_empty() {
        let err = parse_clip("").unwrap_err();
        assert!(err.message.contains("Invalid clip format"));
    }

    // =========================================================================
    // screenshot_format_str tests
    // =========================================================================

    #[test]
    fn screenshot_format_str_mapping() {
        assert_eq!(screenshot_format_str(ScreenshotFormat::Png), "png");
        assert_eq!(screenshot_format_str(ScreenshotFormat::Jpeg), "jpeg");
        assert_eq!(screenshot_format_str(ScreenshotFormat::Webp), "webp");
    }

    // =========================================================================
    // extract_clip_from_box_model tests
    // =========================================================================

    #[test]
    fn extract_clip_valid_box_model() {
        let box_model = serde_json::json!({
            "model": {
                "content": [10.0, 20.0, 210.0, 20.0, 210.0, 120.0, 10.0, 120.0]
            }
        });
        let clip = extract_clip_from_box_model(&box_model).unwrap();
        assert!((clip.x - 10.0).abs() < f64::EPSILON);
        assert!((clip.y - 20.0).abs() < f64::EPSILON);
        assert!((clip.width - 200.0).abs() < f64::EPSILON);
        assert!((clip.height - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn extract_clip_insufficient_content() {
        let box_model = serde_json::json!({
            "model": { "content": [10.0, 20.0] }
        });
        assert!(extract_clip_from_box_model(&box_model).is_none());
    }

    #[test]
    fn extract_clip_missing_model() {
        let box_model = serde_json::json!({});
        assert!(extract_clip_from_box_model(&box_model).is_none());
    }
}
