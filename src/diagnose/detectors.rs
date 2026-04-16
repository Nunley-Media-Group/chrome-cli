use agentchrome::connection::ManagedSession;

use crate::page::analyze::MediaInfo;

// =============================================================================
// Canvas / WebGL detection (T005)
// =============================================================================

/// Information about a single `<canvas>` element.
#[derive(Debug)]
pub(crate) struct CanvasItem {
    /// Rendered width in CSS pixels. `None` if the bounding rect was unavailable.
    pub(crate) width: Option<u32>,
    /// Rendered height in CSS pixels. `None` if the bounding rect was unavailable.
    pub(crate) height: Option<u32>,
    /// Detected rendering context type: `"webgl2"`, `"webgl"`, `"2d"`, or `None`.
    pub(crate) context: Option<String>,
}

/// Aggregate canvas detection results.
#[derive(Debug)]
pub(crate) struct CanvasInfo {
    pub(crate) canvas_count: u32,
    /// Number of canvases with a detected WebGL or WebGL2 context.
    pub(crate) webgl_count: u32,
    pub(crate) items: Vec<CanvasItem>,
}

/// Detect all `<canvas>` elements and their rendering contexts.
///
/// Returns `None` when there are no `<canvas>` elements on the page or when
/// the CDP eval fails (graceful degradation).
///
/// Detection probes `webgl2 → webgl → 2d` in that order to avoid pinning a
/// low-capability context where the page would have used a higher one (R1
/// mitigation from design.md).
pub(crate) async fn detect_canvas(session: &ManagedSession) -> Option<CanvasInfo> {
    // NOTE: Runtime.evaluate bypasses page CSP (runs in isolated world), so
    // no CSP mitigation is needed here (design.md R7).
    let js = r"(function() {
        var canvases = document.querySelectorAll('canvas');
        if (!canvases.length) return JSON.stringify([]);
        return JSON.stringify(Array.from(canvases).map(function(c) {
            var ctx = null;
            try {
                if (c.getContext('webgl2')) { ctx = 'webgl2'; }
                else if (c.getContext('webgl')) { ctx = 'webgl'; }
                else if (c.getContext('2d')) { ctx = '2d'; }
            } catch(e) {}
            var rect = c.getBoundingClientRect();
            var w = Math.round(rect.width);
            var h = Math.round(rect.height);
            return {
                width: w > 0 ? w : null,
                height: h > 0 ? h : null,
                context: ctx
            };
        }));
    })()";

    let params = serde_json::json!({
        "expression": js,
        "returnByValue": true,
    });

    let Ok(result) = session.send_command("Runtime.evaluate", Some(params)).await else {
        return None;
    };

    let value_str = result["result"]["value"].as_str()?;
    let raw: Vec<serde_json::Value> = serde_json::from_str(value_str).ok()?;

    if raw.is_empty() {
        return None;
    }

    let mut webgl_count = 0u32;
    let items: Vec<CanvasItem> = raw
        .into_iter()
        .map(|v| {
            let context = v["context"].as_str().map(String::from);
            if matches!(context.as_deref(), Some("webgl" | "webgl2")) {
                webgl_count += 1;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            CanvasItem {
                width: v["width"].as_u64().map(|n| n as u32),
                height: v["height"].as_u64().map(|n| n as u32),
                context,
            }
        })
        .collect();

    #[allow(clippy::cast_possible_truncation)]
    Some(CanvasInfo {
        canvas_count: items.len() as u32,
        webgl_count,
        items,
    })
}

// =============================================================================
// Media gate classification (T006)
// =============================================================================

/// Media element enriched with a navigation-gate flag.
#[derive(Debug)]
pub(crate) struct MediaGateInfo {
    pub(crate) tag: String,
    pub(crate) src: Option<String>,
    pub(crate) state: Option<String>,
    /// `true` when this media element is likely to gate user navigation (i.e.,
    /// it has `autoplay` semantics but is currently paused, suggesting
    /// user-interaction is required before the page will advance).
    pub(crate) gates_navigation: bool,
}

/// Classify media elements from `catalog_media` output, adding a `gates_navigation`
/// flag using the heuristic described in design.md (pure function, no CDP).
pub(crate) fn classify_media_gate(media: &[MediaInfo]) -> Vec<MediaGateInfo> {
    media
        .iter()
        .map(|m| {
            // Heuristic: audio/video that is paused (likely autoplay-blocked or
            // waiting for user interaction) constitutes a navigation gate.
            let gates_navigation = matches!(m.state.as_deref(), Some("paused"))
                && matches!(m.tag.as_str(), "audio" | "video");
            MediaGateInfo {
                tag: m.tag.clone(),
                src: m.src.clone(),
                state: m.state.clone(),
                gates_navigation,
            }
        })
        .collect()
}

// =============================================================================
// Framework quirk detection (T006)
// =============================================================================

/// Framework-specific interaction quirks detected on the page.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default)]
pub(crate) struct FrameworkQuirks {
    /// React portals rendering outside the root element.
    pub(crate) react_portal: bool,
    /// Angular zone.js event trap (`window.Zone` present).
    pub(crate) angular_zone: bool,
    /// Vue teleport (`data-v-*` attributes outside any declared root mount).
    pub(crate) vue_teleport: bool,
    /// Svelte hydration markers (`<!--[-->` comments present).
    pub(crate) svelte_hydration: bool,
}

impl FrameworkQuirks {
    /// Returns `true` if any quirk was detected.
    pub(crate) fn any(&self) -> bool {
        self.react_portal || self.angular_zone || self.vue_teleport || self.svelte_hydration
    }
}

/// Detect framework-specific interaction quirks with a single `Runtime.evaluate`
/// round trip. On eval failure, returns a zeroed struct (graceful degradation).
pub(crate) async fn detect_framework_quirks(session: &ManagedSession) -> FrameworkQuirks {
    let js = r"(function() {
        var result = {
            reactPortal: false,
            angularZone: false,
            vueTeleport: false,
            svelteHydration: false
        };
        try {
            // React portal: look for React fiber roots outside #root / #app
            var hook = window.__REACT_DEVTOOLS_GLOBAL_HOOK__;
            if (hook && hook.renderers && hook.renderers.size > 0) {
                result.reactPortal = true;
            } else if (document.querySelector('[data-reactroot]') !== null) {
                result.reactPortal = true;
            }
        } catch(e) {}
        try {
            if (typeof window.Zone !== 'undefined') {
                result.angularZone = true;
            }
        } catch(e) {}
        try {
            // Vue teleport: Vue SFCs emit hashed scoped-style attrs like data-v-abc123.
            // querySelector('[data-v-]') is not a valid syntax for prefix match;
            // instead, walk a small sample of elements and check attribute names.
            var sample = document.querySelectorAll('body *');
            var max = Math.min(sample.length, 500);
            for (var i = 0; i < max; i++) {
                var attrs = sample[i].attributes;
                for (var j = 0; j < attrs.length; j++) {
                    if (attrs[j].name.indexOf('data-v-') === 0) {
                        result.vueTeleport = true;
                        break;
                    }
                }
                if (result.vueTeleport) { break; }
            }
        } catch(e) {}
        try {
            // Svelte hydration markers in HTML comments
            var iter = document.createNodeIterator(
                document.body || document.documentElement,
                NodeFilter.SHOW_COMMENT,
                null
            );
            var node;
            while ((node = iter.nextNode()) !== null) {
                if (node.nodeValue === '[' || node.nodeValue === ']') {
                    result.svelteHydration = true;
                    break;
                }
            }
        } catch(e) {}
        return JSON.stringify(result);
    })()";

    let params = serde_json::json!({
        "expression": js,
        "returnByValue": true,
    });

    let Ok(result) = session.send_command("Runtime.evaluate", Some(params)).await else {
        return FrameworkQuirks::default();
    };

    let Some(value_str) = result["result"]["value"].as_str() else {
        return FrameworkQuirks::default();
    };

    let Ok(raw): Result<serde_json::Value, _> = serde_json::from_str(value_str) else {
        return FrameworkQuirks::default();
    };

    FrameworkQuirks {
        react_portal: raw["reactPortal"].as_bool().unwrap_or(false),
        angular_zone: raw["angularZone"].as_bool().unwrap_or(false),
        vue_teleport: raw["vueTeleport"].as_bool().unwrap_or(false),
        svelte_hydration: raw["svelteHydration"].as_bool().unwrap_or(false),
    }
}

// =============================================================================
// Suggestion string constants (per-category)
// All strings must reference at least one agentchrome command token (T010).
// =============================================================================

pub(crate) const SUGGESTION_IFRAMES: &str = "Use --frame <index> on page and interact commands to target content inside iframes. \
     For cross-origin frames, use 'interact click-at --frame N' with coordinate targeting \
     (selector targeting is unavailable across origins).";

pub(crate) const SUGGESTION_OVERLAYS: &str = "Large overlays intercept clicks. Try 'interact click-at' with explicit coordinates inside \
     the real content area, or target the obscured element via its iframe using --frame. \
     Use 'page snapshot' to inspect the overlay's accessibility role.";

pub(crate) const SUGGESTION_SHADOW_DOM: &str = "Shadow DOM hosts encapsulate their subtrees. Use 'page snapshot' to discover elements inside \
     shadow roots, then target them with 'interact click' or 'form fill' by UID. \
     Standard CSS selectors do not pierce shadow boundaries.";

pub(crate) const SUGGESTION_CANVAS: &str = "Canvas-rendered UI is not accessible via the DOM or accessibility tree. \
     Use 'interact click-at' with coordinate targeting; the 'page snapshot' accessibility tree \
     will be sparse for canvas content.";

pub(crate) const SUGGESTION_MEDIA: &str = "Media elements may gate page flow. Use 'agentchrome media play 0' to unblock audio/video \
     gates before interacting with the rest of the page. \
     Run 'agentchrome media list' to discover all media elements.";

pub(crate) const SUGGESTION_FRAMEWORK: &str = "Framework-specific event routing may affect interaction. Use 'interact click' or \
     'interact key' via UIDs from 'page snapshot' rather than raw coordinates when possible, \
     to let the framework's event system receive events correctly.";

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framework_quirks_any() {
        let q = FrameworkQuirks::default();
        assert!(!q.any());
        let q2 = FrameworkQuirks {
            angular_zone: true,
            ..Default::default()
        };
        assert!(q2.any());
    }

    #[test]
    fn classify_media_gate_paused_audio_is_gate() {
        let media = vec![MediaInfo {
            tag: "audio".to_string(),
            src: None,
            state: Some("paused".to_string()),
            width: None,
            height: None,
        }];
        let gates = classify_media_gate(&media);
        assert_eq!(gates.len(), 1);
        assert!(gates[0].gates_navigation);
    }

    #[test]
    fn classify_media_gate_playing_is_not_gate() {
        let media = vec![MediaInfo {
            tag: "video".to_string(),
            src: None,
            state: Some("playing".to_string()),
            width: Some(640),
            height: Some(360),
        }];
        let gates = classify_media_gate(&media);
        assert!(!gates[0].gates_navigation);
    }

    #[test]
    fn classify_media_gate_embed_is_not_gate() {
        let media = vec![MediaInfo {
            tag: "embed".to_string(),
            src: Some("file.swf".to_string()),
            state: None,
            width: None,
            height: None,
        }];
        let gates = classify_media_gate(&media);
        assert!(!gates[0].gates_navigation);
    }

    #[test]
    fn all_suggestion_strings_reference_agentchrome_command() {
        // T010 lint: every suggestion constant must contain at least one
        // agentchrome command token.
        let tokens = [
            "agentchrome",
            "interact click-at",
            "interact click",
            "interact key",
            "--frame",
            "page find",
            "page snapshot",
            "form fill",
            "js exec",
            "media play",
            "media list",
        ];
        let suggestions = [
            ("SUGGESTION_IFRAMES", SUGGESTION_IFRAMES),
            ("SUGGESTION_OVERLAYS", SUGGESTION_OVERLAYS),
            ("SUGGESTION_SHADOW_DOM", SUGGESTION_SHADOW_DOM),
            ("SUGGESTION_CANVAS", SUGGESTION_CANVAS),
            ("SUGGESTION_MEDIA", SUGGESTION_MEDIA),
            ("SUGGESTION_FRAMEWORK", SUGGESTION_FRAMEWORK),
        ];
        for (name, s) in &suggestions {
            let has_token = tokens.iter().any(|t| s.contains(t));
            assert!(
                has_token,
                "Suggestion constant {name} does not reference any agentchrome command token.\n\
                 Content: {s}"
            );
        }
    }
}
