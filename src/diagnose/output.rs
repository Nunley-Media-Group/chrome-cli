use serde::Serialize;

// =============================================================================
// Top-level result
// =============================================================================

/// Full JSON output for `diagnose`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnoseResult {
    pub(crate) url: String,
    pub(crate) scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) navigation_status: Option<u16>,
    pub(crate) challenges: Vec<Challenge>,
    pub(crate) patterns: Vec<PatternMatch>,
    pub(crate) summary: DiagnoseSummary,
}

// =============================================================================
// Challenge
// =============================================================================

/// A detected automation challenge in a specific category.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Challenge {
    pub(crate) category: String,
    pub(crate) severity: String,
    pub(crate) summary: String,
    pub(crate) details: ChallengeDetails,
    pub(crate) suggestion: Option<String>,
}

/// Per-category detail structures, serialised as an untagged inline object.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum ChallengeDetails {
    Iframes(IframesChallengeDetails),
    Overlays(OverlaysChallengeDetails),
    ShadowDom(ShadowDomChallengeDetails),
    Canvas(CanvasChallengeDetails),
    Media(MediaChallengeDetails),
    Framework(FrameworkChallengeDetails),
}

// --- Iframes ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IframesChallengeDetails {
    pub(crate) count: u32,
    pub(crate) cross_origin_count: u32,
    pub(crate) items: Vec<IframeChallengeItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IframeChallengeItem {
    pub(crate) index: u32,
    pub(crate) url: String,
    pub(crate) name: String,
    pub(crate) visible: bool,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) cross_origin: bool,
    /// `None` when the frame is cross-origin and its internals cannot be
    /// measured. Serialised as JSON `null`, never coerced to `0`.
    pub(crate) interactive_element_count: Option<u32>,
}

// --- Overlays ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlaysChallengeDetails {
    pub(crate) items: Vec<OverlayChallengeItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayChallengeItem {
    pub(crate) selector: String,
    pub(crate) z_index: i64,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) covers_interactive: bool,
}

// --- Shadow DOM ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShadowDomChallengeDetails {
    pub(crate) host_count: u32,
}

// --- Canvas ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CanvasChallengeDetails {
    pub(crate) canvas_count: u32,
    pub(crate) webgl_count: u32,
    pub(crate) items: Vec<CanvasChallengeItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CanvasChallengeItem {
    /// `None` when the canvas has no accessible bounding rect.
    pub(crate) width: Option<u32>,
    /// `None` when the canvas has no accessible bounding rect.
    pub(crate) height: Option<u32>,
    /// `None` when no context could be detected.
    pub(crate) context: Option<String>,
}

// --- Media ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaChallengeDetails {
    pub(crate) items: Vec<MediaChallengeItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaChallengeItem {
    pub(crate) tag: String,
    pub(crate) src: Option<String>,
    pub(crate) state: Option<String>,
    pub(crate) gates_navigation: bool,
}

// --- Framework ---

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FrameworkChallengeDetails {
    pub(crate) react_portal: bool,
    pub(crate) angular_zone: bool,
    pub(crate) vue_teleport: bool,
    pub(crate) svelte_hydration: bool,
}

// =============================================================================
// Pattern match
// =============================================================================

/// A matched known automation pattern.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatternMatch {
    pub(crate) name: String,
    pub(crate) matched: bool,
    pub(crate) confidence: String,
    pub(crate) evidence: String,
    pub(crate) suggestion: String,
}

// =============================================================================
// Summary
// =============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnoseSummary {
    pub(crate) challenge_count: u32,
    pub(crate) pattern_match_count: u32,
    pub(crate) has_high_severity: bool,
    pub(crate) straightforward: bool,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnose_result_serializes_camel_case() {
        let result = DiagnoseResult {
            url: "https://example.com".to_string(),
            scope: "diagnosed".to_string(),
            navigation_status: Some(200),
            challenges: vec![],
            patterns: vec![],
            summary: DiagnoseSummary {
                challenge_count: 0,
                pattern_match_count: 0,
                has_high_severity: false,
                straightforward: true,
            },
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["url"], "https://example.com");
        assert_eq!(json["scope"], "diagnosed");
        assert_eq!(json["navigationStatus"], 200);
        assert_eq!(json["challenges"], serde_json::json!([]));
        assert_eq!(json["summary"]["challengeCount"], 0);
        assert_eq!(json["summary"]["patternMatchCount"], 0);
        assert_eq!(json["summary"]["hasHighSeverity"], false);
        assert_eq!(json["summary"]["straightforward"], true);
        // No snake_case keys
        assert!(json.get("navigation_status").is_none());
    }

    #[test]
    fn navigation_status_omitted_when_none() {
        let result = DiagnoseResult {
            url: "https://example.com".to_string(),
            scope: "current".to_string(),
            navigation_status: None,
            challenges: vec![],
            patterns: vec![],
            summary: DiagnoseSummary {
                challenge_count: 0,
                pattern_match_count: 0,
                has_high_severity: false,
                straightforward: true,
            },
        };
        let json = serde_json::to_value(&result).unwrap();
        assert!(json.get("navigationStatus").is_none());
    }

    #[test]
    fn iframe_challenge_item_null_interactive_count() {
        let item = IframeChallengeItem {
            index: 1,
            url: "https://other.example.com".to_string(),
            name: String::new(),
            visible: true,
            width: 960,
            height: 540,
            cross_origin: true,
            interactive_element_count: None,
        };
        let json = serde_json::to_value(&item).unwrap();
        // null, not omitted, not 0
        assert_eq!(json["interactiveElementCount"], serde_json::Value::Null);
    }

    #[test]
    fn challenge_details_iframes_serializes_cleanly() {
        let details = ChallengeDetails::Iframes(IframesChallengeDetails {
            count: 2,
            cross_origin_count: 1,
            items: vec![],
        });
        let json = serde_json::to_value(&details).unwrap();
        assert_eq!(json["count"], 2);
        assert_eq!(json["crossOriginCount"], 1);
    }

    #[test]
    fn pattern_match_serializes_camel_case() {
        let pm = PatternMatch {
            name: "storyline-acc-blocker".to_string(),
            matched: true,
            confidence: "high".to_string(),
            evidence: "div.acc-blocker covers 100%".to_string(),
            suggestion: "Use agentchrome interact --frame N click-at X Y".to_string(),
        };
        let json = serde_json::to_value(&pm).unwrap();
        assert_eq!(json["name"], "storyline-acc-blocker");
        assert_eq!(json["matched"], true);
        assert_eq!(json["confidence"], "high");
    }
}
