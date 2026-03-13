mod element;
mod find;
mod screenshot;
mod snapshot;
mod text;
mod wait;

use std::time::Duration;

use serde::Serialize;

use agentchrome::cdp::{CdpClient, CdpConfig};
use agentchrome::connection::{ManagedSession, resolve_connection, resolve_target};
use agentchrome::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, PageArgs, PageCommand, PageResizeArgs};
use crate::emulate::apply_emulate_state;

// =============================================================================
// Output formatting
// =============================================================================

fn print_output(value: &impl Serialize, output: &crate::cli::OutputFormat) -> Result<(), AppError> {
    let json = if output.pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };
    let json = json.map_err(|e| AppError {
        message: format!("serialization error: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;
    println!("{json}");
    Ok(())
}

// =============================================================================
// Config helper
// =============================================================================

fn cdp_config(global: &GlobalOpts) -> CdpConfig {
    let mut config = CdpConfig::default();
    if let Some(timeout_ms) = global.timeout {
        config.command_timeout = Duration::from_millis(timeout_ms);
    }
    config
}

// =============================================================================
// Session setup
// =============================================================================

async fn setup_session(global: &GlobalOpts) -> Result<(CdpClient, ManagedSession), AppError> {
    let conn = resolve_connection(&global.host, global.port, global.ws_url.as_deref()).await?;
    let target = resolve_target(
        &conn.host,
        conn.port,
        global.tab.as_deref(),
        global.page_id.as_deref(),
    )
    .await?;

    let config = cdp_config(global);
    let client = CdpClient::connect(&conn.ws_url, config).await?;
    let session = client.create_session(&target.id).await?;
    let mut managed = ManagedSession::new(session);
    apply_emulate_state(&mut managed).await?;
    managed.install_dialog_interceptors().await;

    Ok((client, managed))
}

// =============================================================================
// Page info helper
// =============================================================================

async fn get_page_info(managed: &ManagedSession) -> Result<(String, String), AppError> {
    let url_result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": "location.href" })),
        )
        .await?;

    let title_result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": "document.title" })),
        )
        .await?;

    let url = url_result["result"]["value"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let title = title_result["result"]["value"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    Ok((url, title))
}

// =============================================================================
// Viewport dimensions helper (shared by screenshot + element)
// =============================================================================

/// Get the current viewport dimensions via `Runtime.evaluate`.
async fn get_viewport_dimensions(managed: &ManagedSession) -> Result<(u32, u32), AppError> {
    let result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": "JSON.stringify({ width: window.innerWidth, height: window.innerHeight })",
                "returnByValue": true,
            })),
        )
        .await
        .map_err(|e| {
            AppError::screenshot_failed(&format!("Failed to get viewport dimensions: {e}"))
        })?;

    let value_str = result["result"]["value"]
        .as_str()
        .ok_or_else(|| AppError::screenshot_failed("Failed to read viewport dimensions"))?;
    let dims: serde_json::Value = serde_json::from_str(value_str).map_err(|e| {
        AppError::screenshot_failed(&format!("Failed to parse viewport dimensions: {e}"))
    })?;

    #[allow(clippy::cast_possible_truncation)]
    let width = dims["width"].as_u64().unwrap_or(1280) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let height = dims["height"].as_u64().unwrap_or(720) as u32;

    Ok((width, height))
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Execute the `page` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_page(global: &GlobalOpts, args: &PageArgs) -> Result<(), AppError> {
    match &args.command {
        PageCommand::Text(text_args) => text::execute_text(global, text_args).await,
        PageCommand::Snapshot(snap_args) => snapshot::execute_snapshot(global, snap_args).await,
        PageCommand::Find(find_args) => find::execute_find(global, find_args).await,
        PageCommand::Screenshot(ss_args) => screenshot::execute_screenshot(global, ss_args).await,
        PageCommand::Resize(resize_args) => execute_page_resize(global, resize_args).await,
        PageCommand::Element(elem_args) => element::execute_element(global, elem_args).await,
        PageCommand::Wait(wait_args) => wait::execute_wait(global, wait_args).await,
    }
}

async fn execute_page_resize(global: &GlobalOpts, args: &PageResizeArgs) -> Result<(), AppError> {
    crate::emulate::execute_resize(global, &args.size).await
}
