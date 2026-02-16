use std::time::Duration;

use serde::Serialize;

use chrome_cli::cdp::{CdpClient, CdpConfig, CdpEvent};
use chrome_cli::connection::{ManagedSession, resolve_connection, resolve_target};
use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{
    GlobalOpts, NavigateArgs, NavigateCommand, NavigateReloadArgs, NavigateUrlArgs, WaitUntil,
};
use crate::emulate::apply_emulate_state;

/// Default navigation wait timeout in milliseconds.
const DEFAULT_NAVIGATE_TIMEOUT_MS: u64 = 30_000;

/// Network idle threshold in milliseconds.
const NETWORK_IDLE_MS: u64 = 500;

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct NavigateResult {
    url: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
}

#[derive(Serialize)]
struct HistoryResult {
    url: String,
    title: String,
}

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
// Dispatcher
// =============================================================================

/// Execute the `navigate` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_navigate(global: &GlobalOpts, args: &NavigateArgs) -> Result<(), AppError> {
    match &args.command {
        Some(NavigateCommand::Back) => execute_back(global).await,
        Some(NavigateCommand::Forward) => execute_forward(global).await,
        Some(NavigateCommand::Reload(reload_args)) => execute_reload(global, reload_args).await,
        None => execute_url(global, &args.url_args).await,
    }
}

// =============================================================================
// Session setup
// =============================================================================

async fn setup_session(global: &GlobalOpts) -> Result<(CdpClient, ManagedSession), AppError> {
    let conn = resolve_connection(&global.host, global.port, global.ws_url.as_deref()).await?;
    let target = resolve_target(&conn.host, conn.port, global.tab.as_deref()).await?;

    let config = cdp_config(global);
    let client = CdpClient::connect(&conn.ws_url, config).await?;
    let session = client.create_session(&target.id).await?;
    let mut managed = ManagedSession::new(session);
    apply_emulate_state(&mut managed).await?;

    Ok((client, managed))
}

// =============================================================================
// URL navigation
// =============================================================================

async fn execute_url(global: &GlobalOpts, args: &NavigateUrlArgs) -> Result<(), AppError> {
    let url = args.url.as_deref().ok_or_else(|| AppError {
        message: "URL is required. Usage: chrome-cli navigate <URL>".into(),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;

    let timeout_ms = args.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS);
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Enable required domains
    managed.ensure_domain("Page").await?;
    managed.ensure_domain("Network").await?;

    // Subscribe to events BEFORE navigating
    let response_rx = managed.subscribe("Network.responseReceived").await?;

    // Subscribe for wait strategy
    let wait_rx = match args.wait_until {
        WaitUntil::Load => Some(managed.subscribe("Page.loadEventFired").await?),
        WaitUntil::Domcontentloaded => Some(managed.subscribe("Page.domContentEventFired").await?),
        WaitUntil::Networkidle | WaitUntil::None => None,
    };

    // For network idle, we need request tracking subscriptions
    let network_subs = if args.wait_until == WaitUntil::Networkidle {
        let req_rx = managed.subscribe("Network.requestWillBeSent").await?;
        let fin_rx = managed.subscribe("Network.loadingFinished").await?;
        let fail_rx = managed.subscribe("Network.loadingFailed").await?;
        Some((req_rx, fin_rx, fail_rx))
    } else {
        None
    };

    // Build navigate params
    let params = serde_json::json!({ "url": url });
    if args.ignore_cache {
        // Page.navigate doesn't have ignoreCache; we use Network.setCacheDisabled instead
        managed
            .send_command(
                "Network.setCacheDisabled",
                Some(serde_json::json!({ "cacheDisabled": true })),
            )
            .await?;
    }

    // Navigate
    let result = managed.send_command("Page.navigate", Some(params)).await?;

    // Check for navigation errors (e.g., DNS failure)
    if let Some(error_text) = result["errorText"].as_str() {
        if !error_text.is_empty() {
            return Err(AppError::navigation_failed(error_text));
        }
    }

    let frame_id = result["frameId"].as_str().unwrap_or_default().to_string();

    // Wait according to strategy
    match args.wait_until {
        WaitUntil::Load | WaitUntil::Domcontentloaded => {
            if let Some(rx) = wait_rx {
                wait_for_event(rx, timeout_ms, &format!("{:?}", args.wait_until)).await?;
            }
        }
        WaitUntil::Networkidle => {
            if let Some((req_rx, fin_rx, fail_rx)) = network_subs {
                wait_for_network_idle(req_rx, fin_rx, fail_rx, timeout_ms).await?;
            }
        }
        WaitUntil::None => {}
    }

    // Extract HTTP status from responseReceived events
    let status = extract_http_status(response_rx, &frame_id);

    // Get final page info
    let (page_url, page_title) = get_page_info(&managed).await?;

    let output = NavigateResult {
        url: page_url,
        title: page_title,
        status,
    };

    print_output(&output, &global.output)
}

// =============================================================================
// History: Back
// =============================================================================

async fn execute_back(global: &GlobalOpts) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("Page").await?;

    // Get navigation history
    let history = managed
        .send_command("Page.getNavigationHistory", None)
        .await?;

    #[allow(clippy::cast_possible_truncation)]
    let current_index = history["currentIndex"].as_u64().unwrap_or(0) as usize;

    if current_index == 0 {
        return Err(AppError {
            message: "Cannot go back: already at the beginning of history.".into(),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }

    let entries = history["entries"].as_array().ok_or_else(|| AppError {
        message: "Invalid navigation history response".into(),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;

    let target_entry = &entries[current_index - 1];
    let entry_id = target_entry["id"].as_i64().unwrap_or(0);

    // Subscribe to frameNavigated before navigating (fires reliably for cross-origin)
    let nav_rx = managed.subscribe("Page.frameNavigated").await?;

    // Navigate to history entry
    managed
        .send_command(
            "Page.navigateToHistoryEntry",
            Some(serde_json::json!({ "entryId": entry_id })),
        )
        .await?;

    // Wait for navigation
    wait_for_event(nav_rx, DEFAULT_NAVIGATE_TIMEOUT_MS, "navigation").await?;

    let (page_url, page_title) = get_page_info(&managed).await?;

    let output = HistoryResult {
        url: page_url,
        title: page_title,
    };

    print_output(&output, &global.output)
}

// =============================================================================
// History: Forward
// =============================================================================

async fn execute_forward(global: &GlobalOpts) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("Page").await?;

    let history = managed
        .send_command("Page.getNavigationHistory", None)
        .await?;

    #[allow(clippy::cast_possible_truncation)]
    let current_index = history["currentIndex"].as_u64().unwrap_or(0) as usize;

    let entries = history["entries"].as_array().ok_or_else(|| AppError {
        message: "Invalid navigation history response".into(),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;

    let next_index = current_index + 1;
    if next_index >= entries.len() {
        return Err(AppError {
            message: "Cannot go forward: already at the end of history.".into(),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }

    let target_entry = &entries[next_index];
    let entry_id = target_entry["id"].as_i64().unwrap_or(0);

    let nav_rx = managed.subscribe("Page.frameNavigated").await?;

    managed
        .send_command(
            "Page.navigateToHistoryEntry",
            Some(serde_json::json!({ "entryId": entry_id })),
        )
        .await?;

    wait_for_event(nav_rx, DEFAULT_NAVIGATE_TIMEOUT_MS, "navigation").await?;

    let (page_url, page_title) = get_page_info(&managed).await?;

    let output = HistoryResult {
        url: page_url,
        title: page_title,
    };

    print_output(&output, &global.output)
}

// =============================================================================
// Reload
// =============================================================================

async fn execute_reload(global: &GlobalOpts, args: &NavigateReloadArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("Page").await?;

    let load_rx = managed.subscribe("Page.loadEventFired").await?;

    let params = serde_json::json!({ "ignoreCache": args.ignore_cache });
    managed.send_command("Page.reload", Some(params)).await?;

    wait_for_event(load_rx, DEFAULT_NAVIGATE_TIMEOUT_MS, "load").await?;

    let (page_url, page_title) = get_page_info(&managed).await?;

    let output = HistoryResult {
        url: page_url,
        title: page_title,
    };

    print_output(&output, &global.output)
}

// =============================================================================
// Wait strategies
// =============================================================================

async fn wait_for_event(
    mut rx: tokio::sync::mpsc::Receiver<CdpEvent>,
    timeout_ms: u64,
    strategy: &str,
) -> Result<(), AppError> {
    let timeout = Duration::from_millis(timeout_ms);
    tokio::select! {
        event = rx.recv() => {
            match event {
                Some(_) => Ok(()),
                None => Err(AppError {
                    message: format!("Event channel closed while waiting for {strategy}"),
                    code: ExitCode::GeneralError,
                    custom_json: None,
                }),
            }
        }
        () = tokio::time::sleep(timeout) => {
            Err(AppError::navigation_timeout(timeout_ms, strategy))
        }
    }
}

async fn wait_for_network_idle(
    mut req_rx: tokio::sync::mpsc::Receiver<CdpEvent>,
    mut fin_rx: tokio::sync::mpsc::Receiver<CdpEvent>,
    mut fail_rx: tokio::sync::mpsc::Receiver<CdpEvent>,
    timeout_ms: u64,
) -> Result<(), AppError> {
    let timeout = Duration::from_millis(timeout_ms);
    let idle_duration = Duration::from_millis(NETWORK_IDLE_MS);
    let deadline = tokio::time::Instant::now() + timeout;

    let mut in_flight: i64 = 0;
    let idle_timer = tokio::time::sleep(idle_duration);
    tokio::pin!(idle_timer);

    loop {
        tokio::select! {
            event = req_rx.recv() => {
                if event.is_some() {
                    in_flight += 1;
                    // Reset idle timer
                    idle_timer.as_mut().reset(tokio::time::Instant::now() + idle_duration);
                }
            }
            event = fin_rx.recv() => {
                if event.is_some() {
                    in_flight = (in_flight - 1).max(0);
                    if in_flight == 0 {
                        idle_timer.as_mut().reset(tokio::time::Instant::now() + idle_duration);
                    }
                }
            }
            event = fail_rx.recv() => {
                if event.is_some() {
                    in_flight = (in_flight - 1).max(0);
                    if in_flight == 0 {
                        idle_timer.as_mut().reset(tokio::time::Instant::now() + idle_duration);
                    }
                }
            }
            () = &mut idle_timer => {
                if in_flight == 0 {
                    return Ok(());
                }
                // Reset timer if still in-flight
                idle_timer.as_mut().reset(tokio::time::Instant::now() + idle_duration);
            }
            () = tokio::time::sleep_until(deadline) => {
                return Err(AppError::navigation_timeout(timeout_ms, "networkidle"));
            }
        }
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Get the current page URL and title via `Runtime.evaluate`.
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

/// Extract the HTTP status code from buffered `Network.responseReceived` events,
/// matching the main frame.
fn extract_http_status(
    mut rx: tokio::sync::mpsc::Receiver<CdpEvent>,
    frame_id: &str,
) -> Option<u16> {
    // Drain all buffered events
    let mut status = None;
    while let Ok(event) = rx.try_recv() {
        // Match the response for the main frame document
        let event_frame = event.params["frameId"].as_str().unwrap_or_default();
        let resource_type = event.params["type"].as_str().unwrap_or_default();
        if event_frame == frame_id && resource_type == "Document" {
            if let Some(s) = event.params["response"]["status"].as_u64() {
                #[allow(clippy::cast_possible_truncation)]
                {
                    status = Some(s as u16);
                }
            }
        }
    }
    status
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigate_result_serialization() {
        let result = NavigateResult {
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            status: Some(200),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["url"], "https://example.com");
        assert_eq!(json["title"], "Example");
        assert_eq!(json["status"], 200);
    }

    #[test]
    fn navigate_result_without_status() {
        let result = NavigateResult {
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            status: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["url"], "https://example.com");
        assert!(json.get("status").is_none());
    }

    #[test]
    fn history_result_serialization() {
        let result = HistoryResult {
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["url"], "https://example.com");
        assert_eq!(json["title"], "Example");
    }

    #[test]
    fn wait_until_default_is_load() {
        let default = WaitUntil::default();
        assert_eq!(default, WaitUntil::Load);
    }
}
