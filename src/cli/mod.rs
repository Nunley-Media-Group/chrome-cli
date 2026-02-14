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

    /// Automatically dismiss any dialogs that appear during command execution
    #[arg(long, global = true)]
    pub auto_dismiss_dialogs: bool,

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
    Console(ConsoleArgs),

    /// Network request monitoring and interception
    #[command(
        long_about = "Monitor and intercept network requests. List recent requests, filter by \
            URL pattern or resource type, capture request/response bodies, and stream requests \
            in real time."
    )]
    Network(NetworkArgs),

    /// Mouse, keyboard, and scroll interactions
    #[command(
        long_about = "Simulate user interactions: click elements, type text, press key \
            combinations, scroll the page, hover over elements, and perform drag-and-drop \
            operations."
    )]
    Interact(InteractArgs),

    /// Form input and submission
    #[command(
        long_about = "Fill in form fields, select dropdown options, toggle checkboxes, and clear \
            fields. Supports targeting fields by UID (from accessibility snapshot) or CSS selector."
    )]
    Form(FormArgs),

    /// Device and network emulation
    #[command(
        long_about = "Emulate different devices, screen sizes, and network conditions. Set \
            custom user agents, viewport dimensions, device scale factor, and network throttling \
            profiles."
    )]
    Emulate(EmulateArgs),

    /// Performance tracing and metrics
    #[command(
        long_about = "Collect performance metrics, capture trace files, measure page load timing, \
            and analyze runtime performance. Outputs metrics as structured JSON for analysis."
    )]
    Perf(PerfArgs),

    /// Browser dialog handling (alert, confirm, prompt, beforeunload)
    #[command(
        long_about = "Detect and handle browser JavaScript dialogs (alert, confirm, prompt, \
            beforeunload). Query whether a dialog is open, accept or dismiss it, and provide \
            prompt text. Useful for automation scripts that need to respond to dialogs \
            programmatically."
    )]
    Dialog(DialogArgs),
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

    /// Resize the viewport to the given dimensions
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
    Handle(DialogHandleArgs),

    /// Check whether a dialog is currently open
    Info,
}

/// Arguments for `dialog handle`.
#[derive(Args)]
pub struct DialogHandleArgs {
    /// Action to take on the dialog
    pub action: DialogAction,

    /// Text to provide for prompt dialogs (only used with accept action)
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
    Click(ClickArgs),

    /// Click at viewport coordinates
    ClickAt(ClickAtArgs),

    /// Hover over an element
    Hover(HoverArgs),

    /// Drag from one element to another
    Drag(DragArgs),

    /// Type text character-by-character into the focused element
    Type(TypeArgs),

    /// Press a key or key combination (e.g. Enter, Control+A)
    Key(KeyArgs),

    /// Scroll the page or a container element
    Scroll(ScrollArgs),
}

/// Arguments for `interact click`.
#[derive(Args)]
pub struct ClickArgs {
    /// Target element (UID like 's1' or CSS selector like 'css:#button')
    pub target: String,

    /// Perform a double-click instead of single click
    #[arg(long, conflicts_with = "right")]
    pub double: bool,

    /// Perform a right-click (context menu) instead of left click
    #[arg(long, conflicts_with = "double")]
    pub right: bool,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,
}

/// Arguments for `interact click-at`.
#[derive(Args)]
pub struct ClickAtArgs {
    /// X coordinate in viewport
    pub x: f64,

    /// Y coordinate in viewport
    pub y: f64,

    /// Perform a double-click instead of single click
    #[arg(long, conflicts_with = "right")]
    pub double: bool,

    /// Perform a right-click (context menu) instead of left click
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
    Fill(FormFillArgs),

    /// Fill multiple form fields at once from JSON
    FillMany(FormFillManyArgs),

    /// Clear a form field's value
    Clear(FormClearArgs),
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
    Read(ConsoleReadArgs),

    /// Stream console messages in real-time (tail -f style)
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
    List(NetworkListArgs),

    /// Get detailed information about a specific network request
    Get(NetworkGetArgs),

    /// Stream network requests in real-time (tail -f style)
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
    Set(EmulateSetArgs),

    /// Clear all emulation overrides
    Reset,

    /// Show current emulation settings
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

    /// Set geolocation override as LAT,LONG (e.g. 37.7749,-122.4194)
    #[arg(long, conflicts_with = "no_geolocation")]
    pub geolocation: Option<String>,

    /// Clear geolocation override
    #[arg(long, conflicts_with = "geolocation")]
    pub no_geolocation: bool,

    /// Set custom user agent string
    #[arg(long, conflicts_with = "no_user_agent")]
    pub user_agent: Option<String>,

    /// Reset user agent to browser default
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
