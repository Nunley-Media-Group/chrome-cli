mod audit;
mod capabilities_cli;
mod cli;
mod console;
mod cookie;
mod coord_helpers;
mod script;
// Re-export the library's coords module so `crate::coords` works in shared code (cli/mod.rs).
mod coords {
    pub use agentchrome::coords::*;
}
mod diagnose;
mod dialog;
mod dom;
mod emulate;
mod examples;
mod form;
mod interact;
mod js;
mod media;
mod navigate;
mod network;
mod output;
mod page;
mod perf;
mod skill;
mod skill_check;
mod snapshot;
mod tabs;

use std::io::Write as _;
use std::time::Duration;

use clap::{
    CommandFactory, Parser,
    error::{ContextKind, ContextValue, ErrorKind},
};
use serde::Serialize;

use agentchrome::chrome::{
    self, Channel, LaunchConfig, discover_chrome, find_available_port, find_chrome_executable,
    launch_chrome, query_version,
};
use agentchrome::config;
use agentchrome::connection::{self, extract_port_from_ws_url};
use agentchrome::error::{AppError, ExitCode};
use agentchrome::session::{self, SessionData};

use cli::{
    ChromeChannel, Cli, Command, CompletionsArgs, ConfigCommand, ConnectArgs, GlobalOpts, ManArgs,
    ScriptSubcommand,
};

#[tokio::main]
async fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // --help and --version are informational, not errors — print as-is
            if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                e.print().expect("failed to write to stdout");
                std::process::exit(0);
            }
            // All other clap errors → JSON on stderr with exit code 1
            let msg = e.kind().to_string();
            let full = e.to_string();
            let clean = full
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    !trimmed.is_empty()
                        && !trimmed.starts_with("For more information")
                        && !trimmed.starts_with("Usage:")
                })
                .map(|line| line.strip_prefix("error: ").unwrap_or(line).trim())
                .collect::<Vec<_>>()
                .join(", ");
            let clean = if clean.is_empty() { msg } else { clean };
            let argv: Vec<String> = std::env::args().collect();
            let clean = match syntax_hint(&e, &argv) {
                Some(hint) => format!("{clean}. {hint}"),
                None => clean,
            };
            let app_err = AppError {
                message: clean,
                code: ExitCode::GeneralError,
                custom_json: None,
            };
            app_err.print_json_stderr();
            std::process::exit(app_err.code as i32);
        }
    };

    if let Err(e) = run(&cli).await {
        e.print_json_stderr();
        #[allow(clippy::cast_possible_truncation)]
        std::process::exit(e.code as i32);
    }
}

/// If a clap `UnknownArgument` error was raised for a flag whose correct form
/// is a positional on the invoked subcommand, build a
/// `"Did you mean: agentchrome <sub> <value>"` suffix. Handles:
///
/// - `--uid` / `--selector` on any positional-target subcommand — value is read
///   from argv (e.g., `interact click --uid s6` → `... interact click s6`).
/// - `--accept` / `--dismiss` on `dialog handle` — the closed `DialogAction`
///   enum means the value is the flag name itself (e.g.,
///   `dialog handle --accept` → `... dialog handle accept`).
///
/// Returns `None` for any other error kind, any other flag, or when the
/// subcommand path / value cannot be recovered from argv.
fn syntax_hint(err: &clap::Error, argv: &[String]) -> Option<String> {
    if !matches!(err.kind(), ErrorKind::UnknownArgument) {
        return None;
    }
    let invalid = match err.get(ContextKind::InvalidArg)? {
        ContextValue::String(s) => s.clone(),
        ContextValue::Strings(v) => v.first()?.clone(),
        _ => return None,
    };
    // Clap renders InvalidArg as the flag name plus optional `<VALUE>` / `=VALUE` suffix.
    let bare = invalid.split([' ', '=']).next().unwrap_or("");
    let sub_path = resolve_subcommand_path(argv)?;

    let value = match bare {
        "--uid" | "--selector" => extract_flag_value(argv, bare)?,
        "--accept" | "--dismiss" if sub_path == "dialog handle" => {
            bare.trim_start_matches('-').to_string()
        }
        _ => return None,
    };
    Some(format!("Did you mean: agentchrome {sub_path} {value}"))
}

fn extract_flag_value(argv: &[String], flag: &str) -> Option<String> {
    let eq_prefix = format!("{flag}=");
    for (i, arg) in argv.iter().enumerate() {
        if let Some(v) = arg.strip_prefix(&eq_prefix) {
            return Some(v.to_string());
        }
        if arg == flag {
            return argv.get(i + 1).cloned();
        }
    }
    None
}

fn resolve_subcommand_path(argv: &[String]) -> Option<String> {
    let mut cmd = Cli::command();
    let mut path: Vec<String> = Vec::new();
    for arg in argv.iter().skip(1) {
        match cmd.find_subcommand(arg).cloned() {
            Some(sub) => {
                path.push(arg.clone());
                cmd = sub;
            }
            None => {
                if !path.is_empty() {
                    break;
                }
            }
        }
    }
    if path.is_empty() {
        None
    } else {
        Some(path.join(" "))
    }
}

async fn run(cli: &Cli) -> Result<(), AppError> {
    // Load config file (if any) and apply defaults to global opts
    let (config_path, config_file) = config::load_config(cli.global.config.as_deref());
    skill_check::emit_stale_notice_if_any(&config_file);
    let global = apply_config_defaults(&cli.global, &config_file);

    match &cli.command {
        Command::Config(args) => {
            let resolved = build_resolved_config(&global, &config_file, config_path);
            execute_config(&args.command, &resolved, cli.global.config.as_deref())
        }
        Command::Connect(args) => execute_connect(&global, args).await,
        Command::Tabs(args) => tabs::execute_tabs(&global, args).await,
        Command::Navigate(args) => navigate::execute_navigate(&global, args).await,
        Command::Page(args) => page::execute_page(&global, args).await,
        Command::Dom(args) => dom::execute_dom(&global, args).await,
        Command::Js(args) => js::execute_js(&global, args).await,
        Command::Console(args) => console::execute_console(&global, args).await,
        Command::Network(args) => network::execute_network(&global, args).await,
        Command::Interact(args) => interact::execute_interact(&global, args).await,
        Command::Form(args) => form::execute_form(&global, args).await,
        Command::Media(args) => media::execute_media(&global, args).await,
        Command::Emulate(args) => emulate::execute_emulate(&global, args).await,
        Command::Perf(args) => perf::execute_perf(&global, args).await,
        Command::Cookie(args) => cookie::execute_cookie(&global, args).await,
        Command::Dialog(args) => dialog::execute_dialog(&global, args).await,
        Command::Audit(args) => audit::execute_audit(&global, args).await,
        Command::Diagnose(args) => diagnose::execute_diagnose(&global, args).await,
        Command::Skill(args) => skill::execute_skill(&global, args),
        Command::Examples(args) => examples::execute_examples(&global, args),
        Command::Capabilities(args) => capabilities_cli::execute_capabilities(&global, args),
        Command::Completions(args) => execute_completions(args),
        Command::Man(args) => execute_man(args),
        Command::Script(args) => execute_script(&global, args).await,
    }
}

/// Build a fully resolved config from merged `GlobalOpts` and config file sections.
///
/// This is used by `config show` to display the final merged configuration from
/// all sources (CLI flags > env vars > config file > defaults).
fn build_resolved_config(
    global: &GlobalOpts,
    config_file: &config::ConfigFile,
    config_path: Option<std::path::PathBuf>,
) -> config::ResolvedConfig {
    config::ResolvedConfig {
        config_path,
        connection: config::ResolvedConnection {
            host: global.host.clone(),
            port: global.port_or_default(),
            timeout_ms: global.timeout.unwrap_or(30_000),
        },
        launch: config::ResolvedLaunch {
            executable: config_file.launch.executable.clone(),
            channel: config_file
                .launch
                .channel
                .clone()
                .unwrap_or_else(|| "stable".to_string()),
            headless: config_file.launch.headless.unwrap_or(false),
            extra_args: config_file.launch.extra_args.clone().unwrap_or_default(),
        },
        output: config::ResolvedOutput {
            format: config_file
                .output
                .format
                .clone()
                .unwrap_or_else(|| "json".to_string()),
            large_response_threshold: global
                .output
                .large_response_threshold
                .or(config_file.output.large_response_threshold)
                .unwrap_or(output::DEFAULT_THRESHOLD),
        },
        tabs: config::ResolvedTabs {
            auto_activate: config_file.tabs.auto_activate.unwrap_or(true),
            filter_internal: config_file.tabs.filter_internal.unwrap_or(true),
        },
    }
}

/// Apply config file defaults to global opts for fields that weren't set by CLI/env.
///
/// The precedence chain is: CLI flags > env vars > config file > built-in defaults.
/// Since clap already handles CLI flags and env vars (via `env = "..."` attributes),
/// we only fill in values from the config file for fields that are still at their defaults.
fn apply_config_defaults(cli_global: &GlobalOpts, config: &config::ConfigFile) -> GlobalOpts {
    // We can't easily detect "user provided --host" vs "default_value was used" for String
    // fields. For Option fields, None means unset; for host (which has default_value), we
    // check whether it matches the built-in default.
    let host_is_default =
        cli_global.host == "127.0.0.1" && std::env::var("AGENTCHROME_HOST").is_err();

    GlobalOpts {
        port: cli_global.port.or(config.connection.port),
        host: if host_is_default {
            config
                .connection
                .host
                .clone()
                .unwrap_or_else(|| cli_global.host.clone())
        } else {
            cli_global.host.clone()
        },
        ws_url: cli_global.ws_url.clone(),
        timeout: cli_global.timeout.or(config.connection.timeout_ms),
        tab: cli_global.tab.clone(),
        page_id: cli_global.page_id.clone(),
        auto_dismiss_dialogs: cli_global.auto_dismiss_dialogs,
        config: cli_global.config.clone(),
        keepalive_interval: cli_global
            .keepalive_interval
            .or(config.keepalive.interval_ms),
        no_keepalive: cli_global.no_keepalive,
        output: cli::OutputFormat {
            json: cli_global.output.json,
            pretty: cli_global.output.pretty,
            plain: cli_global.output.plain,
            large_response_threshold: cli_global
                .output
                .large_response_threshold
                .or(config.output.large_response_threshold),
        },
    }
}

#[derive(Serialize)]
struct ConfigInitOutput {
    created: String,
}

#[derive(Serialize)]
struct ConfigPathOutput {
    config_path: Option<String>,
}

/// Execute config subcommands. `global_config_raw` is the pre-resolution
/// `--config` value; only the `Init` arm consumes it (as a destination fallback).
fn execute_config(
    cmd: &ConfigCommand,
    resolved: &config::ResolvedConfig,
    global_config_raw: Option<&std::path::Path>,
) -> Result<(), AppError> {
    match cmd {
        ConfigCommand::Show => {
            print_json(resolved)?;
            Ok(())
        }
        ConfigCommand::Init(args) => {
            if let (Some(p), Some(g)) = (args.path.as_deref(), global_config_raw) {
                if p != g {
                    eprintln!(
                        "note: --path overrode --config (--path={}, --config={})",
                        p.display(),
                        g.display()
                    );
                }
            }
            let destination = args.path.as_deref().or(global_config_raw);
            let path = config::init_config(destination).map_err(|e| {
                let dest = destination
                    .map_or_else(|| "<default>".to_string(), |p| p.display().to_string());
                AppError {
                    message: format!("config init failed for {dest}: {e}"),
                    code: ExitCode::GeneralError,
                    custom_json: None,
                }
            })?;
            print_json(&ConfigInitOutput {
                created: path.display().to_string(),
            })?;
            Ok(())
        }
        ConfigCommand::Path => {
            print_json(&ConfigPathOutput {
                config_path: resolved
                    .config_path
                    .as_ref()
                    .map(|p| p.display().to_string()),
            })?;
            Ok(())
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
fn execute_completions(args: &CompletionsArgs) -> Result<(), AppError> {
    let mut cmd = Cli::command();
    clap_complete::generate(args.shell, &mut cmd, "agentchrome", &mut std::io::stdout());
    Ok(())
}

fn execute_man(args: &ManArgs) -> Result<(), AppError> {
    let cmd = Cli::command();

    let short_name = args.command.as_deref().unwrap_or("agentchrome");
    let target = match &args.command {
        None => cmd.clone(),
        Some(name) => find_subcommand(&cmd, name).ok_or_else(|| AppError {
            message: format!("unknown command: {name}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?,
    };

    let manifest = agentchrome::capabilities::build_manifest(&cmd, false);
    let examples = agentchrome::examples_data::all_examples();
    let buf =
        agentchrome::man_enrichment::render_enriched(target, short_name, &manifest, &examples)
            .map_err(|e| AppError {
                message: format!("failed to render man page: {e}"),
                code: ExitCode::GeneralError,
                custom_json: None,
            })?;

    std::io::stdout().write_all(&buf).map_err(|e| AppError {
        message: format!("failed to write man page: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;
    Ok(())
}

fn find_subcommand(cmd: &clap::Command, name: &str) -> Option<clap::Command> {
    let parent_name = cmd.get_name().to_string();
    for sub in cmd.get_subcommands() {
        if sub.get_name() == name {
            let full_name = format!("{parent_name}-{name}");
            let leaked: &'static str = Box::leak(full_name.into_boxed_str());
            return Some(sub.clone().name(leaked));
        }
    }
    None
}

#[derive(Serialize)]
struct ConnectionInfo {
    ws_url: String,
    port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pid: Option<u32>,
}

#[derive(Serialize)]
struct StatusInfo {
    /// `true` when a session file was found, regardless of reachability.
    /// Scripts use this as the no-error discriminator in place of exit codes.
    active: bool,
    ws_url: String,
    port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pid: Option<u32>,
    timestamp: String,
    reachable: bool,
    /// Last auto-reconnect timestamp, or `null` if never.
    #[serde(skip_serializing_if = "Option::is_none")]
    last_reconnect_at: Option<String>,
    /// Cumulative auto-reconnects within this session file's lifetime.
    reconnect_count: u32,
    keepalive: KeepaliveStatus,
}

#[derive(Serialize)]
struct NoSessionStatus {
    /// Always `false`. Emitted when `connect --status` finds no session file.
    /// Kept as a flat object rather than a variant so script consumers can
    /// read `active` uniformly.
    active: bool,
}

#[derive(Serialize)]
struct KeepaliveStatus {
    /// `null` when keep-alive is disabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    interval_ms: Option<u64>,
    enabled: bool,
}

#[derive(Serialize)]
struct DisconnectInfo {
    disconnected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    killed_pid: Option<u32>,
}

fn print_json(value: &impl Serialize) -> Result<(), AppError> {
    let json = serde_json::to_string(value).map_err(|e| AppError {
        message: format!("serialization error: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;
    println!("{json}");
    Ok(())
}

fn convert_channel(ch: ChromeChannel) -> Channel {
    match ch {
        ChromeChannel::Stable => Channel::Stable,
        ChromeChannel::Canary => Channel::Canary,
        ChromeChannel::Beta => Channel::Beta,
        ChromeChannel::Dev => Channel::Dev,
    }
}

fn warn_if_remote_host(host: &str) {
    if host != "127.0.0.1" && host != "localhost" && host != "::1" {
        eprintln!(
            "warning: connecting to non-localhost host {host}. \
             CDP connections to remote hosts may expose sensitive data."
        );
    }
}

/// Write session data after a successful connect. Non-fatal on failure.
///
/// When `info.pid` is `None` (e.g. auto-discover or direct WS URL), this checks
/// the existing session file and preserves its PID if the port matches. This
/// prevents losing the PID stored by a prior `--launch` when reconnecting to the
/// same Chrome instance.
fn save_session(info: &ConnectionInfo) {
    // Preserve PID and active_tab_id from existing session when reconnecting to the same port.
    let existing = session::read_session()
        .ok()
        .flatten()
        .filter(|existing| existing.port == info.port);

    let data = match existing {
        Some(e) => SessionData {
            ws_url: info.ws_url.clone(),
            port: info.port,
            pid: info.pid.or(e.pid),
            timestamp: session::now_iso8601(),
            ..e
        },
        None => SessionData {
            ws_url: info.ws_url.clone(),
            port: info.port,
            pid: info.pid,
            active_tab_id: None,
            timestamp: session::now_iso8601(),
            last_reconnect_at: None,
            reconnect_count: 0,
        },
    };
    if let Err(e) = session::write_session(&data) {
        eprintln!("warning: could not save session file: {e}");
    }
}

async fn execute_connect(global: &GlobalOpts, args: &ConnectArgs) -> Result<(), AppError> {
    // Handle --status
    if args.status {
        return execute_status(global).await;
    }

    // Handle --disconnect
    if args.disconnect {
        return execute_disconnect();
    }

    let timeout = Duration::from_millis(global.timeout.unwrap_or(30_000));

    warn_if_remote_host(&global.host);

    // Strategy 1: Direct WebSocket URL
    if let Some(ws_url) = &global.ws_url {
        let port = extract_port_from_ws_url(ws_url).unwrap_or(global.port_or_default());
        let info = ConnectionInfo {
            ws_url: ws_url.clone(),
            port,
            pid: None,
        };
        save_session(&info);
        print_json(&info)?;
        return Ok(());
    }

    // Strategy 2: Explicit --launch
    if args.launch {
        return execute_launch(args, timeout).await;
    }

    // Strategy 3: Check existing session first, then auto-discover, then auto-launch.
    // When --port is explicit, skip the session file and only try that port directly.
    if global.port.is_none() {
        if let Some(session_data) = session::read_session()? {
            if connection::health_check(&global.host, session_data.port)
                .await
                .is_ok()
            {
                let info = ConnectionInfo {
                    ws_url: session_data.ws_url,
                    port: session_data.port,
                    pid: session_data.pid,
                };
                save_session(&info);
                print_json(&info)?;
                return Ok(());
            }
        }
    }

    let discover_result = if global.port.is_some() {
        // Explicit --port: try only that port, no DevToolsActivePort fallback
        query_version(&global.host, global.port_or_default())
            .await
            .map(|v| (v.ws_debugger_url, global.port_or_default()))
    } else {
        discover_chrome(&global.host, global.port_or_default()).await
    };

    match discover_result {
        Ok((ws_url, port)) => {
            let info = ConnectionInfo {
                ws_url,
                port,
                pid: None,
            };
            save_session(&info);
            print_json(&info)?;
            Ok(())
        }
        Err(discover_err) => {
            if global.port.is_some() {
                // Explicit --port: don't auto-launch on a different port
                Err(discover_err.into())
            } else {
                // Try auto-launch if Chrome is available
                match execute_launch(args, timeout).await {
                    Ok(()) => Ok(()),
                    Err(_) => Err(discover_err.into()),
                }
            }
        }
    }
}

/// Maximum number of port-retry attempts when Chrome fails to bind.
const MAX_PORT_RETRIES: u8 = 3;

async fn execute_launch(args: &ConnectArgs, timeout: Duration) -> Result<(), AppError> {
    let executable = match &args.chrome_path {
        Some(path) => path.clone(),
        None => find_chrome_executable(convert_channel(args.channel))?,
    };

    let mut last_err = None;
    for _ in 0..MAX_PORT_RETRIES {
        let port = find_available_port()?;

        let config = LaunchConfig {
            executable: executable.clone(),
            port,
            headless: args.headless,
            extra_args: args.chrome_arg.clone(),
            user_data_dir: None,
        };

        match launch_chrome(config, timeout).await {
            Ok(process) => {
                // Query the version to get the WebSocket URL
                let version = query_version("127.0.0.1", port).await?;

                // Detach so Chrome keeps running after we exit
                let (pid, port) = process.detach();

                let info = ConnectionInfo {
                    ws_url: version.ws_debugger_url,
                    port,
                    pid: Some(pid),
                };
                save_session(&info);
                print_json(&info)?;
                return Ok(());
            }
            Err(e @ chrome::ChromeError::LaunchFailed(_)) => {
                last_err = Some(e);
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    }

    Err(last_err
        .unwrap_or_else(|| chrome::ChromeError::LaunchFailed("all port retries exhausted".into()))
        .into())
}

async fn execute_status(global: &GlobalOpts) -> Result<(), AppError> {
    // `--status` exits 0 whether or not a session exists so scripts can poll
    // it as a discovery probe without conflating "no session" with an error.
    let Some(session_data) = session::read_session()? else {
        if global.output.plain {
            println!("active: false");
            return Ok(());
        }
        return output::print_output(&NoSessionStatus { active: false }, &global.output);
    };

    let report = connection::resolve_connection_for_status(&global.host, &session_data).await;
    let session = report.session;

    let keepalive = output::build_keepalive(global);
    let interval_ms = keepalive
        .interval
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
    let keepalive_status = KeepaliveStatus {
        enabled: interval_ms.is_some(),
        interval_ms,
    };

    let status = StatusInfo {
        active: true,
        ws_url: session.ws_url,
        port: session.port,
        pid: session.pid,
        timestamp: session.timestamp,
        reachable: report.reachable,
        last_reconnect_at: session.last_reconnect_at,
        reconnect_count: session.reconnect_count,
        keepalive: keepalive_status,
    };

    if global.output.plain {
        print!("{}", format_plain_status(&status));
        return Ok(());
    }

    let json = if global.output.pretty {
        serde_json::to_string_pretty(&status)
    } else {
        serde_json::to_string(&status)
    };
    let json = json.map_err(|e| AppError {
        message: format!("serialization error: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;
    println!("{json}");
    Ok(())
}

fn format_plain_status(status: &StatusInfo) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "active:    {}", status.active);
    let _ = writeln!(out, "ws_url:    {}", status.ws_url);
    let _ = writeln!(out, "port:      {}", status.port);
    match status.pid {
        Some(pid) => {
            let _ = writeln!(out, "pid:       {pid}");
        }
        None => {
            let _ = writeln!(out, "pid:       -");
        }
    }
    let _ = writeln!(out, "timestamp: {}", status.timestamp);
    let _ = writeln!(out, "reachable: {}", status.reachable);
    if let Some(ref ts) = status.last_reconnect_at {
        let _ = writeln!(out, "last_reconnect_at: {ts}");
    }
    let _ = writeln!(out, "reconnect_count:   {}", status.reconnect_count);
    match status.keepalive.interval_ms {
        Some(ms) => {
            let _ = writeln!(out, "keepalive: enabled ({ms} ms)");
        }
        None => {
            let _ = writeln!(out, "keepalive: disabled");
        }
    }
    out
}

fn execute_disconnect() -> Result<(), AppError> {
    let session_data = session::read_session()?;
    let mut killed_pid = None;

    if let Some(data) = &session_data {
        if let Some(pid) = data.pid {
            kill_process(pid);
            killed_pid = Some(pid);
        }
    }

    session::delete_session()?;

    let output = DisconnectInfo {
        disconnected: true,
        killed_pid,
    };
    print_json(&output)?;
    Ok(())
}

fn kill_process(pid: u32) {
    #[cfg(unix)]
    {
        use std::thread;

        // PID values are always within i32 range on all supported platforms.
        #[allow(clippy::cast_possible_wrap)]
        let pid_i32 = pid as i32;

        // Send SIGTERM to the process group (negative PID) to kill Chrome and
        // all its child processes (renderer, GPU, utility, etc.).
        // SAFETY: libc::kill with a negative pid targets the process group.
        let term_result = unsafe { libc::kill(-pid_i32, libc::SIGTERM) };
        if term_result != 0 {
            // Process group kill failed — try killing just the main process.
            // This can happen if Chrome didn't become a process group leader.
            unsafe { libc::kill(pid_i32, libc::SIGTERM) };
        }

        // Poll for up to ~2 seconds to see if the process has exited.
        let poll_interval = Duration::from_millis(100);
        let max_wait = Duration::from_secs(2);
        let start = std::time::Instant::now();

        while start.elapsed() < max_wait {
            if matches!(chrome::is_process_alive(pid), chrome::ProbeResult::Dead) {
                return;
            }
            thread::sleep(poll_interval);
        }

        // SIGTERM didn't terminate the process within the timeout. Escalate to
        // SIGKILL on the process group, then fall back to the main PID.
        let kill_result = unsafe { libc::kill(-pid_i32, libc::SIGKILL) };
        if kill_result != 0 {
            unsafe { libc::kill(pid_i32, libc::SIGKILL) };
        }
    }
    #[cfg(windows)]
    {
        // /T kills the process tree, /F forces termination.
        let _ = std::process::Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .output();
    }
}

// =============================================================================
// Script execution
// =============================================================================

async fn execute_script(global: &GlobalOpts, args: &cli::ScriptArgs) -> Result<(), AppError> {
    use std::io::Read as _;

    use script::parser::parse_script;
    use script::result::DryRunReport;
    use script::runner::{RunOptions, run_script, validate_dry_run};

    let ScriptSubcommand::Run(run_args) = &args.sub;

    let bytes: Vec<u8> = if run_args.file == "-" {
        let mut buf = Vec::new();
        std::io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| AppError {
                message: format!("failed to read script from stdin: {e}"),
                code: ExitCode::GeneralError,
                custom_json: None,
            })?;
        buf
    } else {
        std::fs::read(&run_args.file).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError {
                    message: format!("script file not found: {}", run_args.file),
                    code: ExitCode::GeneralError,
                    custom_json: None,
                }
            } else {
                AppError {
                    message: format!("failed to read script file '{}': {e}", run_args.file),
                    code: ExitCode::GeneralError,
                    custom_json: None,
                }
            }
        })?
    };

    let script_doc = parse_script(&bytes)?;

    if run_args.dry_run {
        let steps = validate_dry_run(&script_doc)?;
        let report = DryRunReport {
            dispatched: false,
            ok: true,
            steps,
        };
        print_json(&report)?;
        return Ok(());
    }

    let opts_connection = output::connect_from_global(global).await?;
    let target = agentchrome::connection::resolve_target(
        &opts_connection.resolved.host,
        opts_connection.resolved.port,
        global.tab.as_deref(),
        global.page_id.as_deref(),
    )
    .await?;
    let session = opts_connection.client.create_session(&target.id).await?;
    let mut managed = agentchrome::connection::ManagedSession::new(session);

    let run_opts = RunOptions {
        fail_fast: run_args.fail_fast,
        dry_run: false,
    };

    let report = run_script(
        &script_doc,
        &opts_connection.client,
        &mut managed,
        global,
        &run_opts,
    )
    .await?;

    if global.output.pretty {
        let json = serde_json::to_string_pretty(&report).map_err(|e| AppError {
            message: format!("serialization error: {e}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;
        println!("{json}");
    } else {
        print_json(&report)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn resolve_subcommand_path_nested() {
        let a = argv(&["agentchrome", "interact", "click", "--uid", "s6"]);
        assert_eq!(
            resolve_subcommand_path(&a).as_deref(),
            Some("interact click")
        );
    }

    #[test]
    fn resolve_subcommand_path_skips_global_flags() {
        let a = argv(&[
            "agentchrome",
            "--port",
            "9333",
            "interact",
            "click",
            "--uid",
            "s6",
        ]);
        assert_eq!(
            resolve_subcommand_path(&a).as_deref(),
            Some("interact click")
        );
    }

    #[test]
    fn resolve_subcommand_path_returns_none_when_no_subcommand() {
        let a = argv(&["agentchrome", "--help"]);
        assert!(resolve_subcommand_path(&a).is_none());
    }

    #[test]
    fn extract_flag_value_space_form() {
        let a = argv(&["agentchrome", "interact", "click", "--uid", "s6"]);
        assert_eq!(extract_flag_value(&a, "--uid").as_deref(), Some("s6"));
    }

    #[test]
    fn extract_flag_value_equals_form() {
        let a = argv(&["agentchrome", "interact", "click", "--uid=s7"]);
        assert_eq!(extract_flag_value(&a, "--uid").as_deref(), Some("s7"));
    }

    #[test]
    fn syntax_hint_click_uid_produces_did_you_mean() {
        // Trigger a real clap UnknownArgument error for --uid on `interact click`.
        let result = Cli::try_parse_from(["agentchrome", "interact", "click", "--uid", "s6"]);
        let err = result.err().expect("expected clap error");
        assert_eq!(err.kind(), ErrorKind::UnknownArgument);
        let a = argv(&["agentchrome", "interact", "click", "--uid", "s6"]);
        let hint = syntax_hint(&err, &a).expect("expected a hint");
        assert_eq!(hint, "Did you mean: agentchrome interact click s6");
    }

    #[test]
    fn syntax_hint_suppressed_for_unrelated_clap_errors() {
        // `connect --nonexistent-flag` raises UnknownArgument for a different flag.
        let result = Cli::try_parse_from(["agentchrome", "connect", "--nonexistent-flag"]);
        let err = result.err().expect("expected clap error");
        let a = argv(&["agentchrome", "connect", "--nonexistent-flag"]);
        assert!(
            syntax_hint(&err, &a).is_none(),
            "hint must only fire for --uid / --selector misuse"
        );
    }

    #[test]
    fn syntax_hint_dialog_handle_accept_produces_did_you_mean() {
        let result = Cli::try_parse_from(["agentchrome", "dialog", "handle", "--accept"]);
        let err = result.err().expect("expected clap error");
        assert_eq!(err.kind(), ErrorKind::UnknownArgument);
        let a = argv(&["agentchrome", "dialog", "handle", "--accept"]);
        let hint = syntax_hint(&err, &a).expect("expected a hint");
        assert_eq!(hint, "Did you mean: agentchrome dialog handle accept");
    }

    #[test]
    fn syntax_hint_dialog_handle_dismiss_produces_did_you_mean() {
        let result = Cli::try_parse_from(["agentchrome", "dialog", "handle", "--dismiss"]);
        let err = result.err().expect("expected clap error");
        assert_eq!(err.kind(), ErrorKind::UnknownArgument);
        let a = argv(&["agentchrome", "dialog", "handle", "--dismiss"]);
        let hint = syntax_hint(&err, &a).expect("expected a hint");
        assert_eq!(hint, "Did you mean: agentchrome dialog handle dismiss");
    }

    #[test]
    fn syntax_hint_accept_not_fired_on_unrelated_subcommand() {
        // `--accept` as UnknownArgument on a subcommand other than `dialog handle`
        // must NOT produce the dialog-specific hint.
        let result = Cli::try_parse_from(["agentchrome", "connect", "--accept"]);
        let err = result.err().expect("expected clap error");
        assert_eq!(err.kind(), ErrorKind::UnknownArgument);
        let a = argv(&["agentchrome", "connect", "--accept"]);
        assert!(
            syntax_hint(&err, &a).is_none(),
            "hint must only fire for --accept / --dismiss on `dialog handle`"
        );
    }

    #[test]
    fn syntax_hint_ignores_non_unknown_argument_errors() {
        // Missing required positional on `interact click` → DifferentErrorKind.
        let result = Cli::try_parse_from(["agentchrome", "interact", "click"]);
        let err = result.err().expect("expected clap error");
        assert_ne!(err.kind(), ErrorKind::UnknownArgument);
        let a = argv(&["agentchrome", "interact", "click"]);
        assert!(syntax_hint(&err, &a).is_none());
    }
}
