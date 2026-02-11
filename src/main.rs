mod chrome;
mod cli;
mod error;

use std::time::Duration;

use clap::Parser;
use serde::Serialize;

use chrome::{
    Channel, LaunchConfig, discover_chrome, find_available_port, find_chrome_executable,
    launch_chrome, query_version,
};
use cli::{ChromeChannel, Cli, Command, ConnectArgs, GlobalOpts};
use error::AppError;

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
        Command::Tabs => Err(AppError::not_implemented("tabs")),
        Command::Navigate => Err(AppError::not_implemented("navigate")),
        Command::Page => Err(AppError::not_implemented("page")),
        Command::Dom => Err(AppError::not_implemented("dom")),
        Command::Js => Err(AppError::not_implemented("js")),
        Command::Console => Err(AppError::not_implemented("console")),
        Command::Network => Err(AppError::not_implemented("network")),
        Command::Interact => Err(AppError::not_implemented("interact")),
        Command::Form => Err(AppError::not_implemented("form")),
        Command::Emulate => Err(AppError::not_implemented("emulate")),
        Command::Perf => Err(AppError::not_implemented("perf")),
    }
}

#[derive(Serialize)]
struct ConnectionInfo {
    ws_url: String,
    port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pid: Option<u32>,
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

async fn execute_connect(global: &GlobalOpts, args: &ConnectArgs) -> Result<(), AppError> {
    let timeout = Duration::from_millis(global.timeout.unwrap_or(30_000));

    warn_if_remote_host(&global.host);

    // Strategy 1: Direct WebSocket URL
    if let Some(ws_url) = &global.ws_url {
        // Extract port from URL if possible, otherwise use global port
        let port = extract_port_from_ws_url(ws_url).unwrap_or(global.port);
        let info = ConnectionInfo {
            ws_url: ws_url.clone(),
            port,
            pid: None,
        };
        println!("{}", serde_json::to_string(&info).unwrap());
        return Ok(());
    }

    // Strategy 2: Explicit --launch
    if args.launch {
        return execute_launch(args, timeout).await;
    }

    // Strategy 3: Auto-discover, then auto-launch
    match discover_chrome(&global.host, global.port).await {
        Ok((ws_url, port)) => {
            let info = ConnectionInfo {
                ws_url,
                port,
                pid: None,
            };
            println!("{}", serde_json::to_string(&info).unwrap());
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
                println!("{}", serde_json::to_string(&info).unwrap());
                return Ok(());
            }
            Err(e @ chrome::ChromeError::LaunchFailed(_)) => {
                last_err = Some(e);
                // Retry with a different port
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    }

    Err(last_err
        .unwrap_or_else(|| chrome::ChromeError::LaunchFailed("all port retries exhausted".into()))
        .into())
}

fn extract_port_from_ws_url(url: &str) -> Option<u16> {
    // Parse "ws://host:port/path" or "wss://host:port/path"
    let without_scheme = url
        .strip_prefix("ws://")
        .or_else(|| url.strip_prefix("wss://"))?;
    let host_port = without_scheme.split('/').next()?;
    let port_str = host_port.rsplit(':').next()?;
    port_str.parse().ok()
}
