use std::sync::LazyLock;

use serde::Serialize;

// =============================================================================
// Strategy types
// =============================================================================

/// Lightweight listing shape — returned by `examples strategies [--json]`.
/// Progressive disclosure: three fields only, ~100–200 bytes per entry.
#[derive(Serialize, Clone)]
pub struct StrategySummary {
    pub name: String,
    pub title: String,
    pub summary: String,
}

/// Full strategy shape — returned only by `examples strategies <name> [--json]`.
#[derive(Serialize, Clone)]
pub struct Strategy {
    pub name: String,
    pub title: String,
    pub summary: String,
    pub scenarios: Vec<String>,
    pub capabilities: Vec<String>,
    pub limitations: Vec<String>,
    pub workarounds: Vec<Workaround>,
    pub recommended_sequence: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct Workaround {
    pub description: String,
    pub commands: Vec<String>,
}

// =============================================================================
// Strategy data
// =============================================================================

static STRATEGIES: LazyLock<Vec<Strategy>> = LazyLock::new(build_strategies);

#[allow(clippy::too_many_lines)]
fn build_strategies() -> Vec<Strategy> {
    vec![
        // -------------------------------------------------------------------------
        // 1. iframes
        // -------------------------------------------------------------------------
        Strategy {
            name: "iframes".into(),
            title: "Working with iframes".into(),
            summary: "Target and interact with elements inside iframes and frames".into(),
            scenarios: vec![
                "A SCORM course is embedded in an iframe".into(),
                "A cross-origin payment widget is rendered as an iframe".into(),
                "Content is lazy-loaded into a frame after navigation".into(),
                "Nested iframes require multiple levels of frame targeting".into(),
            ],
            capabilities: vec![
                "agentchrome page frames — enumerate all frames by index with URL and dimensions"
                    .into(),
                "agentchrome page snapshot --frame N — accessibility tree of a specific frame"
                    .into(),
                "agentchrome interact --frame N click <uid> — click inside a frame".into(),
                "agentchrome dom --frame N select <selector> — query DOM inside a frame".into(),
                "agentchrome js --frame N exec <script> — execute JavaScript inside a frame".into(),
                "agentchrome form --frame N fill <uid> <value> — fill a form field in a frame"
                    .into(),
                "agentchrome page --frame N coords --selector <sel> — get element coordinates in a frame".into(),
            ],
            limitations: vec![
                "Cross-origin frames expose only URL and dimensions; interactive element counts are null".into(),
                "Directly piercing into nested frames requires a separate --frame call per level".into(),
                "Frame indices are not stable across page reloads — always run page frames first to get current indices".into(),
            ],
            workarounds: vec![
                Workaround {
                    description: "Read text from a cross-origin frame via js exec against the frame".into(),
                    commands: vec![
                        "agentchrome js --frame 1 exec \"document.title\"".into(),
                    ],
                },
                Workaround {
                    description: "Locate the correct frame index before targeting it".into(),
                    commands: vec![
                        "agentchrome page frames".into(),
                        "agentchrome page snapshot --frame 1".into(),
                    ],
                },
            ],
            recommended_sequence: vec![
                "agentchrome page frames".into(),
                "agentchrome page snapshot --frame 1".into(),
                "agentchrome interact --frame 1 click s3".into(),
            ],
        },
        // -------------------------------------------------------------------------
        // 2. overlays
        // -------------------------------------------------------------------------
        Strategy {
            name: "overlays".into(),
            title: "Handling overlays".into(),
            summary: "Detect, dismiss, and bypass full-viewport overlays and acc-blockers".into(),
            scenarios: vec![
                "A cookie-consent banner blocks interaction with page content".into(),
                "A modal dialog or overlay prevents clicking the underlying page".into(),
                "An accessibility blocker (acc-blocker) intercepts all mouse events".into(),
                "A GDPR consent wall appears before content loads".into(),
            ],
            capabilities: vec![
                "agentchrome diagnose --current — scan for overlays, acc-blockers, and modal patterns".into(),
                "agentchrome page analyze — inspect page structure for overlay elements".into(),
                "agentchrome page hittest X Y — confirm what element receives a click at given coordinates".into(),
                "agentchrome interact click <uid> --wait-until networkidle — click dismiss button and wait".into(),
                "agentchrome page snapshot — find the dismiss button UID in the overlay".into(),
                "agentchrome js exec <script> — programmatically remove or hide overlay elements".into(),
            ],
            limitations: vec![
                "Overlays that use iframes require --frame targeting to interact with their contents".into(),
                "Acc-blockers that intercept pointer events at the OS level cannot be bypassed via CDP mouse events alone".into(),
                "Some overlays re-appear after dismissal if their trigger condition persists".into(),
            ],
            workarounds: vec![
                Workaround {
                    description: "Dismiss a cookie-consent overlay via JavaScript when no clickable UID is found".into(),
                    commands: vec![
                        "agentchrome js exec \"document.querySelector('#cookie-banner').style.display='none'\"".into(),
                    ],
                },
                Workaround {
                    description: "Remove an acc-blocker overlay element directly from the DOM".into(),
                    commands: vec![
                        "agentchrome js exec \"document.querySelector('.acc-blocker')?.remove()\"".into(),
                    ],
                },
                Workaround {
                    description: "Verify the overlay is gone before proceeding".into(),
                    commands: vec![
                        "agentchrome page hittest 400 300".into(),
                    ],
                },
            ],
            recommended_sequence: vec![
                "agentchrome diagnose --current".into(),
                "agentchrome page snapshot".into(),
                "agentchrome interact click <overlay-dismiss-uid>".into(),
                "agentchrome page hittest 400 300".into(),
            ],
        },
        // -------------------------------------------------------------------------
        // 3. scorm
        // -------------------------------------------------------------------------
        Strategy {
            name: "scorm".into(),
            title: "Automating SCORM / LMS players".into(),
            summary: "Drive SCORM courses: iframes, media gates, navigation buttons".into(),
            scenarios: vec![
                "A SCORM course is embedded inside an LMS iframe and uses its own navigation".into(),
                "Narration audio must finish (or be skipped) before the Next button becomes active".into(),
                "A course has multiple nested frames: LMS shell > SCORM content > media player".into(),
                "The course uses a media gate — clicking Next is blocked until media completes".into(),
            ],
            capabilities: vec![
                "agentchrome page frames — identify the LMS iframe and its SCORM content frame".into(),
                "agentchrome page snapshot --frame N — find navigation buttons inside the SCORM frame".into(),
                "agentchrome media --frame N list — list audio/video elements in a frame".into(),
                "agentchrome media seek-end --all — seek all media elements to end (bypasses narration gates)".into(),
                "agentchrome interact --frame N click <uid> — click Next/Continue inside the SCORM frame".into(),
                "agentchrome interact --frame N click-at X Y — click at coordinates when UIDs are unavailable".into(),
            ],
            limitations: vec![
                "Cross-origin SCORM frames may restrict JavaScript access; use --frame CDP targeting instead of js exec".into(),
                "Some LMS players re-gate after seeking media — check if Next is still disabled after seek-end".into(),
                "Frame indices change if the LMS loads content into a new iframe; re-run page frames after each navigation".into(),
            ],
            workarounds: vec![
                Workaround {
                    description: "Bypass a narration gate by seeking all media to end before clicking Next".into(),
                    commands: vec![
                        "agentchrome media --frame 1 list".into(),
                        "agentchrome media seek-end --all".into(),
                        "agentchrome interact --frame 1 click <next-button-uid>".into(),
                    ],
                },
                Workaround {
                    description: "Find the correct frame index when the LMS uses nested frames".into(),
                    commands: vec![
                        "agentchrome page frames".into(),
                        "agentchrome page snapshot --frame 1".into(),
                        "agentchrome page snapshot --frame 2".into(),
                    ],
                },
            ],
            recommended_sequence: vec![
                "agentchrome page frames".into(),
                "agentchrome page snapshot --frame 1".into(),
                "agentchrome media --frame 1 list".into(),
                "agentchrome media seek-end --all".into(),
                "agentchrome interact --frame 1 click <next-button-uid>".into(),
            ],
        },
        // -------------------------------------------------------------------------
        // 4. drag-and-drop
        // -------------------------------------------------------------------------
        Strategy {
            name: "drag-and-drop".into(),
            title: "Drag-and-drop interactions".into(),
            summary: "Coordinate drags, decomposed mousedown/mouseup, step interpolation".into(),
            scenarios: vec![
                "A Kanban board requires dragging a card from one column to another".into(),
                "An HTML5 drag-and-drop zone needs a slow, interpolated drag to trigger drop events".into(),
                "A slider control requires dragging from one percentage point to another".into(),
                "A custom drag widget uses mousedown/mousemove/mouseup and does not use HTML5 DnD".into(),
            ],
            capabilities: vec![
                "agentchrome interact drag-at X1 Y1 X2 Y2 — drag from absolute coordinates to coordinates".into(),
                "agentchrome interact drag-at X1 Y1 X2 Y2 --steps N — slow drag with N movement steps (triggers HTML5 DnD events)".into(),
                "agentchrome interact mousedown-at X Y — press mouse button at coordinates without releasing".into(),
                "agentchrome interact mouseup-at X Y — release mouse button at coordinates".into(),
                "agentchrome interact click-at P% P% --relative-to <selector> — use element-relative percentage coordinates".into(),
                "agentchrome interact drag-at P1% P1% P2% P2% --relative-to <selector> — drag within an element using percentages".into(),
                "agentchrome page coords --selector <sel> — get element bounding box to calculate drag coordinates".into(),
            ],
            limitations: vec![
                "HTML5 drag-and-drop events (dragstart, dragover, drop) require --steps >= 5 to fire correctly in most browsers".into(),
                "Pointer-events: none CSS on a drop target prevents mouse events from landing; check with page hittest".into(),
                "Native OS drag-and-drop (e.g., file drag from desktop) is not supported via CDP".into(),
            ],
            workarounds: vec![
                Workaround {
                    description: "Use --steps for HTML5 drag-and-drop to ensure all drag events fire".into(),
                    commands: vec![
                        "agentchrome interact drag-at 100 200 300 400 --steps 10".into(),
                    ],
                },
                Workaround {
                    description: "Decompose drag into mousedown + mouseup for custom drag widgets".into(),
                    commands: vec![
                        "agentchrome interact mousedown-at 100 200".into(),
                        "agentchrome interact mouseup-at 300 400".into(),
                    ],
                },
                Workaround {
                    description: "Use percentage coordinates for slider elements to avoid hardcoding pixel values".into(),
                    commands: vec![
                        "agentchrome interact drag-at 10% 50% 90% 50% --relative-to css:#slider-track".into(),
                    ],
                },
            ],
            recommended_sequence: vec![
                "agentchrome page coords --selector css:#drag-source".into(),
                "agentchrome page hittest 100 200".into(),
                "agentchrome interact drag-at 100 200 300 400 --steps 10".into(),
            ],
        },
        // -------------------------------------------------------------------------
        // 5. shadow-dom
        // -------------------------------------------------------------------------
        Strategy {
            name: "shadow-dom".into(),
            title: "Piercing shadow DOM".into(),
            summary: "Target elements inside shadow roots with --pierce-shadow".into(),
            scenarios: vec![
                "A web component renders its UI inside a shadow root, hiding elements from normal CSS selectors".into(),
                "A design system uses shadow DOM for encapsulation; form inputs are inside shadow roots".into(),
                "An LMS course player is built with custom elements that use shadow DOM extensively".into(),
            ],
            capabilities: vec![
                "agentchrome page snapshot --pierce-shadow — include shadow DOM elements in the accessibility tree".into(),
                "agentchrome dom --pierce-shadow select <selector> — query elements inside shadow roots".into(),
                "agentchrome interact click <uid> — click elements found via pierce-shadow snapshot".into(),
                "agentchrome js exec <script> — traverse shadow roots manually via JavaScript".into(),
            ],
            limitations: vec![
                "CSS selectors do not cross shadow boundaries without --pierce-shadow".into(),
                "Deeply nested shadow roots (shadow inside shadow) may require multiple levels of piercing".into(),
                "Cross-origin shadow DOM is not accessible via CDP piercing".into(),
            ],
            workarounds: vec![
                Workaround {
                    description: "Use js exec to access a shadow root when --pierce-shadow is insufficient".into(),
                    commands: vec![
                        "agentchrome js exec \"document.querySelector('my-component').shadowRoot.querySelector('button').click()\"".into(),
                    ],
                },
                Workaround {
                    description: "Enumerate shadow host elements first, then pierce into each".into(),
                    commands: vec![
                        "agentchrome dom select \"css:my-component\"".into(),
                        "agentchrome page snapshot --pierce-shadow".into(),
                    ],
                },
            ],
            recommended_sequence: vec![
                "agentchrome page snapshot --pierce-shadow".into(),
                "agentchrome dom --pierce-shadow select \"css:my-component input\"".into(),
                "agentchrome interact click <uid>".into(),
            ],
        },
        // -------------------------------------------------------------------------
        // 6. spa-navigation-waits
        // -------------------------------------------------------------------------
        Strategy {
            name: "spa-navigation-waits".into(),
            title: "SPA navigation waits".into(),
            summary: "Wait for SPA/async rendering via --wait-until and polling".into(),
            // Motivation: issues #144, #145, #178 reported timing failures in SPAs
            scenarios: vec![
                "A React or Vue app performs client-side navigation without a full page reload".into(),
                "Clicking a button triggers an async data fetch before new content renders".into(),
                "A Next.js or Nuxt app uses route transitions that delay DOM updates".into(),
                "An infinite scroll container loads more items asynchronously on interaction".into(),
            ],
            capabilities: vec![
                "agentchrome navigate <url> --wait-until networkidle — wait for network to settle after navigation".into(),
                "agentchrome navigate <url> --wait-until selector:css:#content — wait for a specific element to appear".into(),
                "agentchrome interact click <uid> --wait-until networkidle — click and wait for network idle".into(),
                "agentchrome interact click <uid> --wait-until selector:css:.loaded — click and wait for element".into(),
                "agentchrome page find <text> — poll for visible text after async render".into(),
                "agentchrome js exec <script> — check framework-specific readiness flags".into(),
            ],
            limitations: vec![
                "networkidle waits up to the command timeout; very busy SPAs may never reach networkidle".into(),
                "selector wait requires the element to appear in the DOM; elements rendered inside shadow DOM or iframes need additional targeting".into(),
                "There is no built-in 'wait for React hydration complete' — use js exec to check React internals if needed".into(),
            ],
            workarounds: vec![
                Workaround {
                    description: "Check React or Vue readiness via js exec before interacting".into(),
                    commands: vec![
                        "agentchrome js exec \"window.__reactFiberNodeMap ? 'ready' : 'not ready'\"".into(),
                    ],
                },
                Workaround {
                    description: "Poll for a content element after clicking a navigation link".into(),
                    commands: vec![
                        "agentchrome interact click <nav-link-uid> --wait-until networkidle".into(),
                        "agentchrome page find \"Expected Page Title\"".into(),
                    ],
                },
            ],
            recommended_sequence: vec![
                "agentchrome navigate https://app.example.com --wait-until networkidle".into(),
                "agentchrome interact click <nav-link-uid> --wait-until networkidle".into(),
                "agentchrome page snapshot".into(),
            ],
        },
        // -------------------------------------------------------------------------
        // 7. react-controlled-inputs
        // -------------------------------------------------------------------------
        Strategy {
            name: "react-controlled-inputs".into(),
            title: "Filling React / controlled inputs".into(),
            summary: "When form fill works vs needing js exec for controlled fields".into(),
            scenarios: vec![
                "A React form field uses controlled input with onChange and the value does not update after fill".into(),
                "A Vue v-model field ignores native input events dispatched by form fill".into(),
                "An ARIA combobox requires typing then pressing Enter or Tab to commit the selection".into(),
                "A custom input component wraps a hidden native input; the visible element needs js exec".into(),
            ],
            capabilities: vec![
                "agentchrome form fill <uid> <value> — fills a field and dispatches native input/change events (works for most standard inputs)".into(),
                "agentchrome form fill --confirm-key Tab <uid> <value> — fill and press Tab to confirm (for ARIA comboboxes)".into(),
                "agentchrome form fill --confirm-key Enter <uid> <value> — fill and press Enter to confirm".into(),
                "agentchrome js exec <script> — set React-controlled input value via the React internal setter".into(),
                "agentchrome dom events css:#my-input — inspect event listeners to understand which events the field needs".into(),
            ],
            limitations: vec![
                "React controlled inputs with synthetic event systems may require using the React value setter instead of native input events".into(),
                "Some design-system components (MUI, Ant Design) have complex event handling that native fill cannot replicate".into(),
                "form fill does not support multi-step ARIA combobox flows that require selecting from a dropdown after typing".into(),
            ],
            workarounds: vec![
                Workaround {
                    description: "Set a React controlled input value using the React internal property descriptor".into(),
                    commands: vec![
                        "agentchrome js exec --uid s5 \"(el) => { const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; setter.call(el, 'my value'); el.dispatchEvent(new Event('input', {bubbles: true})); el.dispatchEvent(new Event('change', {bubbles: true})); }\"".into(),
                    ],
                },
                Workaround {
                    description: "Fill an ARIA combobox and confirm with a key press".into(),
                    commands: vec![
                        "agentchrome form fill --confirm-key Tab s5 \"Acme Corp\"".into(),
                    ],
                },
                Workaround {
                    description: "Inspect event listeners first to determine the correct fill strategy".into(),
                    commands: vec![
                        "agentchrome dom events css:#my-input".into(),
                    ],
                },
            ],
            recommended_sequence: vec![
                "agentchrome page snapshot".into(),
                "agentchrome form fill <uid> <value>".into(),
                "agentchrome page snapshot".into(),
            ],
        },
        // -------------------------------------------------------------------------
        // 8. debugging-failed-interactions
        // -------------------------------------------------------------------------
        Strategy {
            name: "debugging-failed-interactions".into(),
            title: "Debugging failed interactions".into(),
            summary: "Meta-workflow: diagnose -> hittest -> coords -> console -> network".into(),
            scenarios: vec![
                "A click command reports success but the UI does not change".into(),
                "An interaction times out or fails with a generic error".into(),
                "A form fill appears to work but the field value is not accepted by the application".into(),
                "An element is found in the snapshot but clicking it has no effect".into(),
            ],
            capabilities: vec![
                "agentchrome diagnose --current — scan the current page for overlays, acc-blockers, and automation challenges".into(),
                "agentchrome page hittest X Y — verify what element actually receives clicks at specific coordinates".into(),
                "agentchrome page coords --selector <sel> — get the element's bounding box to derive correct click coordinates".into(),
                "agentchrome console read --errors-only — check for JavaScript errors after a failed interaction".into(),
                "agentchrome network list --type xhr,fetch — inspect API calls triggered (or not triggered) by the interaction".into(),
                "agentchrome page snapshot — re-examine the accessibility tree to see post-interaction state".into(),
                "agentchrome page analyze — detect structural issues like hidden elements or zero-size containers".into(),
            ],
            limitations: vec![
                "hittest reports the topmost element at coordinates; overlapping transparent elements may intercept clicks".into(),
                "console read only shows messages buffered since the last page load or console clear".into(),
            ],
            workarounds: vec![
                Workaround {
                    description: "Confirm the element is truly clickable at its reported coordinates".into(),
                    commands: vec![
                        "agentchrome page coords --selector css:#my-button".into(),
                        "agentchrome page hittest 400 300".into(),
                    ],
                },
                Workaround {
                    description: "Check for JavaScript errors that may indicate why the interaction failed".into(),
                    commands: vec![
                        "agentchrome console read --errors-only".into(),
                        "agentchrome network list --type xhr,fetch".into(),
                    ],
                },
            ],
            recommended_sequence: vec![
                "agentchrome diagnose --current".into(),
                "agentchrome page hittest 400 300".into(),
                "agentchrome page coords --selector css:#target-element".into(),
                "agentchrome console read --errors-only".into(),
                "agentchrome network list --type xhr,fetch".into(),
                "agentchrome page snapshot".into(),
            ],
        },
        // -------------------------------------------------------------------------
        // 9. authentication-cookie-reuse
        // -------------------------------------------------------------------------
        Strategy {
            name: "authentication-cookie-reuse".into(),
            title: "Reusing authentication via cookies".into(),
            summary: "Persist and replay session cookies across agentchrome invocations".into(),
            scenarios: vec![
                "An LMS requires login before accessing course content; re-logging in on every run is slow".into(),
                "A session cookie obtained after OAuth login should be reused in subsequent automation runs".into(),
                "Multiple agentchrome scripts need to share the same authenticated session".into(),
            ],
            capabilities: vec![
                "agentchrome cookie list — list all cookies for the current page".into(),
                "agentchrome cookie list --json — export cookies as JSON for persistence".into(),
                "agentchrome cookie set --name <n> --value <v> --domain <d> — set a cookie programmatically".into(),
                "agentchrome cookie delete --name <n> --domain <d> — remove a specific cookie".into(),
                "agentchrome cookie clear — remove all cookies for the current context".into(),
            ],
            limitations: vec![
                "HttpOnly cookies cannot be read by JavaScript but ARE accessible via agentchrome cookie list (CDP bypasses the HttpOnly restriction)".into(),
                "Session cookies expire when the browser closes unless the server sets an explicit expiry; plan for re-login fallback".into(),
                "Cross-origin cookies with SameSite=Strict cannot be injected across domains".into(),
            ],
            workarounds: vec![
                Workaround {
                    description: "Export session cookies after manual login and replay them in subsequent runs".into(),
                    commands: vec![
                        "agentchrome cookie list --json".into(),
                        "agentchrome cookie set --name session_id --value <value> --domain app.example.com".into(),
                    ],
                },
                Workaround {
                    description: "Clear stale cookies and start a fresh session when re-login is needed".into(),
                    commands: vec![
                        "agentchrome cookie clear".into(),
                        "agentchrome navigate https://app.example.com/login".into(),
                    ],
                },
            ],
            recommended_sequence: vec![
                "agentchrome navigate https://app.example.com/login".into(),
                "agentchrome cookie list --json".into(),
                "agentchrome cookie set --name session_id --value <value> --domain app.example.com".into(),
                "agentchrome navigate https://app.example.com/protected-page".into(),
            ],
        },
        // -------------------------------------------------------------------------
        // 10. multi-tab-workflows
        // -------------------------------------------------------------------------
        Strategy {
            name: "multi-tab-workflows".into(),
            title: "Multi-tab workflows".into(),
            summary: "Handle SSO-style new-tab flows and coordinate across tabs".into(),
            scenarios: vec![
                "An SSO login opens a new tab for the identity provider; the script must switch to that tab".into(),
                "A 'Open in new tab' link creates a new tab that the script must interact with".into(),
                "Multiple tabs are open and the script needs to target a specific tab by ID".into(),
                "A workflow requires operating on two tabs alternately (e.g., copy data from one, paste into another)".into(),
            ],
            capabilities: vec![
                "agentchrome tabs list — list all open tabs with their IDs, titles, and URLs".into(),
                "agentchrome tabs create <url> — open a new tab and get its ID".into(),
                "agentchrome tabs activate <tab-id> — switch focus to a specific tab".into(),
                "agentchrome tabs close <tab-id> — close a specific tab".into(),
                "agentchrome --tab <tab-id> <command> — target any command at a specific tab without activating it".into(),
            ],
            limitations: vec![
                "agentchrome commands operate on the active tab by default; use --tab to target inactive tabs explicitly".into(),
                "New tabs opened by window.open() or target=_blank links may take a moment to appear in tabs list".into(),
                "If a tab is closed by the page itself (e.g., after OAuth redirect), its ID becomes invalid".into(),
            ],
            workarounds: vec![
                Workaround {
                    description: "Detect a new tab opened by a click by comparing tabs list before and after".into(),
                    commands: vec![
                        "agentchrome tabs list".into(),
                        "agentchrome interact click <open-in-new-tab-uid>".into(),
                        "agentchrome tabs list".into(),
                        "agentchrome tabs activate <new-tab-id>".into(),
                    ],
                },
                Workaround {
                    description: "Operate on a background tab without switching focus".into(),
                    commands: vec![
                        "agentchrome --tab <tab-id> page snapshot".into(),
                    ],
                },
            ],
            recommended_sequence: vec![
                "agentchrome tabs list".into(),
                "agentchrome interact click <link-uid>".into(),
                "agentchrome tabs list".into(),
                "agentchrome tabs activate <new-tab-id>".into(),
                "agentchrome page snapshot".into(),
            ],
        },
    ]
}

// =============================================================================
// Helper functions
// =============================================================================

/// Cheap listing: borrow from the cached `STRATEGIES` and project to summary form.
pub fn strategy_summaries() -> Vec<StrategySummary> {
    STRATEGIES
        .iter()
        .map(|s| StrategySummary {
            name: s.name.clone(),
            title: s.title.clone(),
            summary: s.summary.clone(),
        })
        .collect()
}

/// Detail lookup — linear scan by exact name, returning a borrow of the cached entry.
pub fn find_strategy(name: &str) -> Option<&'static Strategy> {
    STRATEGIES.iter().find(|s| s.name == name)
}

/// Plain-text listing: one line per strategy, `<name> — <summary>`.
pub(super) fn format_plain_strategy_list(summaries: &[StrategySummary]) -> String {
    let mut out = String::new();
    for summary in summaries {
        super::write_em_dash_line(&mut out, &summary.name, &summary.summary);
    }
    out
}

/// Plain-text detail: sectioned guide.
pub(super) fn format_plain_strategy_detail(strategy: &Strategy) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    super::write_em_dash_line(&mut out, &strategy.name, &strategy.title);

    out.push('\n');
    let _ = writeln!(out, "SCENARIOS");
    for s in &strategy.scenarios {
        let _ = writeln!(out, "  - {s}");
    }

    out.push('\n');
    let _ = writeln!(out, "CURRENT CAPABILITIES");
    for c in &strategy.capabilities {
        let _ = writeln!(out, "  {c}");
    }

    out.push('\n');
    let _ = writeln!(out, "LIMITATIONS");
    for l in &strategy.limitations {
        let _ = writeln!(out, "  - {l}");
    }

    out.push('\n');
    let _ = writeln!(out, "WORKAROUNDS");
    for w in &strategy.workarounds {
        let _ = writeln!(out, "  # {}", w.description);
        for cmd in &w.commands {
            let _ = writeln!(out, "  {cmd}");
        }
        out.push('\n');
    }

    let _ = writeln!(out, "RECOMMENDED SEQUENCE");
    for (i, cmd) in strategy.recommended_sequence.iter().enumerate() {
        let _ = writeln!(out, "  {}. {cmd}", i + 1);
    }

    out
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // T011: Strategy data integrity tests
    // -------------------------------------------------------------------------

    #[test]
    fn all_strategies_returns_ten_required_guides() {
        let strategies = &*STRATEGIES;
        let names: Vec<&str> = strategies.iter().map(|s| s.name.as_str()).collect();
        let required = [
            "iframes",
            "overlays",
            "scorm",
            "drag-and-drop",
            "shadow-dom",
            "spa-navigation-waits",
            "react-controlled-inputs",
            "debugging-failed-interactions",
            "authentication-cookie-reuse",
            "multi-tab-workflows",
        ];
        for expected in &required {
            assert!(
                names.contains(expected),
                "Missing required strategy: '{expected}'\nPresent: {names:?}"
            );
        }
        assert_eq!(
            strategies.len(),
            10,
            "Expected exactly 10 strategies, got {}",
            strategies.len()
        );
    }

    #[test]
    fn no_duplicate_strategy_names() {
        let strategies = &*STRATEGIES;
        let mut seen = std::collections::HashSet::new();
        for s in strategies {
            assert!(
                seen.insert(s.name.as_str()),
                "Duplicate strategy name: '{}'",
                s.name
            );
        }
    }

    #[test]
    fn every_strategy_has_non_empty_fields() {
        for strategy in STRATEGIES.iter() {
            assert!(
                !strategy.title.is_empty(),
                "Strategy '{}' has empty title",
                strategy.name
            );
            assert!(
                !strategy.summary.is_empty(),
                "Strategy '{}' has empty summary",
                strategy.name
            );
            assert!(
                strategy.scenarios.len() >= 2,
                "Strategy '{}' has fewer than 2 scenarios",
                strategy.name
            );
            assert!(
                strategy.capabilities.len() >= 3,
                "Strategy '{}' has fewer than 3 capabilities",
                strategy.name
            );
            assert!(
                !strategy.limitations.is_empty(),
                "Strategy '{}' has empty limitations",
                strategy.name
            );
            assert!(
                strategy.recommended_sequence.len() >= 3,
                "Strategy '{}' has fewer than 3 recommended_sequence commands",
                strategy.name
            );
        }
    }

    #[test]
    fn recommended_sequences_start_with_agentchrome() {
        for strategy in STRATEGIES.iter() {
            for cmd in &strategy.recommended_sequence {
                assert!(
                    cmd.starts_with("agentchrome"),
                    "Strategy '{}' recommended_sequence command does not start with 'agentchrome': '{cmd}'",
                    strategy.name
                );
            }
            for workaround in &strategy.workarounds {
                for cmd in &workaround.commands {
                    assert!(
                        cmd.starts_with("agentchrome"),
                        "Strategy '{}' workaround command does not start with 'agentchrome': '{cmd}'",
                        strategy.name
                    );
                }
            }
        }
    }

    #[test]
    fn strategy_names_are_kebab_case() {
        for strategy in STRATEGIES.iter() {
            let name = &strategy.name;
            // Kebab-case: lowercase letters and digits only, separated by hyphens
            let is_kebab = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                && !name.starts_with('-')
                && !name.ends_with('-')
                && !name.contains("--");
            assert!(is_kebab, "Strategy name '{name}' is not kebab-case");
        }
    }

    #[test]
    fn strategy_name_does_not_collide_with_command_groups() {
        use super::super::commands::all_examples;
        let command_names: Vec<String> = all_examples().into_iter().map(|g| g.command).collect();
        assert!(
            !command_names.iter().any(|n| n == "strategies"),
            "'strategies' must not be a command group name, but it was found in all_examples()"
        );
    }

    // -------------------------------------------------------------------------
    // T012: Progressive disclosure contract tests
    // -------------------------------------------------------------------------

    #[test]
    fn summary_json_has_only_three_fields() {
        let summaries = strategy_summaries();
        let json = serde_json::to_string(&summaries).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let arr = parsed.as_array().unwrap();
        let detail_fields = [
            "scenarios",
            "capabilities",
            "limitations",
            "workarounds",
            "recommended_sequence",
        ];
        for (i, entry) in arr.iter().enumerate() {
            assert!(
                entry.get("name").is_some(),
                "Entry {i} missing 'name' field"
            );
            assert!(
                entry.get("title").is_some(),
                "Entry {i} missing 'title' field"
            );
            assert!(
                entry.get("summary").is_some(),
                "Entry {i} missing 'summary' field"
            );
            for field in &detail_fields {
                assert!(
                    entry.get(*field).is_none(),
                    "Entry {i} should NOT have '{field}' field in summary listing"
                );
            }
        }
    }

    #[test]
    fn detail_json_has_all_fields() {
        let strategy = find_strategy("iframes").expect("iframes strategy must exist");
        let json = serde_json::to_string(&strategy).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let required_fields = [
            "name",
            "title",
            "summary",
            "scenarios",
            "capabilities",
            "limitations",
            "workarounds",
            "recommended_sequence",
        ];
        for field in &required_fields {
            assert!(
                parsed.get(*field).is_some(),
                "Detail JSON missing '{field}' field"
            );
        }
    }

    #[test]
    fn summary_listing_under_4kb() {
        let json = serde_json::to_string(&strategy_summaries()).unwrap();
        assert!(
            json.len() < 4096,
            "Summary JSON listing is {} bytes, expected < 4096",
            json.len()
        );
    }

    #[test]
    fn plain_listing_under_1kb() {
        let output = format_plain_strategy_list(&strategy_summaries());
        assert!(
            output.len() < 1024,
            "Plain strategy listing is {} bytes, expected < 1024",
            output.len()
        );
    }

    // -------------------------------------------------------------------------
    // T013: Plain-text formatting tests
    // -------------------------------------------------------------------------

    #[test]
    fn plain_list_contains_all_strategy_names() {
        let summaries = strategy_summaries();
        let output = format_plain_strategy_list(&summaries);
        for summary in &summaries {
            assert!(
                output.contains(&summary.name),
                "Plain listing missing strategy name '{}'\noutput: {output}",
                summary.name
            );
        }
    }

    #[test]
    fn plain_list_does_not_start_with_bracket_or_brace() {
        let output = format_plain_strategy_list(&strategy_summaries());
        assert!(
            !output.starts_with('['),
            "Plain listing should not start with '['"
        );
        assert!(
            !output.starts_with('{'),
            "Plain listing should not start with '{{'"
        );
    }

    #[test]
    fn plain_detail_contains_required_section_headers() {
        let required_headers = [
            "SCENARIOS",
            "CURRENT CAPABILITIES",
            "LIMITATIONS",
            "WORKAROUNDS",
            "RECOMMENDED SEQUENCE",
        ];
        for strategy in STRATEGIES.iter() {
            let output = format_plain_strategy_detail(strategy);
            for header in &required_headers {
                assert!(
                    output.contains(header),
                    "Strategy '{}' detail missing header '{header}'\noutput: {output}",
                    strategy.name
                );
            }
        }
    }

    #[test]
    fn plain_detail_contains_every_recommended_sequence_command() {
        for strategy in STRATEGIES.iter() {
            let output = format_plain_strategy_detail(strategy);
            for cmd in &strategy.recommended_sequence {
                assert!(
                    output.contains(cmd.as_str()),
                    "Strategy '{}' detail missing recommended_sequence command: '{cmd}'\noutput: {output}",
                    strategy.name
                );
            }
        }
    }
}
