mod cli;
mod navigate;
mod page;
mod perf;
mod snapshot;
mod tabs;

use std::time::Duration;

use clap::Parser;
use serde::Serialize;

use chrome_cli::chrome::{
    self, Channel, LaunchConfig, discover_chrome, find_available_port, find_chrome_executable,
    launch_chrome, query_version,
};
use chrome_cli::connection::{self, extract_port_from_ws_url};
use chrome_cli::error::{AppError, ExitCode};
use chrome_cli::session::{self, SessionData};

use cli::{ChromeChannel, Cli, Command, ConnectArgs, GlobalOpts};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(&cli).await {
        e.print_json_stderr();
        #[allow(clippy::cast_possible_truncation)]
        std::process::exit(e.code as i32);
    }
}

async fn run(cli: &Cli) -> Result<(), AppError> {
    match &cli.command {
        Command::Connect(args) => execute_connect(&cli.global, args).await,
        Command::Tabs(args) => tabs::execute_tabs(&cli.global, args).await,
        Command::Navigate(args) => navigate::execute_navigate(&cli.global, args).await,
        Command::Page(args) => page::execute_page(&cli.global, args).await,
        Command::Dom => Err(AppError::not_implemented("dom")),
        Command::Js => Err(AppError::not_implemented("js")),
        Command::Console => Err(AppError::not_implemented("console")),
        Command::Network => Err(AppError::not_implemented("network")),
        Command::Interact => Err(AppError::not_implemented("interact")),
        Command::Form => Err(AppError::not_implemented("form")),
        Command::Emulate => Err(AppError::not_implemented("emulate")),
        Command::Perf(args) => perf::execute_perf(&cli.global, args).await,
    }
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
fn save_session(info: &ConnectionInfo) {
    let data = SessionData {
        ws_url: info.ws_url.clone(),
        port: info.port,
        pid: info.pid,
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

    // Strategy 3: Auto-discover, then auto-launch
    match discover_chrome(&global.host, global.port_or_default()).await {
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
            // Try auto-launch if Chrome is available
            match execute_launch(args, timeout).await {
                Ok(()) => Ok(()),
                Err(_) => Err(discover_err.into()),
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
    print_json(&status)?;
    Ok(())
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
        let _ = std::process::Command::new("kill")
            .arg(pid.to_string())
            .output();
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string()])
            .output();
    }
}
