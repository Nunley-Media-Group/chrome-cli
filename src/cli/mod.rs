#![allow(clippy::doc_markdown)]

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

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
    Js,

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
    Perf,
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
}

/// Arguments for `page text`.
#[derive(Args)]
pub struct PageTextArgs {
    /// CSS selector to extract text from a specific element
    #[arg(long)]
    pub selector: Option<String>,
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
