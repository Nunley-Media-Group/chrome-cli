mod capabilities;
mod cli;
mod console;
mod dialog;
mod dom;
mod emulate;
mod examples;
mod form;
mod interact;
mod js;
mod navigate;
mod network;
mod page;
mod perf;
mod snapshot;
mod tabs;

use std::time::Duration;

use clap::{CommandFactory, Parser, error::ErrorKind};
use serde::Serialize;

use chrome_cli::chrome::{
    self, Channel, LaunchConfig, discover_chrome, find_available_port, find_chrome_executable,
    launch_chrome, query_version,
};
use chrome_cli::config;
use chrome_cli::connection::{self, extract_port_from_ws_url};
use chrome_cli::error::{AppError, ExitCode};
use chrome_cli::session::{self, SessionData};

use cli::{
    ChromeChannel, Cli, Command, CompletionsArgs, ConfigCommand, ConnectArgs, GlobalOpts, ManArgs,
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

async fn run(cli: &Cli) -> Result<(), AppError> {
    // Load config file (if any) and apply defaults to global opts
    let (config_path, config_file) = config::load_config(cli.global.config.as_deref());
    let global = apply_config_defaults(&cli.global, &config_file);

    match &cli.command {
        Command::Config(args) => {
            let resolved = build_resolved_config(&global, &config_file, config_path);
            execute_config(&args.command, &resolved)
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
        Command::Emulate(args) => emulate::execute_emulate(&global, args).await,
        Command::Perf(args) => perf::execute_perf(&global, args).await,
        Command::Dialog(args) => dialog::execute_dialog(&global, args).await,
        Command::Examples(args) => examples::execute_examples(&global, args),
        Command::Capabilities(args) => capabilities::execute_capabilities(&global, args),
        Command::Completions(args) => execute_completions(args),
        Command::Man(args) => execute_man(args),
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
        cli_global.host == "127.0.0.1" && std::env::var("CHROME_CLI_HOST").is_err();

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
        auto_dismiss_dialogs: cli_global.auto_dismiss_dialogs,
        config: cli_global.config.clone(),
        output: cli::OutputFormat {
            json: cli_global.output.json,
            pretty: cli_global.output.pretty,
            plain: cli_global.output.plain,
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

/// Execute config subcommands.
fn execute_config(cmd: &ConfigCommand, resolved: &config::ResolvedConfig) -> Result<(), AppError> {
    match cmd {
        ConfigCommand::Show => {
            print_json(resolved)?;
            Ok(())
        }
        ConfigCommand::Init(args) => {
            let path = config::init_config(args.path.as_deref())?;
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
    clap_complete::generate(args.shell, &mut cmd, "chrome-cli", &mut std::io::stdout());
    Ok(())
}

fn execute_man(args: &ManArgs) -> Result<(), AppError> {
    let cmd = Cli::command();

    let target = match &args.command {
        None => cmd,
        Some(name) => find_subcommand(&cmd, name).ok_or_else(|| AppError {
            message: format!("unknown command: {name}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?,
    };

    let man = clap_mangen::Man::new(target);
    man.render(&mut std::io::stdout()).map_err(|e| AppError {
        message: format!("failed to render man page: {e}"),
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
    ws_url: String,
    port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pid: Option<u32>,
    timestamp: String,
    reachable: bool,
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
    let pid = info.pid.or_else(|| existing.as_ref().and_then(|e| e.pid));
    let active_tab_id = existing.and_then(|e| e.active_tab_id);

    let data = SessionData {
        ws_url: info.ws_url.clone(),
        port: info.port,
        pid,
        active_tab_id,
        timestamp: session::now_iso8601(),
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
    let session_data = session::read_session()?.ok_or_else(AppError::no_session)?;

    let reachable = connection::health_check(&global.host, session_data.port)
        .await
        .is_ok();

    let status = StatusInfo {
        ws_url: session_data.ws_url,
        port: session_data.port,
        pid: session_data.pid,
        timestamp: session_data.timestamp,
        reachable,
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
            // kill(pid, 0) checks if the process exists without sending a signal.
            // SAFETY: signal 0 is a null signal used only for existence checks.
            let exists = unsafe { libc::kill(pid_i32, 0) };
            if exists != 0 {
                // Process no longer exists — SIGTERM worked.
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
