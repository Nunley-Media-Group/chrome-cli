#![allow(clippy::doc_markdown)]
// Items used by the binary crate may appear unused from the library crate's perspective.
#![allow(dead_code)]

use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

#[derive(Parser)]
#[command(
    name = "agentchrome",
    version,
    about = "Browser automation via the Chrome DevTools Protocol",
    long_about = "agentchrome is a command-line tool for browser automation via the Chrome DevTools \
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
  agentchrome connect

  # Launch a new headless Chrome and connect
  agentchrome connect --launch --headless

  # List open tabs and navigate to a URL
  agentchrome tabs list
  agentchrome navigate https://example.com

  # Take a full-page screenshot
  agentchrome page screenshot --full-page --file shot.png

  # Execute JavaScript and get the result
  agentchrome js exec \"document.title\"

  # Capture the accessibility tree and fill a form field
  agentchrome page snapshot
  agentchrome form fill s5 \"hello@example.com\"

  # Monitor console output in real time
  agentchrome console follow --timeout 5000

EXIT CODES:
  0  Success
  1  General error (invalid arguments, internal failure)
  2  Connection error (Chrome not running, session expired)
  3  Target error (tab not found, no page targets)
  4  Timeout error (navigation or trace timeout)
  5  Protocol error (CDP protocol failure, dialog handling error)

ENVIRONMENT VARIABLES:
  AGENTCHROME_PORT     CDP port number (default: 9222)
  AGENTCHROME_HOST     CDP host address (default: 127.0.0.1)
  AGENTCHROME_TIMEOUT  Default command timeout in milliseconds
  AGENTCHROME_CONFIG   Path to configuration file",
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
    #[arg(long, global = true, env = "AGENTCHROME_PORT")]
    pub port: Option<u16>,

    /// Chrome DevTools Protocol host address
    #[arg(
        long,
        default_value = "127.0.0.1",
        global = true,
        env = "AGENTCHROME_HOST"
    )]
    pub host: String,

    /// Direct WebSocket URL (overrides --host and --port)
    #[arg(long, global = true)]
    pub ws_url: Option<String>,

    /// Command timeout in milliseconds
    #[arg(long, global = true, env = "AGENTCHROME_TIMEOUT")]
    pub timeout: Option<u64>,

    /// Target tab ID (defaults to the active tab)
    #[arg(long, global = true)]
    pub tab: Option<String>,

    /// Explicit page target ID (bypasses session state; conflicts with --tab)
    #[arg(long, global = true, conflicts_with = "tab")]
    pub page_id: Option<String>,

    /// Automatically dismiss any dialogs that appear during command execution
    #[arg(long, global = true)]
    pub auto_dismiss_dialogs: bool,

    /// Path to configuration file (overrides default search)
    #[arg(long, global = true, env = "AGENTCHROME_CONFIG")]
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
pub struct OutputFormat {
    /// Output as compact JSON (mutually exclusive with --pretty, --plain)
    #[arg(long, global = true, conflicts_with_all = ["pretty", "plain"])]
    pub json: bool,

    /// Output as pretty-printed JSON (mutually exclusive with --json, --plain)
    #[arg(long, global = true, conflicts_with_all = ["json", "plain"])]
    pub pretty: bool,

    /// Output as human-readable plain text (mutually exclusive with --json, --pretty)
    #[arg(long, global = true, conflicts_with_all = ["json", "pretty"])]
    pub plain: bool,

    /// Byte threshold for large-response detection (default: 16384)
    #[arg(long, global = true, value_parser = parse_nonzero_usize)]
    pub large_response_threshold: Option<usize>,
}

fn parse_nonzero_usize(s: &str) -> Result<usize, String> {
    let val: usize = s.parse().map_err(|e| format!("{e}"))?;
    if val == 0 {
        return Err("threshold must be greater than 0".to_string());
    }
    Ok(val)
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
  agentchrome connect

  # Launch a new headless Chrome instance
  agentchrome connect --launch --headless

  # Connect to a specific port
  agentchrome connect --port 9333

  # Check connection status
  agentchrome connect --status

  # Disconnect and remove session file
  agentchrome connect --disconnect"
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
  agentchrome tabs list

  # Open a new tab and get its ID
  agentchrome tabs create https://example.com

  # Close tabs by ID
  agentchrome tabs close ABC123 DEF456

  # Activate a specific tab
  agentchrome tabs activate ABC123"
    )]
    Tabs(TabsArgs),

    /// URL navigation and history
    #[command(
        long_about = "Navigate to URLs, reload pages, go back/forward in history, and wait for \
            navigation events. Supports waiting for load, DOMContentLoaded, or network idle.",
        after_long_help = "\
EXAMPLES:
  # Navigate to a URL and wait for page load
  agentchrome navigate https://example.com

  # Navigate and wait for network idle
  agentchrome navigate https://example.com --wait-until networkidle

  # Go back in browser history
  agentchrome navigate back

  # Reload the current page, bypassing cache
  agentchrome navigate reload --ignore-cache"
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
  agentchrome page text

  # Capture the accessibility tree (assigns UIDs to elements)
  agentchrome page snapshot

  # Take a full-page screenshot
  agentchrome page screenshot --full-page --file page.png

  # Find elements by text
  agentchrome page find \"Sign in\"

  # Resize the viewport
  agentchrome page resize 1280x720"
    )]
    Page(PageArgs),

    /// DOM inspection and manipulation
    #[command(
        long_about = "Query and manipulate the DOM: select elements by CSS selector or XPath, \
            get/set attributes and text, read outerHTML, inspect computed styles, navigate the \
            element tree, and remove elements. Target elements by node ID (from 'dom select'), \
            snapshot UID (from 'page snapshot'), or CSS selector (prefixed with 'css:').",
        after_long_help = "\
EXAMPLES:
  # Select elements by CSS selector
  agentchrome dom select \"h1\"

  # Select by XPath
  agentchrome dom select \"//a[@href]\" --xpath

  # Get an element's attribute
  agentchrome dom get-attribute s3 href

  # Read element text
  agentchrome dom get-text css:h1

  # Set an attribute
  agentchrome dom set-attribute s5 class \"highlight\"

  # View the DOM tree
  agentchrome dom tree --depth 3"
    )]
    Dom(DomArgs),

    /// JavaScript execution in page context
    #[command(
        long_about = "Execute JavaScript expressions or scripts in the page context. Returns \
            the result as structured JSON. Supports both synchronous expressions and async \
            functions.",
        after_long_help = "\
EXAMPLES:
  # Get the page title
  agentchrome js exec \"document.title\"

  # Execute a script file
  agentchrome js exec --file script.js

  # Run code on a specific element (by UID from snapshot)
  agentchrome js exec --uid s3 \"(el) => el.textContent\"

  # Read from stdin
  echo 'document.URL' | agentchrome js exec -"
    )]
    Js(JsArgs),

    /// Console message reading and monitoring
    #[command(
        long_about = "Read and monitor browser console messages (log, warn, error, info). \
            Can capture existing messages or stream new messages in real time.",
        after_long_help = "\
EXAMPLES:
  # Read recent console messages
  agentchrome console read

  # Show only error messages
  agentchrome console read --errors-only

  # Stream console messages in real time
  agentchrome console follow

  # Stream errors for 10 seconds
  agentchrome console follow --errors-only --timeout 10000"
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
  agentchrome network list

  # Filter by resource type
  agentchrome network list --type xhr,fetch

  # Get details of a specific request
  agentchrome network get 42

  # Stream network requests in real time
  agentchrome network follow --url api.example.com"
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
  agentchrome interact click s5

  # Click by CSS selector
  agentchrome interact click css:#submit-btn

  # Type text into the focused element
  agentchrome interact type \"Hello, world!\"

  # Press a key combination
  agentchrome interact key Control+A

  # Scroll down one viewport height
  agentchrome interact scroll"
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
  agentchrome form fill s5 \"hello@example.com\"

  # Fill by CSS selector
  agentchrome form fill css:#email \"user@example.com\"

  # Fill multiple fields at once
  agentchrome form fill-many '[{\"uid\":\"s5\",\"value\":\"Alice\"},{\"uid\":\"s7\",\"value\":\"alice@example.com\"}]'

  # Clear a field
  agentchrome form clear s5

  # Upload a file
  agentchrome form upload s10 ./photo.jpg"
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
  agentchrome emulate set --viewport 375x667 --device-scale 2 --mobile

  # Simulate slow 3G network
  agentchrome emulate set --network 3g

  # Force dark mode
  agentchrome emulate set --color-scheme dark

  # Check current emulation settings
  agentchrome emulate status

  # Clear all emulation overrides
  agentchrome emulate reset"
    )]
    Emulate(EmulateArgs),

    /// Performance tracing and metrics
    #[command(
        long_about = "Collect performance metrics, capture trace files, measure page load timing, \
            and analyze runtime performance. Outputs metrics as structured JSON for analysis.",
        after_long_help = "\
EXAMPLES:
  # Quick Core Web Vitals measurement
  agentchrome perf vitals

  # Record a trace until Ctrl+C
  agentchrome perf record

  # Record a trace for 5 seconds
  agentchrome perf record --duration 5000

  # Record with page reload
  agentchrome perf record --reload --duration 5000

  # Analyze a trace for render-blocking resources
  agentchrome perf analyze RenderBlocking --trace-file trace.json"
    )]
    Perf(PerfArgs),

    /// Browser cookie management (list, set, delete, clear)
    #[command(
        long_about = "Manage browser cookies via the Chrome DevTools Protocol. List cookies \
            for the current page or all cookies, set new cookies with optional flags, delete \
            specific cookies by name, or clear all cookies. Provides full access to HttpOnly \
            and Secure cookies that are not accessible via document.cookie.",
        after_long_help = "\
EXAMPLES:
  # List cookies for the current page
  agentchrome cookie list

  # List all cookies (not scoped to current URL)
  agentchrome cookie list --all

  # List cookies filtered by domain
  agentchrome cookie list --domain example.com

  # Set a cookie
  agentchrome cookie set session_id abc123 --domain example.com

  # Set a secure, HttpOnly cookie with expiry
  agentchrome cookie set token xyz --domain example.com --secure --http-only --same-site Strict --expires 1735689600

  # Delete a specific cookie
  agentchrome cookie delete session_id --domain example.com

  # Clear all cookies
  agentchrome cookie clear"
    )]
    Cookie(CookieArgs),

    /// Browser dialog handling (alert, confirm, prompt, beforeunload)
    #[command(
        long_about = "Detect and handle browser JavaScript dialogs (alert, confirm, prompt, \
            beforeunload). Query whether a dialog is open, accept or dismiss it, and provide \
            prompt text. Useful for automation scripts that need to respond to dialogs \
            programmatically.",
        after_long_help = "\
EXAMPLES:
  # Check if a dialog is open
  agentchrome dialog info

  # Accept an alert or confirm dialog
  agentchrome dialog handle accept

  # Dismiss a dialog
  agentchrome dialog handle dismiss

  # Accept a prompt with text
  agentchrome dialog handle accept --text \"my input\""
    )]
    Dialog(DialogArgs),

    /// Media element control (list, play, pause, seek)
    #[command(
        long_about = "Discover and control HTML5 audio and video elements on the current page. \
            List all media elements with playback state, play/pause individual elements, seek to \
            a specific time or to the end of the media. Supports targeting by index or CSS \
            selector, bulk operations with --all, and frame-scoped media control with --frame.",
        after_long_help = "\
EXAMPLES:
  # List all media elements on the page
  agentchrome media list

  # Play a media element by index
  agentchrome media play 0

  # Pause a media element
  agentchrome media pause 0

  # Seek to 15.5 seconds
  agentchrome media seek 0 15.5

  # Seek all media elements to end (skip narration gates)
  agentchrome media seek-end --all

  # List media elements inside an iframe
  agentchrome media --frame 0 list

  # Play a media element by CSS selector
  agentchrome media play css:audio.narration"
    )]
    Media(MediaArgs),

    /// Run audits against the current page (Lighthouse)
    #[command(
        long_about = "Run external audits against the current browser page. Currently supports \
            Google Lighthouse for measuring performance, accessibility, SEO, best practices, \
            and PWA scores. Connects Lighthouse to the managed Chrome session via the CDP port \
            and returns structured JSON category scores on stdout.",
        after_long_help = "\
EXAMPLES:
  # Run a full Lighthouse audit on the current page
  agentchrome audit lighthouse

  # Audit a specific URL
  agentchrome audit lighthouse https://example.com

  # Only measure performance and accessibility
  agentchrome audit lighthouse --only performance,accessibility

  # Save the full Lighthouse report to a file
  agentchrome audit lighthouse --output-file report.json"
    )]
    Audit(AuditArgs),

    /// Agentic tool skill installation and management
    #[command(
        long_about = "Install, update, uninstall, or list agentchrome skill files for agentic \
            coding tools (Claude Code, Windsurf, Aider, Continue.dev, GitHub Copilot, Cursor). \
            The skill file is a minimal signpost that tells the AI agent what agentchrome is \
            and how to discover its capabilities via the CLI's built-in help system. Auto-detects \
            the active agentic environment, or use --tool to target a specific tool.",
        after_long_help = "\
EXAMPLES:
  # Auto-detect and install
  agentchrome skill install

  # Install for a specific tool
  agentchrome skill install --tool claude-code

  # List supported tools and installation status
  agentchrome skill list

  # Update installed skill to current version
  agentchrome skill update --tool claude-code

  # Remove an installed skill
  agentchrome skill uninstall --tool claude-code"
    )]
    Skill(SkillArgs),

    /// Configuration file management (show, init, path)
    #[command(
        long_about = "Manage the agentchrome configuration file. Show the resolved configuration \
            from all sources, create a default config file, or display the active config file path. \
            Config files use TOML format and are searched in priority order: --config flag, \
            $AGENTCHROME_CONFIG env var, project-local, XDG config dir, home directory.",
        after_long_help = "\
EXAMPLES:
  # Show the resolved configuration
  agentchrome config show

  # Create a default config file
  agentchrome config init

  # Create a config at a custom path
  agentchrome config init --path ./my-config.toml

  # Show the active config file path
  agentchrome config path"
    )]
    Config(ConfigArgs),

    /// Generate shell completion scripts
    #[command(
        long_about = "Generate shell completion scripts for tab-completion of commands, flags, \
            and enum values. Pipe the output to the appropriate file for your shell.",
        after_long_help = "\
EXAMPLES:
  # Bash
  agentchrome completions bash > /etc/bash_completion.d/agentchrome

  # Zsh
  agentchrome completions zsh > ~/.zfunc/_agentchrome

  # Fish
  agentchrome completions fish > ~/.config/fish/completions/agentchrome.fish

  # PowerShell
  agentchrome completions powershell >> $PROFILE

  # Elvish
  agentchrome completions elvish >> ~/.elvish/rc.elv"
    )]
    Completions(CompletionsArgs),

    /// Show usage examples for commands
    #[command(
        long_about = "Show usage examples for agentchrome commands. Without arguments, lists all \
            command groups with a brief description and one example each. With a command name, \
            shows detailed examples for that specific command group.",
        after_long_help = "\
EXAMPLES:
  # List all command groups with summary examples
  agentchrome examples

  # Show detailed examples for the navigate command
  agentchrome examples navigate

  # Get all examples as JSON (for programmatic use)
  agentchrome examples --json

  # Pretty-printed JSON output
  agentchrome examples --pretty"
    )]
    Examples(ExamplesArgs),

    /// Output a machine-readable manifest of all CLI capabilities
    #[command(
        long_about = "Output a complete, machine-readable JSON manifest describing every command, \
            subcommand, flag, argument, and type in the CLI. Designed for AI agents and tooling \
            that need to programmatically discover the CLI surface. The manifest is generated at \
            runtime from the clap command tree, so it is always in sync with the binary.",
        after_long_help = "\
EXAMPLES:
  # Full capabilities manifest
  agentchrome capabilities

  # Pretty-printed for readability
  agentchrome capabilities --pretty

  # Capabilities for a specific command
  agentchrome capabilities --command navigate

  # Compact listing (names and descriptions only)
  agentchrome capabilities --compact"
    )]
    Capabilities(CapabilitiesArgs),

    /// Display man pages for agentchrome commands
    #[command(
        long_about = "Display man pages for agentchrome commands. Without arguments, displays \
            the main agentchrome man page. With a subcommand name, displays the man page for \
            that specific command. Output is in roff format, suitable for piping to a pager.",
        after_long_help = "\
EXAMPLES:
  # Display the main agentchrome man page
  agentchrome man

  # Display the man page for the connect command
  agentchrome man connect

  # Display the man page for the tabs command
  agentchrome man tabs

  # Pipe to a pager
  agentchrome man navigate | less"
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
  agentchrome tabs list

  # Include internal Chrome pages
  agentchrome tabs list --all"
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
  agentchrome tabs create

  # Open a URL
  agentchrome tabs create https://example.com

  # Open in the background
  agentchrome tabs create https://example.com --background"
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
  agentchrome tabs close ABC123

  # Close multiple tabs
  agentchrome tabs close ABC123 DEF456 GHI789"
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
  agentchrome tabs activate ABC123

  # Activate silently
  agentchrome tabs activate ABC123 --quiet"
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
  agentchrome navigate back"
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
  agentchrome navigate forward"
    )]
    Forward,

    /// Reload the current page
    #[command(
        long_about = "Reload the current page. Use --ignore-cache to bypass the browser cache \
            and force a full reload from the server. Returns JSON with the page URL after reload.",
        after_long_help = "\
EXAMPLES:
  # Reload the page
  agentchrome navigate reload

  # Reload bypassing cache
  agentchrome navigate reload --ignore-cache"
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

    /// Wait for a CSS selector to appear after the page loads.
    /// Useful for SPA sites where content renders asynchronously after the load event.
    /// Example: --wait-for-selector "div.email-list"
    #[arg(long)]
    pub wait_for_selector: Option<String>,
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
    /// Target frame by index, path (1/0), or 'auto'
    #[arg(long)]
    pub frame: Option<String>,

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
  agentchrome page text

  # Get text from a specific element
  agentchrome page text --selector \"#main-content\""
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
  agentchrome page snapshot

  # Verbose output with extra properties
  agentchrome page snapshot --verbose

  # Save to a file
  agentchrome page snapshot --file snapshot.txt"
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
  agentchrome page find \"Sign in\"

  # Find by CSS selector
  agentchrome page find --selector \"button.primary\"

  # Find by accessibility role
  agentchrome page find --role button

  # Exact text match with limit
  agentchrome page find \"Submit\" --exact --limit 1"
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
  agentchrome page screenshot --file shot.png

  # Full-page screenshot
  agentchrome page screenshot --full-page --file full.png

  # Screenshot a specific element by UID
  agentchrome page screenshot --uid s3 --file element.png

  # JPEG format with quality
  agentchrome page screenshot --format jpeg --quality 80 --file shot.jpg"
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
  agentchrome page resize 1280x720

  # Mobile viewport
  agentchrome page resize 375x667"
    )]
    Resize(PageResizeArgs),

    /// Query a single element's properties by UID or CSS selector
    #[command(
        long_about = "Query a single element's state by accessibility UID (from 'page snapshot') \
            or CSS selector. Returns role, name, tag name, bounding box, accessibility properties, \
            and viewport visibility as JSON.",
        after_long_help = "\
EXAMPLES:
  # Query by UID
  agentchrome page element s10

  # Query by CSS selector
  agentchrome page element \"css:#checkout\"

  # Plain text output
  agentchrome page element s10 --plain"
    )]
    Element(PageElementArgs),

    /// Wait until a condition is met on the current page
    #[command(
        arg_required_else_help = true,
        long_about = "Wait until a specified condition is met on the current page. Supports \
            waiting for a URL to match a glob pattern, text to appear, a CSS selector to match, \
            network activity to settle, or a JavaScript expression to evaluate to truthy. \
            Exactly one condition must be specified. The command blocks until the condition is \
            satisfied or the timeout is reached.",
        after_long_help = "\
EXAMPLES:
  # Wait for URL to match a glob pattern
  agentchrome page wait --url \"*/dashboard*\"

  # Wait for text to appear
  agentchrome page wait --text \"Products\"

  # Wait for a CSS selector to match
  agentchrome page wait --selector \"#results-table\"

  # Wait for at least 5 elements to match a selector
  agentchrome page wait --selector \".item\" --count 5

  # Wait for network to settle
  agentchrome page wait --network-idle

  # Wait for a JavaScript expression to become truthy
  agentchrome page wait --js-expression \"document.querySelector('.btn').disabled === false\"

  # Wait for audio element to finish playing
  agentchrome page wait --js-expression \"document.querySelector('audio').ended\"

  # Custom timeout and poll interval
  agentchrome page wait --text \"loaded\" --timeout 5000 --interval 200"
    )]
    Wait(PageWaitArgs),

    /// List all frames (iframes, framesets) in the page hierarchy
    #[command(
        long_about = "List all frames in the current page, including iframes and frameset frames. \
            Returns a JSON array with each frame's index, ID, URL, name, security origin, \
            dimensions, and nesting depth. Use the index with --frame on other commands to target \
            a specific frame.",
        after_long_help = "\
EXAMPLES:
  # List all frames
  agentchrome page frames

  # Pretty-printed output
  agentchrome page --pretty frames"
    )]
    Frames,

    /// List all workers (service, shared, dedicated) associated with the page
    #[command(
        long_about = "List all workers associated with the current page, including Service Workers, \
            Shared Workers, and dedicated Web Workers. Returns a JSON array with each worker's \
            index, target ID, type, script URL, and status.",
        after_long_help = "\
EXAMPLES:
  # List all workers
  agentchrome page workers"
    )]
    Workers,

    /// Hit test at viewport coordinates to identify click targets and overlays
    #[command(
        name = "hittest",
        long_about = "Hit test at the given viewport coordinates to identify which element \
            receives a click event. Returns the actual hit target, any intercepting overlay \
            elements, and the full z-index stack at those coordinates. Useful for debugging \
            failed click interactions caused by invisible overlays.",
        after_long_help = "\
EXAMPLES:
  # Hit test at viewport coordinates
  agentchrome page hittest 100 200

  # Hit test within a specific iframe
  agentchrome page hittest 50 50 --frame 1"
    )]
    HitTest(PageHitTestArgs),

    /// Analyze page structure: iframes, frameworks, overlays, media, shadow DOM
    #[command(
        long_about = "Analyze the structural composition of the current page. Returns a JSON \
            report covering iframe hierarchy, detected frontend frameworks, interactive element \
            counts, media elements, overlay blockers, and shadow DOM presence. Useful for \
            understanding an unfamiliar page before choosing an automation strategy.",
        after_long_help = "\
EXAMPLES:
  # Analyze current page
  agentchrome page analyze

  # Analyze within a specific iframe
  agentchrome page analyze --frame 1"
    )]
    Analyze,
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

    /// CSS selector for the inner scrollable element (requires --full-page)
    #[arg(long)]
    pub scroll_container: Option<String>,

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

    /// Return only interactive and semantically meaningful elements (reduces token usage for AI agents)
    #[arg(long)]
    pub compact: bool,

    /// Include shadow DOM content in the accessibility tree
    #[arg(long)]
    pub pierce_shadow: bool,
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
    /// Record a performance trace (long-running, stops on Ctrl+C or --duration)
    #[command(
        long_about = "Record a performance trace in a single long-running session. The trace \
            captures JavaScript execution, layout, paint, network, and other browser activity. \
            Recording continues until you press Ctrl+C or the --duration timeout elapses. \
            Use --reload to reload the page before recording. The trace is saved to a JSON \
            file that can be opened in Chrome DevTools or analyzed with 'perf analyze'.",
        after_long_help = "\
EXAMPLES:
  # Record until Ctrl+C
  agentchrome perf record

  # Record for 5 seconds
  agentchrome perf record --duration 5000

  # Record with page reload
  agentchrome perf record --reload --duration 5000

  # Save to a specific file
  agentchrome perf record --file my-trace.json"
    )]
    Record(PerfRecordArgs),

    /// Analyze a specific performance insight from a trace
    #[command(
        long_about = "Analyze a previously saved trace file for a specific performance insight. \
            Available insights: DocumentLatency (document request timing), LCPBreakdown (Largest \
            Contentful Paint phases), RenderBlocking (render-blocking resources), LongTasks \
            (JavaScript tasks > 50ms). Returns structured JSON with the analysis results.",
        after_long_help = "\
EXAMPLES:
  # Analyze LCP breakdown
  agentchrome perf analyze LCPBreakdown --trace-file trace.json

  # Find render-blocking resources
  agentchrome perf analyze RenderBlocking --trace-file trace.json

  # Identify long tasks
  agentchrome perf analyze LongTasks --trace-file trace.json"
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
  agentchrome perf vitals

  # Save the underlying trace file
  agentchrome perf vitals --file vitals-trace.json"
    )]
    Vitals(PerfVitalsArgs),
}

/// Arguments for `perf record`.
#[derive(Args)]
pub struct PerfRecordArgs {
    /// Reload the page before recording
    #[arg(long)]
    pub reload: bool,
    /// Auto-stop after this many milliseconds
    #[arg(long)]
    pub duration: Option<u64>,
    /// Path to save the trace file (default: auto-generated)
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
    /// Target frame by index, path (1/0), or 'auto'
    #[arg(long)]
    pub frame: Option<String>,

    #[command(subcommand)]
    pub command: JsCommand,
}

/// JavaScript subcommands.
#[derive(Subcommand)]
pub enum JsCommand {
    /// Execute JavaScript in the page context
    #[command(
        long_about = "Execute a JavaScript expression or script in the page context and return \
            the result as JSON. Code can be provided as an inline argument, via --code (recommended \
            for cross-platform quoting), read from a file with --file, or piped via stdin using \
            '--stdin' or '-'. When --uid is specified, the code is wrapped in a function that \
            receives the element as its first argument. By default, promise results are awaited; \
            use --no-await to return immediately.",
        after_long_help = "\
EXAMPLES:
  # Evaluate an expression
  agentchrome js exec \"document.title\"

  # Use --code for cross-platform quoting (recommended on Windows)
  agentchrome js exec --code \"document.querySelector('div')\"

  # Execute a script file
  agentchrome js exec --file script.js

  # Run code on a specific element
  agentchrome js exec --uid s3 \"(el) => el.textContent\"

  # Read from stdin
  echo 'document.URL' | agentchrome js exec --stdin

  # Legacy stdin syntax (also works)
  echo 'document.URL' | agentchrome js exec -

  # Skip awaiting promises
  agentchrome js exec --no-await \"fetch('/api/data')\""
    )]
    Exec(JsExecArgs),
}

/// Arguments for `js exec`.
#[derive(Args)]
pub struct JsExecArgs {
    /// JavaScript code to execute (use '-' to read from stdin)
    #[arg(conflicts_with_all = ["file", "code_flag", "stdin"])]
    pub code: Option<String>,

    /// JavaScript code as a named argument (avoids shell quoting issues on Windows)
    #[arg(long = "code", id = "code_flag", conflicts_with_all = ["code", "file", "stdin"])]
    pub code_flag: Option<String>,

    /// Read JavaScript code from stdin
    #[arg(long, conflicts_with_all = ["code", "code_flag", "file"])]
    pub stdin: bool,

    /// Read JavaScript from a file instead of inline argument
    #[arg(long, conflicts_with_all = ["code", "code_flag", "stdin"])]
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

    /// Worker index from 'page workers' for executing JS in a worker context
    #[arg(long)]
    pub worker: Option<u32>,
}

/// Arguments for the `cookie` subcommand group.
#[derive(Args)]
pub struct CookieArgs {
    #[command(subcommand)]
    pub command: CookieCommand,
}

/// Cookie subcommands.
#[derive(Subcommand)]
pub enum CookieCommand {
    /// List cookies for the current page or all cookies
    #[command(
        long_about = "List cookies associated with the current page. By default, returns cookies \
            scoped to the current page's URLs. Use --all to list all browser cookies regardless \
            of URL. Use --domain to filter by a specific domain.",
        after_long_help = "\
EXAMPLES:
  # List cookies for the current page
  agentchrome cookie list

  # List all cookies
  agentchrome cookie list --all

  # Filter by domain
  agentchrome cookie list --domain example.com"
    )]
    List(CookieListArgs),

    /// Set a browser cookie
    #[command(
        long_about = "Set a browser cookie with the given name and value. The --domain flag is \
            strongly recommended to scope the cookie correctly. Additional flags control path, \
            security attributes, SameSite policy, and expiry time.",
        after_long_help = "\
EXAMPLES:
  # Set a basic cookie
  agentchrome cookie set session_id abc123 --domain example.com

  # Set a secure, HttpOnly cookie
  agentchrome cookie set token xyz --domain example.com --secure --http-only

  # Set a cookie with SameSite and expiry
  agentchrome cookie set prefs dark --domain example.com --same-site Lax --expires 1735689600"
    )]
    Set(CookieSetArgs),

    /// Delete a specific cookie by name
    #[command(
        long_about = "Delete a cookie by name. Use --domain to scope the deletion to a specific \
            domain when multiple cookies share the same name across different domains.",
        after_long_help = "\
EXAMPLES:
  # Delete a cookie by name
  agentchrome cookie delete session_id

  # Delete a cookie scoped to a specific domain
  agentchrome cookie delete session_id --domain example.com"
    )]
    Delete(CookieDeleteArgs),

    /// Clear all cookies
    #[command(
        long_about = "Remove all browser cookies. Returns the number of cookies that were cleared.",
        after_long_help = "\
EXAMPLES:
  # Clear all cookies
  agentchrome cookie clear"
    )]
    Clear,
}

/// Arguments for `cookie list`.
#[derive(Args)]
pub struct CookieListArgs {
    /// Filter cookies by domain
    #[arg(long)]
    pub domain: Option<String>,

    /// List all cookies (not scoped to current URL)
    #[arg(long)]
    pub all: bool,
}

/// Arguments for `cookie set`.
#[derive(Args)]
pub struct CookieSetArgs {
    /// Cookie name
    pub name: String,

    /// Cookie value
    pub value: String,

    /// Cookie domain (strongly recommended)
    #[arg(long)]
    pub domain: Option<String>,

    /// Cookie path
    #[arg(long, default_value = "/")]
    pub path: String,

    /// Set cookie as Secure (HTTPS only)
    #[arg(long)]
    pub secure: bool,

    /// Set cookie as HttpOnly (not accessible via JavaScript)
    #[arg(long)]
    pub http_only: bool,

    /// SameSite attribute: Strict, Lax, or None
    #[arg(long, value_name = "POLICY")]
    pub same_site: Option<String>,

    /// Expiry as Unix timestamp (seconds since epoch)
    #[arg(long)]
    pub expires: Option<f64>,
}

/// Arguments for `cookie delete`.
#[derive(Args)]
pub struct CookieDeleteArgs {
    /// Cookie name to delete
    pub name: String,

    /// Scope deletion to a specific domain
    #[arg(long)]
    pub domain: Option<String>,
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
  agentchrome dialog handle accept

  # Dismiss a confirm dialog
  agentchrome dialog handle dismiss

  # Accept a prompt with text
  agentchrome dialog handle accept --text \"my response\""
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
  agentchrome dialog info"
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

/// Arguments for the `media` subcommand group.
#[derive(Args)]
pub struct MediaArgs {
    /// Target frame by index, path (1/0), or 'auto'
    #[arg(long)]
    pub frame: Option<String>,

    #[command(subcommand)]
    pub command: MediaCommand,
}

/// Media subcommands.
#[derive(Subcommand)]
pub enum MediaCommand {
    /// List all audio and video elements on the page
    #[command(
        long_about = "Enumerate all HTML5 <audio> and <video> elements on the page and return \
            their playback state. Each element includes its index (for targeting), tag type, \
            source URLs, duration, current time, playback state, mute/volume status, and \
            readyState. Returns an empty array if no media elements exist.",
        after_long_help = "\
EXAMPLES:
  # List all media elements
  agentchrome media list

  # List media in a specific iframe
  agentchrome media --frame 0 list"
    )]
    List,

    /// Play a media element
    #[command(
        long_about = "Start playback of a media element identified by index (from 'media list') \
            or CSS selector (prefixed with 'css:'). Returns the updated playback state as JSON. \
            Use --all to play all media elements on the page.",
        after_long_help = "\
EXAMPLES:
  # Play by index
  agentchrome media play 0

  # Play by CSS selector
  agentchrome media play css:audio.narration

  # Play all media elements
  agentchrome media play --all"
    )]
    Play(MediaTargetArgs),

    /// Pause a media element
    #[command(
        long_about = "Pause playback of a media element identified by index or CSS selector. \
            Returns the updated playback state as JSON. Use --all to pause all media elements.",
        after_long_help = "\
EXAMPLES:
  # Pause by index
  agentchrome media pause 0

  # Pause all media elements
  agentchrome media pause --all"
    )]
    Pause(MediaTargetArgs),

    /// Seek a media element to a specific time
    #[command(
        long_about = "Set the current playback position of a media element to a specific time \
            in seconds. The time is clamped to the element's duration by the browser. Returns \
            the updated playback state as JSON. Use --all with --time to seek all elements.",
        after_long_help = "\
EXAMPLES:
  # Seek to 15.5 seconds
  agentchrome media seek 0 15.5

  # Seek all elements to 10 seconds
  agentchrome media seek --all --time 10.0"
    )]
    Seek(MediaSeekArgs),

    /// Seek a media element to its end (duration)
    #[command(
        long_about = "Set the current playback position of a media element to its total duration, \
            effectively ending playback. This is the primary use case for skipping audio narration \
            gates in SCORM courses. Returns the updated playback state as JSON. Use --all to seek \
            all media elements to their end.",
        after_long_help = "\
EXAMPLES:
  # Seek a specific element to end
  agentchrome media seek-end 0

  # Seek all media elements to end
  agentchrome media seek-end --all"
    )]
    SeekEnd(MediaTargetArgs),
}

/// Arguments for media commands that target a single element or all elements.
#[derive(Args)]
pub struct MediaTargetArgs {
    /// Media element index (from 'media list') or CSS selector (prefixed with 'css:')
    pub target: Option<String>,

    /// Apply to all media elements on the page
    #[arg(long, conflicts_with = "target")]
    pub all: bool,
}

/// Arguments for `media seek`.
#[derive(Args)]
pub struct MediaSeekArgs {
    /// Media element index or CSS selector (not required with --all)
    pub target: Option<String>,

    /// Time in seconds to seek to (positional for single target, --time for --all)
    #[arg(conflicts_with = "time")]
    pub time_pos: Option<f64>,

    /// Apply to all media elements on the page
    #[arg(long, conflicts_with = "target")]
    pub all: bool,

    /// Time in seconds to seek to (use with --all)
    #[arg(long, conflicts_with = "time_pos")]
    pub time: Option<f64>,
}

/// Arguments for the `audit` subcommand group.
#[derive(Args)]
pub struct AuditArgs {
    #[command(subcommand)]
    pub command: AuditCommand,
}

/// Audit subcommands.
#[derive(Subcommand)]
pub enum AuditCommand {
    /// Run a Google Lighthouse audit
    #[command(
        long_about = "Run a Google Lighthouse audit against the current page (or a given URL). \
            Requires the `lighthouse` CLI to be installed (`npm install -g lighthouse`). Connects \
            Lighthouse to the managed Chrome session via the CDP port and returns structured JSON \
            category scores on stdout. Use --only to limit which categories are measured. Use \
            --output-file to save the full Lighthouse JSON report.",
        after_long_help = "\
EXAMPLES:
  # Full audit on the current page
  agentchrome audit lighthouse

  # Audit a specific URL
  agentchrome audit lighthouse https://example.com

  # Only performance and accessibility
  agentchrome audit lighthouse --only performance,accessibility

  # Save the full report
  agentchrome audit lighthouse --output-file report.json"
    )]
    Lighthouse(AuditLighthouseArgs),
}

/// Arguments for `audit lighthouse`.
#[derive(Args)]
pub struct AuditLighthouseArgs {
    /// URL to audit (defaults to the active page URL)
    pub url: Option<String>,

    /// Comma-separated list of categories to measure (e.g. performance,accessibility)
    #[arg(long)]
    pub only: Option<String>,

    /// Save the full Lighthouse JSON report to this file
    #[arg(long)]
    pub output_file: Option<PathBuf>,
}

/// Arguments for the `skill` subcommand group.
#[derive(Args)]
pub struct SkillArgs {
    #[command(subcommand)]
    pub command: SkillCommand,
}

/// Skill subcommands.
#[derive(Subcommand)]
pub enum SkillCommand {
    /// Install the agentchrome skill for an agentic coding tool
    #[command(
        long_about = "Install a concise agentchrome skill/instruction file for the detected (or \
            specified) agentic coding tool. The skill tells the AI agent what agentchrome is \
            and how to discover its capabilities. Re-running install overwrites the existing \
            skill file (idempotent).",
        after_long_help = "\
EXAMPLES:
  # Auto-detect tool and install
  agentchrome skill install

  # Install for a specific tool
  agentchrome skill install --tool claude-code"
    )]
    Install(SkillInstallArgs),

    /// Remove a previously installed agentchrome skill
    #[command(
        long_about = "Remove a previously installed agentchrome skill file for the detected (or \
            specified) agentic coding tool. For tools with shared rule files, only the \
            agentchrome section is removed.",
        after_long_help = "\
EXAMPLES:
  # Auto-detect and uninstall
  agentchrome skill uninstall

  # Uninstall for a specific tool
  agentchrome skill uninstall --tool cursor"
    )]
    Uninstall(SkillToolArgs),

    /// Update an installed skill to the current version
    #[command(
        long_about = "Replace an installed agentchrome skill file with the current version's \
            content. Errors if no skill is currently installed for the tool.",
        after_long_help = "\
EXAMPLES:
  # Update for auto-detected tool
  agentchrome skill update

  # Update for a specific tool
  agentchrome skill update --tool claude-code"
    )]
    Update(SkillToolArgs),

    /// List supported agentic tools and installation status
    #[command(
        long_about = "List all supported agentic coding tools with their detection method, \
            install path, and whether a skill is currently installed.",
        after_long_help = "\
EXAMPLES:
  # List all tools
  agentchrome skill list"
    )]
    List,
}

/// Arguments for `skill install`.
#[derive(Args)]
pub struct SkillInstallArgs {
    /// Target tool (auto-detected if omitted)
    #[arg(long, value_enum)]
    pub tool: Option<ToolName>,
}

/// Arguments for `skill uninstall` and `skill update`.
#[derive(Args)]
pub struct SkillToolArgs {
    /// Target tool (auto-detected if omitted)
    #[arg(long, value_enum)]
    pub tool: Option<ToolName>,
}

/// Supported agentic coding tools.
#[derive(Debug, Clone, ValueEnum)]
pub enum ToolName {
    ClaudeCode,
    Windsurf,
    Aider,
    Continue,
    CopilotJb,
    Cursor,
}

/// Mouse button for decomposed mouse events.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MouseButton {
    /// Left mouse button (default)
    Left,
    /// Middle mouse button (scroll wheel)
    Middle,
    /// Right mouse button (context menu)
    Right,
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
    /// Target frame by index, path (1/0), or 'auto'
    #[arg(long)]
    pub frame: Option<String>,

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
  agentchrome interact click s5

  # Click by CSS selector
  agentchrome interact click css:#submit-btn

  # Double-click
  agentchrome interact click s5 --double

  # Right-click (context menu)
  agentchrome interact click s5 --right"
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
  agentchrome interact click-at 100 200

  # Double-click at coordinates
  agentchrome interact click-at 100 200 --double"
    )]
    ClickAt(ClickAtArgs),

    /// Hover over an element
    #[command(
        long_about = "Move the mouse over an element identified by UID or CSS selector. \
            Triggers hover effects, tooltips, and mouseover events. Does not click.",
        after_long_help = "\
EXAMPLES:
  # Hover by UID
  agentchrome interact hover s3

  # Hover by CSS selector
  agentchrome interact hover css:.tooltip-trigger"
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
  agentchrome interact drag s3 s7

  # Drag using CSS selectors
  agentchrome interact drag css:#item css:#dropzone"
    )]
    Drag(DragArgs),

    /// Drag from coordinates to coordinates
    #[command(
        long_about = "Drag from one set of viewport coordinates to another. Simulates mouse down \
            at the source coordinates, move to the target coordinates, and mouse up at the target. \
            Use --steps to interpolate intermediate mousemove events for applications that track \
            drag movement (e.g., canvas-based interfaces).",
        after_long_help = "\
EXAMPLES:
  # Drag from (100,200) to (300,400)
  agentchrome interact drag-at 100 200 300 400

  # Drag with interpolated steps
  agentchrome interact drag-at 0 0 500 500 --steps 10

  # Drag inside an iframe
  agentchrome interact --frame 1 drag-at 50 60 200 300"
    )]
    DragAt(DragAtArgs),

    /// Press mouse button at coordinates (no release)
    #[command(
        name = "mousedown-at",
        long_about = "Dispatch only a mousePressed event at specific viewport coordinates. \
            No mouseReleased event is sent, allowing decomposed mouse interactions such as \
            long-press, drag sequences across multiple invocations, or custom interaction \
            patterns. Use --button to specify left, middle, or right mouse button.",
        after_long_help = "\
EXAMPLES:
  # Mousedown at coordinates
  agentchrome interact mousedown-at 100 200

  # Right-button mousedown
  agentchrome interact mousedown-at 100 200 --button right

  # Mousedown inside an iframe
  agentchrome interact --frame 1 mousedown-at 50 60"
    )]
    MouseDownAt(MouseDownAtArgs),

    /// Release mouse button at coordinates
    #[command(
        name = "mouseup-at",
        long_about = "Dispatch only a mouseReleased event at specific viewport coordinates. \
            No mousePressed event is sent, allowing decomposed mouse interactions such as \
            completing a drag started by a prior mousedown-at invocation. \
            Use --button to specify left, middle, or right mouse button.",
        after_long_help = "\
EXAMPLES:
  # Mouseup at coordinates
  agentchrome interact mouseup-at 300 400

  # Right-button mouseup
  agentchrome interact mouseup-at 300 400 --button right

  # Mouseup inside an iframe
  agentchrome interact --frame 1 mouseup-at 50 60"
    )]
    MouseUpAt(MouseUpAtArgs),

    /// Type text character-by-character into the focused element
    #[command(
        long_about = "Type text character-by-character into the currently focused element. \
            Simulates individual key press and release events for each character. Use --delay \
            to add a pause between keystrokes. To focus an element first, use 'interact click'.",
        after_long_help = "\
EXAMPLES:
  # Type text
  agentchrome interact type \"Hello, world!\"

  # Type with delay between keystrokes
  agentchrome interact type \"slow typing\" --delay 50"
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
  agentchrome interact key Enter

  # Select all (Ctrl+A)
  agentchrome interact key Control+A

  # Press Tab 3 times
  agentchrome interact key Tab --repeat 3

  # Multi-modifier combo
  agentchrome interact key Control+Shift+ArrowRight"
    )]
    Key(KeyArgs),

    /// Scroll the page or a container element
    #[command(
        long_about = "Scroll the page or a specific container element. By default, scrolls \
            down by one viewport height. Use --direction to scroll in other directions, \
            --amount to set a custom distance in pixels, or the shortcut flags --to-top, \
            --to-bottom, --to-element to scroll to specific positions. Use --selector or \
            --uid to scroll within a specific scrollable container by CSS selector or \
            accessibility UID. Use --container for the legacy combined-target syntax. \
            Use --smooth for animated scrolling.",
        after_long_help = "\
EXAMPLES:
  # Scroll down one viewport height
  agentchrome interact scroll

  # Scroll up 200 pixels
  agentchrome interact scroll --direction up --amount 200

  # Scroll to bottom of page
  agentchrome interact scroll --to-bottom

  # Scroll until an element is visible
  agentchrome interact scroll --to-element s15

  # Scroll a container by CSS selector
  agentchrome interact scroll --selector \".stage\" --direction down

  # Scroll a container by UID (requires prior snapshot)
  agentchrome interact scroll --uid s42 --direction down --amount 300

  # Smooth scroll within a container
  agentchrome interact scroll --container css:.scrollable --smooth"
    )]
    Scroll(ScrollArgs),
}

/// Arguments for `interact click`.
#[allow(clippy::struct_excessive_bools)]
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

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,

    /// Wait strategy after click (e.g., for SPA navigation).
    /// If omitted, click returns immediately with a brief navigation check.
    #[arg(long, value_enum)]
    pub wait_until: Option<WaitUntil>,
}

/// Arguments for `interact click-at`.
#[allow(clippy::struct_excessive_bools)]
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

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,

    /// Wait strategy after click (e.g., for SPA navigation).
    /// If omitted, click returns immediately after dispatching.
    #[arg(long, value_enum)]
    pub wait_until: Option<WaitUntil>,
}

/// Arguments for `interact hover`.
#[derive(Args)]
pub struct HoverArgs {
    /// Target element (UID like 's1' or CSS selector like 'css:#button')
    pub target: String,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,
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

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,
}

/// Arguments for `interact drag-at`.
#[derive(Args)]
pub struct DragAtArgs {
    /// Source X coordinate in viewport pixels
    pub from_x: f64,

    /// Source Y coordinate in viewport pixels
    pub from_y: f64,

    /// Target X coordinate in viewport pixels
    pub to_x: f64,

    /// Target Y coordinate in viewport pixels
    pub to_y: f64,

    /// Number of intermediate mousemove steps for interpolated drag movement
    #[arg(long)]
    pub steps: Option<u32>,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,
}

/// Arguments for `interact mousedown-at`.
#[derive(Args)]
pub struct MouseDownAtArgs {
    /// X coordinate in viewport pixels
    pub x: f64,

    /// Y coordinate in viewport pixels
    pub y: f64,

    /// Mouse button to press
    #[arg(long, value_enum)]
    pub button: Option<MouseButton>,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,
}

/// Arguments for `interact mouseup-at`.
#[derive(Args)]
pub struct MouseUpAtArgs {
    /// X coordinate in viewport pixels
    pub x: f64,

    /// Y coordinate in viewport pixels
    pub y: f64,

    /// Mouse button to release
    #[arg(long, value_enum)]
    pub button: Option<MouseButton>,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,
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

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,
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

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,
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

    /// CSS selector to target a scrollable container (e.g., '.stage', '#panel')
    #[arg(long, conflicts_with_all = ["uid", "to_element", "to_top", "to_bottom", "container"])]
    pub selector: Option<String>,

    /// Accessibility UID to target a scrollable container (e.g., 's42', requires prior snapshot)
    #[arg(long, conflicts_with_all = ["selector", "to_element", "to_top", "to_bottom", "container"])]
    pub uid: Option<String>,

    /// Scroll within a container element (UID like 's3' or CSS selector like 'css:.scrollable')
    #[arg(long, conflicts_with_all = ["to_element", "to_top", "to_bottom", "selector", "uid"])]
    pub container: Option<String>,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,
}

/// Arguments for the `form` subcommand group.
#[derive(Args)]
pub struct FormArgs {
    /// Target frame by index, path (1/0), or 'auto'
    #[arg(long)]
    pub frame: Option<String>,

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
  agentchrome form fill s5 \"hello@example.com\"

  # Fill by CSS selector
  agentchrome form fill css:#email \"user@example.com\"

  # Select a dropdown option
  agentchrome form fill s8 \"Option B\""
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
  agentchrome form fill-many '[{\"uid\":\"s5\",\"value\":\"Alice\"},{\"uid\":\"s7\",\"value\":\"alice@example.com\"}]'

  # Fill from a JSON file
  agentchrome form fill-many --file form-data.json"
    )]
    FillMany(FormFillManyArgs),

    /// Clear a form field's value
    #[command(
        long_about = "Clear the value of a form field identified by UID or CSS selector. \
            Sets the field to an empty string and dispatches change and input events.",
        after_long_help = "\
EXAMPLES:
  # Clear a field by UID
  agentchrome form clear s5

  # Clear by CSS selector
  agentchrome form clear css:#search-input"
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
  agentchrome form upload s10 ./photo.jpg

  # Upload multiple files
  agentchrome form upload css:#file-input ./doc1.pdf ./doc2.pdf"
    )]
    Upload(FormUploadArgs),

    /// Submit a form programmatically
    #[command(
        long_about = "Submit a form identified by UID (from 'page snapshot', e.g., 's3') or \
            CSS selector (prefixed with 'css:', e.g., 'css:#login-form'). The target can be \
            the form element itself or any element inside the form — the parent form is \
            resolved automatically. Uses requestSubmit() to respect browser validation.",
        after_long_help = "\
EXAMPLES:
  # Submit by form UID
  agentchrome form submit s3

  # Submit by CSS selector
  agentchrome form submit css:#login-form

  # Submit targeting an input inside the form
  agentchrome form submit s5

  # Include updated snapshot after submit
  agentchrome form submit s3 --include-snapshot"
    )]
    Submit(FormSubmitArgs),
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

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,
}

/// Arguments for `form fill-many`.
#[derive(Args)]
pub struct FormFillManyArgs {
    /// Inline JSON array of {uid, value} objects
    #[arg(value_name = "JSON")]
    pub input: Option<String>,

    /// Read JSON from a file instead of inline argument
    #[arg(long)]
    pub file: Option<PathBuf>,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,
}

/// Arguments for `form clear`.
#[derive(Args)]
pub struct FormClearArgs {
    /// Target element (UID like 's1' or CSS selector like 'css:#email')
    pub target: String,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,
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

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,
}

/// Arguments for `form submit`.
#[derive(Args)]
pub struct FormSubmitArgs {
    /// Target element (UID like 's3' or CSS selector like 'css:#login-form')
    #[arg(value_name = "TARGET")]
    pub target: String,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,

    /// Use compact mode for the included snapshot (only interactive and landmark elements)
    #[arg(long)]
    pub compact: bool,
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
  agentchrome console read

  # Get details of a specific message
  agentchrome console read 42

  # Show only errors
  agentchrome console read --errors-only

  # Filter by type
  agentchrome console read --type warn,error --limit 20"
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
  agentchrome console follow

  # Stream errors only for 10 seconds
  agentchrome console follow --errors-only --timeout 10000

  # Stream specific message types
  agentchrome console follow --type log,warn"
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
  agentchrome network list

  # Filter by resource type
  agentchrome network list --type xhr,fetch

  # Filter by URL pattern
  agentchrome network list --url api.example.com

  # Filter by status code
  agentchrome network list --status 4xx"
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
  agentchrome network get 42

  # Save the response body to a file
  agentchrome network get 42 --save-response body.json

  # Save both request and response bodies
  agentchrome network get 42 --save-request req.json --save-response resp.json"
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
  agentchrome network follow

  # Stream API requests only
  agentchrome network follow --type xhr,fetch --url /api/

  # Stream with headers for 30 seconds
  agentchrome network follow --verbose --timeout 30000"
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

    /// Filter network requests by originating frame index
    #[arg(long)]
    pub frame: Option<String>,
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

/// Arguments for `page element`.
#[derive(Args)]
pub struct PageElementArgs {
    /// Element target: UID (s1, s2, ...) or CSS selector (css:#id, css:.class)
    pub target: String,
}

/// Arguments for `page hittest`.
#[derive(Args)]
pub struct PageHitTestArgs {
    /// X viewport coordinate
    pub x: u32,

    /// Y viewport coordinate
    pub y: u32,
}

/// Arguments for `page wait`.
#[derive(Args)]
pub struct PageWaitArgs {
    /// Wait for the page URL to match a glob pattern
    #[arg(long, group = "condition")]
    pub url: Option<String>,

    /// Wait for text to appear in the page content
    #[arg(long, group = "condition")]
    pub text: Option<String>,

    /// Wait for a CSS selector to match an element in the DOM
    #[arg(long, group = "condition")]
    pub selector: Option<String>,

    /// Wait for network activity to settle (no requests for 500ms)
    #[arg(long, group = "condition")]
    pub network_idle: bool,

    /// Wait for a JavaScript expression to evaluate to a truthy value
    #[arg(long, group = "condition")]
    pub js_expression: Option<String>,

    /// Minimum number of elements that must match the selector (requires --selector)
    #[arg(long, requires = "selector", default_value = "1")]
    pub count: u64,

    /// Poll interval in milliseconds (for --url, --text, --selector, --js-expression)
    #[arg(long, default_value = "100")]
    pub interval: u64,
}

/// Arguments for the `dom` subcommand group.
#[derive(Args)]
pub struct DomArgs {
    /// Target frame by index, path (1/0), or 'auto'
    #[arg(long)]
    pub frame: Option<String>,

    /// Pierce open shadow DOM boundaries for element queries
    #[arg(long)]
    pub pierce_shadow: bool,

    #[command(subcommand)]
    pub command: DomCommand,
}

/// DOM inspection and manipulation subcommands.
#[derive(Subcommand)]
pub enum DomCommand {
    /// Select elements by CSS selector or XPath
    #[command(
        long_about = "Query elements in the DOM by CSS selector (default) or XPath expression \
            (with --xpath). Returns a JSON array of matching elements with their node IDs, \
            tag names, attributes, and text content. Node IDs can be used with other dom \
            subcommands.",
        after_long_help = "\
EXAMPLES:
  # Select by CSS selector
  agentchrome dom select \"h1\"

  # Select by XPath
  agentchrome dom select \"//a[@href]\" --xpath

  # Select with a complex CSS selector
  agentchrome dom select \"div.content > p:first-child\""
    )]
    Select(DomSelectArgs),

    /// Get a single attribute value from an element
    #[command(
        name = "get-attribute",
        long_about = "Read a single attribute value from a DOM element. The element can be \
            targeted by node ID (from 'dom select'), snapshot UID (from 'page snapshot'), \
            or CSS selector (prefixed with 'css:'). Returns the attribute name and value.",
        after_long_help = "\
EXAMPLES:
  # Get href by UID
  agentchrome dom get-attribute s3 href

  # Get class by CSS selector
  agentchrome dom get-attribute css:h1 class"
    )]
    GetAttribute(DomGetAttributeArgs),

    /// Get the text content of an element
    #[command(
        name = "get-text",
        long_about = "Read the textContent of a DOM element. Returns the combined text of \
            the element and all its descendants.",
        after_long_help = "\
EXAMPLES:
  # Get text by UID
  agentchrome dom get-text s3

  # Get text by CSS selector
  agentchrome dom get-text css:h1"
    )]
    GetText(DomNodeIdArgs),

    /// Get the outer HTML of an element
    #[command(
        name = "get-html",
        long_about = "Read the outerHTML of a DOM element, including the element itself and \
            all its children as an HTML string.",
        after_long_help = "\
EXAMPLES:
  # Get HTML by UID
  agentchrome dom get-html s3

  # Get HTML by CSS selector
  agentchrome dom get-html css:div.content"
    )]
    GetHtml(DomNodeIdArgs),

    /// Set an attribute on an element
    #[command(
        name = "set-attribute",
        long_about = "Set or update an attribute on a DOM element. Creates the attribute if \
            it doesn't exist, or updates its value if it does.",
        after_long_help = "\
EXAMPLES:
  # Set class attribute
  agentchrome dom set-attribute s5 class \"highlight\"

  # Set data attribute by CSS selector
  agentchrome dom set-attribute css:#main data-active true"
    )]
    SetAttribute(DomSetAttributeArgs),

    /// Set the text content of an element
    #[command(
        name = "set-text",
        long_about = "Replace the textContent of a DOM element, removing all child nodes \
            and setting the element's content to the given text.",
        after_long_help = "\
EXAMPLES:
  # Set text by UID
  agentchrome dom set-text s3 \"New heading\"

  # Set text by CSS selector
  agentchrome dom set-text css:h1 \"Updated Title\""
    )]
    SetText(DomSetTextArgs),

    /// Remove an element from the DOM
    #[command(
        long_about = "Remove a DOM element and all its children from the document. This is \
            irreversible within the current page session.",
        after_long_help = "\
EXAMPLES:
  # Remove by UID
  agentchrome dom remove s3

  # Remove by CSS selector
  agentchrome dom remove css:div.ad-banner"
    )]
    Remove(DomNodeIdArgs),

    /// Get computed CSS styles of an element
    #[command(
        name = "get-style",
        long_about = "Read the computed CSS styles of a DOM element. Without a property name, \
            returns all computed styles. With a property name, returns just that property's \
            value.",
        after_long_help = "\
EXAMPLES:
  # Get all computed styles
  agentchrome dom get-style s3

  # Get a specific property
  agentchrome dom get-style s3 display

  # Get style by CSS selector
  agentchrome dom get-style css:h1 color"
    )]
    GetStyle(DomGetStyleArgs),

    /// Set the inline style of an element
    #[command(
        name = "set-style",
        long_about = "Set the inline style attribute of a DOM element. The style string replaces \
            the entire inline style. Use CSS property syntax.",
        after_long_help = "\
EXAMPLES:
  # Set inline style
  agentchrome dom set-style s3 \"color: red; font-size: 24px\"

  # Set style by CSS selector
  agentchrome dom set-style css:h1 \"display: none\""
    )]
    SetStyle(DomSetStyleArgs),

    /// Get the parent element
    #[command(
        long_about = "Navigate to the parent element of a DOM node. Returns the parent's \
            node ID, tag name, attributes, and text content.",
        after_long_help = "\
EXAMPLES:
  # Get parent of a UID
  agentchrome dom parent s3

  # Get parent by CSS selector
  agentchrome dom parent css:span.label"
    )]
    Parent(DomNodeIdArgs),

    /// List direct child elements
    #[command(
        long_about = "List the direct child elements (element nodes only, nodeType 1) of a \
            DOM node. Returns a JSON array of child elements.",
        after_long_help = "\
EXAMPLES:
  # List children by UID
  agentchrome dom children s3

  # List children by CSS selector
  agentchrome dom children css:div.container"
    )]
    Children(DomNodeIdArgs),

    /// List sibling elements
    #[command(
        long_about = "List the sibling elements of a DOM node (other children of the same \
            parent, excluding the target element itself).",
        after_long_help = "\
EXAMPLES:
  # List siblings by UID
  agentchrome dom siblings s3

  # List siblings by CSS selector
  agentchrome dom siblings css:li.active"
    )]
    Siblings(DomNodeIdArgs),

    /// Pretty-print the DOM tree
    #[command(
        long_about = "Display the DOM tree as indented plain text. By default shows the full \
            document tree. Use --depth to limit traversal depth and --root to start from a \
            specific element.",
        after_long_help = "\
EXAMPLES:
  # Show the full DOM tree
  agentchrome dom tree

  # Limit depth to 3 levels
  agentchrome dom tree --depth 3

  # Show tree from a specific element
  agentchrome dom tree --root css:div.content"
    )]
    Tree(DomTreeArgs),

    /// List event listeners attached to an element
    #[command(
        long_about = "List all event listeners attached to a DOM element. Shows listeners \
            registered via addEventListener and inline handlers (e.g., onclick). Output \
            includes event type, capture/bubble phase, once/passive flags, and handler \
            source location.",
        after_long_help = "\
EXAMPLES:
  # List listeners by UID
  agentchrome dom events s3

  # List listeners by CSS selector
  agentchrome dom events css:button

  # List listeners in a frame
  agentchrome dom --frame 0 events css:button"
    )]
    Events(DomNodeIdArgs),
}

/// Arguments for `dom select`.
#[derive(Args)]
pub struct DomSelectArgs {
    /// CSS selector or XPath expression to query
    pub selector: String,

    /// Interpret the selector as an XPath expression instead of CSS
    #[arg(long)]
    pub xpath: bool,
}

/// Arguments for `dom get-attribute`.
#[derive(Args)]
pub struct DomGetAttributeArgs {
    /// Target element (node ID, UID like 's1', or CSS selector like 'css:#el')
    pub node_id: String,

    /// Attribute name to read
    pub attribute: String,
}

/// Arguments for `dom set-attribute`.
#[derive(Args)]
pub struct DomSetAttributeArgs {
    /// Target element (node ID, UID like 's1', or CSS selector like 'css:#el')
    pub node_id: String,

    /// Attribute name to set
    pub attribute: String,

    /// Attribute value
    pub value: String,
}

/// Arguments for `dom get-style`.
#[derive(Args)]
pub struct DomGetStyleArgs {
    /// Target element (node ID, UID like 's1', or CSS selector like 'css:#el')
    pub node_id: String,

    /// CSS property name (omit for all computed styles)
    pub property: Option<String>,
}

/// Arguments for `dom set-style`.
#[derive(Args)]
pub struct DomSetStyleArgs {
    /// Target element (node ID, UID like 's1', or CSS selector like 'css:#el')
    pub node_id: String,

    /// CSS style text (e.g. "color: red; font-size: 24px")
    pub style: String,
}

/// Arguments for `dom set-text`.
#[derive(Args)]
pub struct DomSetTextArgs {
    /// Target element (node ID, UID like 's1', or CSS selector like 'css:#el')
    pub node_id: String,

    /// Text content to set
    pub text: String,
}

/// Shared arguments for dom subcommands that take only a node ID.
///
/// Used by get-text, get-html, remove, parent, children, siblings.
#[derive(Args)]
pub struct DomNodeIdArgs {
    /// Target element (node ID, UID like 's1', or CSS selector like 'css:#el')
    pub node_id: String,
}

/// Arguments for `dom tree`.
#[derive(Args)]
pub struct DomTreeArgs {
    /// Maximum tree depth to display
    #[arg(long)]
    pub depth: Option<u32>,

    /// Start the tree from a specific element (node ID, UID, or CSS selector)
    #[arg(long)]
    pub root: Option<String>,
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
  agentchrome emulate set --viewport 375x667 --device-scale 2 --mobile

  # Simulate slow network
  agentchrome emulate set --network 3g

  # Set geolocation (San Francisco)
  agentchrome emulate set --geolocation 37.7749,-122.4194

  # Force dark mode with custom user agent
  agentchrome emulate set --color-scheme dark --user-agent \"CustomBot/1.0\"

  # Throttle CPU (4x slowdown)
  agentchrome emulate set --cpu 4"
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
  agentchrome emulate reset"
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
  agentchrome emulate status"
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
  agentchrome config show

  # Show config from a specific file
  agentchrome --config ./my-config.toml config show"
    )]
    Show,

    /// Create a default config file with commented example values
    #[command(
        long_about = "Create a new configuration file with all available settings documented \
            as comments. By default, the file is created at the XDG config directory \
            (~/.config/agentchrome/config.toml on Linux, ~/Library/Application Support/\
            agentchrome/config.toml on macOS). Use --path to specify a custom location. \
            Will not overwrite an existing file.",
        after_long_help = "\
EXAMPLES:
  # Create default config file
  agentchrome config init

  # Create at a custom path
  agentchrome config init --path ./my-config.toml"
    )]
    Init(ConfigInitArgs),

    /// Show the active config file path (or null if none)
    #[command(
        long_about = "Show the path of the active configuration file. Searches in priority \
            order: --config flag, $AGENTCHROME_CONFIG env var, project-local \
            (.agentchrome.toml), XDG config dir, home directory (~/.agentchrome.toml). \
            Returns JSON with {\"path\": \"...\"} or {\"path\": null} if no config file is found.",
        after_long_help = "\
EXAMPLES:
  # Show active config path
  agentchrome config path"
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

/// Arguments for the `capabilities` subcommand.
#[derive(Args)]
pub struct CapabilitiesArgs {
    /// Show capabilities for a specific command only
    #[arg(long)]
    pub command: Option<String>,

    /// Minimal output: command names and descriptions only
    #[arg(long)]
    pub compact: bool,
}
