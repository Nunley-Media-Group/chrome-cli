use serde::Serialize;

use crate::cdp::CdpClient;
use crate::connection::ManagedSession;
use crate::error::{AppError, ExitCode};

// =============================================================================
// Types
// =============================================================================

/// Parsed `--frame` CLI argument.
#[derive(Debug, Clone)]
pub enum FrameArg {
    /// Flat integer index (e.g., `--frame 2`).
    Index(u32),
    /// Parent-child path (e.g., `--frame 1/0`).
    Path(Vec<u32>),
    /// Automatic search (e.g., `--frame auto`).
    Auto,
}

/// Metadata for a single frame in the page hierarchy.
#[derive(Debug, Clone, Serialize)]
pub struct FrameInfo {
    pub index: u32,
    pub id: String,
    pub url: String,
    pub name: String,
    #[serde(rename = "securityOrigin")]
    pub security_origin: String,
    pub unreachable: bool,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    #[serde(skip)]
    pub parent_id: Option<String>,
    #[serde(skip)]
    pub child_ids: Vec<String>,
}

/// Resolved frame targeting context for CDP operations.
///
/// Either reuses the page session with frame-scoped parameters (same-origin),
/// or holds a separate OOPIF session (cross-origin).
pub enum FrameContext {
    /// Main frame — no additional scoping needed.
    MainFrame,
    /// Same-origin iframe: reuse page session, pass `frameId` to CDP methods.
    SameOrigin {
        frame_id: String,
        execution_context_id: i64,
    },
    /// Cross-origin OOPIF: separate CDP session attached to the frame target.
    OutOfProcess {
        session: ManagedSession,
        frame_id: String,
    },
}

// =============================================================================
// Argument parsing
// =============================================================================

/// Parse a `--frame` CLI value into a [`FrameArg`].
///
/// Accepts an integer, a slash-separated path (e.g. `1/0`), or the literal `auto`.
///
/// # Errors
///
/// Returns `AppError` if the value is not a valid integer, path, or `"auto"`.
pub fn parse_frame_arg(value: &str) -> Result<FrameArg, AppError> {
    if value == "auto" {
        return Ok(FrameArg::Auto);
    }

    if value.contains('/') {
        let segments: Result<Vec<u32>, _> = value.split('/').map(str::parse::<u32>).collect();
        match segments {
            Ok(path) if !path.is_empty() => Ok(FrameArg::Path(path)),
            _ => Err(AppError {
                message: format!(
                    "Invalid frame path: '{value}'. \
                     Expected slash-separated integers (e.g., 1/0)."
                ),
                code: ExitCode::GeneralError,
                custom_json: None,
            }),
        }
    } else {
        value
            .parse::<u32>()
            .map(FrameArg::Index)
            .map_err(|_| AppError {
                message: format!(
                    "Invalid frame value: '{value}'. \
                 Expected integer index, path (1/0), or 'auto'."
                ),
                code: ExitCode::GeneralError,
                custom_json: None,
            })
    }
}

// =============================================================================
// Frame enumeration
// =============================================================================

/// Enumerate all frames via `Page.getFrameTree`.
///
/// Returns an ordered list with the main frame at index 0, followed by child
/// frames in depth-first document order.
///
/// # Errors
///
/// Returns `AppError` if the CDP `Page.getFrameTree` call fails.
pub async fn list_frames(session: &mut ManagedSession) -> Result<Vec<FrameInfo>, AppError> {
    session.ensure_domain("Page").await.map_err(|e| AppError {
        message: format!("Failed to enable Page domain: {e}"),
        code: ExitCode::ProtocolError,
        custom_json: None,
    })?;

    let result = session
        .send_command("Page.getFrameTree", None)
        .await
        .map_err(|e| AppError {
            message: format!("Failed to get frame tree: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let frame_tree = &result["frameTree"];
    let mut frames = Vec::new();
    let mut index = 0u32;
    traverse_frame_tree(frame_tree, 0, None, &mut index, &mut frames);

    // Try to get dimensions for child frames via DOM.getFrameOwner + DOM.getBoxModel.
    let child_ids: Vec<(usize, String)> = frames
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, f)| (i, f.id.clone()))
        .collect();
    for (i, frame_id) in child_ids {
        if let Ok(dims) = get_frame_dimensions(session, &frame_id).await {
            frames[i].width = dims.0;
            frames[i].height = dims.1;
        }
    }

    // Main frame dimensions from viewport.
    if !frames.is_empty()
        && let Ok(dims) = get_viewport_dimensions(session).await
    {
        frames[0].width = dims.0;
        frames[0].height = dims.1;
    }

    Ok(frames)
}

fn traverse_frame_tree(
    node: &serde_json::Value,
    depth: u32,
    parent_id: Option<&str>,
    index: &mut u32,
    frames: &mut Vec<FrameInfo>,
) {
    let frame = &node["frame"];
    let current_index = *index;
    *index += 1;

    let id = frame["id"].as_str().unwrap_or_default().to_string();
    let child_frames = node["childFrames"].as_array();

    let child_ids: Vec<String> = child_frames
        .map(|children| {
            children
                .iter()
                .filter_map(|c| c["frame"]["id"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    frames.push(FrameInfo {
        index: current_index,
        id: id.clone(),
        url: frame["url"].as_str().unwrap_or_default().to_string(),
        name: frame["name"].as_str().unwrap_or_default().to_string(),
        security_origin: frame["securityOrigin"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        unreachable: frame["unreachableUrl"].as_str().is_some(),
        width: 0,
        height: 0,
        depth,
        parent_id: parent_id.map(String::from),
        child_ids,
    });

    if let Some(children) = child_frames {
        for child in children {
            traverse_frame_tree(child, depth + 1, Some(&id), index, frames);
        }
    }
}

/// Get the viewport dimensions for the main frame.
async fn get_viewport_dimensions(session: &ManagedSession) -> Result<(u32, u32), AppError> {
    let result = session
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": "JSON.stringify({w:window.innerWidth,h:window.innerHeight})",
                "returnByValue": true,
            })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("Failed to get viewport dimensions: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let val_str = result["result"]["value"].as_str().unwrap_or("{}");
    let dims: serde_json::Value = serde_json::from_str(val_str).unwrap_or_default();

    #[allow(clippy::cast_possible_truncation)]
    let w = dims["w"].as_u64().unwrap_or(0) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let h = dims["h"].as_u64().unwrap_or(0) as u32;

    Ok((w, h))
}

/// Get the dimensions of a child frame via its owner element.
async fn get_frame_dimensions(
    session: &ManagedSession,
    frame_id: &str,
) -> Result<(u32, u32), AppError> {
    let owner = session
        .send_command(
            "DOM.getFrameOwner",
            Some(serde_json::json!({ "frameId": frame_id })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("Failed to get frame owner: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let backend_node_id = owner["backendNodeId"]
        .as_i64()
        .ok_or_else(AppError::frame_detached)?;

    let box_model = session
        .send_command(
            "DOM.getBoxModel",
            Some(serde_json::json!({ "backendNodeId": backend_node_id })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("Failed to get frame box model: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let content = box_model["model"]["content"]
        .as_array()
        .ok_or_else(AppError::frame_detached)?;

    // Content quad: [x1,y1, x2,y2, x3,y3, x4,y4]
    if content.len() >= 8 {
        let x1 = content[0].as_f64().unwrap_or(0.0);
        let y1 = content[1].as_f64().unwrap_or(0.0);
        let x3 = content[4].as_f64().unwrap_or(0.0);
        let y3 = content[5].as_f64().unwrap_or(0.0);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let w = (x3 - x1).abs() as u32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let h = (y3 - y1).abs() as u32;

        Ok((w, h))
    } else {
        Ok((0, 0))
    }
}

// =============================================================================
// Frame resolution
// =============================================================================

/// Resolve a [`FrameArg`] to a [`FrameContext`].
///
/// For `FrameArg::Auto`, use [`resolve_frame_auto`] instead.
///
/// # Errors
///
/// Returns `AppError` if the frame index is invalid or CDP calls fail.
pub async fn resolve_frame(
    client: &CdpClient,
    session: &mut ManagedSession,
    arg: &FrameArg,
) -> Result<FrameContext, AppError> {
    match arg {
        FrameArg::Index(0) => Ok(FrameContext::MainFrame),
        FrameArg::Index(n) => {
            let frames = list_frames(session).await?;
            let frame = frames
                .iter()
                .find(|f| f.index == *n)
                .ok_or_else(|| AppError::frame_not_found(*n))?;
            resolve_frame_by_info(client, session, frame).await
        }
        FrameArg::Path(segments) => {
            let frames = list_frames(session).await?;
            resolve_frame_by_path(client, session, &frames, segments).await
        }
        FrameArg::Auto => Err(AppError {
            message: "--frame auto requires a target UID. Use resolve_frame_auto() instead.".into(),
            code: ExitCode::GeneralError,
            custom_json: None,
        }),
    }
}

/// Resolve a single frame to a [`FrameContext`], detecting OOPIF vs same-origin.
async fn resolve_frame_by_info(
    client: &CdpClient,
    session: &mut ManagedSession,
    frame: &FrameInfo,
) -> Result<FrameContext, AppError> {
    // Check if the frame has a separate target (OOPIF).
    let targets_result = client.send_command("Target.getTargets", None).await;

    if let Ok(targets_result) = targets_result
        && let Some(targets) = targets_result["targetInfos"].as_array()
    {
        for target in targets {
            let target_type = target["type"].as_str().unwrap_or_default();
            let target_id = target["targetId"].as_str().unwrap_or_default();

            // Match OOPIF by target ID == frame ID, or by iframe type + URL match.
            let is_match = target_id == frame.id
                || (target_type == "iframe" && target["url"].as_str() == Some(frame.url.as_str()));

            if is_match {
                let oopif_session =
                    client
                        .create_session(target_id)
                        .await
                        .map_err(|e| AppError {
                            message: format!("Failed to attach to frame target: {e}"),
                            code: ExitCode::ProtocolError,
                            custom_json: None,
                        })?;
                return Ok(FrameContext::OutOfProcess {
                    session: ManagedSession::new(oopif_session),
                    frame_id: frame.id.clone(),
                });
            }
        }
    }

    // Same-origin frame — find its execution context.
    let context_id = find_execution_context(session, &frame.id).await?;

    Ok(FrameContext::SameOrigin {
        frame_id: frame.id.clone(),
        execution_context_id: context_id,
    })
}

/// Find the execution context ID for a same-origin frame.
///
/// Subscribes to `Runtime.executionContextCreated` events (which replay existing
/// contexts when the `Runtime` domain is enabled) and matches by `frameId`.
/// Falls back to `Page.createIsolatedWorld` if no matching context is found.
async fn find_execution_context(
    session: &mut ManagedSession,
    frame_id: &str,
) -> Result<i64, AppError> {
    // Subscribe to context events BEFORE enabling Runtime so that the replayed
    // executionContextCreated events are captured (they arrive immediately
    // after Runtime.enable completes). An earlier implementation enabled first
    // and subscribed second, which caused a race: events arrived between the
    // two calls and were lost.
    let mut rx = session
        .subscribe("Runtime.executionContextCreated")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to execution contexts: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    session
        .ensure_domain("Runtime")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to enable Runtime domain: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    // Drain replayed events looking for the frame's default execution context.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(500);
    while let Ok(result) = tokio::time::timeout_at(deadline, rx.recv()).await {
        if let Some(event) = result {
            let ctx = &event.params["context"];
            let aux = &ctx["auxData"];
            if aux["frameId"].as_str() == Some(frame_id)
                && aux["isDefault"].as_bool() == Some(true)
                && let Some(id) = ctx["id"].as_i64()
            {
                return Ok(id);
            }
        }
    }

    // Fallback: create an isolated world for the frame. This shares the
    // frame's DOM but has its own JS global scope, so page-script variables
    // won't be visible. This path is only taken if the Runtime domain was
    // already enabled before this call (making ensure_domain a no-op and
    // skipping the context replay).
    session.ensure_domain("Page").await.map_err(|e| AppError {
        message: format!("Failed to enable Page domain: {e}"),
        code: ExitCode::ProtocolError,
        custom_json: None,
    })?;

    let result = session
        .send_command(
            "Page.createIsolatedWorld",
            Some(serde_json::json!({
                "frameId": frame_id,
                "grantUniversalAccess": true,
            })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("Failed to create isolated world for frame: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    result["executionContextId"]
        .as_i64()
        .ok_or_else(|| AppError {
            message: "Failed to obtain execution context for frame".into(),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })
}

/// Resolve a nested frame path (e.g. `[1, 0]`) by traversing parent→child.
async fn resolve_frame_by_path(
    client: &CdpClient,
    session: &mut ManagedSession,
    frames: &[FrameInfo],
    segments: &[u32],
) -> Result<FrameContext, AppError> {
    let main_frame = frames.first().ok_or_else(|| AppError::frame_not_found(0))?;

    let path_str = segments
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>()
        .join("/");

    let mut current = main_frame;

    for &segment in segments {
        #[allow(clippy::cast_possible_truncation)]
        let child_count = current.child_ids.len() as u32;
        if segment >= child_count {
            return Err(AppError::frame_path_invalid(
                &path_str,
                segment,
                child_count,
            ));
        }
        let child_id = &current.child_ids[segment as usize];
        current = frames
            .iter()
            .find(|f| &f.id == child_id)
            .ok_or_else(AppError::frame_detached)?;
    }

    if current.index == 0 {
        return Ok(FrameContext::MainFrame);
    }

    resolve_frame_by_info(client, session, current).await
}

// =============================================================================
// Auto frame detection
// =============================================================================

/// Maximum number of frames to search in `--frame auto` mode.
const MAX_AUTO_FRAMES: usize = 50;

/// Search all frames for a UID and return the first matching frame context.
///
/// Returns `(FrameContext, frame_index)` so callers can include the frame index
/// in their output.
///
/// `snapshot_hint` is an optional `(frame_index, uid_map)` pair from a previously
/// persisted snapshot state. When provided, the function checks this hint first:
/// if the UID is present in the map, the frame at `frame_index` is resolved
/// immediately without scanning all frames. This is the fast path for the common
/// case where the most recent snapshot captured the target element.
///
/// If the hint is absent or doesn't contain the UID, the function iterates all
/// frames in document order (up to [`MAX_AUTO_FRAMES`]) and, for each frame,
/// calls `Accessibility.getFullAXTree` to build a transient UID map, checking
/// whether the UID appears. The first frame that contains the UID is returned.
///
/// # Errors
///
/// Returns `AppError::element_not_in_any_frame()` if the UID is not found in
/// any frame.
pub async fn resolve_frame_auto<S: ::std::hash::BuildHasher>(
    client: &CdpClient,
    session: &mut ManagedSession,
    uid: &str,
    snapshot_hint: Option<(u32, &std::collections::HashMap<String, i64, S>)>,
) -> Result<(FrameContext, u32), AppError> {
    // Fast path: use the persisted snapshot state hint if the UID is in it.
    if let Some((frame_index, uid_map)) = snapshot_hint
        && uid_map.contains_key(uid)
    {
        let ctx = if frame_index == 0 {
            FrameContext::MainFrame
        } else {
            let frames = list_frames(session).await?;
            let frame = frames
                .iter()
                .find(|f| f.index == frame_index)
                .ok_or_else(|| AppError::frame_not_found(frame_index))?;
            resolve_frame_by_info(client, session, frame).await?
        };
        return Ok((ctx, frame_index));
    }

    // Slow path: iterate frames in document order, taking a quick AX snapshot of each.
    let frames = list_frames(session).await?;
    let limit = frames.len().min(MAX_AUTO_FRAMES);

    for frame in frames.iter().take(limit) {
        // For the main frame, always check via the primary session.
        if frame.index == 0 {
            if uid_in_frame_snapshot(session, None)
                .await
                .is_ok_and(|map| map.contains_key(uid))
            {
                return Ok((FrameContext::MainFrame, 0));
            }
            continue;
        }

        // For child frames, take a quick AX snapshot and check for the UID.
        // We pass the frameId as a parameter so we stay in the same CDP session
        // (same-origin path). This avoids the cost of attaching to OOPIF targets
        // just for the UID presence check; if the UID is found here we resolve
        // the full FrameContext afterwards.
        let frame_id = frame.id.as_str();
        match uid_in_frame_snapshot(session, Some(frame_id)).await {
            Ok(map) if map.contains_key(uid) => {
                let ctx = resolve_frame_by_info(client, session, frame).await?;
                return Ok((ctx, frame.index));
            }
            _ => {}
        }
    }

    Err(AppError::element_not_in_any_frame())
}

/// Take a minimal accessibility snapshot of one frame and return a UID → backendDOMNodeId
/// map. `frame_id` is `None` for the main frame.
///
/// Errors are silently converted to an empty map so the auto-search loop can skip
/// frames that fail (e.g. cross-origin frames whose AX tree is inaccessible).
async fn uid_in_frame_snapshot(
    session: &ManagedSession,
    frame_id: Option<&str>,
) -> Result<std::collections::HashMap<String, i64>, AppError> {
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

    let ax_params = frame_id.map(|id| serde_json::json!({ "frameId": id }));
    let result = session
        .send_command("Accessibility.getFullAXTree", ax_params)
        .await
        .map_err(|e| AppError {
            message: format!("Accessibility.getFullAXTree failed during auto-search: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let nodes = result["nodes"].as_array().ok_or_else(|| AppError {
        message: "Accessibility.getFullAXTree response missing 'nodes' during auto-search".into(),
        code: ExitCode::ProtocolError,
        custom_json: None,
    })?;

    // Build a lightweight UID map without constructing the full tree.
    let mut uid_counter: usize = 0;
    let mut uid_map = std::collections::HashMap::new();

    for node in nodes {
        if node["ignored"].as_bool().unwrap_or(false) {
            continue;
        }
        let role = node["role"]["value"].as_str().unwrap_or_default();
        if INTERACTIVE_ROLES.contains(&role)
            && let Some(backend_id) = node["backendDOMNodeId"].as_i64()
        {
            uid_counter += 1;
            let uid = format!("s{uid_counter}");
            uid_map.insert(uid, backend_id);
        }
    }

    Ok(uid_map)
}

// =============================================================================
// Accessors
// =============================================================================

/// Get the [`ManagedSession`] to use for CDP commands in a frame context.
///
/// Returns the OOPIF session for cross-origin frames, or the page session for
/// main frame / same-origin frames.
#[must_use]
pub fn frame_session<'a>(
    ctx: &'a FrameContext,
    page_session: &'a ManagedSession,
) -> &'a ManagedSession {
    match ctx {
        FrameContext::OutOfProcess { session, .. } => session,
        FrameContext::MainFrame | FrameContext::SameOrigin { .. } => page_session,
    }
}

/// Get a mutable reference to the [`ManagedSession`] for a frame context.
///
/// Returns the OOPIF session for cross-origin frames, or the page session for
/// main frame / same-origin frames.
#[must_use]
pub fn frame_session_mut<'a>(
    ctx: &'a mut FrameContext,
    page_session: &'a mut ManagedSession,
) -> &'a mut ManagedSession {
    match ctx {
        FrameContext::OutOfProcess { session, .. } => session,
        FrameContext::MainFrame | FrameContext::SameOrigin { .. } => page_session,
    }
}

/// Get the frame ID for passing to CDP methods that accept a `frameId` parameter.
///
/// Returns `None` for the main frame (no scoping needed).
#[must_use]
pub fn frame_id(ctx: &FrameContext) -> Option<&str> {
    match ctx {
        FrameContext::MainFrame => None,
        FrameContext::SameOrigin { frame_id, .. } | FrameContext::OutOfProcess { frame_id, .. } => {
            Some(frame_id)
        }
    }
}

/// Get the execution context ID for `Runtime.evaluate` in a same-origin frame.
///
/// Returns `None` for main frame and OOPIF (which use their own session).
#[must_use]
pub fn execution_context_id(ctx: &FrameContext) -> Option<i64> {
    match ctx {
        FrameContext::SameOrigin {
            execution_context_id,
            ..
        } => Some(*execution_context_id),
        _ => None,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_integer_index() {
        let arg = parse_frame_arg("0").unwrap();
        assert!(matches!(arg, FrameArg::Index(0)));

        let arg = parse_frame_arg("5").unwrap();
        assert!(matches!(arg, FrameArg::Index(5)));
    }

    #[test]
    fn parse_path() {
        let arg = parse_frame_arg("1/0").unwrap();
        if let FrameArg::Path(segments) = arg {
            assert_eq!(segments, vec![1, 0]);
        } else {
            panic!("expected Path variant");
        }

        let arg = parse_frame_arg("2/1/0").unwrap();
        if let FrameArg::Path(segments) = arg {
            assert_eq!(segments, vec![2, 1, 0]);
        } else {
            panic!("expected Path variant");
        }
    }

    #[test]
    fn parse_auto() {
        let arg = parse_frame_arg("auto").unwrap();
        assert!(matches!(arg, FrameArg::Auto));
    }

    #[test]
    fn parse_invalid_string() {
        let err = parse_frame_arg("abc").unwrap_err();
        assert!(err.message.contains("Invalid frame value"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn parse_invalid_path() {
        let err = parse_frame_arg("1/abc").unwrap_err();
        assert!(err.message.contains("Invalid frame path"));
    }

    #[test]
    fn parse_empty_string() {
        let err = parse_frame_arg("").unwrap_err();
        assert!(err.message.contains("Invalid frame value"));
    }

    #[test]
    fn frame_context_accessors() {
        // MainFrame
        let ctx = FrameContext::MainFrame;
        assert!(frame_id(&ctx).is_none());
        assert!(execution_context_id(&ctx).is_none());

        // SameOrigin
        let ctx = FrameContext::SameOrigin {
            frame_id: "F1".to_string(),
            execution_context_id: 42,
        };
        assert_eq!(frame_id(&ctx), Some("F1"));
        assert_eq!(execution_context_id(&ctx), Some(42));
    }
}
