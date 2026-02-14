#![allow(clippy::doc_markdown)]
// Items used by the binary crate may appear unused from the library crate's perspective.
#![allow(dead_code)]

use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

#[derive(Parser)]
#[command(
    name = "chrome-cli",
    version,
    about = "Browser automation via the Chrome DevTools Protocol",
    long_about = "chrome-cli is a command-line tool for browser automation via the Chrome DevTools \
        Protocol (CDP). It provides subcommands for connecting to Chrome/Chromium instances, \
        managing tabs, navigating pages, inspecting the DOM, executing JavaScript, monitoring \
        console output, intercepting network requests, simulating user interactions, filling forms, \
        emulating devices, and collecting performance metrics.\n\n\
        Designed for AI agents and shell scripting, every subcommand produces structured JSON \
        output on stdout and structured JSON errors on stderr. Global flags control connection \
        settings, output format, and target tab selection.",
    after_long_help = "\
QUICK START:
  # Connect to a running Chrome instance
  chrome-cli connect

  # Launch a new headless Chrome and connect
  chrome-cli connect --launch --headless

  # List open tabs and navigate to a URL
  chrome-cli tabs list
  chrome-cli navigate https://example.com

  # Take a full-page screenshot
  chrome-cli page screenshot --full-page --file shot.png

  # Execute JavaScript and get the result
  chrome-cli js exec \"document.title\"

  # Capture the accessibility tree and fill a form field
  chrome-cli page snapshot
  chrome-cli form fill s5 \"hello@example.com\"

  # Monitor console output in real time
  chrome-cli console follow --timeout 5000

EXIT CODES:
  0  Success
  1  General error (invalid arguments, internal failure)
  2  Connection error (Chrome not running, session expired)
  3  Target error (tab not found, no page targets)
  4  Timeout error (navigation or trace timeout)
  5  Protocol error (CDP protocol failure, dialog handling error)

ENVIRONMENT VARIABLES:
  CHROME_CLI_PORT     CDP port number (default: 9222)
  CHROME_CLI_HOST     CDP host address (default: 127.0.0.1)
  CHROME_CLI_TIMEOUT  Default command timeout in milliseconds
  CHROME_CLI_CONFIG   Path to configuration file",
    term_width = 100
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args)]
pub struct GlobalOpts {
    /// Chrome DevTools Protocol port number [default: 9222]
    #[arg(long, global = true, env = "CHROME_CLI_PORT")]
    pub port: Option<u16>,

    /// Chrome DevTools Protocol host address
    #[arg(
        long,
        default_value = "127.0.0.1",
        global = true,
        env = "CHROME_CLI_HOST"
    )]
    pub host: String,

    /// Direct WebSocket URL (overrides --host and --port)
    #[arg(long, global = true)]
    pub ws_url: Option<String>,

    /// Command timeout in milliseconds
    #[arg(long, global = true, env = "CHROME_CLI_TIMEOUT")]
    pub timeout: Option<u64>,

    /// Target tab ID (defaults to the active tab)
    #[arg(long, global = true)]
    pub tab: Option<String>,

    /// Automatically dismiss any dialogs that appear during command execution
    #[arg(long, global = true)]
    pub auto_dismiss_dialogs: bool,

    /// Path to configuration file (overrides default search)
    #[arg(long, global = true, env = "CHROME_CLI_CONFIG")]
    pub config: Option<PathBuf>,

    #[command(flatten)]
    pub output: OutputFormat,
}

impl GlobalOpts {
    /// Returns the port if explicitly provided, or the default (9222).
    /// Default CDP port when none is explicitly provided.
    const DEFAULT_PORT: u16 = 9222;

    #[must_use]
    pub fn port_or_default(&self) -> u16 {
        self.port.unwrap_or(Self::DEFAULT_PORT)
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Args)]
#[group(multiple = false)]
pub struct OutputFormat {
    /// Output as compact JSON (mutually exclusive with --pretty, --plain)
    #[arg(long, global = true)]
    pub json: bool,

    /// Output as pretty-printed JSON (mutually exclusive with --json, --plain)
    #[arg(long, global = true)]
    pub pretty: bool,

    /// Output as human-readable plain text (mutually exclusive with --json, --pretty)
    #[arg(long, global = true)]
    pub plain: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Connect to or launch a Chrome instance
    #[command(
        long_about = "Connect to a running Chrome/Chromium instance via the Chrome DevTools \
            Protocol, or launch a new one. Tests the connection and prints browser metadata \
            (browser version, WebSocket URL, user agent). The session is persisted to a \
            local file so subsequent commands reuse the same connection.",
        after_long_help = "\
EXAMPLES:
  # Connect to Chrome on the default port (9222)
  chrome-cli connect

  # Launch a new headless Chrome instance
  chrome-cli connect --launch --headless

  # Connect to a specific port
  chrome-cli connect --port 9333

  # Check connection status
  chrome-cli connect --status

  # Disconnect and remove session file
  chrome-cli connect --disconnect"
    )]
    Connect(ConnectArgs),

    /// Tab management (list, create, close, activate)
    #[command(
        long_about = "Tab management commands: list open tabs, create new tabs, close tabs, and \
            activate (focus) a specific tab. Each operation returns structured JSON with tab IDs \
            and metadata.",
        after_long_help = "\
EXAMPLES:
  # List all open tabs
  chrome-cli tabs list

  # Open a new tab and get its ID
  chrome-cli tabs create https://example.com

  # Close tabs by ID
  chrome-cli tabs close ABC123 DEF456

  # Activate a specific tab
  chrome-cli tabs activate ABC123"
    )]
    Tabs(TabsArgs),

    /// URL navigation and history
    #[command(
        long_about = "Navigate to URLs, reload pages, go back/forward in history, and wait for \
            navigation events. Supports waiting for load, DOMContentLoaded, or network idle.",
        after_long_help = "\
EXAMPLES:
  # Navigate to a URL and wait for page load
  chrome-cli navigate https://example.com

  # Navigate and wait for network idle
  chrome-cli navigate https://example.com --wait-until networkidle

  # Go back in browser history
  chrome-cli navigate back

  # Reload the current page, bypassing cache
  chrome-cli navigate reload --ignore-cache"
    )]
    Navigate(NavigateArgs),

    /// Page inspection (screenshot, text, accessibility tree, find)
    #[command(
        long_about = "Inspect the current page: capture screenshots (full page or element), \
            extract visible text, dump the accessibility tree, or search for text/elements on \
            the page.",
        after_long_help = "\
EXAMPLES:
  # Extract all visible text from the page
  chrome-cli page text

  # Capture the accessibility tree (assigns UIDs to elements)
  chrome-cli page snapshot

  # Take a full-page screenshot
  chrome-cli page screenshot --full-page --file page.png

  # Find elements by text
  chrome-cli page find \"Sign in\"

  # Resize the viewport
  chrome-cli page resize 1280x720"
    )]
    Page(PageArgs),

    /// DOM inspection and manipulation
    #[command(
        long_about = "Query and manipulate the DOM: select elements by CSS selector or XPath, \
            get/set attributes, read innerHTML/outerHTML, and modify element properties. \
            (Not yet implemented — use 'page snapshot' and 'js exec' as alternatives.)",
        after_long_help = "\
EXAMPLES:
  # (DOM commands are not yet implemented)
  # Use these alternatives:
  chrome-cli page snapshot
  chrome-cli js exec \"document.querySelector('#myId').textContent\""
    )]
    Dom,

    /// JavaScript execution in page context
    #[command(
        long_about = "Execute JavaScript expressions or scripts in the page context. Returns \
            the result as structured JSON. Supports both synchronous expressions and async \
            functions.",
        after_long_help = "\
EXAMPLES:
  # Get the page title
  chrome-cli js exec \"document.title\"

  # Execute a script file
  chrome-cli js exec --file script.js

  # Run code on a specific element (by UID from snapshot)
  chrome-cli js exec --uid s3 \"(el) => el.textContent\"

  # Read from stdin
  echo 'document.URL' | chrome-cli js exec -"
    )]
    Js(JsArgs),

    /// Console message reading and monitoring
    #[command(
        long_about = "Read and monitor browser console messages (log, warn, error, info). \
            Can capture existing messages or stream new messages in real time.",
        after_long_help = "\
EXAMPLES:
  # Read recent console messages
  chrome-cli console read

  # Show only error messages
  chrome-cli console read --errors-only

  # Stream console messages in real time
  chrome-cli console follow

  # Stream errors for 10 seconds
  chrome-cli console follow --errors-only --timeout 10000"
    )]
    Console(ConsoleArgs),

    /// Network request monitoring and interception
    #[command(
        long_about = "Monitor and intercept network requests. List recent requests, filter by \
            URL pattern or resource type, capture request/response bodies, and stream requests \
            in real time.",
        after_long_help = "\
EXAMPLES:
  # List recent network requests
  chrome-cli network list

  # Filter by resource type
  chrome-cli network list --type xhr,fetch

  # Get details of a specific request
  chrome-cli network get 42

  # Stream network requests in real time
  chrome-cli network follow --url api.example.com"
    )]
    Network(NetworkArgs),

    /// Mouse, keyboard, and scroll interactions
    #[command(
        long_about = "Simulate user interactions: click elements, type text, press key \
            combinations, scroll the page, hover over elements, and perform drag-and-drop \
            operations. Target elements by UID (from 'page snapshot') or CSS selector \
            (prefixed with 'css:').",
        after_long_help = "\
EXAMPLES:
  # Click an element by UID
  chrome-cli interact click s5

  # Click by CSS selector
  chrome-cli interact click css:#submit-btn

  # Type text into the focused element
  chrome-cli interact type \"Hello, world!\"

  # Press a key combination
  chrome-cli interact key Control+A

  # Scroll down one viewport height
  chrome-cli interact scroll"
    )]
    Interact(InteractArgs),

    /// Form input and submission
    #[command(
        long_about = "Fill in form fields, select dropdown options, toggle checkboxes, and clear \
            fields. Supports targeting fields by UID (from accessibility snapshot) or CSS \
            selector (prefixed with 'css:'). Run 'page snapshot' first to discover field UIDs.",
        after_long_help = "\
EXAMPLES:
  # Fill a field by UID (from page snapshot)
  chrome-cli form fill s5 \"hello@example.com\"

  # Fill by CSS selector
  chrome-cli form fill css:#email \"user@example.com\"

  # Fill multiple fields at once
  chrome-cli form fill-many '[{\"uid\":\"s5\",\"value\":\"Alice\"},{\"uid\":\"s7\",\"value\":\"alice@example.com\"}]'

  # Clear a field
  chrome-cli form clear s5

  # Upload a file
  chrome-cli form upload s10 ./photo.jpg"
    )]
    Form(FormArgs),

    /// Device and network emulation
    #[command(
        long_about = "Emulate different devices, screen sizes, and network conditions. Set \
            custom user agents, viewport dimensions, device scale factor, and network throttling \
            profiles.",
        after_long_help = "\
EXAMPLES:
  # Emulate a mobile device
  chrome-cli emulate set --viewport 375x667 --device-scale 2 --mobile

  # Simulate slow 3G network
  chrome-cli emulate set --network 3g

  # Force dark mode
  chrome-cli emulate set --color-scheme dark

  # Check current emulation settings
  chrome-cli emulate status

  # Clear all emulation overrides
  chrome-cli emulate reset"
    )]
    Emulate(EmulateArgs),

    /// Performance tracing and metrics
    #[command(
        long_about = "Collect performance metrics, capture trace files, measure page load timing, \
            and analyze runtime performance. Outputs metrics as structured JSON for analysis.",
        after_long_help = "\
EXAMPLES:
  # Quick Core Web Vitals measurement
  chrome-cli perf vitals

  # Start a trace, reload the page, then stop
  chrome-cli perf start --reload
  chrome-cli perf stop

  # Analyze a trace for render-blocking resources
  chrome-cli perf analyze RenderBlocking --trace-file trace.json

  # Auto-stop trace after page load
  chrome-cli perf start --reload --auto-stop"
    )]
    Perf(PerfArgs),

    /// Browser dialog handling (alert, confirm, prompt, beforeunload)
    #[command(
        long_about = "Detect and handle browser JavaScript dialogs (alert, confirm, prompt, \
            beforeunload). Query whether a dialog is open, accept or dismiss it, and provide \
            prompt text. Useful for automation scripts that need to respond to dialogs \
            programmatically.",
        after_long_help = "\
EXAMPLES:
  # Check if a dialog is open
  chrome-cli dialog info

  # Accept an alert or confirm dialog
  chrome-cli dialog handle accept

  # Dismiss a dialog
  chrome-cli dialog handle dismiss

  # Accept a prompt with text
  chrome-cli dialog handle accept --text \"my input\""
    )]
    Dialog(DialogArgs),

    /// Configuration file management (show, init, path)
    #[command(
        long_about = "Manage the chrome-cli configuration file. Show the resolved configuration \
            from all sources, create a default config file, or display the active config file path. \
            Config files use TOML format and are searched in priority order: --config flag, \
            $CHROME_CLI_CONFIG env var, project-local, XDG config dir, home directory.",
        after_long_help = "\
EXAMPLES:
  # Show the resolved configuration
  chrome-cli config show

  # Create a default config file
  chrome-cli config init

  # Create a config at a custom path
  chrome-cli config init --path ./my-config.toml

  # Show the active config file path
  chrome-cli config path"
    )]
    Config(ConfigArgs),

    /// Generate shell completion scripts
    #[command(
        long_about = "Generate shell completion scripts for tab-completion of commands, flags, \
            and enum values. Pipe the output to the appropriate file for your shell.",
        after_long_help = "\
EXAMPLES:
  # Bash
  chrome-cli completions bash > /etc/bash_completion.d/chrome-cli

  # Zsh
  chrome-cli completions zsh > ~/.zfunc/_chrome-cli

  # Fish
  chrome-cli completions fish > ~/.config/fish/completions/chrome-cli.fish

  # PowerShell
  chrome-cli completions powershell >> $PROFILE

  # Elvish
  chrome-cli completions elvish >> ~/.elvish/rc.elv"
    )]
    Completions(CompletionsArgs),

    /// Show usage examples for commands
    #[command(
        long_about = "Show usage examples for chrome-cli commands. Without arguments, lists all \
            command groups with a brief description and one example each. With a command name, \
            shows detailed examples for that specific command group.",
        after_long_help = "\
EXAMPLES:
  # List all command groups with summary examples
  chrome-cli examples

  # Show detailed examples for the navigate command
  chrome-cli examples navigate

  # Get all examples as JSON (for programmatic use)
  chrome-cli examples --json

  # Pretty-printed JSON output
  chrome-cli examples --pretty"
    )]
    Examples(ExamplesArgs),

    /// Display man pages for chrome-cli commands
    #[command(
        long_about = "Display man pages for chrome-cli commands. Without arguments, displays \
            the main chrome-cli man page. With a subcommand name, displays the man page for \
            that specific command. Output is in roff format, suitable for piping to a pager.",
        after_long_help = "\
EXAMPLES:
  # Display the main chrome-cli man page
  chrome-cli man

  # Display the man page for the connect command
  chrome-cli man connect

  # Display the man page for the tabs command
  chrome-cli man tabs

  # Pipe to a pager
  chrome-cli man navigate | less"
    )]
    Man(ManArgs),
}

/// Chrome release channel to use when launching.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ChromeChannel {
    Stable,
    Canary,
    Beta,
    Dev,
}

/// Arguments for the `tabs` subcommand group.
#[derive(Args)]
pub struct TabsArgs {
    #[command(subcommand)]
    pub command: TabsCommand,
}

/// Tab management subcommands.
#[derive(Subcommand)]
pub enum TabsCommand {
    /// List open tabs
    #[command(
        long_about = "List all open browser tabs. Returns JSON with each tab's ID, title, URL, \
            and type. By default, only page tabs are shown; use --all to include internal \
            Chrome pages (chrome://, chrome-extension://).",
        after_long_help = "\
EXAMPLES:
  # List page tabs
  chrome-cli tabs list

  # Include internal Chrome pages
  chrome-cli tabs list --all"
    )]
    List(TabsListArgs),

    /// Create a new tab
    #[command(
        long_about = "Create a new browser tab. Optionally specify a URL to open; defaults to \
            about:blank. Returns JSON with the new tab's ID and URL. Use --background to open \
            the tab without switching focus to it.",
        after_long_help = "\
EXAMPLES:
  # Open a blank tab
  chrome-cli tabs create

  # Open a URL
  chrome-cli tabs create https://example.com

  # Open in the background
  chrome-cli tabs create https://example.com --background"
    )]
    Create(TabsCreateArgs),

    /// Close one or more tabs
    #[command(
        long_about = "Close one or more browser tabs by their IDs. Accepts multiple tab IDs \
            as arguments. Returns JSON confirming which tabs were closed. Cannot close the \
            last remaining tab (Chrome requires at least one open tab).",
        after_long_help = "\
EXAMPLES:
  # Close a single tab
  chrome-cli tabs close ABC123

  # Close multiple tabs
  chrome-cli tabs close ABC123 DEF456 GHI789"
    )]
    Close(TabsCloseArgs),

    /// Activate (focus) a tab
    #[command(
        long_about = "Activate (bring to front) a specific browser tab by its ID. The tab \
            becomes the active target for subsequent commands. Returns JSON confirming the \
            activated tab.",
        after_long_help = "\
EXAMPLES:
  # Activate a tab by ID
  chrome-cli tabs activate ABC123

  # Activate silently
  chrome-cli tabs activate ABC123 --quiet"
    )]
    Activate(TabsActivateArgs),
}

/// Arguments for `tabs list`.
#[derive(Args)]
pub struct TabsListArgs {
    /// Include internal Chrome pages (chrome://, chrome-extension://)
    #[arg(long)]
    pub all: bool,
}

/// Arguments for `tabs create`.
#[derive(Args)]
pub struct TabsCreateArgs {
    /// URL to open (defaults to about:blank)
    pub url: Option<String>,

    /// Open the tab in the background without activating it
    #[arg(long)]
    pub background: bool,
}

/// Arguments for `tabs close`.
#[derive(Args)]
pub struct TabsCloseArgs {
    /// Tab ID(s) or index(es) to close
    #[arg(required = true)]
    pub targets: Vec<String>,
}

/// Arguments for `tabs activate`.
#[derive(Args)]
pub struct TabsActivateArgs {
    /// Tab ID or index to activate
    pub target: String,

    /// Suppress output after activation
    #[arg(long)]
    pub quiet: bool,
}

/// Arguments for the `navigate` subcommand group.
#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct NavigateArgs {
    #[command(subcommand)]
    pub command: Option<NavigateCommand>,

    #[command(flatten)]
    pub url_args: NavigateUrlArgs,
}

/// Navigate subcommands.
#[derive(Subcommand)]
pub enum NavigateCommand {
    /// Go back in browser history
    #[command(
        long_about = "Navigate back one step in the browser's session history, equivalent to \
            clicking the browser's back button. Returns JSON with the new URL after navigation.",
        after_long_help = "\
EXAMPLES:
  # Go back
  chrome-cli navigate back"
    )]
    Back,

    /// Go forward in browser history
    #[command(
        long_about = "Navigate forward one step in the browser's session history, equivalent to \
            clicking the browser's forward button. Only works if the user previously navigated \
            back. Returns JSON with the new URL after navigation.",
        after_long_help = "\
EXAMPLES:
  # Go forward
  chrome-cli navigate forward"
    )]
    Forward,

    /// Reload the current page
    #[command(
        long_about = "Reload the current page. Use --ignore-cache to bypass the browser cache \
            and force a full reload from the server. Returns JSON with the page URL after reload.",
        after_long_help = "\
EXAMPLES:
  # Reload the page
  chrome-cli navigate reload

  # Reload bypassing cache
  chrome-cli navigate reload --ignore-cache"
    )]
    Reload(NavigateReloadArgs),
}

/// Arguments for direct URL navigation (`navigate <URL>`).
#[derive(Args)]
pub struct NavigateUrlArgs {
    /// URL to navigate to
    pub url: Option<String>,

    /// Wait strategy after navigation
    #[arg(long, value_enum, default_value_t = WaitUntil::Load)]
    pub wait_until: WaitUntil,

    /// Navigation timeout in milliseconds
    #[arg(long)]
    pub timeout: Option<u64>,

    /// Bypass the browser cache
    #[arg(long)]
    pub ignore_cache: bool,
}

/// Arguments for `navigate reload`.
#[derive(Args)]
pub struct NavigateReloadArgs {
    /// Bypass the browser cache on reload
    #[arg(long)]
    pub ignore_cache: bool,
}

/// Arguments for the `page` subcommand group.
#[derive(Args)]
pub struct PageArgs {
    #[command(subcommand)]
    pub command: PageCommand,
}

/// Page inspection subcommands.
#[derive(Subcommand)]
pub enum PageCommand {
    /// Extract visible text from the page
    #[command(
        long_about = "Extract the visible text content from the current page or a specific \
            element. Returns the text as a plain string. Useful for reading page content \
            without HTML markup.",
        after_long_help = "\
EXAMPLES:
  # Get all visible text
  chrome-cli page text

  # Get text from a specific element
  chrome-cli page text --selector \"#main-content\""
    )]
    Text(PageTextArgs),

    /// Capture the accessibility tree of the page
    #[command(
        long_about = "Capture the accessibility tree (AX tree) of the current page. Each \
            interactive element is assigned a UID (e.g., s1, s2, s3) that can be used with \
            'interact', 'form', and 'js exec --uid' commands. Use --verbose to include \
            additional properties like checked, disabled, and level.",
        after_long_help = "\
EXAMPLES:
  # Capture the accessibility tree
  chrome-cli page snapshot

  # Verbose output with extra properties
  chrome-cli page snapshot --verbose

  # Save to a file
  chrome-cli page snapshot --file snapshot.txt"
    )]
    Snapshot(PageSnapshotArgs),

    /// Find elements by text, CSS selector, or accessibility role
    #[command(
        long_about = "Search for elements on the page by text content, CSS selector, or \
            accessibility role. Returns matching elements with their UIDs, roles, and names. \
            By default, performs a case-insensitive substring match; use --exact for exact \
            matching.",
        after_long_help = "\
EXAMPLES:
  # Find elements by text
  chrome-cli page find \"Sign in\"

  # Find by CSS selector
  chrome-cli page find --selector \"button.primary\"

  # Find by accessibility role
  chrome-cli page find --role button

  # Exact text match with limit
  chrome-cli page find \"Submit\" --exact --limit 1"
    )]
    Find(PageFindArgs),

    /// Capture a screenshot of the page, an element, or a region
    #[command(
        long_about = "Capture a screenshot of the current page, a specific element, or a \
            viewport region. Supports PNG (default), JPEG, and WebP formats. Use --full-page \
            to capture the entire scrollable page, --selector or --uid to capture a specific \
            element, or --clip to capture a region. Note: --full-page conflicts with \
            --selector, --uid, and --clip.",
        after_long_help = "\
EXAMPLES:
  # Screenshot the visible viewport
  chrome-cli page screenshot --file shot.png

  # Full-page screenshot
  chrome-cli page screenshot --full-page --file full.png

  # Screenshot a specific element by UID
  chrome-cli page screenshot --uid s3 --file element.png

  # JPEG format with quality
  chrome-cli page screenshot --format jpeg --quality 80 --file shot.jpg"
    )]
    Screenshot(PageScreenshotArgs),

    /// Resize the viewport to the given dimensions
    #[command(
        long_about = "Resize the browser viewport to the specified dimensions. The size is \
            given as WIDTHxHEIGHT in pixels (e.g., 1280x720). Useful for testing responsive \
            layouts. See also: 'emulate set --viewport' for device emulation.",
        after_long_help = "\
EXAMPLES:
  # Resize to 1280x720
  chrome-cli page resize 1280x720

  # Mobile viewport
  chrome-cli page resize 375x667"
    )]
    Resize(PageResizeArgs),
}

/// Image format for screenshots.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum ScreenshotFormat {
    /// PNG (lossless, default)
    #[default]
    Png,
    /// JPEG (lossy)
    Jpeg,
    /// WebP (lossy)
    Webp,
}

/// Arguments for `page screenshot`.
#[derive(Args)]
pub struct PageScreenshotArgs {
    /// Capture the entire scrollable page, not just the visible viewport
    #[arg(long)]
    pub full_page: bool,

    /// Capture a specific element by CSS selector (conflicts with --full-page)
    #[arg(long)]
    pub selector: Option<String>,

    /// Capture a specific element by UID from 'page snapshot' (conflicts with --full-page)
    #[arg(long)]
    pub uid: Option<String>,

    /// Image format [default: png] [possible values: png, jpeg, webp]
    #[arg(long, value_enum, default_value_t = ScreenshotFormat::Png)]
    pub format: ScreenshotFormat,

    /// JPEG/WebP compression quality, 0-100 (ignored for PNG)
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=100))]
    pub quality: Option<u8>,

    /// Save screenshot to a file instead of base64-encoded stdout
    #[arg(long)]
    pub file: Option<PathBuf>,

    /// Capture a specific viewport region as X,Y,WIDTH,HEIGHT (e.g. 10,20,200,100)
    #[arg(long)]
    pub clip: Option<String>,
}

/// Arguments for `page text`.
#[derive(Args)]
pub struct PageTextArgs {
    /// CSS selector to extract text from a specific element
    #[arg(long)]
    pub selector: Option<String>,
}

/// Arguments for `page snapshot`.
#[derive(Args)]
pub struct PageSnapshotArgs {
    /// Include additional element properties (checked, disabled, level, etc.)
    #[arg(long)]
    pub verbose: bool,

    /// Save snapshot to file instead of stdout
    #[arg(long)]
    pub file: Option<PathBuf>,
}

/// Arguments for `page find`.
#[derive(Args)]
pub struct PageFindArgs {
    /// Text to search for (searches accessible names, text content, labels)
    pub query: Option<String>,

    /// Find by CSS selector instead of text
    #[arg(long)]
    pub selector: Option<String>,

    /// Filter by accessibility role (button, link, textbox, etc.)
    #[arg(long)]
    pub role: Option<String>,

    /// Require exact text match (default: case-insensitive substring)
    #[arg(long)]
    pub exact: bool,

    /// Maximum results to return
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
}

/// Arguments for the `perf` subcommand group.
#[derive(Args)]
pub struct PerfArgs {
    #[command(subcommand)]
    pub command: PerfCommand,
}

/// Performance tracing subcommands.
#[derive(Subcommand)]
pub enum PerfCommand {
    /// Start a performance trace recording
    #[command(
        long_about = "Start recording a performance trace. The trace captures JavaScript \
            execution, layout, paint, network, and other browser activity. Use --reload to \
            automatically reload the page before tracing. Use --auto-stop to stop recording \
            after the page load completes. The trace is saved to a JSON file that can be \
            opened in Chrome DevTools or analyzed with 'perf analyze'.",
        after_long_help = "\
EXAMPLES:
  # Start tracing
  chrome-cli perf start

  # Start with page reload
  chrome-cli perf start --reload

  # Auto-stop after page load
  chrome-cli perf start --reload --auto-stop

  # Save to a specific file
  chrome-cli perf start --file my-trace.json"
    )]
    Start(PerfStartArgs),

    /// Stop the active trace and collect data
    #[command(
        long_about = "Stop the active performance trace and save the collected data. Returns \
            JSON with the trace file path and summary metrics. The trace file can be opened \
            in Chrome DevTools (Performance tab) or analyzed with 'perf analyze'.",
        after_long_help = "\
EXAMPLES:
  # Stop and save
  chrome-cli perf stop

  # Stop and save to a specific file
  chrome-cli perf stop --file trace-output.json"
    )]
    Stop(PerfStopArgs),

    /// Analyze a specific performance insight from a trace
    #[command(
        long_about = "Analyze a previously saved trace file for a specific performance insight. \
            Available insights: DocumentLatency (document request timing), LCPBreakdown (Largest \
            Contentful Paint phases), RenderBlocking (render-blocking resources), LongTasks \
            (JavaScript tasks > 50ms). Returns structured JSON with the analysis results.",
        after_long_help = "\
EXAMPLES:
  # Analyze LCP breakdown
  chrome-cli perf analyze LCPBreakdown --trace-file trace.json

  # Find render-blocking resources
  chrome-cli perf analyze RenderBlocking --trace-file trace.json

  # Identify long tasks
  chrome-cli perf analyze LongTasks --trace-file trace.json"
    )]
    Analyze(PerfAnalyzeArgs),

    /// Quick Core Web Vitals measurement
    #[command(
        long_about = "Perform a quick Core Web Vitals measurement. Automatically starts a \
            trace, reloads the page, collects vitals (LCP, FID, CLS), and stops the trace. \
            Returns structured JSON with the web vitals metrics.",
        after_long_help = "\
EXAMPLES:
  # Measure web vitals
  chrome-cli perf vitals

  # Save the underlying trace file
  chrome-cli perf vitals --file vitals-trace.json"
    )]
    Vitals(PerfVitalsArgs),
}

/// Arguments for `perf start`.
#[derive(Args)]
pub struct PerfStartArgs {
    /// Reload the page before tracing
    #[arg(long)]
    pub reload: bool,
    /// Automatically stop after page load completes
    #[arg(long)]
    pub auto_stop: bool,
    /// Path to save the trace file (default: auto-generated)
    #[arg(long)]
    pub file: Option<PathBuf>,
}

/// Arguments for `perf stop`.
#[derive(Args)]
pub struct PerfStopArgs {
    /// Override output file path for the trace
    #[arg(long)]
    pub file: Option<PathBuf>,
}

/// Arguments for `perf analyze`.
#[derive(Args)]
pub struct PerfAnalyzeArgs {
    /// Insight to analyze: DocumentLatency, LCPBreakdown, RenderBlocking, LongTasks
    pub insight: String,
    /// Path to a previously saved trace JSON file
    #[arg(long)]
    pub trace_file: PathBuf,
}

/// Arguments for `perf vitals`.
#[derive(Args)]
pub struct PerfVitalsArgs {
    /// Path to save the trace file (default: auto-generated temp)
    #[arg(long)]
    pub file: Option<PathBuf>,
}

/// Arguments for the `js` subcommand group.
#[derive(Args)]
pub struct JsArgs {
    #[command(subcommand)]
    pub command: JsCommand,
}

/// JavaScript subcommands.
#[derive(Subcommand)]
pub enum JsCommand {
    /// Execute JavaScript in the page context
    #[command(
        long_about = "Execute a JavaScript expression or script in the page context and return \
            the result as JSON. Code can be provided as an inline argument, read from a file \
            with --file, or piped via stdin using '-'. When --uid is specified, the code is \
            wrapped in a function that receives the element as its first argument. By default, \
            promise results are awaited; use --no-await to return immediately.",
        after_long_help = "\
EXAMPLES:
  # Evaluate an expression
  chrome-cli js exec \"document.title\"

  # Execute a script file
  chrome-cli js exec --file script.js

  # Run code on a specific element
  chrome-cli js exec --uid s3 \"(el) => el.textContent\"

  # Read from stdin
  echo 'document.URL' | chrome-cli js exec -

  # Skip awaiting promises
  chrome-cli js exec --no-await \"fetch('/api/data')\""
    )]
    Exec(JsExecArgs),
}

/// Arguments for `js exec`.
#[derive(Args)]
pub struct JsExecArgs {
    /// JavaScript code to execute (use '-' to read from stdin; conflicts with --file)
    #[arg(conflicts_with = "file")]
    pub code: Option<String>,

    /// Read JavaScript from a file instead of inline argument (conflicts with CODE)
    #[arg(long)]
    pub file: Option<PathBuf>,

    /// Element UID from 'page snapshot'; code is wrapped in a function receiving the element
    #[arg(long)]
    pub uid: Option<String>,

    /// Return promise objects without awaiting them
    #[arg(long, action = ArgAction::SetTrue)]
    pub no_await: bool,

    /// Execution timeout in milliseconds (overrides global --timeout)
    #[arg(long)]
    pub timeout: Option<u64>,

    /// Truncate result output exceeding this size in bytes
    #[arg(long)]
    pub max_size: Option<usize>,
}

/// Arguments for the `dialog` subcommand group.
#[derive(Args)]
pub struct DialogArgs {
    #[command(subcommand)]
    pub command: DialogCommand,
}

/// Dialog subcommands.
#[derive(Subcommand)]
pub enum DialogCommand {
    /// Accept or dismiss the current browser dialog
    #[command(
        long_about = "Accept or dismiss the currently open browser dialog (alert, confirm, \
            prompt, or beforeunload). A dialog must be open before this command can be used. \
            For prompt dialogs, use --text to provide the response text when accepting.",
        after_long_help = "\
EXAMPLES:
  # Accept an alert
  chrome-cli dialog handle accept

  # Dismiss a confirm dialog
  chrome-cli dialog handle dismiss

  # Accept a prompt with text
  chrome-cli dialog handle accept --text \"my response\""
    )]
    Handle(DialogHandleArgs),

    /// Check whether a dialog is currently open
    #[command(
        long_about = "Check whether a JavaScript dialog (alert, confirm, prompt, or \
            beforeunload) is currently open. Returns JSON with the dialog's type, message, \
            and default prompt text if applicable. Returns {\"open\": false} when no dialog \
            is present.",
        after_long_help = "\
EXAMPLES:
  # Check for open dialog
  chrome-cli dialog info"
    )]
    Info,
}

/// Arguments for `dialog handle`.
#[derive(Args)]
pub struct DialogHandleArgs {
    /// Action to take: accept or dismiss
    pub action: DialogAction,

    /// Response text for prompt dialogs (only used with 'accept' action)
    #[arg(long)]
    pub text: Option<String>,
}

/// Action to take on a browser dialog.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DialogAction {
    /// Accept (OK) the dialog
    Accept,
    /// Dismiss (Cancel) the dialog
    Dismiss,
}

/// Wait strategy for navigation commands.
#[derive(Debug, Clone, Copy, ValueEnum, Default, PartialEq, Eq)]
pub enum WaitUntil {
    /// Wait for the load event
    #[default]
    Load,
    /// Wait for DOMContentLoaded event
    Domcontentloaded,
    /// Wait until network is idle (no requests for 500ms)
    Networkidle,
    /// Return immediately after initiating navigation
    None,
}

/// Arguments for the `connect` subcommand.
#[allow(clippy::struct_excessive_bools)]
#[derive(Args)]
pub struct ConnectArgs {
    /// Launch a new Chrome instance instead of connecting to an existing one
    #[arg(long)]
    pub launch: bool,

    /// Show current connection status (conflicts with --launch, --disconnect)
    #[arg(long, conflicts_with_all = ["launch", "disconnect"])]
    pub status: bool,

    /// Disconnect and remove session file (conflicts with --launch, --status)
    #[arg(long, conflicts_with_all = ["launch", "status"])]
    pub disconnect: bool,

    /// Launch Chrome in headless mode
    #[arg(long, requires = "launch")]
    pub headless: bool,

    /// Chrome release channel to launch [default: stable] [possible values: stable, canary, beta, dev]
    #[arg(long, requires = "launch", default_value = "stable")]
    pub channel: ChromeChannel,

    /// Path to a Chrome/Chromium executable (overrides channel-based discovery)
    #[arg(long, requires = "launch")]
    pub chrome_path: Option<PathBuf>,

    /// Additional arguments to pass to Chrome (can be repeated)
    #[arg(long, requires = "launch")]
    pub chrome_arg: Vec<String>,
}

/// Arguments for the `interact` subcommand group.
#[derive(Args)]
pub struct InteractArgs {
    #[command(subcommand)]
    pub command: InteractCommand,
}

/// Interact subcommands.
#[derive(Subcommand)]
pub enum InteractCommand {
    /// Click an element by UID or CSS selector
    #[command(
        long_about = "Click an element identified by UID (from 'page snapshot', e.g., 's5') or \
            CSS selector (prefixed with 'css:', e.g., 'css:#submit'). By default, performs a \
            left single-click at the element's center. Use --double for double-click or --right \
            for right-click (context menu). These flags are mutually exclusive.",
        after_long_help = "\
EXAMPLES:
  # Click by UID
  chrome-cli interact click s5

  # Click by CSS selector
  chrome-cli interact click css:#submit-btn

  # Double-click
  chrome-cli interact click s5 --double

  # Right-click (context menu)
  chrome-cli interact click s5 --right"
    )]
    Click(ClickArgs),

    /// Click at viewport coordinates
    #[command(
        long_about = "Click at specific viewport coordinates (X, Y in pixels). Useful when \
            targeting elements that are not in the accessibility tree or for precise coordinate-\
            based interactions. Use --double for double-click or --right for right-click.",
        after_long_help = "\
EXAMPLES:
  # Click at coordinates
  chrome-cli interact click-at 100 200

  # Double-click at coordinates
  chrome-cli interact click-at 100 200 --double"
    )]
    ClickAt(ClickAtArgs),

    /// Hover over an element
    #[command(
        long_about = "Move the mouse over an element identified by UID or CSS selector. \
            Triggers hover effects, tooltips, and mouseover events. Does not click.",
        after_long_help = "\
EXAMPLES:
  # Hover by UID
  chrome-cli interact hover s3

  # Hover by CSS selector
  chrome-cli interact hover css:.tooltip-trigger"
    )]
    Hover(HoverArgs),

    /// Drag from one element to another
    #[command(
        long_about = "Drag from one element to another. Both source and target are identified \
            by UID or CSS selector. Simulates mouse down on the source, move to the target, \
            and mouse up on the target.",
        after_long_help = "\
EXAMPLES:
  # Drag between elements by UID
  chrome-cli interact drag s3 s7

  # Drag using CSS selectors
  chrome-cli interact drag css:#item css:#dropzone"
    )]
    Drag(DragArgs),

    /// Type text character-by-character into the focused element
    #[command(
        long_about = "Type text character-by-character into the currently focused element. \
            Simulates individual key press and release events for each character. Use --delay \
            to add a pause between keystrokes. To focus an element first, use 'interact click'.",
        after_long_help = "\
EXAMPLES:
  # Type text
  chrome-cli interact type \"Hello, world!\"

  # Type with delay between keystrokes
  chrome-cli interact type \"slow typing\" --delay 50"
    )]
    Type(TypeArgs),

    /// Press a key or key combination (e.g. Enter, Control+A)
    #[command(
        long_about = "Press a key or key combination. Supports modifier keys (Control, Shift, \
            Alt, Meta) combined with regular keys using '+' separator. Use --repeat to press \
            the key multiple times. Common keys: Enter, Tab, Escape, Backspace, ArrowUp, \
            ArrowDown, ArrowLeft, ArrowRight, Home, End, PageUp, PageDown, Delete.",
        after_long_help = "\
EXAMPLES:
  # Press Enter
  chrome-cli interact key Enter

  # Select all (Ctrl+A)
  chrome-cli interact key Control+A

  # Press Tab 3 times
  chrome-cli interact key Tab --repeat 3

  # Multi-modifier combo
  chrome-cli interact key Control+Shift+ArrowRight"
    )]
    Key(KeyArgs),

    /// Scroll the page or a container element
    #[command(
        long_about = "Scroll the page or a specific container element. By default, scrolls \
            down by one viewport height. Use --direction to scroll in other directions, \
            --amount to set a custom distance in pixels, or the shortcut flags --to-top, \
            --to-bottom, --to-element to scroll to specific positions. Use --container to \
            scroll within a scrollable child element. Use --smooth for animated scrolling.",
        after_long_help = "\
EXAMPLES:
  # Scroll down one viewport height
  chrome-cli interact scroll

  # Scroll up 200 pixels
  chrome-cli interact scroll --direction up --amount 200

  # Scroll to bottom of page
  chrome-cli interact scroll --to-bottom

  # Scroll until an element is visible
  chrome-cli interact scroll --to-element s15

  # Smooth scroll within a container
  chrome-cli interact scroll --container css:.scrollable --smooth"
    )]
    Scroll(ScrollArgs),
}

/// Arguments for `interact click`.
#[derive(Args)]
pub struct ClickArgs {
    /// Target element (UID like 's1' or CSS selector like 'css:#button')
    pub target: String,

    /// Perform a double-click instead of single click (conflicts with --right)
    #[arg(long, conflicts_with = "right")]
    pub double: bool,

    /// Perform a right-click (context menu) instead of left click (conflicts with --double)
    #[arg(long, conflicts_with = "double")]
    pub right: bool,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,
}

/// Arguments for `interact click-at`.
#[derive(Args)]
pub struct ClickAtArgs {
    /// X coordinate in viewport pixels
    pub x: f64,

    /// Y coordinate in viewport pixels
    pub y: f64,

    /// Perform a double-click instead of single click (conflicts with --right)
    #[arg(long, conflicts_with = "right")]
    pub double: bool,

    /// Perform a right-click (context menu) instead of left click (conflicts with --double)
    #[arg(long, conflicts_with = "double")]
    pub right: bool,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,
}

/// Arguments for `interact hover`.
#[derive(Args)]
pub struct HoverArgs {
    /// Target element (UID like 's1' or CSS selector like 'css:#button')
    pub target: String,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,
}

/// Arguments for `interact drag`.
#[derive(Args)]
pub struct DragArgs {
    /// Source element to drag from (UID or CSS selector)
    pub from: String,

    /// Target element to drag to (UID or CSS selector)
    pub to: String,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,
}

/// Arguments for `interact type`.
#[derive(Args)]
pub struct TypeArgs {
    /// Text to type character-by-character
    #[arg(required = true)]
    pub text: String,

    /// Delay between keystrokes in milliseconds (default: 0 for instant)
    #[arg(long, default_value_t = 0)]
    pub delay: u64,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,
}

/// Arguments for `interact key`.
#[derive(Args)]
pub struct KeyArgs {
    /// Key or key combination to press (e.g. Enter, Control+A, Shift+ArrowDown)
    #[arg(required = true)]
    pub keys: String,

    /// Number of times to press the key
    #[arg(long, default_value_t = 1)]
    pub repeat: u32,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,
}

/// Scroll direction for `interact scroll`.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum ScrollDirection {
    /// Scroll down (default)
    #[default]
    Down,
    /// Scroll up
    Up,
    /// Scroll left
    Left,
    /// Scroll right
    Right,
}

/// Arguments for `interact scroll`.
#[allow(clippy::struct_excessive_bools)]
#[derive(Args)]
pub struct ScrollArgs {
    /// Scroll direction
    #[arg(long, value_enum, default_value_t = ScrollDirection::Down,
           conflicts_with_all = ["to_element", "to_top", "to_bottom"])]
    pub direction: ScrollDirection,

    /// Scroll distance in pixels (default: viewport height for vertical, viewport width for horizontal)
    #[arg(long, conflicts_with_all = ["to_element", "to_top", "to_bottom"])]
    pub amount: Option<u32>,

    /// Scroll until a specific element is in view (UID like 's5' or CSS selector like 'css:#footer')
    #[arg(long, conflicts_with_all = ["direction", "amount", "to_top", "to_bottom", "container"])]
    pub to_element: Option<String>,

    /// Scroll to the top of the page
    #[arg(long, conflicts_with_all = ["direction", "amount", "to_element", "to_bottom", "container"])]
    pub to_top: bool,

    /// Scroll to the bottom of the page
    #[arg(long, conflicts_with_all = ["direction", "amount", "to_element", "to_top", "container"])]
    pub to_bottom: bool,

    /// Use smooth scrolling behavior
    #[arg(long)]
    pub smooth: bool,

    /// Scroll within a container element (UID like 's3' or CSS selector like 'css:.scrollable')
    #[arg(long, conflicts_with_all = ["to_element", "to_top", "to_bottom"])]
    pub container: Option<String>,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,
}

/// Arguments for the `form` subcommand group.
#[derive(Args)]
pub struct FormArgs {
    #[command(subcommand)]
    pub command: FormCommand,
}

/// Form subcommands.
#[derive(Subcommand)]
pub enum FormCommand {
    /// Fill a form field by UID or CSS selector
    #[command(
        long_about = "Set the value of a form field identified by UID (from 'page snapshot', \
            e.g., 's5') or CSS selector (prefixed with 'css:', e.g., 'css:#email'). Works \
            with text inputs, textareas, select dropdowns, and checkboxes. Dispatches change \
            and input events to trigger form validation.",
        after_long_help = "\
EXAMPLES:
  # Fill by UID
  chrome-cli form fill s5 \"hello@example.com\"

  # Fill by CSS selector
  chrome-cli form fill css:#email \"user@example.com\"

  # Select a dropdown option
  chrome-cli form fill s8 \"Option B\""
    )]
    Fill(FormFillArgs),

    /// Fill multiple form fields at once from JSON
    #[command(
        long_about = "Fill multiple form fields in a single command. Accepts a JSON array of \
            {uid, value} objects either as an inline argument or from a file with --file. Each \
            field is filled in order. Useful for completing entire forms in one step.",
        after_long_help = "\
EXAMPLES:
  # Fill multiple fields inline
  chrome-cli form fill-many '[{\"uid\":\"s5\",\"value\":\"Alice\"},{\"uid\":\"s7\",\"value\":\"alice@example.com\"}]'

  # Fill from a JSON file
  chrome-cli form fill-many --file form-data.json"
    )]
    FillMany(FormFillManyArgs),

    /// Clear a form field's value
    #[command(
        long_about = "Clear the value of a form field identified by UID or CSS selector. \
            Sets the field to an empty string and dispatches change and input events.",
        after_long_help = "\
EXAMPLES:
  # Clear a field by UID
  chrome-cli form clear s5

  # Clear by CSS selector
  chrome-cli form clear css:#search-input"
    )]
    Clear(FormClearArgs),

    /// Upload files to a file input element
    #[command(
        long_about = "Upload one or more files to a file input element identified by UID or \
            CSS selector. The element must be an <input type=\"file\">. Multiple file paths \
            can be specified for multi-file upload inputs.",
        after_long_help = "\
EXAMPLES:
  # Upload a single file
  chrome-cli form upload s10 ./photo.jpg

  # Upload multiple files
  chrome-cli form upload css:#file-input ./doc1.pdf ./doc2.pdf"
    )]
    Upload(FormUploadArgs),
}

/// Arguments for `form fill`.
#[derive(Args)]
pub struct FormFillArgs {
    /// Target element (UID like 's1' or CSS selector like 'css:#email')
    pub target: String,

    /// Value to set on the form field
    pub value: String,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,
}

/// Arguments for `form fill-many`.
#[derive(Args)]
pub struct FormFillManyArgs {
    /// Inline JSON array of {uid, value} objects
    pub json: Option<String>,

    /// Read JSON from a file instead of inline argument
    #[arg(long)]
    pub file: Option<PathBuf>,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,
}

/// Arguments for `form clear`.
#[derive(Args)]
pub struct FormClearArgs {
    /// Target element (UID like 's1' or CSS selector like 'css:#email')
    pub target: String,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,
}

/// Arguments for `form upload`.
#[derive(Args)]
pub struct FormUploadArgs {
    /// Target file input element (UID like 's5' or CSS selector like 'css:#file-input')
    pub target: String,

    /// File paths to upload
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,
}

/// Arguments for the `console` subcommand group.
#[derive(Args)]
pub struct ConsoleArgs {
    #[command(subcommand)]
    pub command: ConsoleCommand,
}

/// Console subcommands.
#[derive(Subcommand)]
pub enum ConsoleCommand {
    /// List console messages or get details of a specific message
    #[command(
        long_about = "Read captured console messages from the current page. Without arguments, \
            lists recent messages with their IDs, types, and text. Pass a message ID to get \
            full details including stack trace and arguments. Filter by type or use --errors-only \
            for error and assert messages only.",
        after_long_help = "\
EXAMPLES:
  # List recent console messages
  chrome-cli console read

  # Get details of a specific message
  chrome-cli console read 42

  # Show only errors
  chrome-cli console read --errors-only

  # Filter by type
  chrome-cli console read --type warn,error --limit 20"
    )]
    Read(ConsoleReadArgs),

    /// Stream console messages in real-time (tail -f style)
    #[command(
        long_about = "Stream new console messages in real time as they are logged, similar to \
            'tail -f'. Each message is printed as a JSON line. Use --timeout to auto-exit \
            after a specified duration. Filter by type or use --errors-only to stream only \
            error and assert messages.",
        after_long_help = "\
EXAMPLES:
  # Stream all console output
  chrome-cli console follow

  # Stream errors only for 10 seconds
  chrome-cli console follow --errors-only --timeout 10000

  # Stream specific message types
  chrome-cli console follow --type log,warn"
    )]
    Follow(ConsoleFollowArgs),
}

/// Arguments for `console read`.
#[derive(Args)]
pub struct ConsoleReadArgs {
    /// Message ID to get detailed information about a specific message
    pub msg_id: Option<u64>,

    /// Filter by message type (comma-separated: log,error,warn,info,debug,dir,table,trace,assert,count,timeEnd)
    #[arg(long, value_name = "TYPES", conflicts_with = "errors_only")]
    pub r#type: Option<String>,

    /// Show only error and assert messages (shorthand for --type error,assert)
    #[arg(long, conflicts_with = "type")]
    pub errors_only: bool,

    /// Maximum number of messages to return
    #[arg(long, default_value_t = 50)]
    pub limit: usize,

    /// Pagination page number (0-based)
    #[arg(long, default_value_t = 0)]
    pub page: usize,

    /// Include messages from previous navigations
    #[arg(long)]
    pub include_preserved: bool,
}

/// Arguments for `console follow`.
#[derive(Args)]
pub struct ConsoleFollowArgs {
    /// Filter by message type (comma-separated: log,error,warn,info,debug,dir,table,trace,assert,count,timeEnd)
    #[arg(long, value_name = "TYPES", conflicts_with = "errors_only")]
    pub r#type: Option<String>,

    /// Show only error and assert messages (shorthand for --type error,assert)
    #[arg(long, conflicts_with = "type")]
    pub errors_only: bool,

    /// Auto-exit after the specified number of milliseconds
    #[arg(long)]
    pub timeout: Option<u64>,
}

/// Arguments for the `network` subcommand group.
#[derive(Args)]
pub struct NetworkArgs {
    #[command(subcommand)]
    pub command: NetworkCommand,
}

/// Network subcommands.
#[derive(Subcommand)]
pub enum NetworkCommand {
    /// List network requests or get details of a specific request
    #[command(
        long_about = "List captured network requests from the current page. Returns JSON with \
            each request's ID, method, URL, status, resource type, and timing. Filter by \
            resource type, URL pattern, HTTP status code, or HTTP method. Use --limit and \
            --page for pagination.",
        after_long_help = "\
EXAMPLES:
  # List recent requests
  chrome-cli network list

  # Filter by resource type
  chrome-cli network list --type xhr,fetch

  # Filter by URL pattern
  chrome-cli network list --url api.example.com

  # Filter by status code
  chrome-cli network list --status 4xx"
    )]
    List(NetworkListArgs),

    /// Get detailed information about a specific network request
    #[command(
        long_about = "Get detailed information about a specific network request by its numeric \
            ID. Returns JSON with full request and response headers, timing breakdown, and \
            body size. Use --save-request or --save-response to save the request or response \
            body to a file.",
        after_long_help = "\
EXAMPLES:
  # Get request details
  chrome-cli network get 42

  # Save the response body to a file
  chrome-cli network get 42 --save-response body.json

  # Save both request and response bodies
  chrome-cli network get 42 --save-request req.json --save-response resp.json"
    )]
    Get(NetworkGetArgs),

    /// Stream network requests in real-time (tail -f style)
    #[command(
        long_about = "Stream network requests in real time as they are made, similar to \
            'tail -f'. Each request is printed as a JSON line. Filter by resource type, \
            URL pattern, or HTTP method. Use --timeout to auto-exit after a specified \
            duration. Use --verbose to include request and response headers.",
        after_long_help = "\
EXAMPLES:
  # Stream all network requests
  chrome-cli network follow

  # Stream API requests only
  chrome-cli network follow --type xhr,fetch --url /api/

  # Stream with headers for 30 seconds
  chrome-cli network follow --verbose --timeout 30000"
    )]
    Follow(NetworkFollowArgs),
}

/// Arguments for `network list`.
#[derive(Args)]
pub struct NetworkListArgs {
    /// Filter by resource type (comma-separated: document,stylesheet,image,media,font,script,xhr,fetch,websocket,manifest,other)
    #[arg(long, value_name = "TYPES")]
    pub r#type: Option<String>,

    /// Filter by URL pattern (substring match)
    #[arg(long)]
    pub url: Option<String>,

    /// Filter by HTTP status code (exact like 404 or wildcard like 4xx)
    #[arg(long)]
    pub status: Option<String>,

    /// Filter by HTTP method (GET, POST, etc.)
    #[arg(long)]
    pub method: Option<String>,

    /// Maximum number of requests to return
    #[arg(long, default_value_t = 50)]
    pub limit: usize,

    /// Pagination page number (0-based)
    #[arg(long, default_value_t = 0)]
    pub page: usize,

    /// Include requests from previous navigations
    #[arg(long)]
    pub include_preserved: bool,
}

/// Arguments for `network get`.
#[derive(Args)]
pub struct NetworkGetArgs {
    /// Numeric request ID to inspect
    pub req_id: u64,

    /// Save request body to a file
    #[arg(long)]
    pub save_request: Option<PathBuf>,

    /// Save response body to a file
    #[arg(long)]
    pub save_response: Option<PathBuf>,
}

/// Arguments for `network follow`.
#[derive(Args)]
pub struct NetworkFollowArgs {
    /// Filter by resource type (comma-separated: document,stylesheet,image,media,font,script,xhr,fetch,websocket,manifest,other)
    #[arg(long, value_name = "TYPES")]
    pub r#type: Option<String>,

    /// Filter by URL pattern (substring match)
    #[arg(long)]
    pub url: Option<String>,

    /// Filter by HTTP method (GET, POST, etc.)
    #[arg(long)]
    pub method: Option<String>,

    /// Auto-exit after the specified number of milliseconds
    #[arg(long)]
    pub timeout: Option<u64>,

    /// Include request and response headers in stream output
    #[arg(long)]
    pub verbose: bool,
}

/// Arguments for `page resize`.
#[derive(Args)]
pub struct PageResizeArgs {
    /// Viewport size as WIDTHxHEIGHT (e.g. 1280x720)
    pub size: String,
}

/// Arguments for the `emulate` subcommand group.
#[derive(Args)]
pub struct EmulateArgs {
    #[command(subcommand)]
    pub command: EmulateCommand,
}

/// Emulate subcommands.
#[derive(Subcommand)]
pub enum EmulateCommand {
    /// Apply one or more emulation overrides
    #[command(
        long_about = "Apply one or more device or network emulation overrides. Multiple \
            overrides can be combined in a single command (e.g., viewport + network + user \
            agent). Overrides persist until 'emulate reset' is called or the browser is closed. \
            Note: --geolocation and --no-geolocation are mutually exclusive, as are \
            --user-agent and --no-user-agent.",
        after_long_help = "\
EXAMPLES:
  # Emulate a mobile device
  chrome-cli emulate set --viewport 375x667 --device-scale 2 --mobile

  # Simulate slow network
  chrome-cli emulate set --network 3g

  # Set geolocation (San Francisco)
  chrome-cli emulate set --geolocation 37.7749,-122.4194

  # Force dark mode with custom user agent
  chrome-cli emulate set --color-scheme dark --user-agent \"CustomBot/1.0\"

  # Throttle CPU (4x slowdown)
  chrome-cli emulate set --cpu 4"
    )]
    Set(EmulateSetArgs),

    /// Clear all emulation overrides
    #[command(
        long_about = "Clear all device and network emulation overrides, restoring the browser \
            to its default settings. This removes viewport, user agent, geolocation, network \
            throttling, CPU throttling, and color scheme overrides.",
        after_long_help = "\
EXAMPLES:
  # Reset all overrides
  chrome-cli emulate reset"
    )]
    Reset,

    /// Show current emulation settings
    #[command(
        long_about = "Display the current emulation state including viewport dimensions, \
            user agent, device scale factor, network conditions, CPU throttling, and color \
            scheme. Returns JSON with the active emulation configuration.",
        after_long_help = "\
EXAMPLES:
  # Check emulation status
  chrome-cli emulate status"
    )]
    Status,
}

/// Arguments for `emulate set`.
#[allow(clippy::struct_excessive_bools)]
#[derive(Args)]
pub struct EmulateSetArgs {
    /// Network condition profile: offline, slow-4g, 4g, 3g, none
    #[arg(long, value_enum)]
    pub network: Option<NetworkProfile>,

    /// CPU throttling rate (1 = no throttling, 2-20 = slowdown factor)
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=20))]
    pub cpu: Option<u32>,

    /// Set geolocation override as LAT,LONG (e.g. 37.7749,-122.4194; conflicts with --no-geolocation)
    #[arg(long, conflicts_with = "no_geolocation")]
    pub geolocation: Option<String>,

    /// Clear geolocation override (conflicts with --geolocation)
    #[arg(long, conflicts_with = "geolocation")]
    pub no_geolocation: bool,

    /// Set custom user agent string (conflicts with --no-user-agent)
    #[arg(long, conflicts_with = "no_user_agent")]
    pub user_agent: Option<String>,

    /// Reset user agent to browser default (conflicts with --user-agent)
    #[arg(long, conflicts_with = "user_agent")]
    pub no_user_agent: bool,

    /// Force color scheme: dark, light, auto
    #[arg(long, value_enum)]
    pub color_scheme: Option<ColorScheme>,

    /// Set viewport dimensions as WIDTHxHEIGHT (e.g. 375x667)
    #[arg(long)]
    pub viewport: Option<String>,

    /// Set device pixel ratio (e.g. 2.0)
    #[arg(long)]
    pub device_scale: Option<f64>,

    /// Emulate mobile device (touch events, mobile viewport)
    #[arg(long)]
    pub mobile: bool,
}

/// Network condition profiles for emulation.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum NetworkProfile {
    /// Fully offline (no network)
    Offline,
    /// Slow 4G (150ms latency, 1.6 Mbps down, 750 Kbps up)
    #[value(name = "slow-4g")]
    Slow4g,
    /// 4G (20ms latency, 4 Mbps down, 3 Mbps up)
    #[value(name = "4g")]
    FourG,
    /// 3G (100ms latency, 750 Kbps down, 250 Kbps up)
    #[value(name = "3g")]
    ThreeG,
    /// No throttling (disable network emulation)
    None,
}

/// Color scheme for emulation.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ColorScheme {
    /// Force dark mode
    Dark,
    /// Force light mode
    Light,
    /// Reset to browser default
    Auto,
}

/// Arguments for the `config` subcommand group.
#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

/// Config management subcommands.
#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Display the resolved configuration from all sources
    #[command(
        long_about = "Display the fully resolved configuration by merging all sources in \
            priority order: CLI flags > environment variables > config file > defaults. \
            Returns JSON showing every setting and its effective value. Useful for debugging \
            which settings are active.",
        after_long_help = "\
EXAMPLES:
  # Show resolved config
  chrome-cli config show

  # Show config from a specific file
  chrome-cli --config ./my-config.toml config show"
    )]
    Show,

    /// Create a default config file with commented example values
    #[command(
        long_about = "Create a new configuration file with all available settings documented \
            as comments. By default, the file is created at the XDG config directory \
            (~/.config/chrome-cli/config.toml on Linux, ~/Library/Application Support/\
            chrome-cli/config.toml on macOS). Use --path to specify a custom location. \
            Will not overwrite an existing file.",
        after_long_help = "\
EXAMPLES:
  # Create default config file
  chrome-cli config init

  # Create at a custom path
  chrome-cli config init --path ./my-config.toml"
    )]
    Init(ConfigInitArgs),

    /// Show the active config file path (or null if none)
    #[command(
        long_about = "Show the path of the active configuration file. Searches in priority \
            order: --config flag, $CHROME_CLI_CONFIG env var, project-local \
            (.chrome-cli.toml), XDG config dir, home directory (~/.chrome-cli.toml). \
            Returns JSON with {\"path\": \"...\"} or {\"path\": null} if no config file is found.",
        after_long_help = "\
EXAMPLES:
  # Show active config path
  chrome-cli config path"
    )]
    Path,
}

/// Arguments for `config init`.
#[derive(Args)]
pub struct ConfigInitArgs {
    /// Create config file at a custom path instead of the default XDG location
    #[arg(long)]
    pub path: Option<PathBuf>,
}

/// Arguments for the `completions` subcommand.
#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for (bash, zsh, fish, powershell, elvish)
    pub shell: Shell,
}

/// Arguments for the `man` subcommand.
#[derive(Args)]
pub struct ManArgs {
    /// Subcommand to display man page for (omit for top-level)
    pub command: Option<String>,
}

/// Arguments for the `examples` subcommand.
#[derive(Args)]
pub struct ExamplesArgs {
    /// Command group to show examples for (e.g., navigate, tabs, page)
    pub command: Option<String>,
}
