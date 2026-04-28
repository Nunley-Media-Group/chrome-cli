use crate::page::analyze::{IframeInfo, MediaInfo, OverlayInfo, ShadowDomInfo};

use super::detectors::{CanvasInfo, FrameworkQuirks};
use super::output::PatternMatch;

// =============================================================================
// Detector bundle
// =============================================================================

/// All detector outputs passed to each pattern rule. Pattern detectors are
/// pure functions over this bundle — they must NOT make CDP calls.
pub(crate) struct DetectorBundle<'a> {
    pub(crate) iframes: &'a [IframeInfo],
    pub(crate) frameworks: &'a [String],
    pub(crate) overlays: &'a [OverlayInfo],
    pub(crate) shadow_dom: &'a ShadowDomInfo,
    /// Reserved for future pattern rules; currently unused by the initial set.
    #[allow(dead_code)]
    pub(crate) media: &'a [MediaInfo],
    pub(crate) canvas: Option<&'a CanvasInfo>,
    pub(crate) framework_quirks: &'a FrameworkQuirks,
}

// =============================================================================
// Pattern rule
// =============================================================================

pub(crate) struct PatternRule {
    /// Canonical pattern name; stored for future introspection / pattern-dump tooling.
    #[allow(dead_code)]
    pub(crate) name: &'static str,
    pub(crate) detector: fn(&DetectorBundle<'_>) -> Option<PatternMatch>,
}

// =============================================================================
// Pattern database
// =============================================================================

pub(crate) static PATTERN_DB: &[PatternRule] = &[
    PatternRule {
        name: "storyline-acc-blocker",
        detector: detect_storyline_acc_blocker,
    },
    PatternRule {
        name: "scorm-player",
        detector: detect_scorm_player,
    },
    PatternRule {
        name: "react-portal",
        detector: detect_react_portal,
    },
];

/// Run all patterns against the detector bundle and return only matched entries.
pub(crate) fn match_all(bundle: &DetectorBundle<'_>) -> Vec<PatternMatch> {
    PATTERN_DB
        .iter()
        .filter_map(|rule| (rule.detector)(bundle))
        .collect()
}

// =============================================================================
// Individual pattern detectors
// =============================================================================

fn detect_storyline_acc_blocker(b: &DetectorBundle<'_>) -> Option<PatternMatch> {
    // Signal 1: an overlay whose selector contains "acc-blocker"
    let acc_blocker = b
        .overlays
        .iter()
        .find(|o| o.selector.contains("acc-blocker"))?;

    // Signal 2 (optional): Storyline framework signature
    let has_storyline = b.frameworks.iter().any(|f| f == "Storyline");

    // Require at least one corroborating signal beyond the bare selector match
    // to avoid false positives on unrelated pages that happen to use the
    // "acc-blocker" class on a non-covering element.
    if !has_storyline && !acc_blocker.covers_interactive {
        return None;
    }

    let confidence = if has_storyline && acc_blocker.covers_interactive {
        "high"
    } else {
        "medium"
    };

    let evidence = format!(
        "{} covers a {}×{}px region at z-index {}{}",
        acc_blocker.selector,
        acc_blocker.width,
        acc_blocker.height,
        acc_blocker.z_index,
        if has_storyline {
            "; Storyline framework signature detected"
        } else {
            ""
        }
    );

    Some(PatternMatch {
        name: "storyline-acc-blocker".to_string(),
        matched: true,
        confidence: confidence.to_string(),
        evidence,
        suggestion: "Articulate Storyline renders course content inside an iframe and shields the \
            main frame with an acc-blocker overlay. Target the content iframe directly with \
            'agentchrome interact --frame N click-at X Y' where N is the Storyline iframe index (see \
            challenges.iframes.details.items)."
            .to_string(),
    })
}

fn detect_scorm_player(b: &DetectorBundle<'_>) -> Option<PatternMatch> {
    // Signal: SCORM API present in framework list
    let has_scorm = b.frameworks.iter().any(|f| f == "SCORM");
    if !has_scorm {
        return None;
    }

    // Confidence: higher when an iframe is also present (typical SCORM setup)
    let confidence = if b.iframes.is_empty() {
        "medium"
    } else {
        "high"
    };

    let evidence = format!(
        "SCORM API (window.API or window.API_1484_11) detected{}",
        if b.iframes.is_empty() {
            String::new()
        } else {
            format!("; {} iframe(s) present", b.iframes.len())
        }
    );

    Some(PatternMatch {
        name: "scorm-player".to_string(),
        matched: true,
        confidence: confidence.to_string(),
        evidence,
        suggestion: "SCORM players expose window.API or window.API_1484_11. \
            Course content is usually inside an iframe — use 'agentchrome interact --frame N click-at X Y' \
            or 'agentchrome page --frame N snapshot' to inspect and interact with the frame content. \
            Run 'agentchrome diagnose --current' to identify the iframe index."
            .to_string(),
    })
}

fn detect_react_portal(b: &DetectorBundle<'_>) -> Option<PatternMatch> {
    // Signal: React portal quirk detected (React devtools hook present)
    if !b.framework_quirks.react_portal {
        return None;
    }

    // Confidence: higher when React framework is also in the frameworks list
    let has_react_framework = b.frameworks.iter().any(|f| f == "React");
    let confidence = if has_react_framework {
        "high"
    } else {
        "medium"
    };

    let evidence = if has_react_framework {
        "React devtools hook and React framework signature both detected".to_string()
    } else {
        "React devtools hook detected (portal root may render outside main React root)".to_string()
    };

    Some(PatternMatch {
        name: "react-portal".to_string(),
        matched: true,
        confidence: confidence.to_string(),
        evidence,
        suggestion: "React portals render modal/dialog content outside the main React root. \
            Dialogs, tooltips, and overlays may appear at document body level. \
            Use 'page snapshot' to find portal-rendered elements and 'interact click' by UID. \
            If a dialog blocks interaction, try 'interact key Escape' to dismiss it."
            .to_string(),
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::page::analyze::{IframeInfo, MediaInfo, OverlayInfo, ShadowDomInfo};

    use super::*;
    use crate::diagnose::detectors::{CanvasInfo, FrameworkQuirks};

    fn empty_bundle<'a>(
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

    fn no_shadow() -> ShadowDomInfo {
        ShadowDomInfo {
            present: false,
            host_count: 0,
        }
    }

    // -----------------------------------------------------------------------
    // storyline-acc-blocker
    // -----------------------------------------------------------------------

    #[test]
    fn storyline_acc_blocker_positive_high_confidence() {
        let overlays = vec![OverlayInfo {
            selector: "div.acc-blocker".to_string(),
            z_index: 9999,
            width: 1920,
            height: 1080,
            covers_interactive: true,
        }];
        let frameworks = vec!["Storyline".to_string()];
        let shadow = no_shadow();
        let quirks = FrameworkQuirks::default();
        let bundle = empty_bundle(&[], &frameworks, &overlays, &shadow, &[], None, &quirks);
        let m = detect_storyline_acc_blocker(&bundle).unwrap();
        assert_eq!(m.name, "storyline-acc-blocker");
        assert_eq!(m.confidence, "high");
        assert!(m.evidence.contains("9999"));
        assert!(m.suggestion.contains("interact --frame N click-at"));
    }

    #[test]
    fn storyline_acc_blocker_medium_confidence_no_framework() {
        let overlays = vec![OverlayInfo {
            selector: "div.acc-blocker".to_string(),
            z_index: 100,
            width: 800,
            height: 600,
            covers_interactive: true,
        }];
        let shadow = no_shadow();
        let quirks = FrameworkQuirks::default();
        let bundle = empty_bundle(&[], &[], &overlays, &shadow, &[], None, &quirks);
        let m = detect_storyline_acc_blocker(&bundle).unwrap();
        assert_eq!(m.confidence, "medium");
    }

    #[test]
    fn storyline_acc_blocker_skipped_when_no_corroborating_signal() {
        // An overlay whose selector matches but with no framework signature and
        // no interactive coverage must NOT match — bare class-name containment
        // would otherwise produce a noisy false positive.
        let overlays = vec![OverlayInfo {
            selector: "div.acc-blocker".to_string(),
            z_index: 10,
            width: 100,
            height: 100,
            covers_interactive: false,
        }];
        let shadow = no_shadow();
        let quirks = FrameworkQuirks::default();
        let bundle = empty_bundle(&[], &[], &overlays, &shadow, &[], None, &quirks);
        assert!(detect_storyline_acc_blocker(&bundle).is_none());
    }

    #[test]
    fn storyline_acc_blocker_no_match_without_overlay() {
        let shadow = no_shadow();
        let quirks = FrameworkQuirks::default();
        let bundle = empty_bundle(&[], &[], &[], &shadow, &[], None, &quirks);
        assert!(detect_storyline_acc_blocker(&bundle).is_none());
    }

    // -----------------------------------------------------------------------
    // scorm-player
    // -----------------------------------------------------------------------

    #[test]
    fn scorm_player_positive_high_confidence_with_iframe() {
        let frameworks = vec!["SCORM".to_string()];
        let iframes = vec![IframeInfo {
            index: 1,
            url: "https://cdn.example.com/course".to_string(),
            name: String::new(),
            visible: true,
            width: 960,
            height: 540,
            cross_origin: true,
        }];
        let shadow = no_shadow();
        let quirks = FrameworkQuirks::default();
        let bundle = empty_bundle(&iframes, &frameworks, &[], &shadow, &[], None, &quirks);
        let m = detect_scorm_player(&bundle).unwrap();
        assert_eq!(m.confidence, "high");
        assert!(m.suggestion.contains("interact --frame N click-at"));
    }

    #[test]
    fn scorm_player_medium_confidence_no_iframe() {
        let frameworks = vec!["SCORM".to_string()];
        let shadow = no_shadow();
        let quirks = FrameworkQuirks::default();
        let bundle = empty_bundle(&[], &frameworks, &[], &shadow, &[], None, &quirks);
        let m = detect_scorm_player(&bundle).unwrap();
        assert_eq!(m.confidence, "medium");
    }

    #[test]
    fn scorm_player_no_match_without_scorm_framework() {
        let shadow = no_shadow();
        let quirks = FrameworkQuirks::default();
        let bundle = empty_bundle(&[], &[], &[], &shadow, &[], None, &quirks);
        assert!(detect_scorm_player(&bundle).is_none());
    }

    // -----------------------------------------------------------------------
    // react-portal
    // -----------------------------------------------------------------------

    #[test]
    fn react_portal_positive_high_confidence_with_framework() {
        let frameworks = vec!["React".to_string()];
        let shadow = no_shadow();
        let quirks = FrameworkQuirks {
            react_portal: true,
            ..Default::default()
        };
        let bundle = empty_bundle(&[], &frameworks, &[], &shadow, &[], None, &quirks);
        let m = detect_react_portal(&bundle).unwrap();
        assert_eq!(m.confidence, "high");
        assert!(m.suggestion.contains("page snapshot"));
    }

    #[test]
    fn react_portal_medium_confidence_no_framework_list() {
        let shadow = no_shadow();
        let quirks = FrameworkQuirks {
            react_portal: true,
            ..Default::default()
        };
        let bundle = empty_bundle(&[], &[], &[], &shadow, &[], None, &quirks);
        let m = detect_react_portal(&bundle).unwrap();
        assert_eq!(m.confidence, "medium");
    }

    #[test]
    fn react_portal_no_match_without_quirk() {
        let shadow = no_shadow();
        let quirks = FrameworkQuirks::default();
        let bundle = empty_bundle(&[], &[], &[], &shadow, &[], None, &quirks);
        assert!(detect_react_portal(&bundle).is_none());
    }

    // -----------------------------------------------------------------------
    // match_all + suggestion lint
    // -----------------------------------------------------------------------

    #[test]
    fn match_all_returns_empty_for_clean_page() {
        let shadow = no_shadow();
        let quirks = FrameworkQuirks::default();
        let bundle = empty_bundle(&[], &[], &[], &shadow, &[], None, &quirks);
        let results = match_all(&bundle);
        assert!(results.is_empty());
    }

    #[test]
    fn all_pattern_suggestions_reference_agentchrome_command() {
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
        ];

        // Synthesize a bundle that matches all patterns
        let iframes = vec![IframeInfo {
            index: 1,
            url: "https://cdn.example.com/course".to_string(),
            name: String::new(),
            visible: true,
            width: 960,
            height: 540,
            cross_origin: true,
        }];
        let frameworks = vec![
            "Storyline".to_string(),
            "SCORM".to_string(),
            "React".to_string(),
        ];
        let overlays = vec![OverlayInfo {
            selector: "div.acc-blocker".to_string(),
            z_index: 9999,
            width: 1920,
            height: 1080,
            covers_interactive: true,
        }];
        let shadow = no_shadow();
        let media: Vec<MediaInfo> = vec![];
        let quirks = FrameworkQuirks {
            react_portal: true,
            ..Default::default()
        };
        let bundle = empty_bundle(
            &iframes,
            &frameworks,
            &overlays,
            &shadow,
            &media,
            None,
            &quirks,
        );
        let matches = match_all(&bundle);
        assert!(!matches.is_empty(), "Expected at least one pattern match");
        for pm in &matches {
            let has_token = tokens.iter().any(|t| pm.suggestion.contains(t));
            assert!(
                has_token,
                "Pattern '{}' suggestion does not reference any agentchrome command token.\n\
                 Suggestion: {}",
                pm.name, pm.suggestion
            );
        }
    }
}
