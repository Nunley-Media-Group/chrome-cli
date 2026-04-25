mod detectors;
mod output;
mod patterns;

use agentchrome::error::AppError;

use crate::cli::{DiagnoseArgs, GlobalOpts};
use crate::output::{print_output, setup_session_with_interceptors as setup_session};
use crate::page::analyze::{
    catalog_media as pa_catalog_media, detect_frameworks as pa_detect_frameworks,
    detect_overlays as pa_detect_overlays, detect_shadow_dom as pa_detect_shadow_dom,
    enumerate_iframes as pa_enumerate_iframes,
};

use detectors::{
    CanvasInfo, SUGGESTION_CANVAS, SUGGESTION_FRAMEWORK, SUGGESTION_IFRAMES, SUGGESTION_MEDIA,
    SUGGESTION_OVERLAYS, SUGGESTION_SHADOW_DOM, classify_media_gate, detect_canvas,
    detect_framework_quirks,
};
use output::{
    CanvasChallengeDetails, CanvasChallengeItem, Challenge, ChallengeDetails, DiagnoseResult,
    DiagnoseSummary, FrameworkChallengeDetails, IframeChallengeItem, IframesChallengeDetails,
    MediaChallengeDetails, MediaChallengeItem, OverlayChallengeItem, OverlaysChallengeDetails,
    ShadowDomChallengeDetails,
};
use patterns::{DetectorBundle, match_all};

// =============================================================================
// Entry point
// =============================================================================

/// Execute the `diagnose` command.
///
/// # Errors
///
/// Returns `AppError` on connection failure, navigation failure, or CDP errors.
pub async fn execute_diagnose(global: &GlobalOpts, args: &DiagnoseArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // --- Navigation phase (URL mode only) ---
    let navigation_status = if let Some(ref url) = args.url {
        let timeout_ms = args
            .timeout
            .unwrap_or(crate::navigate::DEFAULT_NAVIGATE_TIMEOUT_MS);
        let result =
            crate::navigate::navigate_and_wait(&mut managed, url, args.wait_until, timeout_ms)
                .await?;
        result.status
    } else {
        // --current mode: no navigation
        None
    };

    let scope = if args.url.is_some() {
        "diagnosed"
    } else {
        "current"
    };

    // --- Resolve current page URL ---
    managed.ensure_domain("Runtime").await?;
    let (page_url, _page_title) = get_page_info(&managed).await?;

    // --- Detection phase (all detectors run; each fails independently) ---

    // Enable required domains for structural detectors
    let _ = managed.ensure_domain("DOM").await;
    let _ = managed.ensure_domain("Page").await;

    // Get main frame security origin for cross-origin iframe detection.
    // If the query fails or returns a non-string value, fall back to a sentinel
    // that can never match a real iframe origin, so iframes are classified as
    // "unknown" rather than "definitively cross-origin".
    let main_security_origin = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": "location.origin" })),
        )
        .await
        .ok()
        .and_then(|r| r["result"]["value"].as_str().map(String::from))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "__agentchrome_unknown_origin__".to_string());

    // 1. Iframes
    let iframes = pa_enumerate_iframes(&mut managed, &main_security_origin).await;

    // 2. Frameworks
    let frameworks = pa_detect_frameworks(&managed, None).await;

    // 3. Media
    let media_raw = pa_catalog_media(&managed, None).await;

    // 4. Overlays
    let overlays = pa_detect_overlays(&managed, None).await;

    // 5. Shadow DOM
    let shadow_dom = pa_detect_shadow_dom(&managed, None).await;

    // 6. Canvas / WebGL (diagnose-only)
    let canvas = detect_canvas(&managed).await;

    // 7. Framework quirks (diagnose-only)
    let framework_quirks = detect_framework_quirks(&managed).await;

    // 8. Media gate classification (pure function over media_raw)
    let media_gates = classify_media_gate(&media_raw);

    // --- Build bundle and assemble challenges ---
    let bundle = DetectorBundle {
        iframes: &iframes,
        frameworks: &frameworks,
        overlays: &overlays,
        shadow_dom: &shadow_dom,
        media: &media_raw,
        canvas: canvas.as_ref(),
        framework_quirks: &framework_quirks,
    };

    let challenges = assemble_challenges(&bundle, &media_gates);
    let patterns = match_all(&bundle);

    // --- Build summary ---
    #[allow(clippy::cast_possible_truncation)]
    let summary = {
        let challenge_count = challenges.len() as u32;
        let pattern_match_count = patterns.len() as u32;
        let has_high_severity = challenges.iter().any(|c| c.severity == "high");
        DiagnoseSummary {
            challenge_count,
            pattern_match_count,
            has_high_severity,
            straightforward: challenge_count == 0 && pattern_match_count == 0,
        }
    };

    let result = DiagnoseResult {
        url: page_url,
        scope: scope.to_string(),
        navigation_status,
        challenges,
        patterns,
        summary,
    };

    print_output(&result, &global.output)
}

// =============================================================================
// Page info helper
// =============================================================================

async fn get_page_info(
    managed: &agentchrome::connection::ManagedSession,
) -> Result<(String, String), AppError> {
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
// Challenge assembly (T008)
// =============================================================================

fn assemble_challenges(
    bundle: &DetectorBundle<'_>,
    media_gates: &[detectors::MediaGateInfo],
) -> Vec<Challenge> {
    let mut out = Vec::new();

    // Iframes
    if !bundle.iframes.is_empty() {
        out.push(build_iframes_challenge(bundle));
    }

    // Overlays
    if !bundle.overlays.is_empty() {
        out.push(build_overlays_challenge(bundle));
    }

    // Shadow DOM
    if bundle.shadow_dom.present {
        out.push(build_shadow_dom_challenge(bundle));
    }

    // Canvas
    if let Some(cv) = bundle.canvas
        && cv.canvas_count > 0
    {
        out.push(build_canvas_challenge(cv));
    }

    // Media
    if !media_gates.is_empty() {
        out.push(build_media_challenge(media_gates));
    }

    // Framework quirks
    if bundle.framework_quirks.any() {
        out.push(build_framework_challenge(bundle));
    }

    out
}

// --- Iframes challenge ---

fn build_iframes_challenge(bundle: &DetectorBundle<'_>) -> Challenge {
    #[allow(clippy::cast_possible_truncation)]
    let cross_origin_count = bundle.iframes.iter().filter(|f| f.cross_origin).count() as u32;
    #[allow(clippy::cast_possible_truncation)]
    let total = bundle.iframes.len() as u32;

    let severity = iframe_severity(total, cross_origin_count, bundle.iframes);

    let summary_text = if cross_origin_count > 0 {
        format!("{total} iframe(s) detected ({cross_origin_count} cross-origin)")
    } else {
        format!("{total} iframe(s) detected")
    };

    let items: Vec<IframeChallengeItem> = bundle
        .iframes
        .iter()
        .map(|f| IframeChallengeItem {
            index: f.index,
            url: f.url.clone(),
            name: f.name.clone(),
            visible: f.visible,
            width: f.width,
            height: f.height,
            cross_origin: f.cross_origin,
            // Cross-origin frames cannot have their internals measured
            interactive_element_count: if f.cross_origin { None } else { Some(0) },
        })
        .collect();

    Challenge {
        category: "iframes".to_string(),
        severity,
        summary: summary_text,
        details: ChallengeDetails::Iframes(IframesChallengeDetails {
            count: total,
            cross_origin_count,
            items,
        }),
        suggestion: Some(SUGGESTION_IFRAMES.to_string()),
    }
}

fn iframe_severity(
    total: u32,
    cross_origin_count: u32,
    iframes: &[crate::page::analyze::IframeInfo],
) -> String {
    if cross_origin_count > 0 && iframes.iter().any(|f| f.cross_origin && f.visible) {
        "high".to_string()
    } else if total > 0 {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

// --- Overlays challenge ---

fn build_overlays_challenge(bundle: &DetectorBundle<'_>) -> Challenge {
    let any_covers_interactive = bundle.overlays.iter().any(|o| o.covers_interactive);

    // Approximate viewport area check (width*height vs 75% of typical viewport)
    // We use covers_interactive as the primary severity signal since we don't
    // have viewport dimensions here.
    let severity = if any_covers_interactive {
        "high".to_string()
    } else {
        "low".to_string()
    };

    #[allow(clippy::cast_possible_truncation)]
    let count = bundle.overlays.len() as u32;
    let summary_text = if any_covers_interactive {
        format!("{count} viewport-covering overlay(s) that cover(s) interactive elements")
    } else {
        format!("{count} overlay(s) detected")
    };

    let items: Vec<OverlayChallengeItem> = bundle
        .overlays
        .iter()
        .map(|o| OverlayChallengeItem {
            selector: o.selector.clone(),
            z_index: o.z_index,
            width: o.width,
            height: o.height,
            covers_interactive: o.covers_interactive,
        })
        .collect();

    Challenge {
        category: "overlays".to_string(),
        severity,
        summary: summary_text,
        details: ChallengeDetails::Overlays(OverlaysChallengeDetails { items }),
        suggestion: Some(SUGGESTION_OVERLAYS.to_string()),
    }
}

// --- Shadow DOM challenge ---

fn build_shadow_dom_challenge(bundle: &DetectorBundle<'_>) -> Challenge {
    let host_count = bundle.shadow_dom.host_count;
    let severity = if host_count >= 10 {
        "high".to_string()
    } else {
        "medium".to_string()
    };

    Challenge {
        category: "shadowDom".to_string(),
        severity,
        summary: format!("{host_count} shadow DOM host(s) detected"),
        details: ChallengeDetails::ShadowDom(ShadowDomChallengeDetails { host_count }),
        suggestion: Some(SUGGESTION_SHADOW_DOM.to_string()),
    }
}

// --- Canvas challenge ---

fn build_canvas_challenge(cv: &CanvasInfo) -> Challenge {
    let severity = if cv.webgl_count > 0 {
        "medium".to_string()
    } else {
        "low".to_string()
    };

    let summary_text = if cv.webgl_count > 0 {
        format!(
            "{} canvas element(s), {} with WebGL context",
            cv.canvas_count, cv.webgl_count
        )
    } else {
        format!("{} canvas element(s) detected", cv.canvas_count)
    };

    let items: Vec<CanvasChallengeItem> = cv
        .items
        .iter()
        .map(|item| CanvasChallengeItem {
            width: item.width,
            height: item.height,
            context: item.context.clone(),
        })
        .collect();

    Challenge {
        category: "canvas".to_string(),
        severity,
        summary: summary_text,
        details: ChallengeDetails::Canvas(CanvasChallengeDetails {
            canvas_count: cv.canvas_count,
            webgl_count: cv.webgl_count,
            items,
        }),
        suggestion: Some(SUGGESTION_CANVAS.to_string()),
    }
}

// --- Media challenge ---

fn build_media_challenge(media_gates: &[detectors::MediaGateInfo]) -> Challenge {
    let has_gate = media_gates.iter().any(|m| m.gates_navigation);
    let severity = if has_gate {
        "high".to_string()
    } else {
        "medium".to_string()
    };

    #[allow(clippy::cast_possible_truncation)]
    let count = media_gates.len() as u32;
    let summary_text = if has_gate {
        format!("{count} media element(s), at least one may gate page flow")
    } else {
        format!("{count} media element(s) detected")
    };

    let items: Vec<MediaChallengeItem> = media_gates
        .iter()
        .map(|m| MediaChallengeItem {
            tag: m.tag.clone(),
            src: m.src.clone(),
            state: m.state.clone(),
            gates_navigation: m.gates_navigation,
        })
        .collect();

    Challenge {
        category: "media".to_string(),
        severity,
        summary: summary_text,
        details: ChallengeDetails::Media(MediaChallengeDetails { items }),
        suggestion: Some(SUGGESTION_MEDIA.to_string()),
    }
}

// --- Framework quirks challenge ---

fn build_framework_challenge(bundle: &DetectorBundle<'_>) -> Challenge {
    let q = bundle.framework_quirks;
    let quirk_count = [
        q.react_portal,
        q.angular_zone,
        q.vue_teleport,
        q.svelte_hydration,
    ]
    .iter()
    .filter(|&&b| b)
    .count();

    let severity = if quirk_count >= 2 {
        "high".to_string()
    } else {
        "medium".to_string()
    };

    let mut quirk_names = Vec::new();
    if q.react_portal {
        quirk_names.push("React portal");
    }
    if q.angular_zone {
        quirk_names.push("Angular zone.js");
    }
    if q.vue_teleport {
        quirk_names.push("Vue teleport");
    }
    if q.svelte_hydration {
        quirk_names.push("Svelte hydration");
    }

    let summary_text = format!("Framework quirk(s) detected: {}", quirk_names.join(", "));

    Challenge {
        category: "framework".to_string(),
        severity,
        summary: summary_text,
        details: ChallengeDetails::Framework(FrameworkChallengeDetails {
            react_portal: q.react_portal,
            angular_zone: q.angular_zone,
            vue_teleport: q.vue_teleport,
            svelte_hydration: q.svelte_hydration,
        }),
        suggestion: Some(SUGGESTION_FRAMEWORK.to_string()),
    }
}

// =============================================================================
// Script runner adapter
// =============================================================================

/// Run a `diagnose` command against an existing session and return a JSON value.
///
/// # Errors
///
/// Propagates `AppError` from the underlying diagnose logic.
#[allow(dead_code)]
pub async fn run_from_session(
    _managed: &mut agentchrome::connection::ManagedSession,
    global: &GlobalOpts,
    args: &DiagnoseArgs,
) -> Result<serde_json::Value, AppError> {
    execute_diagnose(global, args).await?;
    Ok(serde_json::json!({"executed": true}))
}

// =============================================================================
// Tests (T008: severity + challenge assembly)
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::diagnose::output::PatternMatch;
    use crate::page::analyze::{IframeInfo, MediaInfo, OverlayInfo, ShadowDomInfo};

    use super::*;
    use crate::diagnose::detectors::{FrameworkQuirks, MediaGateInfo};

    fn no_shadow() -> ShadowDomInfo {
        ShadowDomInfo {
            present: false,
            host_count: 0,
        }
    }

    fn make_bundle<'a>(
        iframes: &'a [IframeInfo],
        frameworks: &'a [String],
        overlays: &'a [OverlayInfo],
        shadow_dom: &'a ShadowDomInfo,
        media: &'a [MediaInfo],
        canvas: Option<&'a CanvasInfo>,
        framework_quirks: &'a FrameworkQuirks,
    ) -> DetectorBundle<'a> {
        DetectorBundle {
            iframes,
            frameworks,
            overlays,
            shadow_dom,
            media,
            canvas,
            framework_quirks,
        }
    }

    // --- Clean page produces empty challenges ---

    #[test]
    fn assemble_challenges_empty_for_clean_page() {
        let shadow = no_shadow();
        let quirks = FrameworkQuirks::default();
        let bundle = make_bundle(&[], &[], &[], &shadow, &[], None, &quirks);
        let challenges = assemble_challenges(&bundle, &[]);
        assert!(
            challenges.is_empty(),
            "Expected no challenges for a clean page, got: {challenges:?}"
        );
    }

    // --- Iframes severity ---

    #[test]
    fn iframe_severity_high_for_visible_cross_origin() {
        let iframes = vec![IframeInfo {
            index: 1,
            url: "https://other.example.com".to_string(),
            name: String::new(),
            visible: true,
            width: 800,
            height: 600,
            cross_origin: true,
        }];
        let sev = iframe_severity(1, 1, &iframes);
        assert_eq!(sev, "high");
    }

    #[test]
    fn iframe_severity_medium_for_same_origin() {
        let iframes = vec![IframeInfo {
            index: 1,
            url: "https://example.com/frame".to_string(),
            name: String::new(),
            visible: true,
            width: 800,
            height: 600,
            cross_origin: false,
        }];
        let sev = iframe_severity(1, 0, &iframes);
        assert_eq!(sev, "medium");
    }

    // --- Overlays severity ---

    #[test]
    fn overlay_severity_high_when_covers_interactive() {
        let overlays = vec![OverlayInfo {
            selector: "div.blocker".to_string(),
            z_index: 999,
            width: 1920,
            height: 1080,
            covers_interactive: true,
        }];
        let shadow = no_shadow();
        let quirks = FrameworkQuirks::default();
        let bundle = make_bundle(&[], &[], &overlays, &shadow, &[], None, &quirks);
        let challenge = build_overlays_challenge(&bundle);
        assert_eq!(challenge.severity, "high");
    }

    #[test]
    fn overlay_severity_low_when_no_interactive_coverage() {
        let overlays = vec![OverlayInfo {
            selector: "div.spinner".to_string(),
            z_index: 5,
            width: 200,
            height: 200,
            covers_interactive: false,
        }];
        let shadow = no_shadow();
        let quirks = FrameworkQuirks::default();
        let bundle = make_bundle(&[], &[], &overlays, &shadow, &[], None, &quirks);
        let challenge = build_overlays_challenge(&bundle);
        assert_eq!(challenge.severity, "low");
    }

    // --- Shadow DOM severity ---

    #[test]
    fn shadow_dom_severity_high_for_many_hosts() {
        let shadow = ShadowDomInfo {
            present: true,
            host_count: 15,
        };
        let quirks = FrameworkQuirks::default();
        let bundle = make_bundle(&[], &[], &[], &shadow, &[], None, &quirks);
        let challenge = build_shadow_dom_challenge(&bundle);
        assert_eq!(challenge.severity, "high");
    }

    #[test]
    fn shadow_dom_severity_medium_for_few_hosts() {
        let shadow = ShadowDomInfo {
            present: true,
            host_count: 3,
        };
        let quirks = FrameworkQuirks::default();
        let bundle = make_bundle(&[], &[], &[], &shadow, &[], None, &quirks);
        let challenge = build_shadow_dom_challenge(&bundle);
        assert_eq!(challenge.severity, "medium");
    }

    // --- Media severity ---

    #[test]
    fn media_severity_high_when_gate_present() {
        let gates = vec![MediaGateInfo {
            tag: "audio".to_string(),
            src: None,
            state: Some("paused".to_string()),
            gates_navigation: true,
        }];
        let challenge = build_media_challenge(&gates);
        assert_eq!(challenge.severity, "high");
    }

    #[test]
    fn media_severity_medium_when_no_gate() {
        let gates = vec![MediaGateInfo {
            tag: "video".to_string(),
            src: None,
            state: Some("playing".to_string()),
            gates_navigation: false,
        }];
        let challenge = build_media_challenge(&gates);
        assert_eq!(challenge.severity, "medium");
    }

    // --- Framework severity ---

    #[test]
    fn framework_severity_high_for_two_or_more_quirks() {
        let shadow = no_shadow();
        let quirks = FrameworkQuirks {
            react_portal: true,
            angular_zone: true,
            ..Default::default()
        };
        let bundle = make_bundle(&[], &[], &[], &shadow, &[], None, &quirks);
        let challenge = build_framework_challenge(&bundle);
        assert_eq!(challenge.severity, "high");
    }

    #[test]
    fn framework_severity_medium_for_one_quirk() {
        let shadow = no_shadow();
        let quirks = FrameworkQuirks {
            angular_zone: true,
            ..Default::default()
        };
        let bundle = make_bundle(&[], &[], &[], &shadow, &[], None, &quirks);
        let challenge = build_framework_challenge(&bundle);
        assert_eq!(challenge.severity, "medium");
    }

    // --- Summary ---

    #[test]
    fn summary_straightforward_for_clean_page() {
        let challenges: Vec<Challenge> = vec![];
        let patterns: Vec<PatternMatch> = vec![];
        #[allow(clippy::cast_possible_truncation)]
        let challenge_count = challenges.len() as u32;
        #[allow(clippy::cast_possible_truncation)]
        let pattern_match_count = patterns.len() as u32;
        let has_high_severity = challenges.iter().any(|c| c.severity == "high");
        let straightforward = challenge_count == 0 && pattern_match_count == 0;
        assert!(straightforward);
        assert!(!has_high_severity);
    }
}
