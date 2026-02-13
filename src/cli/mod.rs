#![allow(clippy::doc_markdown)]

use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

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
    #[arg(long, global = true)]
    pub port: Option<u16>,

    /// Chrome DevTools Protocol host address
    #[arg(long, default_value = "127.0.0.1", global = true)]
    pub host: String,

    /// Direct WebSocket URL (overrides --host and --port)
    #[arg(long, global = true)]
    pub ws_url: Option<String>,

    /// Command timeout in milliseconds
    #[arg(long, global = true)]
    pub timeout: Option<u64>,

    /// Target tab ID (defaults to the active tab)
    #[arg(long, global = true)]
    pub tab: Option<String>,

    #[command(flatten)]
    pub output: OutputFormat,
}

impl GlobalOpts {
    /// Returns the port if explicitly provided, or the default (9222).
    #[must_use]
    pub fn port_or_default(&self) -> u16 {
        self.port
            .unwrap_or(chrome_cli::connection::DEFAULT_CDP_PORT)
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Args)]
#[group(multiple = false)]
pub struct OutputFormat {
    /// Output as compact JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Output as pretty-printed JSON
    #[arg(long, global = true)]
    pub pretty: bool,

    /// Output as human-readable plain text
    #[arg(long, global = true)]
    pub plain: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Connect to or launch a Chrome instance
    #[command(
        long_about = "Connect to a running Chrome/Chromium instance via the Chrome DevTools \
            Protocol, or launch a new one. Tests the connection and prints browser metadata."
    )]
    Connect(ConnectArgs),

    /// Tab management (list, create, close, activate)
    #[command(
        long_about = "Tab management commands: list open tabs, create new tabs, close tabs, and \
            activate (focus) a specific tab. Each operation returns structured JSON with tab IDs \
            and metadata."
    )]
    Tabs(TabsArgs),

    /// URL navigation and history
    #[command(
        long_about = "Navigate to URLs, reload pages, go back/forward in history, and wait for \
            navigation events. Supports waiting for load, DOMContentLoaded, or network idle."
    )]
    Navigate(NavigateArgs),

    /// Page inspection (screenshot, text, accessibility tree, find)
    #[command(
        long_about = "Inspect the current page: capture screenshots (full page or element), \
            extract visible text, dump the accessibility tree, or search for text/elements on \
            the page."
    )]
    Page(PageArgs),

    /// DOM inspection and manipulation
    #[command(
        long_about = "Query and manipulate the DOM: select elements by CSS selector or XPath, \
            get/set attributes, read innerHTML/outerHTML, and modify element properties."
    )]
    Dom,

    /// JavaScript execution in page context
    #[command(
        long_about = "Execute JavaScript expressions or scripts in the page context. Returns \
            the result as structured JSON. Supports both synchronous expressions and async \
            functions."
    )]
    Js(JsArgs),

    /// Console message reading and monitoring
    #[command(
        long_about = "Read and monitor browser console messages (log, warn, error, info). \
            Can capture existing messages or stream new messages in real time."
    )]
    Console,

    /// Network request monitoring and interception
    #[command(
        long_about = "Monitor and intercept network requests. List recent requests, filter by \
            URL pattern or resource type, capture request/response bodies, and set up request \
            interception rules."
    )]
    Network,

    /// Mouse, keyboard, and scroll interactions
    #[command(
        long_about = "Simulate user interactions: click elements, type text, press key \
            combinations, scroll the page, hover over elements, and perform drag-and-drop \
            operations."
    )]
    Interact,

    /// Form input and submission
    #[command(
        long_about = "Fill in form fields, select dropdown options, toggle checkboxes, upload \
            files, and submit forms. Supports targeting fields by selector, name, or label."
    )]
    Form,

    /// Device and network emulation
    #[command(
        long_about = "Emulate different devices, screen sizes, and network conditions. Set \
            custom user agents, viewport dimensions, device scale factor, and network throttling \
            profiles."
    )]
    Emulate,

    /// Performance tracing and metrics
    #[command(
        long_about = "Collect performance metrics, capture trace files, measure page load timing, \
            and analyze runtime performance. Outputs metrics as structured JSON for analysis."
    )]
    Perf(PerfArgs),
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
    List(TabsListArgs),

    /// Create a new tab
    Create(TabsCreateArgs),

    /// Close one or more tabs
    Close(TabsCloseArgs),

    /// Activate (focus) a tab
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
    Back,

    /// Go forward in browser history
    Forward,

    /// Reload the current page
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
    Text(PageTextArgs),

    /// Capture the accessibility tree of the page
    Snapshot(PageSnapshotArgs),

    /// Find elements by text, CSS selector, or accessibility role
    Find(PageFindArgs),

    /// Capture a screenshot of the page, an element, or a region
    Screenshot(PageScreenshotArgs),
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
    /// Capture the entire scrollable page (not just the viewport)
    #[arg(long)]
    pub full_page: bool,

    /// Screenshot a specific element by CSS selector
    #[arg(long)]
    pub selector: Option<String>,

    /// Screenshot a specific element by accessibility UID (from snapshot)
    #[arg(long)]
    pub uid: Option<String>,

    /// Image format: png (default), jpeg, webp
    #[arg(long, value_enum, default_value_t = ScreenshotFormat::Png)]
    pub format: ScreenshotFormat,

    /// JPEG/WebP quality (0-100, ignored for PNG)
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=100))]
    pub quality: Option<u8>,

    /// Save screenshot to a file path
    #[arg(long)]
    pub file: Option<PathBuf>,

    /// Capture a specific viewport region (X,Y,WIDTH,HEIGHT)
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
    Start(PerfStartArgs),
    /// Stop the active trace and collect data
    Stop(PerfStopArgs),
    /// Analyze a specific performance insight from a trace
    Analyze(PerfAnalyzeArgs),
    /// Quick Core Web Vitals measurement
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
    /// Insight name to analyze (e.g., LCPBreakdown, RenderBlocking)
    pub insight: String,
    /// Path to a previously saved trace file
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
    Exec(JsExecArgs),
}

/// Arguments for `js exec`.
#[derive(Args)]
pub struct JsExecArgs {
    /// JavaScript code to execute (use '-' to read from stdin)
    #[arg(conflicts_with = "file")]
    pub code: Option<String>,

    /// Read JavaScript from a file
    #[arg(long)]
    pub file: Option<PathBuf>,

    /// Element UID from snapshot; function receives element as first argument
    #[arg(long)]
    pub uid: Option<String>,

    /// Do not await promise results
    #[arg(long, action = ArgAction::SetTrue)]
    pub no_await: bool,

    /// Execution timeout override in milliseconds
    #[arg(long)]
    pub timeout: Option<u64>,

    /// Truncate results exceeding this size in bytes
    #[arg(long)]
    pub max_size: Option<usize>,
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

    /// Show current connection status
    #[arg(long, conflicts_with_all = ["launch", "disconnect"])]
    pub status: bool,

    /// Disconnect and remove session file
    #[arg(long, conflicts_with_all = ["launch", "status"])]
    pub disconnect: bool,

    /// Launch Chrome in headless mode
    #[arg(long, requires = "launch")]
    pub headless: bool,

    /// Chrome release channel to launch
    #[arg(long, requires = "launch", default_value = "stable")]
    pub channel: ChromeChannel,

    /// Path to a Chrome/Chromium executable (overrides channel-based discovery)
    #[arg(long, requires = "launch")]
    pub chrome_path: Option<PathBuf>,

    /// Additional arguments to pass to Chrome (can be repeated)
    #[arg(long, requires = "launch")]
    pub chrome_arg: Vec<String>,
}
