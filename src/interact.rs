use std::time::Duration;

use serde::Serialize;

use chrome_cli::cdp::{CdpClient, CdpConfig};
use chrome_cli::connection::{ManagedSession, resolve_connection, resolve_target};
use chrome_cli::error::AppError;

use crate::cli::{
    ClickArgs, ClickAtArgs, DragArgs, GlobalOpts, HoverArgs, InteractArgs, InteractCommand,
};
use crate::snapshot;

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct Coords {
    x: f64,
    y: f64,
}

#[derive(Serialize)]
struct DragTargets {
    from: String,
    to: String,
}

#[derive(Serialize)]
struct ClickResult {
    clicked: String,
    url: String,
    navigated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    double_click: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    right_click: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ClickAtResult {
    clicked_at: Coords,
    #[serde(skip_serializing_if = "Option::is_none")]
    double_click: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    right_click: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct HoverResult {
    hovered: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct DragResult {
    dragged: DragTargets,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
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
        code: chrome_cli::error::ExitCode::GeneralError,
    })?;
    println!("{json}");
    Ok(())
}

fn print_click_plain(result: &ClickResult) {
    if result.double_click == Some(true) {
        println!("Double-clicked {}", result.clicked);
    } else if result.right_click == Some(true) {
        println!("Right-clicked {}", result.clicked);
    } else {
        println!("Clicked {}", result.clicked);
    }
}

fn print_click_at_plain(result: &ClickAtResult) {
    if result.double_click == Some(true) {
        println!(
            "Double-clicked at ({}, {})",
            result.clicked_at.x, result.clicked_at.y
        );
    } else if result.right_click == Some(true) {
        println!(
            "Right-clicked at ({}, {})",
            result.clicked_at.x, result.clicked_at.y
        );
    } else {
        println!(
            "Clicked at ({}, {})",
            result.clicked_at.x, result.clicked_at.y
        );
    }
}

fn print_hover_plain(result: &HoverResult) {
    println!("Hovered {}", result.hovered);
}

fn print_drag_plain(result: &DragResult) {
    println!("Dragged {} to {}", result.dragged.from, result.dragged.to);
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
    let managed = ManagedSession::new(session);

    Ok((client, managed))
}

// =============================================================================
// Target resolution helpers
// =============================================================================

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

/// Resolve a target (UID or CSS selector) to a backend DOM node ID.
///
/// For UIDs: reads snapshot state and looks up the backendDOMNodeId.
/// For CSS selectors: queries the DOM and resolves the node.
async fn resolve_target_to_backend_node_id(
    session: &mut ManagedSession,
    target: &str,
) -> Result<i64, AppError> {
    if is_uid(target) {
        // Read snapshot state
        let state = snapshot::read_snapshot_state()?
            .ok_or_else(AppError::no_snapshot_state)?;

        // Lookup UID in the map
        let backend_node_id = state
            .uid_map
            .get(target)
            .copied()
            .ok_or_else(|| AppError::uid_not_found(target))?;

        Ok(backend_node_id)
    } else if is_css_selector(target) {
        // Strip 'css:' prefix
        let selector = &target[4..];

        // Get document root node ID
        let doc_response = session.send_command("DOM.getDocument", None).await?;
        let root_node_id = doc_response["root"]["nodeId"]
            .as_i64()
            .ok_or_else(|| AppError::element_not_found(selector))?;

        // Query selector
        let query_params = serde_json::json!({
            "nodeId": root_node_id,
            "selector": selector,
        });
        let query_response = session
            .send_command("DOM.querySelector", Some(query_params))
            .await?;

        let node_id = query_response["nodeId"].as_i64().unwrap_or(0);
        if node_id == 0 {
            return Err(AppError::element_not_found(selector));
        }

        // Get backend node ID via describeNode
        let describe_params = serde_json::json!({ "nodeId": node_id });
        let describe_response = session
            .send_command("DOM.describeNode", Some(describe_params))
            .await?;

        let backend_node_id = describe_response["node"]["backendNodeId"]
            .as_i64()
            .ok_or_else(|| AppError::element_not_found(selector))?;

        Ok(backend_node_id)
    } else {
        Err(AppError::element_not_found(target))
    }
}

/// Get the center coordinates of an element by its backend node ID.
///
/// Returns (x, y) coordinates of the element's center.
async fn get_element_center(
    session: &mut ManagedSession,
    backend_node_id: i64,
) -> Result<(f64, f64), AppError> {
    let params = serde_json::json!({ "backendNodeId": backend_node_id });
    let response = session
        .send_command("DOM.getBoxModel", Some(params))
        .await
        .map_err(|e| AppError::interaction_failed("get_element_center", &e.to_string()))?;

    let content = response["model"]["content"]
        .as_array()
        .ok_or_else(|| AppError::element_zero_size("element"))?;

    if content.len() < 8 {
        return Err(AppError::element_zero_size("element"));
    }

    // content is [x1, y1, x2, y2, x3, y3, x4, y4]
    // Center = ((x1 + x3) / 2, (y1 + y3) / 2)
    let x1 = content[0].as_f64().unwrap_or(0.0);
    let y1 = content[1].as_f64().unwrap_or(0.0);
    let x3 = content[4].as_f64().unwrap_or(0.0);
    let y3 = content[5].as_f64().unwrap_or(0.0);

    let center_x = (x1 + x3) / 2.0;
    let center_y = (y1 + y3) / 2.0;

    // Check for zero-size
    if (x3 - x1).abs() < 1.0 || (y3 - y1).abs() < 1.0 {
        return Err(AppError::element_zero_size("element"));
    }

    Ok((center_x, center_y))
}

/// Scroll an element into view if needed.
async fn scroll_into_view(
    session: &mut ManagedSession,
    backend_node_id: i64,
) -> Result<(), AppError> {
    let params = serde_json::json!({ "backendNodeId": backend_node_id });
    session
        .send_command("DOM.scrollIntoViewIfNeeded", Some(params))
        .await
        .map_err(|e| AppError::interaction_failed("scroll_into_view", &e.to_string()))?;
    Ok(())
}

/// High-level function to resolve a target to coordinates.
///
/// Steps:
/// 1. Resolve target to backend node ID
/// 2. Scroll element into view
/// 3. Get element center coordinates
async fn resolve_target_coords(
    session: &mut ManagedSession,
    target: &str,
) -> Result<(f64, f64), AppError> {
    let backend_node_id = resolve_target_to_backend_node_id(session, target).await?;
    scroll_into_view(session, backend_node_id).await?;
    get_element_center(session, backend_node_id).await
}

// =============================================================================
// Mouse dispatch helpers
// =============================================================================

/// Dispatch a click (or double-click, or right-click) at the given coordinates.
///
/// - `button`: "left" or "right"
/// - `click_count`: 1 for single click, 2 for double click
async fn dispatch_click(
    session: &mut ManagedSession,
    x: f64,
    y: f64,
    button: &str,
    click_count: u8,
) -> Result<(), AppError> {
    if click_count == 2 {
        // For double-click, send: press(1) → release(1) → press(2) → release(2)
        // First click
        let press_params = serde_json::json!({
            "type": "mousePressed",
            "x": x,
            "y": y,
            "button": button,
            "clickCount": 1,
        });
        session
            .send_command("Input.dispatchMouseEvent", Some(press_params))
            .await
            .map_err(|e| AppError::interaction_failed("mouse_press", &e.to_string()))?;

        let release_params = serde_json::json!({
            "type": "mouseReleased",
            "x": x,
            "y": y,
            "button": button,
            "clickCount": 1,
        });
        session
            .send_command("Input.dispatchMouseEvent", Some(release_params))
            .await
            .map_err(|e| AppError::interaction_failed("mouse_release", &e.to_string()))?;

        // Second click
        let press_params = serde_json::json!({
            "type": "mousePressed",
            "x": x,
            "y": y,
            "button": button,
            "clickCount": 2,
        });
        session
            .send_command("Input.dispatchMouseEvent", Some(press_params))
            .await
            .map_err(|e| AppError::interaction_failed("mouse_press", &e.to_string()))?;

        let release_params = serde_json::json!({
            "type": "mouseReleased",
            "x": x,
            "y": y,
            "button": button,
            "clickCount": 2,
        });
        session
            .send_command("Input.dispatchMouseEvent", Some(release_params))
            .await
            .map_err(|e| AppError::interaction_failed("mouse_release", &e.to_string()))?;
    } else {
        // Single click
        let press_params = serde_json::json!({
            "type": "mousePressed",
            "x": x,
            "y": y,
            "button": button,
            "clickCount": click_count,
        });
        session
            .send_command("Input.dispatchMouseEvent", Some(press_params))
            .await
            .map_err(|e| AppError::interaction_failed("mouse_press", &e.to_string()))?;

        let release_params = serde_json::json!({
            "type": "mouseReleased",
            "x": x,
            "y": y,
            "button": button,
            "clickCount": click_count,
        });
        session
            .send_command("Input.dispatchMouseEvent", Some(release_params))
            .await
            .map_err(|e| AppError::interaction_failed("mouse_release", &e.to_string()))?;
    }

    Ok(())
}

/// Dispatch a hover (mouse move) to the given coordinates.
async fn dispatch_hover(
    session: &mut ManagedSession,
    x: f64,
    y: f64,
) -> Result<(), AppError> {
    let params = serde_json::json!({
        "type": "mouseMoved",
        "x": x,
        "y": y,
    });
    session
        .send_command("Input.dispatchMouseEvent", Some(params))
        .await
        .map_err(|e| AppError::interaction_failed("mouse_move", &e.to_string()))?;
    Ok(())
}

/// Dispatch a drag operation from (`from_x`, `from_y`) to (`to_x`, `to_y`).
async fn dispatch_drag(
    session: &mut ManagedSession,
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
) -> Result<(), AppError> {
    // Press at start position
    let press_params = serde_json::json!({
        "type": "mousePressed",
        "x": from_x,
        "y": from_y,
        "button": "left",
        "clickCount": 1,
    });
    session
        .send_command("Input.dispatchMouseEvent", Some(press_params))
        .await
        .map_err(|e| AppError::interaction_failed("drag_press", &e.to_string()))?;

    // Move to end position
    let move_params = serde_json::json!({
        "type": "mouseMoved",
        "x": to_x,
        "y": to_y,
    });
    session
        .send_command("Input.dispatchMouseEvent", Some(move_params))
        .await
        .map_err(|e| AppError::interaction_failed("drag_move", &e.to_string()))?;

    // Release at end position
    let release_params = serde_json::json!({
        "type": "mouseReleased",
        "x": to_x,
        "y": to_y,
        "button": "left",
        "clickCount": 1,
    });
    session
        .send_command("Input.dispatchMouseEvent", Some(release_params))
        .await
        .map_err(|e| AppError::interaction_failed("drag_release", &e.to_string()))?;

    Ok(())
}

// =============================================================================
// Snapshot refresh helper
// =============================================================================

/// Take a fresh snapshot and write it to snapshot state.
///
/// Returns the snapshot tree as a JSON value.
async fn take_snapshot(
    session: &mut ManagedSession,
    url: &str,
) -> Result<serde_json::Value, AppError> {
    // Enable Accessibility domain
    session.ensure_domain("Accessibility").await?;

    // Get full AX tree
    let response = session
        .send_command("Accessibility.getFullAXTree", None)
        .await?;

    let nodes = response["nodes"]
        .as_array()
        .ok_or_else(|| AppError::snapshot_failed("missing nodes array"))?;

    // Build tree
    let build_result = snapshot::build_tree(nodes, false);

    // Write snapshot state
    let state = snapshot::SnapshotState {
        url: url.to_string(),
        timestamp: chrome_cli::session::now_iso8601(),
        uid_map: build_result.uid_map,
    };
    snapshot::write_snapshot_state(&state)?;

    // Serialize root node as JSON
    let snapshot_json = serde_json::to_value(&build_result.root).map_err(|e| {
        AppError::snapshot_failed(&format!("failed to serialize snapshot: {e}"))
    })?;

    Ok(snapshot_json)
}

// =============================================================================
// Command implementations
// =============================================================================

/// Execute the `interact click` command.
async fn execute_click(global: &GlobalOpts, args: &ClickArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;

    // Enable required domains
    managed.ensure_domain("DOM").await?;
    managed.ensure_domain("Page").await?;

    // Resolve target coordinates
    let (x, y) = resolve_target_coords(&mut managed, &args.target).await?;

    // Subscribe to navigation events
    let _nav_rx = managed.subscribe("Page.frameNavigated").await?;

    // Determine button and click count
    let button = if args.right { "right" } else { "left" };
    let click_count = if args.double { 2 } else { 1 };

    // Dispatch click
    dispatch_click(&mut managed, x, y, button, click_count).await?;

    // Brief wait for potential navigation (100ms)
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Get current URL
    let url_response = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": "window.location.href" })),
        )
        .await?;
    let url = url_response["result"]["value"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // For now, we'll set navigated to false (navigation detection can be enhanced later)
    let navigated = false;

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        Some(take_snapshot(&mut managed, &url).await?)
    } else {
        None
    };

    // Build result
    let result = ClickResult {
        clicked: args.target.clone(),
        url,
        navigated,
        double_click: if args.double { Some(true) } else { None },
        right_click: if args.right { Some(true) } else { None },
        snapshot,
    };

    if global.output.plain {
        print_click_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

/// Execute the `interact click-at` command.
async fn execute_click_at(global: &GlobalOpts, args: &ClickAtArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;

    // Determine button and click count
    let button = if args.right { "right" } else { "left" };
    let click_count = if args.double { 2 } else { 1 };

    // Dispatch click at coordinates
    dispatch_click(&mut managed, args.x, args.y, button, click_count).await?;

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        // Need to get current URL for snapshot state
        managed.ensure_domain("Runtime").await?;
        let url_response = managed
            .send_command(
                "Runtime.evaluate",
                Some(serde_json::json!({ "expression": "window.location.href" })),
            )
            .await?;
        let url = url_response["result"]["value"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Some(take_snapshot(&mut managed, &url).await?)
    } else {
        None
    };

    // Build result
    let result = ClickAtResult {
        clicked_at: Coords { x: args.x, y: args.y },
        double_click: if args.double { Some(true) } else { None },
        right_click: if args.right { Some(true) } else { None },
        snapshot,
    };

    if global.output.plain {
        print_click_at_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

/// Execute the `interact hover` command.
async fn execute_hover(global: &GlobalOpts, args: &HoverArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;

    // Enable DOM domain
    managed.ensure_domain("DOM").await?;

    // Resolve target coordinates
    let (x, y) = resolve_target_coords(&mut managed, &args.target).await?;

    // Dispatch hover
    dispatch_hover(&mut managed, x, y).await?;

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        // Need to get current URL for snapshot state
        managed.ensure_domain("Runtime").await?;
        let url_response = managed
            .send_command(
                "Runtime.evaluate",
                Some(serde_json::json!({ "expression": "window.location.href" })),
            )
            .await?;
        let url = url_response["result"]["value"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Some(take_snapshot(&mut managed, &url).await?)
    } else {
        None
    };

    // Build result
    let result = HoverResult {
        hovered: args.target.clone(),
        snapshot,
    };

    if global.output.plain {
        print_hover_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

/// Execute the `interact drag` command.
async fn execute_drag(global: &GlobalOpts, args: &DragArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;

    // Enable DOM domain
    managed.ensure_domain("DOM").await?;

    // Resolve "from" target
    let from_backend_id = resolve_target_to_backend_node_id(&mut managed, &args.from).await?;
    scroll_into_view(&mut managed, from_backend_id).await?;
    let (from_x, from_y) = get_element_center(&mut managed, from_backend_id).await?;

    // Resolve "to" target
    let (to_x, to_y) = resolve_target_coords(&mut managed, &args.to).await?;

    // Dispatch drag
    dispatch_drag(&mut managed, from_x, from_y, to_x, to_y).await?;

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        // Need to get current URL for snapshot state
        managed.ensure_domain("Runtime").await?;
        let url_response = managed
            .send_command(
                "Runtime.evaluate",
                Some(serde_json::json!({ "expression": "window.location.href" })),
            )
            .await?;
        let url = url_response["result"]["value"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Some(take_snapshot(&mut managed, &url).await?)
    } else {
        None
    };

    // Build result
    let result = DragResult {
        dragged: DragTargets {
            from: args.from.clone(),
            to: args.to.clone(),
        },
        snapshot,
    };

    if global.output.plain {
        print_drag_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Execute the `interact` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_interact(global: &GlobalOpts, args: &InteractArgs) -> Result<(), AppError> {
    match &args.command {
        InteractCommand::Click(click_args) => execute_click(global, click_args).await,
        InteractCommand::ClickAt(click_at_args) => execute_click_at(global, click_at_args).await,
        InteractCommand::Hover(hover_args) => execute_hover(global, hover_args).await,
        InteractCommand::Drag(drag_args) => execute_drag(global, drag_args).await,
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
        assert!(!is_uid("1s"));
    }

    #[test]
    fn is_css_selector_valid() {
        assert!(is_css_selector("css:#button"));
        assert!(is_css_selector("css:.class"));
        assert!(is_css_selector("css:div > p"));
    }

    #[test]
    fn is_css_selector_invalid() {
        assert!(!is_css_selector("#button"));
        assert!(!is_css_selector("s1"));
        assert!(!is_css_selector("button"));
    }

    #[test]
    fn click_result_serialization() {
        let result = ClickResult {
            clicked: "s1".to_string(),
            url: "https://example.com".to_string(),
            navigated: false,
            double_click: None,
            right_click: None,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["clicked"], "s1");
        assert_eq!(json["url"], "https://example.com");
        assert_eq!(json["navigated"], false);
        assert!(json.get("double_click").is_none());
        assert!(json.get("right_click").is_none());
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn click_result_serialization_with_double() {
        let result = ClickResult {
            clicked: "s1".to_string(),
            url: "https://example.com".to_string(),
            navigated: false,
            double_click: Some(true),
            right_click: None,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["double_click"], true);
        assert!(json.get("right_click").is_none());
    }

    #[test]
    fn click_result_serialization_with_right() {
        let result = ClickResult {
            clicked: "s1".to_string(),
            url: "https://example.com".to_string(),
            navigated: false,
            double_click: None,
            right_click: Some(true),
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["right_click"], true);
        assert!(json.get("double_click").is_none());
    }

    #[test]
    fn click_at_result_serialization() {
        let result = ClickAtResult {
            clicked_at: Coords { x: 100.0, y: 200.0 },
            double_click: None,
            right_click: None,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["clicked_at"]["x"], 100.0);
        assert_eq!(json["clicked_at"]["y"], 200.0);
        assert!(json.get("double_click").is_none());
    }

    #[test]
    fn hover_result_serialization() {
        let result = HoverResult {
            hovered: "s3".to_string(),
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["hovered"], "s3");
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn drag_result_serialization() {
        let result = DragResult {
            dragged: DragTargets {
                from: "s1".to_string(),
                to: "s2".to_string(),
            },
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["dragged"]["from"], "s1");
        assert_eq!(json["dragged"]["to"], "s2");
        assert!(json.get("snapshot").is_none());
    }
}
