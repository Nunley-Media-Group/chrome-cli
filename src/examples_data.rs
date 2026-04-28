use serde::Serialize;

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
pub struct CommandGroupSummary {
    pub command: String,
    pub description: String,
    pub examples: Vec<ExampleEntry>,
}

#[derive(Serialize, Clone)]
pub struct CommandGroupListing {
    pub command: String,
    pub description: String,
}

impl From<&CommandGroupSummary> for CommandGroupListing {
    fn from(s: &CommandGroupSummary) -> Self {
        Self {
            command: s.command.clone(),
            description: s.description.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct ExampleEntry {
    pub cmd: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Vec<String>>,
}

// =============================================================================
// Static example data
// =============================================================================

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn all_examples() -> Vec<CommandGroupSummary> {
    vec![
        CommandGroupSummary {
            command: "connect".into(),
            description: "Connect to or launch a Chrome instance".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome connect".into(),
                    description: "Connect to Chrome on the default port (9222)".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome connect --launch --headless".into(),
                    description: "Launch a new headless Chrome instance".into(),
                    flags: Some(vec!["--launch".into(), "--headless".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome connect --port 9333".into(),
                    description: "Connect to Chrome on a specific port".into(),
                    flags: Some(vec!["--port".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome connect --status".into(),
                    description: "Check current connection status".into(),
                    flags: Some(vec!["--status".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome connect --disconnect".into(),
                    description: "Disconnect and remove the session file".into(),
                    flags: Some(vec!["--disconnect".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "tabs".into(),
            description: "Tab management (list, create, close, activate)".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome tabs list".into(),
                    description: "List all open tabs".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome tabs create https://example.com".into(),
                    description: "Open a new tab with a URL".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome tabs close ABC123".into(),
                    description: "Close a tab by its ID".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome tabs activate ABC123".into(),
                    description: "Activate (focus) a tab by its ID".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome tabs list --all".into(),
                    description: "List all tabs including internal Chrome pages".into(),
                    flags: Some(vec!["--all".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "navigate".into(),
            description: "URL navigation and history".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome navigate https://example.com".into(),
                    description: "Navigate to a URL and wait for load".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome navigate https://app.example.com --wait-until networkidle"
                        .into(),
                    description: "Navigate and wait for network idle (for SPAs)".into(),
                    flags: Some(vec!["--wait-until".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome navigate back".into(),
                    description: "Go back in browser history".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome navigate reload --ignore-cache".into(),
                    description: "Reload the page without cache".into(),
                    flags: Some(vec!["--ignore-cache".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "page".into(),
            description: "Page inspection (screenshot, text, accessibility tree, find)".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome page text".into(),
                    description: "Extract all visible text from the page".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome page snapshot".into(),
                    description: "Capture the accessibility tree with element UIDs".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome page snapshot --compact".into(),
                    description: "Compact snapshot with only interactive and landmark elements"
                        .into(),
                    flags: Some(vec!["--compact".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome page screenshot --full-page --file page.png".into(),
                    description: "Take a full-page screenshot".into(),
                    flags: Some(vec!["--full-page".into(), "--file".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome page find \"Sign in\"".into(),
                    description: "Find elements by text".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome page resize 1280x720".into(),
                    description: "Resize the viewport to specific dimensions".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome page frames".into(),
                    description: "List all iframes and frames in the page hierarchy".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome page --frame 1 snapshot".into(),
                    description: "Capture accessibility tree of a specific iframe".into(),
                    flags: Some(vec!["--frame".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome page snapshot --pierce-shadow".into(),
                    description: "Include shadow DOM elements in the accessibility tree".into(),
                    flags: Some(vec!["--pierce-shadow".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome page workers".into(),
                    description: "List service workers, shared workers, and web workers".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome page hittest 100 200".into(),
                    description: "Hit test at viewport coordinates to identify click targets"
                        .into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome page --frame 1 hittest 50 50".into(),
                    description: "Hit test within a specific iframe".into(),
                    flags: Some(vec!["--frame".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome page coords --selector css:#submit".into(),
                    description: "Get frame-local and page-global bounding box for a CSS selector"
                        .into(),
                    flags: Some(vec!["--selector".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome page coords --selector s7".into(),
                    description: "Get bounding box for a snapshot UID".into(),
                    flags: Some(vec!["--selector".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome page --frame 1 coords --selector css:#inner".into(),
                    description: "Get bounding box for an element inside an iframe, reporting both frame-local and page-global coordinates".into(),
                    flags: Some(vec!["--frame".into(), "--selector".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome page analyze".into(),
                    description: "Analyze page structure: iframes, frameworks, overlays, media"
                        .into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome page --frame 1 analyze".into(),
                    description: "Analyze structure within a specific iframe".into(),
                    flags: Some(vec!["--frame".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "markdown".into(),
            description: "Convert browser pages or raw HTML into cleaned Markdown".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome markdown".into(),
                    description: "Convert the current browser page to cleaned Markdown JSON"
                        .into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome markdown --plain".into(),
                    description: "Emit only the Markdown body for the current browser page".into(),
                    flags: Some(vec!["--plain".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome markdown --file article.html --base-url https://example.com/docs/".into(),
                    description: "Convert a local HTML file and resolve relative links".into(),
                    flags: Some(vec!["--file".into(), "--base-url".into()]),
                },
                ExampleEntry {
                    cmd: "cat article.html | agentchrome markdown --stdin --base-url https://example.com/".into(),
                    description: "Convert raw HTML from stdin".into(),
                    flags: Some(vec!["--stdin".into(), "--base-url".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome markdown --url https://example.com/article".into(),
                    description: "Fetch an HTTP/HTTPS URL and convert the response HTML".into(),
                    flags: Some(vec!["--url".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome markdown --file article.html --selector main".into(),
                    description: "Scope conversion to a CSS selector".into(),
                    flags: Some(vec!["--file".into(), "--selector".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome markdown --file article.html --strip-links".into(),
                    description: "Keep link text while removing link destinations".into(),
                    flags: Some(vec!["--file".into(), "--strip-links".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome markdown --file article.html --include-images".into(),
                    description: "Preserve useful images as Markdown image references".into(),
                    flags: Some(vec!["--file".into(), "--include-images".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "diagnose".into(),
            description: "Pre-automation challenge scan (iframes, overlays, media gates, frameworks, patterns)".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome diagnose https://example.com/course".into(),
                    description: "Navigate to a URL and diagnose it for automation challenges".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome diagnose --current".into(),
                    description: "Diagnose the already-loaded page without navigating".into(),
                    flags: Some(vec!["--current".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome diagnose --current | jq -r '.patterns[].suggestion'".into(),
                    description: "Extract all pattern strategy suggestions via jq".into(),
                    flags: Some(vec!["--current".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome diagnose https://app.example.com --wait-until networkidle".into(),
                    description: "Diagnose after waiting for network idle (for SPAs)".into(),
                    flags: Some(vec!["--wait-until".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome diagnose --current | jq '.summary'".into(),
                    description: "Check if the page is straightforward to automate".into(),
                    flags: Some(vec!["--current".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "dom".into(),
            description: "DOM inspection and manipulation".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome dom select \"h1\"".into(),
                    description: "Select elements by CSS selector".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome dom select \"//a[@href]\" --xpath".into(),
                    description: "Select elements by XPath expression".into(),
                    flags: Some(vec!["--xpath".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome dom get-attribute s3 href".into(),
                    description: "Get an element's attribute by UID".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome dom get-text css:h1".into(),
                    description: "Get the text content of an element".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome dom tree --depth 3".into(),
                    description: "View the DOM tree with limited depth".into(),
                    flags: Some(vec!["--depth".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome dom tree css:table --depth 3".into(),
                    description: "View the DOM subtree rooted at an element (positional)".into(),
                    flags: Some(vec!["--depth".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome dom --frame 1 select \"css:button\"".into(),
                    description: "Query elements inside an iframe".into(),
                    flags: Some(vec!["--frame".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome dom --pierce-shadow select \"css:#shadow-btn\"".into(),
                    description: "Query elements inside shadow DOM".into(),
                    flags: Some(vec!["--pierce-shadow".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome dom events css:button".into(),
                    description: "List event listeners on an element".into(),
                    flags: None,
                },
            ],
        },
        CommandGroupSummary {
            command: "js".into(),
            description: "JavaScript execution in page context".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome js exec \"document.title\"".into(),
                    description: "Get the page title".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome js exec --file script.js".into(),
                    description: "Execute a JavaScript file".into(),
                    flags: Some(vec!["--file".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome js exec --uid s3 \"(el) => el.textContent\"".into(),
                    description: "Run code on a specific element by UID".into(),
                    flags: Some(vec!["--uid".into()]),
                },
                ExampleEntry {
                    cmd: "echo 'document.URL' | agentchrome js exec -".into(),
                    description: "Read JavaScript from stdin".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome js --frame 1 exec \"document.title\"".into(),
                    description: "Execute JavaScript inside an iframe".into(),
                    flags: Some(vec!["--frame".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome js exec --worker 0 \"self.registration.scope\"".into(),
                    description: "Execute JavaScript in a Service Worker".into(),
                    flags: Some(vec!["--worker".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "console".into(),
            description: "Console message reading and monitoring".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome console read".into(),
                    description: "Read recent console messages".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome console read --errors-only".into(),
                    description: "Show only error messages".into(),
                    flags: Some(vec!["--errors-only".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome console follow".into(),
                    description: "Stream console messages in real time".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome console follow --errors-only --timeout 10000".into(),
                    description: "Stream errors for 10 seconds".into(),
                    flags: Some(vec!["--errors-only".into(), "--timeout".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome console follow --timeout 10000 --fail-on-error".into(),
                    description: "CI assertion: exit 1 if any console.error is seen within 10s"
                        .into(),
                    flags: Some(vec!["--timeout".into(), "--fail-on-error".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "network".into(),
            description: "Network request monitoring and interception".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome network list".into(),
                    description: "List recent network requests".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome network list --type xhr,fetch".into(),
                    description: "Filter requests by resource type".into(),
                    flags: Some(vec!["--type".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome network get 42".into(),
                    description: "Get details of a specific request by ID".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome network follow --url api.example.com".into(),
                    description: "Stream network requests matching a URL pattern".into(),
                    flags: Some(vec!["--url".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome network list --frame 1".into(),
                    description: "List network requests from a specific frame".into(),
                    flags: Some(vec!["--frame".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "interact".into(),
            description: "Mouse, keyboard, and scroll interactions".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome interact click s5".into(),
                    description: "Click an element by UID".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome interact click css:#submit-btn".into(),
                    description: "Click an element by CSS selector".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome interact click s12 --wait-until networkidle".into(),
                    description: "Click and wait for network idle (for SPA navigation)".into(),
                    flags: Some(vec!["--wait-until".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome interact type \"Hello, world!\"".into(),
                    description: "Type text into the focused element".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome interact key Control+A".into(),
                    description: "Press a key combination".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome interact scroll --to-bottom".into(),
                    description: "Scroll to the bottom of the page".into(),
                    flags: Some(vec!["--to-bottom".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome interact --frame 1 click s3".into(),
                    description: "Click an element inside an iframe".into(),
                    flags: Some(vec!["--frame".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome interact --frame 1 click-at 100 200".into(),
                    description: "Click at coordinates inside an iframe".into(),
                    flags: Some(vec!["--frame".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome interact drag-at 100 200 300 400".into(),
                    description: "Drag from coordinates to coordinates".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome interact drag-at 0 0 500 500 --steps 10".into(),
                    description: "Drag with interpolated movement steps".into(),
                    flags: Some(vec!["--steps".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome interact mousedown-at 100 200".into(),
                    description: "Press mouse button at coordinates (no release)".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome interact mouseup-at 300 400".into(),
                    description: "Release mouse button at coordinates".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome interact click-at 50% 50% --relative-to css:#submit".into(),
                    description: "Click the center of an element using percentage coordinates"
                        .into(),
                    flags: Some(vec!["--relative-to".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome interact click-at 0% 0% --relative-to css:#submit".into(),
                    description: "Click the top-left corner of an element".into(),
                    flags: Some(vec!["--relative-to".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome interact click-at 100% 100% --relative-to s7".into(),
                    description: "Click the bottom-right pixel of an element by UID".into(),
                    flags: Some(vec!["--relative-to".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome interact drag-at 10% 50% 90% 50% --relative-to css:#track"
                        .into(),
                    description: "Drag a slider from 10% to 90% across an element".into(),
                    flags: Some(vec!["--relative-to".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "form".into(),
            description: "Form input and submission".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome form fill s5 \"hello@example.com\"".into(),
                    description: "Fill a form field by UID".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome form fill css:#email \"user@example.com\"".into(),
                    description: "Fill a form field by CSS selector".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome form clear s5".into(),
                    description: "Clear a form field".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome form upload s10 ./photo.jpg".into(),
                    description: "Upload a file to a file input element".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome form fill s5 \"Acme Corp\"".into(),
                    description: "Fill an ARIA combobox field (auto click-type-confirm)".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome form fill --confirm-key Tab s5 \"Acme Corp\"".into(),
                    description: "Fill combobox with custom confirmation key".into(),
                    flags: Some(vec!["--confirm-key".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome form --frame 1 fill s2 \"value\"".into(),
                    description: "Fill a form field inside an iframe".into(),
                    flags: Some(vec!["--frame".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "emulate".into(),
            description: "Device and network emulation".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome emulate set --viewport 375x667 --device-scale 2 --mobile"
                        .into(),
                    description: "Emulate a mobile device".into(),
                    flags: Some(vec![
                        "--viewport".into(),
                        "--device-scale".into(),
                        "--mobile".into(),
                    ]),
                },
                ExampleEntry {
                    cmd: "agentchrome emulate set --network 3g".into(),
                    description: "Simulate slow 3G network".into(),
                    flags: Some(vec!["--network".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome emulate set --color-scheme dark".into(),
                    description: "Force dark mode".into(),
                    flags: Some(vec!["--color-scheme".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome emulate status".into(),
                    description: "Check current emulation settings".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome emulate reset".into(),
                    description: "Clear all emulation overrides".into(),
                    flags: None,
                },
            ],
        },
        CommandGroupSummary {
            command: "perf".into(),
            description: "Performance tracing and metrics".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome perf vitals".into(),
                    description: "Quick Core Web Vitals measurement".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome perf record --duration 5000".into(),
                    description: "Record a trace for 5 seconds".into(),
                    flags: Some(vec!["--duration".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome perf record --reload --duration 5000".into(),
                    description: "Record a trace with page reload".into(),
                    flags: Some(vec!["--reload".into(), "--duration".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome perf analyze RenderBlocking --trace-file trace.json".into(),
                    description: "Analyze render-blocking resources from a trace".into(),
                    flags: Some(vec!["--trace-file".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "dialog".into(),
            description: "Browser dialog handling (alert, confirm, prompt, beforeunload)".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome dialog info".into(),
                    description: "Check if a dialog is currently open".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome dialog handle accept".into(),
                    description: "Accept an alert or confirm dialog".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome dialog handle dismiss".into(),
                    description: "Dismiss a dialog".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome dialog handle accept --text \"my input\"".into(),
                    description: "Accept a prompt dialog with text".into(),
                    flags: Some(vec!["--text".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "skill".into(),
            description: "Agentic tool skill installation and management".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome skill install".into(),
                    description: "Auto-detect agentic tool and install skill".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome skill install --tool cursor".into(),
                    description: "Install skill for a specific tool".into(),
                    flags: Some(vec!["--tool".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome skill install --tool codex".into(),
                    description: "Install the AgentChrome skill for Codex".into(),
                    flags: Some(vec!["--tool".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome skill list".into(),
                    description: "List supported tools and installation status".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome skill update --tool claude-code".into(),
                    description: "Update installed skill to current version".into(),
                    flags: Some(vec!["--tool".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome skill uninstall --tool aider".into(),
                    description: "Remove an installed skill".into(),
                    flags: Some(vec!["--tool".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "media".into(),
            description: "Media element control (list, play, pause, seek audio/video)".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome media list".into(),
                    description: "List all audio and video elements on the page".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome media play 0".into(),
                    description: "Play the first media element".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome media pause 0".into(),
                    description: "Pause the first media element".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome media seek 0 15.5".into(),
                    description: "Seek a media element to 15.5 seconds".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome media seek-end --all".into(),
                    description: "Seek all media elements to end (skip narration gates)".into(),
                    flags: Some(vec!["--all".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome media --frame 0 list".into(),
                    description: "List media elements inside a specific iframe".into(),
                    flags: Some(vec!["--frame".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome media play css:audio.narration".into(),
                    description: "Play a media element by CSS selector".into(),
                    flags: None,
                },
            ],
        },
        CommandGroupSummary {
            command: "config".into(),
            description: "Configuration file management (show, init, path)".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome config show".into(),
                    description: "Show the resolved configuration from all sources".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome config init".into(),
                    description: "Create a default config file".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome config init --path ./my-config.toml".into(),
                    description: "Create a config at a custom path".into(),
                    flags: Some(vec!["--path".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome config path".into(),
                    description: "Show the active config file path".into(),
                    flags: None,
                },
            ],
        },
        CommandGroupSummary {
            command: "script".into(),
            description: "Execute a batch script of agentchrome commands from a JSON file".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome script run script.json".into(),
                    description: "Execute all commands in script.json sequentially".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome script run script.json --fail-fast".into(),
                    description: "Stop on the first failing step and exit non-zero".into(),
                    flags: Some(vec!["--fail-fast".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome script run script.json --dry-run".into(),
                    description: "Validate the script schema without dispatching to Chrome".into(),
                    flags: Some(vec!["--dry-run".into()]),
                },
                ExampleEntry {
                    cmd: "agentchrome script run -".into(),
                    description: "Read the script from stdin instead of a file".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "echo '{\"commands\":[{\"cmd\":[\"page\",\"find\",\"Submit\"],\"bind\":\"match\"},{\"cmd\":[\"interact\",\"click\",\"$vars.match[0].uid\"]}]}' | agentchrome script run -"
                        .into(),
                    description:
                        "Discover an element via page find, bind the result, then click it"
                            .into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "echo '{\"commands\":[{\"cmd\":[\"page\",\"screenshot\",\"--file\",\"out.png\"]}]}' | agentchrome script run -"
                        .into(),
                    description: "Capture a screenshot to a file from inside a script".into(),
                    flags: None,
                },
            ],
        },
    ]
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_examples_returns_expected_groups() {
        let groups = all_examples();
        let names: Vec<&str> = groups.iter().map(|g| g.command.as_str()).collect();
        assert!(names.contains(&"connect"));
        assert!(names.contains(&"tabs"));
        assert!(names.contains(&"navigate"));
        assert!(names.contains(&"page"));
        assert!(names.contains(&"dom"));
        assert!(names.contains(&"js"));
        assert!(names.contains(&"console"));
        assert!(names.contains(&"network"));
        assert!(names.contains(&"interact"));
        assert!(names.contains(&"form"));
        assert!(names.contains(&"emulate"));
        assert!(names.contains(&"perf"));
        assert!(names.contains(&"dialog"));
        assert!(names.contains(&"media"));
        assert!(names.contains(&"skill"));
        assert!(names.contains(&"config"));
    }

    #[test]
    fn each_group_has_at_least_3_examples() {
        for group in all_examples() {
            assert!(
                group.examples.len() >= 3,
                "Group '{}' has only {} examples, expected at least 3",
                group.command,
                group.examples.len()
            );
        }
    }

    #[test]
    fn no_empty_fields() {
        for group in all_examples() {
            assert!(!group.command.is_empty());
            assert!(!group.description.is_empty());
            for example in &group.examples {
                assert!(
                    !example.cmd.is_empty(),
                    "Empty cmd in group '{}'",
                    group.command
                );
                assert!(
                    !example.description.is_empty(),
                    "Empty description in group '{}'",
                    group.command
                );
            }
        }
    }

    #[test]
    fn json_serialization_has_expected_fields() {
        let groups = all_examples();
        let json = serde_json::to_string(&groups).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let arr = parsed.as_array().unwrap();
        assert!(!arr.is_empty());
        for entry in arr {
            assert!(entry.get("command").is_some(), "missing 'command' field");
            assert!(
                entry.get("description").is_some(),
                "missing 'description' field"
            );
            let examples = entry.get("examples").unwrap().as_array().unwrap();
            assert!(!examples.is_empty());
            for ex in examples {
                assert!(ex.get("cmd").is_some(), "missing 'cmd' field");
                assert!(
                    ex.get("description").is_some(),
                    "missing 'description' field"
                );
            }
        }
    }
}
