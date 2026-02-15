use std::time::Duration;

use serde::Serialize;

use chrome_cli::cdp::{CdpClient, CdpConfig, CdpEvent};
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

/// Create a CDP session for dialog commands.
///
/// Unlike the standard `setup_session()` used by other commands, this skips
/// `apply_emulate_state()` because emulation overrides (user-agent, viewport,
/// device scale) are irrelevant for dialog interaction **and** can block when
/// a dialog is already open (e.g. `Runtime.evaluate` inside
/// `apply_emulate_state()` hangs until the dialog is dismissed).
async fn setup_dialog_session(
    global: &GlobalOpts,
) -> Result<(CdpClient, ManagedSession), AppError> {
    let conn = resolve_connection(&global.host, global.port, global.ws_url.as_deref()).await?;
    let target = resolve_target(&conn.host, conn.port, global.tab.as_deref()).await?;

    let config = cdp_config(global);
    let client = CdpClient::connect(&conn.ws_url, config).await?;
    let session = client.create_session(&target.id).await?;
    let managed = ManagedSession::new(session);

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

    // Subscribe to dialog opening event to capture metadata.
    // `Page.handleJavaScriptDialog` and event subscriptions work without
    // `Page.enable` — CDP delivers dialog events at the session level once
    // attached, so we intentionally skip `ensure_domain("Page")` here to
    // avoid blocking when a dialog is already open.
    let dialog_rx = managed.subscribe("Page.javascriptDialogOpening").await?;

    // Build CDP params
    let accept = matches!(args.action, DialogAction::Accept);
    let mut params = serde_json::json!({ "accept": accept });
    if let Some(text) = &args.text {
        params["promptText"] = serde_json::Value::String(text.clone());
    }

    // Handle the dialog
    let handle_result = managed
        .send_command("Page.handleJavaScriptDialog", Some(params))
        .await;

    match handle_result {
        Ok(_) => {
            // Try to extract dialog metadata from the event channel
            let (dialog_type, message, _default_prompt) = drain_dialog_event(dialog_rx);

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
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.contains("No dialog is showing")
                || err_msg.contains("No JavaScript dialog")
                || err_msg.contains("Could not handle dialog")
            {
                Err(AppError::no_dialog_open())
            } else {
                Err(AppError::dialog_handle_failed(&err_msg))
            }
        }
    }
}

// =============================================================================
// Info: query dialog state
// =============================================================================

/// Probe timeout for detecting an open dialog (milliseconds).
const DIALOG_PROBE_TIMEOUT_MS: u64 = 200;

async fn execute_info(global: &GlobalOpts) -> Result<(), AppError> {
    let (_client, managed) = setup_dialog_session(global).await?;

    // Subscribe to dialog opening event.
    // CDP delivers dialog events at the session level without `Page.enable`.
    let dialog_rx = managed.subscribe("Page.javascriptDialogOpening").await?;

    // Probe: try Runtime.evaluate("0") with a short timeout.
    // If a dialog is blocking, this will time out.
    // We intentionally skip `ensure_domain("Runtime")` — `Runtime.evaluate`
    // works without `Runtime.enable`, and `Runtime.enable` itself would block
    // when a dialog is open.
    let probe = managed.send_command(
        "Runtime.evaluate",
        Some(serde_json::json!({ "expression": "0" })),
    );

    let probe_timeout = Duration::from_millis(DIALOG_PROBE_TIMEOUT_MS);
    let dialog_open = match tokio::time::timeout(probe_timeout, probe).await {
        Ok(Ok(_)) => false,          // evaluate succeeded → no dialog blocking
        Ok(Err(_)) | Err(_) => true, // CDP error or timeout → dialog likely blocking
    };

    let result = if dialog_open {
        let (dialog_type, message, default_prompt) = drain_dialog_event(dialog_rx);
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

/// Drain the dialog event channel and extract dialog metadata.
fn drain_dialog_event(mut rx: tokio::sync::mpsc::Receiver<CdpEvent>) -> (String, String, String) {
    if let Ok(event) = rx.try_recv() {
        let dialog_type = event.params["type"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let message = event.params["message"].as_str().unwrap_or("").to_string();
        let default_prompt = event.params["defaultPrompt"]
            .as_str()
            .unwrap_or("")
            .to_string();
        (dialog_type, message, default_prompt)
    } else {
        ("unknown".into(), String::new(), String::new())
    }
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
    fn drain_dialog_event_with_event() {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        tx.try_send(CdpEvent {
            method: "Page.javascriptDialogOpening".into(),
            params: serde_json::json!({
                "type": "confirm",
                "message": "Delete?",
                "defaultPrompt": ""
            }),
            session_id: None,
        })
        .unwrap();
        let (dtype, msg, default) = drain_dialog_event(rx);
        assert_eq!(dtype, "confirm");
        assert_eq!(msg, "Delete?");
        assert_eq!(default, "");
    }

    #[test]
    fn drain_dialog_event_empty_channel() {
        let (_tx, rx) = tokio::sync::mpsc::channel::<CdpEvent>(1);
        let (dtype, msg, default) = drain_dialog_event(rx);
        assert_eq!(dtype, "unknown");
        assert_eq!(msg, "");
        assert_eq!(default, "");
    }
}
