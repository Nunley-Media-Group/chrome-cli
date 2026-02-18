use std::time::Duration;

use serde::Serialize;

use chrome_cli::cdp::{CdpClient, CdpConfig};
use chrome_cli::connection::{ManagedSession, resolve_connection, resolve_target};
use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{DialogAction, DialogArgs, DialogCommand, DialogHandleArgs, GlobalOpts};

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct HandleResult {
    action: String,
    dialog_type: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

#[derive(Serialize)]
struct InfoResult {
    open: bool,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    dialog_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_value: Option<String>,
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

fn print_handle_plain(result: &HandleResult) {
    let action_label = if result.action == "accept" {
        "Accepted"
    } else {
        "Dismissed"
    };
    match &result.text {
        Some(text) => {
            println!(
                "{action_label} {}: \"{}\" (text: \"{text}\")",
                result.dialog_type, result.message
            );
        }
        None => {
            println!(
                "{action_label} {}: \"{}\"",
                result.dialog_type, result.message
            );
        }
    }
}

fn print_info_plain(result: &InfoResult) {
    if !result.open {
        println!("No dialog open");
        return;
    }

    let dialog_type = result.dialog_type.as_deref().unwrap_or("unknown");
    let message = result.message.as_deref().unwrap_or("");

    match &result.default_value {
        Some(default) => {
            println!("Dialog open: {dialog_type} — \"{message}\" (default: \"{default}\")");
        }
        None => {
            println!("Dialog open: {dialog_type} — \"{message}\"");
        }
    }
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

/// Timeout for `Page.enable` during dialog session setup (milliseconds).
///
/// `Page.enable` blocks when a dialog is already open. We use a short timeout
/// so setup can proceed; the blocking itself confirms a dialog is present.
const PAGE_ENABLE_TIMEOUT_MS: u64 = 300;

/// Create a CDP session for dialog commands.
///
/// Unlike the standard `setup_session()` used by other commands, this skips
/// `apply_emulate_state()` because emulation overrides (user-agent, viewport,
/// device scale) are irrelevant for dialog interaction **and** can block when
/// a dialog is already open (e.g. `Runtime.evaluate` inside
/// `apply_emulate_state()` hangs until the dialog is dismissed).
///
/// After creating the session, subscribes to `Page.javascriptDialogOpening`
/// and sends `Page.enable` with a short timeout. While Chrome does NOT
/// re-emit dialog events to new sessions, the event subscription is kept as
/// defense-in-depth for cases where the dialog opens after this session
/// connects.
async fn setup_dialog_session(
    global: &GlobalOpts,
) -> Result<(CdpClient, ManagedSession), AppError> {
    let conn = resolve_connection(&global.host, global.port, global.ws_url.as_deref()).await?;
    let target = resolve_target(&conn.host, conn.port, global.tab.as_deref()).await?;

    let config = cdp_config(global);
    let client = CdpClient::connect(&conn.ws_url, config).await?;
    let session = client.create_session(&target.id).await?;
    let managed = ManagedSession::new(session);

    // Send Page.enable with a short timeout. This blocks when a dialog is
    // already open, which is expected. The timeout lets us proceed.
    let page_enable = managed.send_command("Page.enable", None);
    let _ = tokio::time::timeout(Duration::from_millis(PAGE_ENABLE_TIMEOUT_MS), page_enable).await;

    Ok((client, managed))
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Execute the `dialog` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_dialog(global: &GlobalOpts, args: &DialogArgs) -> Result<(), AppError> {
    match &args.command {
        DialogCommand::Handle(handle_args) => execute_handle(global, handle_args).await,
        DialogCommand::Info => execute_info(global).await,
    }
}

// =============================================================================
// Handle: accept/dismiss dialogs
// =============================================================================

async fn execute_handle(global: &GlobalOpts, args: &DialogHandleArgs) -> Result<(), AppError> {
    let (_client, managed) = setup_dialog_session(global).await?;

    // Read dialog metadata from the interceptor cookie BEFORE handling,
    // because the cookie is cleared by the interceptor after the native
    // dialog function returns (i.e., after we dismiss it).
    let (dialog_type, message, _default_prompt) = read_dialog_cookie(&managed).await;

    // Build CDP params
    let accept = matches!(args.action, DialogAction::Accept);
    let mut params = serde_json::json!({ "accept": accept });
    if let Some(text) = &args.text {
        params["promptText"] = serde_json::Value::String(text.clone());
    }

    // Try standard CDP approach first
    let handle_result = managed
        .send_command("Page.handleJavaScriptDialog", Some(params))
        .await;

    match handle_result {
        Ok(_) => {
            // Standard CDP path worked
        }
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.contains("No dialog is showing")
                || err_msg.contains("No JavaScript dialog")
                || err_msg.contains("Could not handle dialog")
            {
                // CDP doesn't know about the dialog (Page domain wasn't enabled
                // before it opened). Check if a dialog is actually blocking.
                if !probe_dialog_open(&managed).await {
                    return Err(AppError::no_dialog_open());
                }
                // Dialog IS open — fall back to navigation-based dismissal.
                // Page.navigate dismisses blocking dialogs as a side effect.
                dismiss_via_navigation(&managed).await?;

                // Verify the dialog was actually dismissed.
                if probe_dialog_open(&managed).await {
                    return Err(AppError::dialog_handle_failed(
                        "navigation fallback did not dismiss the dialog",
                    ));
                }
            } else {
                return Err(AppError::dialog_handle_failed(&err_msg));
            }
        }
    }

    let result = HandleResult {
        action: if accept {
            "accept".into()
        } else {
            "dismiss".into()
        },
        dialog_type,
        message,
        text: args.text.clone(),
    };

    if global.output.plain {
        print_handle_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

// =============================================================================
// Info: query dialog state
// =============================================================================

/// Probe timeout for detecting an open dialog (milliseconds).
const DIALOG_PROBE_TIMEOUT_MS: u64 = 200;

async fn execute_info(global: &GlobalOpts) -> Result<(), AppError> {
    let (_client, managed) = setup_dialog_session(global).await?;

    // Probe: try Runtime.evaluate("0") with a short timeout.
    // If a dialog is blocking, this will time out.
    let dialog_open = probe_dialog_open(&managed).await;

    let result = if dialog_open {
        // Read dialog metadata from the interceptor cookie.
        let (dialog_type, message, default_prompt) = read_dialog_cookie(&managed).await;
        let default_value = if dialog_type == "prompt" && !default_prompt.is_empty() {
            Some(default_prompt)
        } else {
            None
        };
        InfoResult {
            open: true,
            dialog_type: Some(dialog_type),
            message: Some(message),
            default_value,
        }
    } else {
        InfoResult {
            open: false,
            dialog_type: None,
            message: None,
            default_value: None,
        }
    };

    if global.output.plain {
        print_info_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Check if a JavaScript dialog is blocking the renderer.
///
/// Attempts `Runtime.evaluate("0")` with a short timeout. If it times out
/// or errors, a dialog is likely blocking the renderer.
async fn probe_dialog_open(managed: &ManagedSession) -> bool {
    let probe = managed.send_command(
        "Runtime.evaluate",
        Some(serde_json::json!({ "expression": "0" })),
    );
    match tokio::time::timeout(Duration::from_millis(DIALOG_PROBE_TIMEOUT_MS), probe).await {
        Ok(Ok(_)) => false,          // evaluate succeeded → no dialog blocking
        Ok(Err(_)) | Err(_) => true, // CDP error or timeout → dialog likely blocking
    }
}

/// Timeout for waiting on navigation to complete after dismissing a dialog (ms).
const NAV_DISMISS_TIMEOUT_MS: u64 = 2000;

/// Dismiss a dialog by navigating the page to its current URL.
///
/// When `Page.handleJavaScriptDialog` is unavailable (Page domain was not
/// enabled before the dialog opened), `Page.navigate` can dismiss the dialog
/// as a side effect. The page is reloaded to the same URL, which clears
/// the blocking dialog and restores the page to a usable state.
///
/// `Page.getNavigationHistory` and `Page.navigate` both work without
/// `Page.enable` and are not blocked by the dialog.
async fn dismiss_via_navigation(managed: &ManagedSession) -> Result<(), AppError> {
    let map_err = |e: chrome_cli::cdp::CdpError| AppError::dialog_handle_failed(&e.to_string());

    // Get the current URL from navigation history (works while dialog is blocking)
    let history = managed
        .send_command("Page.getNavigationHistory", None)
        .await
        .map_err(map_err)?;

    let current_url = history["currentIndex"]
        .as_u64()
        .and_then(|idx| usize::try_from(idx).ok())
        .and_then(|idx| history["entries"].get(idx))
        .and_then(|entry| entry["url"].as_str())
        .unwrap_or("about:blank");

    // Navigate to the current URL — this dismisses the blocking dialog
    // and reloads the page.
    let nav_result = tokio::time::timeout(
        Duration::from_millis(NAV_DISMISS_TIMEOUT_MS),
        managed.send_command(
            "Page.navigate",
            Some(serde_json::json!({ "url": current_url })),
        ),
    )
    .await;

    match nav_result {
        Ok(Ok(_)) => {
            // Brief delay for the page to start loading and the dialog to clear
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok(())
        }
        Ok(Err(e)) => Err(AppError::dialog_handle_failed(&e.to_string())),
        Err(_) => Err(AppError::dialog_handle_failed(
            "navigation timed out while dismissing dialog",
        )),
    }
}

/// Cookie name used by the dialog interceptor script to store metadata.
const DIALOG_COOKIE_NAME: &str = "__chrome_cli_dialog";

/// Timeout for reading cookies via CDP (milliseconds).
const COOKIE_READ_TIMEOUT_MS: u64 = 500;

/// Read dialog metadata from the interceptor cookie via `Network.getCookies`.
///
/// The interceptor script (installed by `ManagedSession::install_dialog_interceptors()`)
/// overrides `window.alert`, `window.confirm`, and `window.prompt` to store
/// `{type, message, defaultValue}` in a cookie before calling the original.
///
/// `Network.getCookies` works even while a dialog is blocking the renderer,
/// because the Network domain bypasses the renderer entirely.
///
/// Returns `(type, message, default_value)` or `("unknown", "", "")` if the
/// cookie is not found or cannot be parsed.
async fn read_dialog_cookie(managed: &ManagedSession) -> (String, String, String) {
    let fallback = || ("unknown".into(), String::new(), String::new());

    let result = tokio::time::timeout(
        Duration::from_millis(COOKIE_READ_TIMEOUT_MS),
        managed.send_command("Network.getCookies", None),
    )
    .await;

    let Ok(Ok(cookies)) = result else {
        return fallback();
    };

    let Some(cookie_array) = cookies["cookies"].as_array() else {
        return fallback();
    };

    for cookie in cookie_array {
        if cookie["name"].as_str() == Some(DIALOG_COOKIE_NAME) {
            let encoded = cookie["value"].as_str().unwrap_or("");
            if let Ok(decoded) = urlencoding::decode(encoded) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&decoded) {
                    let dialog_type = parsed["type"].as_str().unwrap_or("unknown").to_string();
                    let message = parsed["message"].as_str().unwrap_or("").to_string();
                    let default_value = parsed["defaultValue"].as_str().unwrap_or("").to_string();
                    return (dialog_type, message, default_value);
                }
            }
        }
    }

    fallback()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_result_serialization_accept() {
        let result = HandleResult {
            action: "accept".into(),
            dialog_type: "alert".into(),
            message: "Hello".into(),
            text: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["action"], "accept");
        assert_eq!(json["dialog_type"], "alert");
        assert_eq!(json["message"], "Hello");
        assert!(json.get("text").is_none());
    }

    #[test]
    fn handle_result_serialization_with_text() {
        let result = HandleResult {
            action: "accept".into(),
            dialog_type: "prompt".into(),
            message: "Enter name:".into(),
            text: Some("Alice".into()),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["action"], "accept");
        assert_eq!(json["dialog_type"], "prompt");
        assert_eq!(json["message"], "Enter name:");
        assert_eq!(json["text"], "Alice");
    }

    #[test]
    fn handle_result_serialization_dismiss() {
        let result = HandleResult {
            action: "dismiss".into(),
            dialog_type: "confirm".into(),
            message: "Are you sure?".into(),
            text: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["action"], "dismiss");
        assert_eq!(json["dialog_type"], "confirm");
        assert_eq!(json["message"], "Are you sure?");
    }

    #[test]
    fn info_result_open() {
        let result = InfoResult {
            open: true,
            dialog_type: Some("prompt".into()),
            message: Some("Enter name:".into()),
            default_value: Some("default".into()),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["open"], true);
        assert_eq!(json["type"], "prompt");
        assert_eq!(json["message"], "Enter name:");
        assert_eq!(json["default_value"], "default");
    }

    #[test]
    fn info_result_closed() {
        let result = InfoResult {
            open: false,
            dialog_type: None,
            message: None,
            default_value: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["open"], false);
        assert!(json.get("type").is_none());
        assert!(json.get("message").is_none());
        assert!(json.get("default_value").is_none());
    }

    #[test]
    fn info_result_open_without_default() {
        let result = InfoResult {
            open: true,
            dialog_type: Some("alert".into()),
            message: Some("Hello".into()),
            default_value: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["open"], true);
        assert_eq!(json["type"], "alert");
        assert!(json.get("default_value").is_none());
    }

    #[test]
    fn plain_text_handle_accept() {
        let result = HandleResult {
            action: "accept".into(),
            dialog_type: "alert".into(),
            message: "Hello".into(),
            text: None,
        };
        // Just verify it doesn't panic
        print_handle_plain(&result);
    }

    #[test]
    fn plain_text_info_closed() {
        let result = InfoResult {
            open: false,
            dialog_type: None,
            message: None,
            default_value: None,
        };
        // Just verify it doesn't panic
        print_info_plain(&result);
    }

    #[test]
    fn parse_dialog_cookie_value() {
        let encoded = "%7B%22type%22%3A%22alert%22%2C%22message%22%3A%22hello%22%2C%22defaultValue%22%3A%22%22%2C%22timestamp%22%3A1234%7D";
        let decoded = urlencoding::decode(encoded).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&decoded).unwrap();
        assert_eq!(parsed["type"], "alert");
        assert_eq!(parsed["message"], "hello");
        assert_eq!(parsed["defaultValue"], "");
    }

    #[test]
    fn parse_dialog_cookie_prompt_with_default() {
        let value = serde_json::json!({
            "type": "prompt",
            "message": "Enter name:",
            "defaultValue": "Alice",
            "timestamp": 1234
        });
        let string = value.to_string();
        let encoded = urlencoding::encode(&string);
        let decoded = urlencoding::decode(&encoded).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&decoded).unwrap();
        assert_eq!(parsed["type"], "prompt");
        assert_eq!(parsed["message"], "Enter name:");
        assert_eq!(parsed["defaultValue"], "Alice");
    }
}
