use std::collections::HashMap;

use serde::Serialize;

use agentchrome::connection::ManagedSession;
use agentchrome::error::AppError;

use crate::cli::GlobalOpts;
use crate::output;

use super::{get_page_info, setup_session};

// =============================================================================
// Output types
// =============================================================================

/// Full output for `page analyze`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeResult {
    pub scope: String,
    pub url: String,
    pub title: String,
    pub iframes: Vec<IframeInfo>,
    pub frameworks: Vec<String>,
    pub interactive_elements: InteractiveElements,
    pub media: Vec<MediaInfo>,
    pub overlays: Vec<OverlayInfo>,
    pub shadow_dom: ShadowDomInfo,
    pub summary: AnalyzeSummary,
}

/// Iframe metadata.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IframeInfo {
    pub(crate) index: u32,
    pub(crate) url: String,
    pub(crate) name: String,
    pub(crate) visible: bool,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) cross_origin: bool,
}

/// Interactive element counts per frame.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveElements {
    pub main: u32,
    pub frames: HashMap<String, Option<u32>>,
}

/// Media element metadata.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaInfo {
    pub(crate) tag: String,
    pub(crate) src: Option<String>,
    pub(crate) state: Option<String>,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
}

/// Overlay/blocker element metadata.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayInfo {
    pub(crate) selector: String,
    pub(crate) z_index: i64,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) covers_interactive: bool,
}

/// Shadow DOM presence metadata.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShadowDomInfo {
    pub(crate) present: bool,
    pub(crate) host_count: u32,
}

/// Summary with aggregate counts and boolean flags.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct AnalyzeSummary {
    pub iframe_count: u32,
    pub interactive_element_count: u32,
    pub has_overlays: bool,
    pub has_media: bool,
    pub has_shadow_dom: bool,
    pub has_frameworks: bool,
}

// =============================================================================
// Summary builder
// =============================================================================

/// Build a domain-specific summary for the `page analyze` large-response gate.
///
/// Fields:
/// - `iframe_count`: number of child iframes detected
/// - `overlay_count`: number of overlay/blocker elements detected
/// - `framework`: first detected framework string, or `null` when none
/// - `has_shadow_dom`: whether any shadow roots are present
pub fn summary_of_analyze(result: &AnalyzeResult) -> serde_json::Value {
    #[allow(clippy::cast_possible_truncation)]
    let iframe_count = result.iframes.len() as u64;
    #[allow(clippy::cast_possible_truncation)]
    let overlay_count = result.overlays.len() as u64;
    let framework = result
        .frameworks
        .first()
        .map_or(serde_json::Value::Null, |s| {
            serde_json::Value::String(s.clone())
        });
    let has_shadow_dom = result.shadow_dom.present;

    serde_json::json!({
        "iframe_count": iframe_count,
        "overlay_count": overlay_count,
        "framework": framework,
        "has_shadow_dom": has_shadow_dom,
    })
}

// =============================================================================
// Analysis dimension: iframe enumeration
// =============================================================================

pub(crate) async fn enumerate_iframes(
    managed: &mut ManagedSession,
    main_security_origin: &str,
) -> Vec<IframeInfo> {
    let Ok(frames) = agentchrome::frame::list_frames(managed).await else {
        return Vec::new();
    };

    // Skip index 0 (the main frame); child frames start at index 1
    frames
        .into_iter()
        .skip(1)
        .map(|f| {
            let cross_origin = f.security_origin != main_security_origin;
            IframeInfo {
                index: f.index,
                url: f.url,
                name: f.name,
                visible: f.width > 0 && f.height > 0,
                width: f.width,
                height: f.height,
                cross_origin,
            }
        })
        .collect()
}

// =============================================================================
// Analysis dimension: framework detection
// =============================================================================

pub(crate) async fn detect_frameworks(
    effective: &ManagedSession,
    context_id: Option<i64>,
) -> Vec<String> {
    let js = r#"(function() {
        var detected = [];
        try { if (typeof window.__REACT_DEVTOOLS_GLOBAL_HOOK__ !== 'undefined' || document.querySelector('[data-reactroot]') !== null) detected.push('React'); } catch(e) {}
        try { if (document.querySelector('[ng-version]') !== null || typeof window.ng !== 'undefined') detected.push('Angular'); } catch(e) {}
        try { if (typeof window.__VUE__ !== 'undefined' || document.querySelector('[data-v-]') !== null) detected.push('Vue'); } catch(e) {}
        try { if (document.querySelector('[class*="svelte-"]') !== null) detected.push('Svelte'); } catch(e) {}
        try { if (document.getElementById('story_content') !== null) detected.push('Storyline'); } catch(e) {}
        try { if (typeof window.API !== 'undefined' || typeof window.API_1484_11 !== 'undefined') detected.push('SCORM'); } catch(e) {}
        return JSON.stringify(detected);
    })()"#;

    let mut params = serde_json::json!({
        "expression": js,
        "returnByValue": true,
    });
    if let Some(ctx_id) = context_id {
        params["contextId"] = serde_json::json!(ctx_id);
    }

    let Ok(result) = effective
        .send_command("Runtime.evaluate", Some(params))
        .await
    else {
        return Vec::new();
    };

    let value_str = result["result"]["value"].as_str().unwrap_or("[]");
    serde_json::from_str(value_str).unwrap_or_default()
}

// =============================================================================
// Analysis dimension: interactive element counting
// =============================================================================

pub(crate) const INTERACTIVE_SELECTOR: &str = r#"a[href], button, input, select, textarea, [role="button"], [role="link"], [role="checkbox"], [role="radio"], [role="tab"], [tabindex]:not([tabindex="-1"])"#;

pub(crate) async fn count_interactive_elements(
    effective: &ManagedSession,
    context_id: Option<i64>,
) -> u32 {
    let js = format!(r"document.querySelectorAll('{INTERACTIVE_SELECTOR}').length");

    let mut params = serde_json::json!({
        "expression": js,
        "returnByValue": true,
    });
    if let Some(ctx_id) = context_id {
        params["contextId"] = serde_json::json!(ctx_id);
    }

    let Ok(result) = effective
        .send_command("Runtime.evaluate", Some(params))
        .await
    else {
        return 0;
    };

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = result["result"]["value"].as_u64().unwrap_or(0) as u32;
    count
}

// =============================================================================
// Analysis dimension: media element cataloging
// =============================================================================

pub(crate) async fn catalog_media(
    effective: &ManagedSession,
    context_id: Option<i64>,
) -> Vec<MediaInfo> {
    let js = r"(function() {
        var media = [];
        var els = document.querySelectorAll('video, audio, embed');
        for (var i = 0; i < els.length; i++) {
            var el = els[i];
            var tag = el.tagName.toLowerCase();
            var src = el.currentSrc || el.src || null;
            var state = null;
            if (tag === 'video' || tag === 'audio') {
                if (el.ended) state = 'ended';
                else if (el.paused) state = 'paused';
                else state = 'playing';
            }
            var rect = el.getBoundingClientRect();
            media.push({
                tag: tag,
                src: src,
                state: state,
                width: Math.round(rect.width),
                height: Math.round(rect.height)
            });
        }
        return JSON.stringify(media);
    })()";

    let mut params = serde_json::json!({
        "expression": js,
        "returnByValue": true,
    });
    if let Some(ctx_id) = context_id {
        params["contextId"] = serde_json::json!(ctx_id);
    }

    let Ok(result) = effective
        .send_command("Runtime.evaluate", Some(params))
        .await
    else {
        return Vec::new();
    };

    let value_str = result["result"]["value"].as_str().unwrap_or("[]");
    let raw: Vec<serde_json::Value> = serde_json::from_str(value_str).unwrap_or_default();

    raw.into_iter()
        .map(|v| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            MediaInfo {
                tag: v["tag"].as_str().unwrap_or("unknown").to_string(),
                src: v["src"].as_str().map(String::from),
                state: v["state"].as_str().map(String::from),
                width: v["width"].as_u64().map(|n| n as u32),
                height: v["height"].as_u64().map(|n| n as u32),
            }
        })
        .collect()
}

// =============================================================================
// Analysis dimension: overlay detection
// =============================================================================

pub(crate) async fn detect_overlays(
    effective: &ManagedSession,
    context_id: Option<i64>,
) -> Vec<OverlayInfo> {
    let js = format!(
        r"(function() {{
        var vpW = window.innerWidth;
        var vpH = window.innerHeight;
        var vpArea = vpW * vpH;
        var overlays = [];
        var all = document.querySelectorAll('*');
        for (var i = 0; i < all.length; i++) {{
            var el = all[i];
            var style = window.getComputedStyle(el);
            var pos = style.position;
            if (pos !== 'fixed' && pos !== 'absolute') continue;
            var zStr = style.zIndex;
            var z = parseInt(zStr, 10);
            if (isNaN(z) || z <= 0) continue;
            var rect = el.getBoundingClientRect();
            var elArea = rect.width * rect.height;
            if (elArea / vpArea < 0.5) continue;
            var selector = el.tagName.toLowerCase();
            if (el.id) selector += '#' + el.id;
            else if (el.className && typeof el.className === 'string') {{
                var first = el.className.trim().split(/\s+/)[0];
                if (first) selector += '.' + first;
            }}
            var hasInteractive = false;
            var allInteractive = document.querySelectorAll('{INTERACTIVE_SELECTOR}');
            for (var j = 0; j < allInteractive.length; j++) {{
                var ie = allInteractive[j];
                if (el.contains(ie)) continue;
                var ir = ie.getBoundingClientRect();
                if (ir.top < rect.bottom && ir.bottom > rect.top && ir.left < rect.right && ir.right > rect.left) {{
                    hasInteractive = true;
                    break;
                }}
            }}
            overlays.push({{
                selector: selector,
                zIndex: z,
                width: Math.round(rect.width),
                height: Math.round(rect.height),
                coversInteractive: hasInteractive
            }});
        }}
        return JSON.stringify(overlays);
    }})()",
    );

    let mut params = serde_json::json!({
        "expression": js,
        "returnByValue": true,
    });
    if let Some(ctx_id) = context_id {
        params["contextId"] = serde_json::json!(ctx_id);
    }

    let Ok(result) = effective
        .send_command("Runtime.evaluate", Some(params))
        .await
    else {
        return Vec::new();
    };

    let value_str = result["result"]["value"].as_str().unwrap_or("[]");
    let raw: Vec<serde_json::Value> = serde_json::from_str(value_str).unwrap_or_default();

    raw.into_iter()
        .map(|v| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            OverlayInfo {
                selector: v["selector"].as_str().unwrap_or("unknown").to_string(),
                z_index: v["zIndex"].as_i64().unwrap_or(0),
                width: v["width"].as_u64().unwrap_or(0) as u32,
                height: v["height"].as_u64().unwrap_or(0) as u32,
                covers_interactive: v["coversInteractive"].as_bool().unwrap_or(false),
            }
        })
        .collect()
}

// =============================================================================
// Analysis dimension: shadow DOM detection
// =============================================================================

pub(crate) async fn detect_shadow_dom(
    effective: &ManagedSession,
    context_id: Option<i64>,
) -> ShadowDomInfo {
    let js = r"(function() {
        var count = 0;
        var all = document.querySelectorAll('*');
        for (var i = 0; i < all.length; i++) {
            if (all[i].shadowRoot) count++;
        }
        return JSON.stringify({ present: count > 0, hostCount: count });
    })()";

    let mut params = serde_json::json!({
        "expression": js,
        "returnByValue": true,
    });
    if let Some(ctx_id) = context_id {
        params["contextId"] = serde_json::json!(ctx_id);
    }

    let Ok(result) = effective
        .send_command("Runtime.evaluate", Some(params))
        .await
    else {
        return ShadowDomInfo {
            present: false,
            host_count: 0,
        };
    };

    let value_str = result["result"]["value"].as_str().unwrap_or("{}");
    let raw: serde_json::Value = serde_json::from_str(value_str).unwrap_or_default();

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    ShadowDomInfo {
        present: raw["present"].as_bool().unwrap_or(false),
        host_count: raw["hostCount"].as_u64().unwrap_or(0) as u32,
    }
}

// =============================================================================
// Command executor
// =============================================================================

/// Execute `page analyze`.
///
/// # Errors
///
/// Returns `AppError` if the session cannot be established, the frame index
/// is invalid, or a CDP call fails critically.
#[allow(clippy::too_many_lines)]
pub async fn execute_analyze(global: &GlobalOpts, frame: Option<&str>) -> Result<(), AppError> {
    let (client, mut managed) = setup_session(global).await?;

    // Resolve optional frame context
    let mut frame_ctx = if let Some(frame_str) = frame {
        let arg = agentchrome::frame::parse_frame_arg(frame_str)?;
        Some(agentchrome::frame::resolve_frame(&client, &mut managed, &arg).await?)
    } else {
        None
    };

    // Enable required domains
    {
        let eff_mut = if let Some(ref mut ctx) = frame_ctx {
            agentchrome::frame::frame_session_mut(ctx, &mut managed)
        } else {
            &mut managed
        };
        eff_mut.ensure_domain("DOM").await?;
        eff_mut.ensure_domain("Runtime").await?;
        eff_mut.ensure_domain("Page").await?;
    }

    let effective = if let Some(ref ctx) = frame_ctx {
        agentchrome::frame::frame_session(ctx, &managed)
    } else {
        &managed
    };

    // Determine scope label
    let scope = frame.map_or_else(|| "main".to_string(), |f| format!("frame:{f}"));

    // Get page info
    let (url, title) = get_page_info(effective).await.unwrap_or_default();

    // Get main frame security origin for cross-origin detection
    let main_security_origin = {
        let origin_result = managed
            .send_command(
                "Runtime.evaluate",
                Some(serde_json::json!({ "expression": "location.origin" })),
            )
            .await;
        origin_result
            .ok()
            .and_then(|r| r["result"]["value"].as_str().map(String::from))
            .unwrap_or_default()
    };

    // Execution context for frame-scoped JS evaluation
    let context_id = frame_ctx
        .as_ref()
        .and_then(agentchrome::frame::execution_context_id);

    // --- Analysis dimensions (run sequentially for CDP safety) ---

    // 1. Iframe enumeration
    let iframes = enumerate_iframes(&mut managed, &main_security_origin).await;

    // Re-resolve effective session after mutable borrow for list_frames
    let effective = if let Some(ref ctx) = frame_ctx {
        agentchrome::frame::frame_session(ctx, &managed)
    } else {
        &managed
    };

    // 2. Framework detection
    let frameworks = detect_frameworks(effective, context_id).await;

    // 3. Interactive element counting (main frame)
    let main_interactive_count = count_interactive_elements(effective, context_id).await;

    // 4. Interactive elements in child frames
    let mut frame_interactive: HashMap<String, Option<u32>> = HashMap::new();
    for iframe in &iframes {
        if iframe.cross_origin {
            // Cross-origin frames may not be accessible
            frame_interactive.insert(iframe.index.to_string(), None);
        } else {
            // Try to count in same-origin iframes by resolving frame context
            let arg = agentchrome::frame::FrameArg::Index(iframe.index);
            match agentchrome::frame::resolve_frame(&client, &mut managed, &arg).await {
                Ok(ctx) => {
                    let frame_session = agentchrome::frame::frame_session(&ctx, &managed);
                    let ctx_id = agentchrome::frame::execution_context_id(&ctx);
                    let count = count_interactive_elements(frame_session, ctx_id).await;
                    frame_interactive.insert(iframe.index.to_string(), Some(count));
                }
                Err(_) => {
                    frame_interactive.insert(iframe.index.to_string(), None);
                }
            }
        }
    }

    // Re-resolve effective session
    let effective = if let Some(ref ctx) = frame_ctx {
        agentchrome::frame::frame_session(ctx, &managed)
    } else {
        &managed
    };

    let interactive_elements = InteractiveElements {
        main: main_interactive_count,
        frames: frame_interactive,
    };

    // 5. Media element cataloging
    let media = catalog_media(effective, context_id).await;

    // 6. Overlay detection
    let overlays = detect_overlays(effective, context_id).await;

    // 7. Shadow DOM detection
    let shadow_dom = detect_shadow_dom(effective, context_id).await;

    // --- Assemble summary ---
    let total_interactive: u32 = main_interactive_count
        + interactive_elements
            .frames
            .values()
            .filter_map(|v| *v)
            .sum::<u32>();

    #[allow(clippy::cast_possible_truncation)]
    let summary = AnalyzeSummary {
        iframe_count: iframes.len() as u32,
        interactive_element_count: total_interactive,
        has_overlays: !overlays.is_empty(),
        has_media: !media.is_empty(),
        has_shadow_dom: shadow_dom.present,
        has_frameworks: !frameworks.is_empty(),
    };

    let result = AnalyzeResult {
        scope,
        url,
        title,
        iframes,
        frameworks,
        interactive_elements,
        media,
        overlays,
        shadow_dom,
        summary,
    };

    output::emit(&result, &global.output, "page analyze", summary_of_analyze)?;
    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_result_serialization_camel_case() {
        let result = AnalyzeResult {
            scope: "main".to_string(),
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            iframes: vec![],
            frameworks: vec![],
            interactive_elements: InteractiveElements {
                main: 5,
                frames: HashMap::new(),
            },
            media: vec![],
            overlays: vec![],
            shadow_dom: ShadowDomInfo {
                present: false,
                host_count: 0,
            },
            summary: AnalyzeSummary {
                iframe_count: 0,
                interactive_element_count: 5,
                has_overlays: false,
                has_media: false,
                has_shadow_dom: false,
                has_frameworks: false,
            },
        };

        let json: serde_json::Value = serde_json::to_value(&result).unwrap();

        // Verify camelCase field naming
        assert_eq!(json["scope"], "main");
        assert_eq!(json["url"], "https://example.com");
        assert_eq!(json["interactiveElements"]["main"], 5);
        assert_eq!(json["shadowDom"]["present"], false);
        assert_eq!(json["shadowDom"]["hostCount"], 0);
        assert_eq!(json["summary"]["iframeCount"], 0);
        assert_eq!(json["summary"]["interactiveElementCount"], 5);
        assert_eq!(json["summary"]["hasOverlays"], false);
        assert_eq!(json["summary"]["hasMedia"], false);
        assert_eq!(json["summary"]["hasShadowDom"], false);
        assert_eq!(json["summary"]["hasFrameworks"], false);

        // No snake_case keys
        assert!(json.get("interactive_elements").is_none());
        assert!(json.get("shadow_dom").is_none());
        assert!(json.get("iframe_count").is_none());
    }

    #[test]
    fn analyze_result_simple_page() {
        let result = AnalyzeResult {
            scope: "main".to_string(),
            url: "about:blank".to_string(),
            title: String::new(),
            iframes: vec![],
            frameworks: vec![],
            interactive_elements: InteractiveElements {
                main: 0,
                frames: HashMap::new(),
            },
            media: vec![],
            overlays: vec![],
            shadow_dom: ShadowDomInfo {
                present: false,
                host_count: 0,
            },
            summary: AnalyzeSummary {
                iframe_count: 0,
                interactive_element_count: 0,
                has_overlays: false,
                has_media: false,
                has_shadow_dom: false,
                has_frameworks: false,
            },
        };

        let json: serde_json::Value = serde_json::to_value(&result).unwrap();

        // AC3: Simple page has empty arrays and zero counts
        assert!(json["iframes"].as_array().unwrap().is_empty());
        assert!(json["frameworks"].as_array().unwrap().is_empty());
        assert!(json["media"].as_array().unwrap().is_empty());
        assert!(json["overlays"].as_array().unwrap().is_empty());
        assert_eq!(json["shadowDom"]["present"], false);
        assert_eq!(json["summary"]["iframeCount"], 0);
        assert_eq!(json["summary"]["hasOverlays"], false);
    }

    #[test]
    fn iframe_info_serialization() {
        let info = IframeInfo {
            index: 1,
            url: "https://child.example.com".to_string(),
            name: "child".to_string(),
            visible: true,
            width: 800,
            height: 600,
            cross_origin: true,
        };

        let json: serde_json::Value = serde_json::to_value(&info).unwrap();
        assert_eq!(json["index"], 1);
        assert_eq!(json["crossOrigin"], true);
        assert!(json.get("cross_origin").is_none());
    }

    #[test]
    fn media_info_null_state() {
        let info = MediaInfo {
            tag: "embed".to_string(),
            src: Some("content.swf".to_string()),
            state: None,
            width: Some(640),
            height: Some(480),
        };

        let json: serde_json::Value = serde_json::to_value(&info).unwrap();
        assert_eq!(json["tag"], "embed");
        assert!(json["state"].is_null()); // AC7: null, not omitted
        assert!(json.get("state").is_some());
    }

    #[test]
    fn overlay_info_serialization() {
        let info = OverlayInfo {
            selector: "div#blocker".to_string(),
            z_index: 9999,
            width: 1280,
            height: 720,
            covers_interactive: true,
        };

        let json: serde_json::Value = serde_json::to_value(&info).unwrap();
        assert_eq!(json["selector"], "div#blocker");
        assert_eq!(json["zIndex"], 9999);
        assert_eq!(json["coversInteractive"], true);
        assert!(json.get("z_index").is_none());
        assert!(json.get("covers_interactive").is_none());
    }

    #[test]
    fn analyze_result_with_all_dimensions() {
        let result = AnalyzeResult {
            scope: "main".to_string(),
            url: "https://example.com".to_string(),
            title: "Test Page".to_string(),
            iframes: vec![IframeInfo {
                index: 1,
                url: "https://child.example.com".to_string(),
                name: "child".to_string(),
                visible: true,
                width: 800,
                height: 600,
                cross_origin: false,
            }],
            frameworks: vec!["React".to_string()],
            interactive_elements: InteractiveElements {
                main: 15,
                frames: {
                    let mut m = HashMap::new();
                    m.insert("1".to_string(), Some(8));
                    m
                },
            },
            media: vec![MediaInfo {
                tag: "video".to_string(),
                src: Some("video.mp4".to_string()),
                state: Some("paused".to_string()),
                width: Some(640),
                height: Some(480),
            }],
            overlays: vec![OverlayInfo {
                selector: "div#blocker".to_string(),
                z_index: 9999,
                width: 1280,
                height: 720,
                covers_interactive: true,
            }],
            shadow_dom: ShadowDomInfo {
                present: true,
                host_count: 3,
            },
            summary: AnalyzeSummary {
                iframe_count: 1,
                interactive_element_count: 23,
                has_overlays: true,
                has_media: true,
                has_shadow_dom: true,
                has_frameworks: true,
            },
        };

        let json: serde_json::Value = serde_json::to_value(&result).unwrap();

        // AC1: All dimensions present
        assert_eq!(json["iframes"].as_array().unwrap().len(), 1);
        assert_eq!(json["frameworks"][0], "React");
        assert_eq!(json["interactiveElements"]["main"], 15);
        assert_eq!(json["interactiveElements"]["frames"]["1"], 8);
        assert_eq!(json["media"][0]["tag"], "video");
        assert_eq!(json["media"][0]["state"], "paused");
        assert_eq!(json["overlays"][0]["zIndex"], 9999);
        assert_eq!(json["shadowDom"]["present"], true);
        assert_eq!(json["shadowDom"]["hostCount"], 3);
        assert_eq!(json["summary"]["iframeCount"], 1);
        assert_eq!(json["summary"]["interactiveElementCount"], 23);
        assert_eq!(json["summary"]["hasOverlays"], true);
    }

    #[test]
    fn cross_origin_iframe_null_interactive_count() {
        let interactive = InteractiveElements {
            main: 10,
            frames: {
                let mut m = HashMap::new();
                m.insert("1".to_string(), None); // Cross-origin: null
                m.insert("2".to_string(), Some(5)); // Same-origin: counted
                m
            },
        };

        let json: serde_json::Value = serde_json::to_value(&interactive).unwrap();
        assert!(json["frames"]["1"].is_null()); // AC5, AC7
        assert_eq!(json["frames"]["2"], 5);
    }

    // =========================================================================
    // summary_of_analyze
    // =========================================================================

    #[allow(clippy::cast_possible_truncation, clippy::bool_to_int_with_if)]
    fn make_result(
        iframes: usize,
        overlays: usize,
        frameworks: Vec<String>,
        shadow_present: bool,
    ) -> AnalyzeResult {
        AnalyzeResult {
            scope: "main".to_string(),
            url: "https://example.com".to_string(),
            title: "Test".to_string(),
            iframes: (0..iframes)
                .map(|i| IframeInfo {
                    index: i as u32,
                    url: format!("https://child{i}.example.com"),
                    name: format!("frame{i}"),
                    visible: true,
                    width: 100,
                    height: 100,
                    cross_origin: false,
                })
                .collect(),
            frameworks,
            interactive_elements: InteractiveElements {
                main: 0,
                frames: HashMap::new(),
            },
            media: vec![],
            overlays: (0..overlays)
                .map(|i| OverlayInfo {
                    selector: format!("div#{i}"),
                    z_index: 999,
                    width: 1280,
                    height: 720,
                    covers_interactive: false,
                })
                .collect(),
            shadow_dom: ShadowDomInfo {
                present: shadow_present,
                host_count: if shadow_present { 1 } else { 0 },
            },
            summary: AnalyzeSummary {
                iframe_count: iframes as u32,
                interactive_element_count: 0,
                has_overlays: overlays > 0,
                has_media: false,
                has_shadow_dom: shadow_present,
                has_frameworks: false,
            },
        }
    }

    #[test]
    fn summary_of_analyze_no_iframes_no_overlays_no_framework() {
        let result = make_result(0, 0, vec![], false);
        let summary = summary_of_analyze(&result);
        assert_eq!(summary["iframe_count"], 0);
        assert_eq!(summary["overlay_count"], 0);
        assert!(summary["framework"].is_null());
        assert_eq!(summary["has_shadow_dom"], false);
    }

    #[test]
    fn summary_of_analyze_with_iframes_and_overlays() {
        let result = make_result(3, 2, vec![], false);
        let summary = summary_of_analyze(&result);
        assert_eq!(summary["iframe_count"], 3);
        assert_eq!(summary["overlay_count"], 2);
        assert!(summary["framework"].is_null());
    }

    #[test]
    fn summary_of_analyze_framework_is_first_detected() {
        let result = make_result(
            0,
            0,
            vec!["React".to_string(), "Angular".to_string()],
            false,
        );
        let summary = summary_of_analyze(&result);
        assert_eq!(summary["framework"], "React");
    }

    #[test]
    fn summary_of_analyze_framework_null_when_empty() {
        // Unmeasurable field must be null, not omitted
        let result = make_result(0, 0, vec![], false);
        let summary = summary_of_analyze(&result);
        assert!(summary.as_object().unwrap().contains_key("framework"));
        assert!(summary["framework"].is_null());
    }

    #[test]
    fn summary_of_analyze_has_shadow_dom() {
        let result = make_result(0, 0, vec![], true);
        let summary = summary_of_analyze(&result);
        assert_eq!(summary["has_shadow_dom"], true);
    }
}
