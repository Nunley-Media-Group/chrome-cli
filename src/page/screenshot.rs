use serde::Serialize;

use agentchrome::connection::ManagedSession;
use agentchrome::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, PageScreenshotArgs, ScreenshotFormat};

use super::{get_viewport_dimensions, print_output, setup_session};

// =============================================================================
// Output types
// =============================================================================

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
// Clip region
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

// =============================================================================
// Clip resolution helpers
// =============================================================================

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
    let state = crate::snapshot::read_snapshot_state()
        .map_err(|e| AppError::screenshot_failed(&format!("Failed to read snapshot state: {e}")))?
        .ok_or_else(|| AppError {
            message: "No snapshot state found. Run 'agentchrome page snapshot' first.".to_string(),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

    let backend_node_id = state
        .uid_map
        .get(uid)
        .ok_or_else(|| AppError::uid_not_found(uid))?;

    // Pass backendNodeId directly to DOM.getBoxModel instead of resolving via
    // describeNode first — the intermediate nodeId was not anchored in the document
    // tree, which caused "Could not find node with given id" errors.
    let box_result = managed
        .send_command(
            "DOM.getBoxModel",
            Some(serde_json::json!({ "backendNodeId": backend_node_id })),
        )
        .await
        .map_err(|e| {
            AppError::screenshot_failed(&format!("Failed to get element bounding box: {e}"))
        })?;

    extract_clip_from_box_model(&box_result)
        .ok_or_else(|| AppError::screenshot_failed("Element has no visible bounding box"))
}

// =============================================================================
// Page/viewport dimension helpers
// =============================================================================

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

// =============================================================================
// Scroll-container helpers
// =============================================================================

/// Get the scroll dimensions of a specific element via `Runtime.evaluate`.
async fn get_container_scroll_dimensions(
    managed: &ManagedSession,
    selector: &str,
) -> Result<(f64, f64), AppError> {
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({sel});
            if (!el) return JSON.stringify({{ error: "not_found" }});
            return JSON.stringify({{ width: el.scrollWidth, height: el.scrollHeight }});
        }})()"#,
        sel =
            serde_json::to_string(selector).expect("serde_json::to_string is infallible for &str")
    );

    let result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": js,
                "returnByValue": true,
            })),
        )
        .await
        .map_err(|e| {
            AppError::screenshot_failed(&format!("Failed to get container scroll dimensions: {e}"))
        })?;

    let value_str = result["result"]["value"]
        .as_str()
        .ok_or_else(|| AppError::screenshot_failed("Failed to read container dimensions"))?;
    let dims: serde_json::Value = serde_json::from_str(value_str).map_err(|e| {
        AppError::screenshot_failed(&format!("Failed to parse container dimensions: {e}"))
    })?;

    if dims.get("error").is_some() {
        return Err(AppError::element_not_found(selector));
    }

    let width = dims["width"].as_f64().unwrap_or(1280.0);
    let height = dims["height"].as_f64().unwrap_or(720.0);

    Ok((width, height))
}

/// Override styles on the target element and its ancestors to make overflow visible.
/// Returns a saved-styles token (JSON string) for restoration.
async fn override_container_styles(
    managed: &ManagedSession,
    selector: &str,
) -> Result<String, AppError> {
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({sel});
            if (!el) return JSON.stringify({{ error: "not_found" }});
            const saved = [];
            let current = el;
            while (current && current !== document.documentElement.parentNode) {{
                saved.push({{ tag: current.tagName, idx: saved.length, css: current.style.cssText }});
                current.style.overflow = "visible";
                current.style.height = "auto";
                current.style.maxHeight = "none";
                current = current.parentElement;
            }}
            return JSON.stringify({{ saved: saved, selector: {sel} }});
        }})()"#,
        sel =
            serde_json::to_string(selector).expect("serde_json::to_string is infallible for &str")
    );

    let result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": js,
                "returnByValue": true,
            })),
        )
        .await
        .map_err(|e| {
            AppError::screenshot_failed(&format!("Failed to override container styles: {e}"))
        })?;

    let value_str = result["result"]["value"]
        .as_str()
        .ok_or_else(|| AppError::screenshot_failed("Failed to read style override result"))?;

    let parsed: serde_json::Value = serde_json::from_str(value_str).map_err(|e| {
        AppError::screenshot_failed(&format!("Failed to parse style override result: {e}"))
    })?;

    if parsed.get("error").is_some() {
        return Err(AppError::element_not_found(selector));
    }

    Ok(value_str.to_string())
}

/// Restore original styles on the target element and its ancestors.
async fn restore_container_styles(
    managed: &ManagedSession,
    saved_token: &str,
) -> Result<(), AppError> {
    let js = format!(
        r"(() => {{
            const data = {saved_token};
            const el = document.querySelector(data.selector);
            if (!el) return;
            let current = el;
            for (const entry of data.saved) {{
                if (!current) break;
                current.style.cssText = entry.css;
                current = current.parentElement;
            }}
        }})()"
    );

    managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": js,
                "returnByValue": true,
            })),
        )
        .await
        .map_err(|e| {
            AppError::screenshot_failed(&format!("Failed to restore container styles: {e}"))
        })?;

    Ok(())
}

// =============================================================================
// Format helpers
// =============================================================================

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

// =============================================================================
// Validation helpers
// =============================================================================

/// Validate `--scroll-container` flag combinations.
fn validate_scroll_container(args: &PageScreenshotArgs) -> Result<(), AppError> {
    if let Some(ref _sc) = args.scroll_container {
        if !args.full_page {
            return Err(AppError::screenshot_failed(
                "--scroll-container requires --full-page",
            ));
        }
        if args.selector.is_some() || args.uid.is_some() || args.clip.is_some() {
            return Err(AppError::screenshot_failed(
                "Cannot combine --scroll-container with --selector, --uid, or --clip",
            ));
        }
    }
    Ok(())
}

// =============================================================================
// Command executor
// =============================================================================

#[allow(clippy::too_many_lines)]
pub async fn execute_screenshot(
    global: &GlobalOpts,
    args: &PageScreenshotArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    // Validate --scroll-container flag combinations (checked first for specific error messages)
    validate_scroll_container(args)?;

    // Validate mutual exclusion: --full-page vs --selector/--uid
    if args.full_page && (args.selector.is_some() || args.uid.is_some()) {
        return Err(AppError::screenshot_failed(
            "Cannot combine --full-page with --selector or --uid",
        ));
    }

    let (client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Resolve optional frame context
    let mut frame_ctx = if let Some(frame_str) = frame {
        let arg = agentchrome::frame::parse_frame_arg(frame_str)?;
        Some(agentchrome::frame::resolve_frame(&client, &mut managed, &arg).await?)
    } else {
        None
    };

    // Enable required domains (needs &mut)
    {
        let eff_mut = if let Some(ref mut ctx) = frame_ctx {
            agentchrome::frame::frame_session_mut(ctx, &mut managed)
        } else {
            &mut managed
        };
        eff_mut.ensure_domain("Page").await?;
        eff_mut.ensure_domain("Runtime").await?;
        if args.selector.is_some() || args.scroll_container.is_some() {
            eff_mut.ensure_domain("DOM").await?;
        }
    }

    let format_str = screenshot_format_str(args.format);

    // Track whether we need to restore container styles after capture
    let mut saved_styles_token: Option<String> = None;

    // Determine capture strategy
    let (clip, capture_beyond_viewport, dimensions) = {
        let effective = if let Some(ref ctx) = frame_ctx {
            agentchrome::frame::frame_session(ctx, &managed)
        } else {
            &managed
        };

        if let Some(ref selector) = args.selector {
            let clip = resolve_selector_clip(effective, selector).await?;
            let dims = clip_dimensions(&clip);
            (Some(clip), false, dims)
        } else if let Some(ref uid) = args.uid {
            let clip = resolve_uid_clip(&mut managed, uid).await?;
            let dims = clip_dimensions(&clip);
            (Some(clip), false, dims)
        } else if let (true, Some(sc_selector)) = (args.full_page, &args.scroll_container) {
            // Get container scroll dimensions
            let (cont_w, cont_h) = get_container_scroll_dimensions(effective, sc_selector).await?;
            // Override styles to make overflow content visible
            let token = override_container_styles(effective, sc_selector).await?;
            saved_styles_token = Some(token);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let dims = (cont_w as u32, cont_h as u32);
            set_viewport_size(effective, dims.0, dims.1).await?;
            (None, true, dims)
        } else if args.full_page {
            let (page_w, page_h) = get_page_dimensions(effective).await?;
            // Auto-detect: warn if full-page dimensions match viewport
            let (_vp_w, vp_h) = get_viewport_dimensions(effective).await?;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let page_h_u32 = page_h as u32;
            if page_h_u32 <= vp_h {
                eprintln!(
                    "warning: full-page dimensions match viewport ({page_h_u32}px). \
                     Content may be inside a scrollable container. \
                     Use --scroll-container <selector> to capture it."
                );
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let dims = (page_w as u32, page_h as u32);
            set_viewport_size(effective, dims.0, dims.1).await?;
            (None, true, dims)
        } else if let Some(ref clip_str) = args.clip {
            let clip = parse_clip(clip_str)?;
            let dims = clip_dimensions(&clip);
            (Some(clip), false, dims)
        } else {
            let dims = get_viewport_dimensions(effective).await?;
            (None, false, dims)
        }
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
    let effective = if let Some(ref ctx) = frame_ctx {
        agentchrome::frame::frame_session(ctx, &managed)
    } else {
        &managed
    };
    let result = effective
        .send_command("Page.captureScreenshot", Some(params))
        .await
        .map_err(|e| AppError::screenshot_failed(&e.to_string()));

    // Restore container styles if we overrode them (before checking the capture result)
    if let Some(ref token) = saved_styles_token {
        let _ = restore_container_styles(effective, token).await;
    }

    // Restore viewport if full-page
    if args.full_page {
        clear_viewport_override(effective).await?;
    }

    // Now unwrap the capture result
    let result = result?;

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

    // =========================================================================
    // validate_scroll_container tests
    // =========================================================================

    #[test]
    fn validate_scroll_container_requires_full_page() {
        let args = PageScreenshotArgs {
            full_page: false,
            selector: None,
            uid: None,
            scroll_container: Some(".main-content".to_string()),
            format: ScreenshotFormat::Png,
            quality: None,
            file: None,
            clip: None,
        };
        let err = validate_scroll_container(&args).unwrap_err();
        assert!(
            err.message
                .contains("--scroll-container requires --full-page")
        );
    }

    #[test]
    fn validate_scroll_container_conflicts_with_selector() {
        let args = PageScreenshotArgs {
            full_page: true,
            selector: Some("#logo".to_string()),
            uid: None,
            scroll_container: Some(".main-content".to_string()),
            format: ScreenshotFormat::Png,
            quality: None,
            file: None,
            clip: None,
        };
        let err = validate_scroll_container(&args).unwrap_err();
        assert!(err.message.contains("Cannot combine --scroll-container"));
    }

    #[test]
    fn validate_scroll_container_conflicts_with_uid() {
        let args = PageScreenshotArgs {
            full_page: true,
            selector: None,
            uid: Some("s1".to_string()),
            scroll_container: Some(".main-content".to_string()),
            format: ScreenshotFormat::Png,
            quality: None,
            file: None,
            clip: None,
        };
        let err = validate_scroll_container(&args).unwrap_err();
        assert!(err.message.contains("Cannot combine --scroll-container"));
    }

    #[test]
    fn validate_scroll_container_conflicts_with_clip() {
        let args = PageScreenshotArgs {
            full_page: true,
            selector: None,
            uid: None,
            scroll_container: Some(".main-content".to_string()),
            format: ScreenshotFormat::Png,
            quality: None,
            file: None,
            clip: Some("0,0,100,100".to_string()),
        };
        let err = validate_scroll_container(&args).unwrap_err();
        assert!(err.message.contains("Cannot combine --scroll-container"));
    }

    #[test]
    fn validate_scroll_container_valid_with_full_page() {
        let args = PageScreenshotArgs {
            full_page: true,
            selector: None,
            uid: None,
            scroll_container: Some(".main-content".to_string()),
            format: ScreenshotFormat::Png,
            quality: None,
            file: None,
            clip: None,
        };
        assert!(validate_scroll_container(&args).is_ok());
    }

    #[test]
    fn validate_scroll_container_none_is_ok() {
        let args = PageScreenshotArgs {
            full_page: false,
            selector: None,
            uid: None,
            scroll_container: None,
            format: ScreenshotFormat::Png,
            quality: None,
            file: None,
            clip: None,
        };
        assert!(validate_scroll_container(&args).is_ok());
    }
}
