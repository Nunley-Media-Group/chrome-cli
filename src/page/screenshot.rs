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
// Command executor
// =============================================================================

#[allow(clippy::too_many_lines)]
pub async fn execute_screenshot(
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
