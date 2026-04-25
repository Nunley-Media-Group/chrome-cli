//! CDP-dependent coordinate helpers for the binary crate.
//!
//! This module wraps the types from `agentchrome::coords` with the CDP calls
//! needed to resolve element bounding boxes and frame viewport offsets.
//! It lives in the binary crate because it depends on `crate::snapshot`.

use agentchrome::connection::ManagedSession;
use agentchrome::coords::BoundingBox;
use agentchrome::error::{AppError, ExitCode};

// =============================================================================
// frame_viewport_offset — moved from src/interact.rs
// =============================================================================

/// Get the top-left offset of a frame's viewport in page coordinates.
///
/// Used by `click-at --frame N` to translate frame-local coordinates to page-global ones.
/// Returns `(0.0, 0.0)` for the main frame.
///
/// **Note:** For OOPIF frames, this must be called with the **main** `managed` session (not the
/// OOPIF session), because the `<iframe>` owner element lives in the parent document.
pub(crate) async fn frame_viewport_offset(
    managed: &ManagedSession,
    frame_ctx: &agentchrome::frame::FrameContext,
) -> Result<(f64, f64), AppError> {
    let Some(frame_id) = agentchrome::frame::frame_id(frame_ctx) else {
        return Ok((0.0, 0.0)); // main frame — no offset
    };

    // Use DOM.getFrameOwner to get the backendNodeId of the <iframe> element. CDP errors
    // (connection / protocol / timeout) propagate via the From<CdpError> for AppError impl,
    // which preserves the proper exit code.
    let owner = managed
        .send_command(
            "DOM.getFrameOwner",
            Some(serde_json::json!({ "frameId": frame_id })),
        )
        .await?;

    let backend_node_id = owner["backendNodeId"].as_i64().unwrap_or(0);
    if backend_node_id == 0 {
        return Ok((0.0, 0.0));
    }

    // Get the box model of the <iframe> element in page coordinates
    let box_model = managed
        .send_command(
            "DOM.getBoxModel",
            Some(serde_json::json!({ "backendNodeId": backend_node_id })),
        )
        .await?;

    let content = box_model["model"]["content"].as_array();
    let (frame_x, frame_y) = if let Some(c) = content {
        let x = c.first().and_then(serde_json::Value::as_f64).unwrap_or(0.0);
        let y = c.get(1).and_then(serde_json::Value::as_f64).unwrap_or(0.0);
        (x, y)
    } else {
        (0.0, 0.0)
    };

    Ok((frame_x, frame_y))
}

// =============================================================================
// resolve_element_box — frame-aware element bounding-box lookup
// =============================================================================

/// Resolve a target (UID or CSS selector) to a [`BoundingBox`] in **frame-local** coordinates.
///
/// Does **not** apply the frame offset — the caller is responsible for that via
/// [`frame_viewport_offset`].
///
/// # Target formats
/// - UID: `"s7"` — looks up the backend node ID in the snapshot state.
/// - CSS: `"css:#submit"` — queries via `DOM.querySelector` for the main frame and OOPIFs, or
///   via `Runtime.evaluate` (in the frame's execution context) for same-origin iframes (whose
///   document is not directly addressable through `DOM.getDocument` on the page session).
///
/// # Errors
///
/// - Returns [`AppError`] with [`ExitCode::TargetError`] if the selector matches nothing.
/// - Returns [`AppError`] with [`ExitCode::GeneralError`] if snapshot state is missing for UID.
pub(crate) async fn resolve_element_box(
    managed: &ManagedSession,
    frame_ctx: Option<&agentchrome::frame::FrameContext>,
    selector: &str,
) -> Result<BoundingBox, AppError> {
    // For same-origin iframes with a CSS selector, route through Runtime.evaluate +
    // objectId so the query runs in the iframe's document, not the main document.
    // The page session's `DOM.getBoxModel` returns coordinates in page-global space, so we
    // subtract the frame offset to convert to the frame-local space this function promises.
    if let Some(ctx) = frame_ctx
        && is_element_css_selector(selector)
        && let Some(exec_ctx_id) = agentchrome::frame::execution_context_id(ctx)
    {
        let css = &selector[4..];
        let object_id = query_selector_in_execution_context(managed, exec_ctx_id, css)
            .await?
            .ok_or_else(|| AppError::css_selector_not_found(css))?;
        let page_global =
            fetch_bounding_box_inner(managed, serde_json::json!({ "objectId": object_id })).await?;
        let (off_x, off_y) = frame_viewport_offset(managed, ctx).await?;
        return Ok(BoundingBox {
            x: page_global.x - off_x,
            y: page_global.y - off_y,
            width: page_global.width,
            height: page_global.height,
        });
    }

    let backend_node_id = resolve_backend_node_id(managed, frame_ctx, selector).await?;
    fetch_bounding_box(managed, backend_node_id).await
}

/// Run `document.querySelector(<css>)` in a specific execution context and return the
/// resulting `RemoteObject.objectId`, or `None` if the selector matched nothing.
async fn query_selector_in_execution_context(
    managed: &ManagedSession,
    execution_context_id: i64,
    css: &str,
) -> Result<Option<String>, AppError> {
    // serde_json produces a JSON-quoted string that is also a valid JS string literal —
    // covers backslashes, quotes, control characters, and U+2028/U+2029 line separators.
    let json_quoted = serde_json::to_string(css).map_err(|e| AppError {
        message: format!("Failed to encode selector: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;
    let expression = format!("document.querySelector({json_quoted})");
    let result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": expression,
                "contextId": execution_context_id,
                "returnByValue": false,
            })),
        )
        .await?;

    if result["result"]["subtype"].as_str() == Some("null") {
        return Ok(None);
    }
    Ok(result["result"]["objectId"]
        .as_str()
        .map(std::string::ToString::to_string))
}

/// Check if a target string is a UID (matches pattern: 's' + digits).
fn is_element_uid(target: &str) -> bool {
    if !target.starts_with('s') {
        return false;
    }
    let rest = &target[1..];
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

/// Check if a target string is a CSS selector (starts with 'css:').
fn is_element_css_selector(target: &str) -> bool {
    target.starts_with("css:")
}

/// Resolve a target string to a CDP `backendNodeId`.
async fn resolve_backend_node_id(
    managed: &ManagedSession,
    frame_ctx: Option<&agentchrome::frame::FrameContext>,
    target: &str,
) -> Result<i64, AppError> {
    if is_element_uid(target) {
        // UID path — read snapshot state; does not depend on frame context
        let state = crate::snapshot::read_snapshot_state()
            .map_err(|e| AppError {
                message: format!("Failed to read snapshot state: {e}"),
                code: ExitCode::GeneralError,
                custom_json: None,
            })?
            .ok_or_else(AppError::no_snapshot_state)?;
        let backend_node_id = state
            .uid_map
            .get(target)
            .copied()
            .ok_or_else(|| AppError::element_target_not_found(target))?;
        Ok(backend_node_id)
    } else if is_element_css_selector(target) {
        let selector = &target[4..];

        // Route queries through the effective frame session so iframe elements are found
        let effective = if let Some(ctx) = frame_ctx {
            agentchrome::frame::frame_session(ctx, managed)
        } else {
            managed
        };

        let doc_response = effective.send_command("DOM.getDocument", None).await?;
        let root_node_id = doc_response["root"]["nodeId"]
            .as_i64()
            .ok_or_else(|| AppError::css_selector_not_found(selector))?;

        let query_params = serde_json::json!({
            "nodeId": root_node_id,
            "selector": selector,
        });
        let query_response = effective
            .send_command("DOM.querySelector", Some(query_params))
            .await?;

        let node_id = query_response["nodeId"].as_i64().unwrap_or(0);
        if node_id == 0 {
            return Err(AppError::css_selector_not_found(selector));
        }

        let describe_params = serde_json::json!({ "nodeId": node_id });
        let describe_response = effective
            .send_command("DOM.describeNode", Some(describe_params))
            .await?;

        let backend_node_id = describe_response["node"]["backendNodeId"]
            .as_i64()
            .ok_or_else(|| AppError::css_selector_not_found(selector))?;

        Ok(backend_node_id)
    } else {
        Err(AppError::element_target_not_found(target))
    }
}

/// Fetch the bounding box for a given `backendNodeId` via `DOM.getBoxModel`.
///
/// Uses `model.border` (the border quad), which corresponds to the rectangle returned by
/// `Element.getBoundingClientRect()` in JS. The border quad is what most users mean when
/// they refer to an element's bounding box — it includes padding and border, but not margin.
async fn fetch_bounding_box(
    managed: &ManagedSession,
    backend_node_id: i64,
) -> Result<BoundingBox, AppError> {
    let params = serde_json::json!({ "backendNodeId": backend_node_id });
    fetch_bounding_box_inner(managed, params).await
}

/// Fetch the bounding box for a node referenced via CDP (by `nodeId`, `backendNodeId`, or
/// `objectId`). Uses the `border` quad from `DOM.getBoxModel`.
///
/// CDP errors (connection / protocol / timeout) propagate via `From<CdpError> for AppError`,
/// preserving the proper exit code. A response with no usable `border` quad returns a zero
/// bounding box (the same fall-through pattern as `page/element.rs`).
async fn fetch_bounding_box_inner(
    managed: &ManagedSession,
    params: serde_json::Value,
) -> Result<BoundingBox, AppError> {
    let box_result = managed
        .send_command("DOM.getBoxModel", Some(params))
        .await?;

    let border = box_result["model"]["border"].as_array();
    let (x, y, width, height) = match border {
        Some(c) if c.len() >= 8 => {
            let x1 = c[0].as_f64().unwrap_or(0.0);
            let y1 = c[1].as_f64().unwrap_or(0.0);
            let x3 = c[4].as_f64().unwrap_or(0.0);
            let y3 = c[5].as_f64().unwrap_or(0.0);
            (x1, y1, x3 - x1, y3 - y1)
        }
        _ => (0.0, 0.0, 0.0, 0.0),
    };

    Ok(BoundingBox {
        x,
        y,
        width,
        height,
    })
}
