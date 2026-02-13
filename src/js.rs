use std::time::Duration;

use serde::Serialize;

use chrome_cli::cdp::{CdpClient, CdpConfig};
use chrome_cli::connection::{ManagedSession, resolve_connection, resolve_target};
use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, JsArgs, JsCommand, JsExecArgs};

// =============================================================================
// Output types
// =============================================================================

#[derive(Debug, Serialize)]
struct JsExecResult {
    result: serde_json::Value,
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    console: Option<Vec<ConsoleEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncated: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ConsoleEntry {
    level: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct JsExecError {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stack: Option<String>,
    code: u8,
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

// =============================================================================
// Config helper
// =============================================================================

fn cdp_config(global: &GlobalOpts, exec_args: &JsExecArgs) -> CdpConfig {
    let mut config = CdpConfig::default();
    // Execution-specific --timeout overrides global --timeout
    #[allow(clippy::cast_possible_truncation)]
    let default_timeout = config.command_timeout.as_millis() as u64;
    let timeout_ms = exec_args
        .timeout
        .or(global.timeout)
        .unwrap_or(default_timeout);
    config.command_timeout = Duration::from_millis(timeout_ms);
    config
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Execute the `js` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_js(global: &GlobalOpts, args: &JsArgs) -> Result<(), AppError> {
    match &args.command {
        JsCommand::Exec(exec_args) => execute_exec(global, exec_args).await,
    }
}

// =============================================================================
// Session setup
// =============================================================================

async fn setup_session(
    global: &GlobalOpts,
    exec_args: &JsExecArgs,
) -> Result<(CdpClient, ManagedSession), AppError> {
    let conn = resolve_connection(&global.host, global.port, global.ws_url.as_deref()).await?;
    let target = resolve_target(&conn.host, conn.port, global.tab.as_deref()).await?;

    let config = cdp_config(global, exec_args);
    let client = CdpClient::connect(&conn.ws_url, config).await?;
    let session = client.create_session(&target.id).await?;
    let managed = ManagedSession::new(session);

    Ok((client, managed))
}

// =============================================================================
// Code resolution
// =============================================================================

/// Resolve the JavaScript code to execute from the various input sources.
fn resolve_code(args: &JsExecArgs) -> Result<String, AppError> {
    if let Some(ref path) = args.file {
        // --file <PATH>
        let path_str = path.display().to_string();
        if !path.exists() {
            return Err(AppError::script_file_not_found(&path_str));
        }
        std::fs::read_to_string(path)
            .map_err(|e| AppError::script_file_read_failed(&path_str, &e.to_string()))
    } else if let Some(ref code) = args.code {
        if code == "-" {
            // Read from stdin
            std::io::read_to_string(std::io::stdin())
                .map_err(|e| AppError::script_file_read_failed("stdin", &e.to_string()))
        } else {
            Ok(code.clone())
        }
    } else {
        Err(AppError::no_js_code())
    }
}

// =============================================================================
// Result type mapping
// =============================================================================

/// Map CDP result type/subtype to the JavaScript type string for our output.
fn js_type_string(result: &serde_json::Value) -> String {
    let type_str = result["type"].as_str().unwrap_or("undefined");
    match type_str {
        "undefined" => "undefined".to_string(),
        "string" => "string".to_string(),
        "number" => "number".to_string(),
        "boolean" => "boolean".to_string(),
        "object" | "function" => type_str.to_string(),
        other => other.to_string(),
    }
}

/// Extract the result value from a CDP evaluate/callFunctionOn response.
fn extract_result_value(result: &serde_json::Value) -> serde_json::Value {
    let type_str = result["type"].as_str().unwrap_or("undefined");
    match type_str {
        "undefined" => serde_json::Value::Null,
        _ => {
            if result.get("value").is_some() {
                result["value"].clone()
            } else {
                // For objects that can't be serialized by value (e.g., promises with --no-await)
                let subtype = result["subtype"].as_str().unwrap_or("");
                let description = result["description"].as_str().unwrap_or("{}");
                if subtype.is_empty() {
                    serde_json::Value::String(description.to_string())
                } else {
                    serde_json::Value::String(format!("[{subtype}: {description}]"))
                }
            }
        }
    }
}

// =============================================================================
// Truncation
// =============================================================================

/// Apply `--max-size` truncation to a result. Returns (possibly truncated value, was truncated).
fn apply_truncation(
    value: serde_json::Value,
    max_size: Option<usize>,
) -> (serde_json::Value, bool) {
    let Some(max) = max_size else {
        return (value, false);
    };

    let serialized = serde_json::to_string(&value).unwrap_or_default();
    if serialized.len() <= max {
        return (value, false);
    }

    // For strings, truncate the string content
    if let Some(s) = value.as_str() {
        // Truncate to approximately max bytes
        let truncated: String = s.chars().take(max).collect();
        (serde_json::Value::String(truncated), true)
    } else {
        // For non-strings, truncate the serialized form
        let truncated = &serialized[..max.min(serialized.len())];
        // Try to return a valid JSON value; fall back to string
        if let Ok(v) = serde_json::from_str(truncated) {
            (v, true)
        } else {
            (serde_json::Value::String(truncated.to_string()), true)
        }
    }
}

// =============================================================================
// Console capture
// =============================================================================

/// Extract console entries from collected Runtime.consoleAPICalled events.
fn extract_console_entries(events: &[serde_json::Value]) -> Vec<ConsoleEntry> {
    events
        .iter()
        .filter_map(|params| {
            let level = params["type"].as_str().unwrap_or("log").to_string();
            let args = params["args"].as_array()?;
            let text = args
                .iter()
                .filter_map(|arg| {
                    arg["value"].as_str().map(String::from).or_else(|| {
                        // For non-string values, use the description or JSON representation
                        arg["description"]
                            .as_str()
                            .map(String::from)
                            .or_else(|| serde_json::to_string(&arg["value"]).ok())
                    })
                })
                .collect::<Vec<_>>()
                .join(" ");
            Some(ConsoleEntry { level, text })
        })
        .collect()
}

// =============================================================================
// Execution
// =============================================================================

#[allow(clippy::too_many_lines)]
async fn execute_exec(global: &GlobalOpts, args: &JsExecArgs) -> Result<(), AppError> {
    let code = resolve_code(args)?;
    let (_client, mut managed) = setup_session(global, args).await?;

    // Enable Runtime domain
    managed.ensure_domain("Runtime").await?;

    // Subscribe to console events before execution
    let mut console_rx = managed
        .subscribe("Runtime.consoleAPICalled")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to console events: {e}"),
            code: ExitCode::GeneralError,
        })?;

    let await_promise = !args.no_await;

    // Execute based on whether --uid is provided
    let result = if let Some(ref uid) = args.uid {
        // Element context execution via Runtime.callFunctionOn
        execute_with_uid(&mut managed, &code, uid, await_promise).await?
    } else {
        // Expression evaluation via Runtime.evaluate
        execute_expression(&managed, &code, await_promise).await?
    };

    // Collect console events (drain with a short timeout)
    let mut console_events = Vec::new();
    let drain_deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(100);
    loop {
        let remaining = drain_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, console_rx.recv()).await {
            Ok(Some(event)) => console_events.push(event.params),
            Ok(None) | Err(_) => break,
        }
    }

    // Check for exception
    if let Some(exception_details) = result.get("exceptionDetails") {
        let exception = &exception_details["exception"];
        let error_desc = exception["description"]
            .as_str()
            .or_else(|| exception_details["text"].as_str())
            .unwrap_or("unknown error")
            .to_string();

        // Build stack trace string
        let stack = exception["description"]
            .as_str()
            .map(String::from)
            .or_else(|| {
                exception_details["stackTrace"]["callFrames"]
                    .as_array()
                    .map(|frames| {
                        frames
                            .iter()
                            .map(|f| {
                                let func = f["functionName"].as_str().unwrap_or("<anonymous>");
                                let line = f["lineNumber"].as_i64().unwrap_or(0);
                                let col = f["columnNumber"].as_i64().unwrap_or(0);
                                format!("    at {func} (<anonymous>:{line}:{col})")
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
            });

        let js_error = JsExecError {
            error: error_desc.clone(),
            stack,
            code: ExitCode::GeneralError as u8,
        };
        let err_json = serde_json::to_string(&js_error)
            .unwrap_or_else(|_| format!(r#"{{"error":"{error_desc}","code":1}}"#));
        eprintln!("{err_json}");
        return Err(AppError::js_execution_failed(&error_desc));
    }

    // Extract result value and type
    let cdp_result = &result["result"];
    let js_type = js_type_string(cdp_result);
    let value = extract_result_value(cdp_result);

    // Apply truncation
    let (value, was_truncated) = apply_truncation(value, args.max_size);

    // Build console entries
    let console_entries = extract_console_entries(&console_events);

    let output = JsExecResult {
        result: value.clone(),
        r#type: js_type,
        console: if console_entries.is_empty() {
            None
        } else {
            Some(console_entries)
        },
        truncated: if was_truncated { Some(true) } else { None },
    };

    // Output
    if global.output.plain {
        // Plain mode: print raw value
        match &value {
            serde_json::Value::String(s) => print!("{s}"),
            serde_json::Value::Null => print!("undefined"),
            other => {
                let s = serde_json::to_string(other).unwrap_or_default();
                print!("{s}");
            }
        }
        return Ok(());
    }

    print_output(&output, &global.output)
}

/// Execute a JavaScript expression via Runtime.evaluate.
async fn execute_expression(
    managed: &ManagedSession,
    code: &str,
    await_promise: bool,
) -> Result<serde_json::Value, AppError> {
    let params = serde_json::json!({
        "expression": code,
        "returnByValue": true,
        "awaitPromise": await_promise,
        "generatePreview": true,
    });

    managed
        .send_command("Runtime.evaluate", Some(params))
        .await
        .map_err(|e| {
            // Check if it's a timeout error
            let err_str = format!("{e:?}");
            if err_str.contains("CommandTimeout") {
                AppError {
                    message: format!("JavaScript execution timed out: {e}"),
                    code: ExitCode::TimeoutError,
                }
            } else {
                AppError {
                    message: format!("JavaScript execution failed: {e}"),
                    code: ExitCode::GeneralError,
                }
            }
        })
}

/// Execute a function with an element context via Runtime.callFunctionOn.
async fn execute_with_uid(
    managed: &mut ManagedSession,
    code: &str,
    uid: &str,
    await_promise: bool,
) -> Result<serde_json::Value, AppError> {
    // Read snapshot state to get backendNodeId
    let state = crate::snapshot::read_snapshot_state()
        .map_err(|e| AppError {
            message: format!("Failed to read snapshot state: {e}"),
            code: ExitCode::GeneralError,
        })?
        .ok_or_else(|| AppError {
            message: "No snapshot state found. Run 'chrome-cli page snapshot' first.".to_string(),
            code: ExitCode::GeneralError,
        })?;

    let backend_node_id = state
        .uid_map
        .get(uid)
        .ok_or_else(|| AppError::uid_not_found(uid))?;

    // Enable DOM domain for resolveNode
    managed.ensure_domain("DOM").await?;

    // Resolve backendNodeId to a remote objectId
    let resolve_result = managed
        .send_command(
            "DOM.resolveNode",
            Some(serde_json::json!({ "backendNodeId": backend_node_id })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("Failed to resolve UID '{uid}': {e}"),
            code: ExitCode::GeneralError,
        })?;

    let object_id = resolve_result["object"]["objectId"]
        .as_str()
        .ok_or_else(|| AppError {
            message: format!("UID '{uid}' could not be resolved to a DOM object"),
            code: ExitCode::GeneralError,
        })?;

    // Call the function on the resolved element
    let params = serde_json::json!({
        "functionDeclaration": code,
        "objectId": object_id,
        "arguments": [{ "objectId": object_id }],
        "returnByValue": true,
        "awaitPromise": await_promise,
    });

    managed
        .send_command("Runtime.callFunctionOn", Some(params))
        .await
        .map_err(|e| {
            let err_str = format!("{e:?}");
            if err_str.contains("CommandTimeout") {
                AppError {
                    message: format!("JavaScript execution timed out: {e}"),
                    code: ExitCode::TimeoutError,
                }
            } else {
                AppError {
                    message: format!("JavaScript execution failed: {e}"),
                    code: ExitCode::GeneralError,
                }
            }
        })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // JsExecResult serialization tests
    // =========================================================================

    #[test]
    fn js_exec_result_basic_string() {
        let result = JsExecResult {
            result: serde_json::Value::String("hello".to_string()),
            r#type: "string".to_string(),
            console: None,
            truncated: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["result"], "hello");
        assert_eq!(json["type"], "string");
        assert!(json.get("console").is_none());
        assert!(json.get("truncated").is_none());
    }

    #[test]
    fn js_exec_result_number() {
        let result = JsExecResult {
            result: serde_json::json!(42),
            r#type: "number".to_string(),
            console: None,
            truncated: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["result"], 42);
        assert_eq!(json["type"], "number");
    }

    #[test]
    fn js_exec_result_with_console() {
        let result = JsExecResult {
            result: serde_json::json!(42),
            r#type: "number".to_string(),
            console: Some(vec![ConsoleEntry {
                level: "log".to_string(),
                text: "hello".to_string(),
            }]),
            truncated: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["result"], 42);
        let console = json["console"].as_array().unwrap();
        assert_eq!(console.len(), 1);
        assert_eq!(console[0]["level"], "log");
        assert_eq!(console[0]["text"], "hello");
    }

    #[test]
    fn js_exec_result_with_truncation() {
        let result = JsExecResult {
            result: serde_json::Value::String("truncated...".to_string()),
            r#type: "string".to_string(),
            console: None,
            truncated: Some(true),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["truncated"], true);
    }

    #[test]
    fn js_exec_result_object_type() {
        let result = JsExecResult {
            result: serde_json::json!({"key": "val"}),
            r#type: "object".to_string(),
            console: None,
            truncated: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["result"]["key"], "val");
        assert_eq!(json["type"], "object");
    }

    #[test]
    fn js_exec_result_null_value() {
        let result = JsExecResult {
            result: serde_json::Value::Null,
            r#type: "object".to_string(),
            console: None,
            truncated: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert!(json["result"].is_null());
        assert_eq!(json["type"], "object");
    }

    #[test]
    fn js_exec_result_undefined_type() {
        let result = JsExecResult {
            result: serde_json::Value::Null,
            r#type: "undefined".to_string(),
            console: None,
            truncated: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["type"], "undefined");
    }

    #[test]
    fn js_exec_result_boolean() {
        let result = JsExecResult {
            result: serde_json::json!(true),
            r#type: "boolean".to_string(),
            console: None,
            truncated: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["result"], true);
        assert_eq!(json["type"], "boolean");
    }

    #[test]
    fn js_exec_result_array() {
        let result = JsExecResult {
            result: serde_json::json!([1, 2, 3]),
            r#type: "object".to_string(),
            console: None,
            truncated: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        let arr = json["result"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(json["type"], "object");
    }

    // =========================================================================
    // JsExecError serialization tests
    // =========================================================================

    #[test]
    fn js_exec_error_serialization() {
        let err = JsExecError {
            error: "ReferenceError: foo is not defined".to_string(),
            stack: Some("ReferenceError: foo is not defined\n    at <anonymous>:1:1".to_string()),
            code: 1,
        };
        let json: serde_json::Value = serde_json::to_value(&err).unwrap();
        assert_eq!(json["error"], "ReferenceError: foo is not defined");
        assert!(json["stack"].as_str().unwrap().contains("<anonymous>:1:1"));
        assert_eq!(json["code"], 1);
    }

    #[test]
    fn js_exec_error_without_stack() {
        let err = JsExecError {
            error: "Error occurred".to_string(),
            stack: None,
            code: 1,
        };
        let json: serde_json::Value = serde_json::to_value(&err).unwrap();
        assert_eq!(json["error"], "Error occurred");
        assert!(json.get("stack").is_none());
    }

    // =========================================================================
    // js_type_string tests
    // =========================================================================

    #[test]
    fn js_type_string_all_types() {
        assert_eq!(
            js_type_string(&serde_json::json!({"type": "string"})),
            "string"
        );
        assert_eq!(
            js_type_string(&serde_json::json!({"type": "number"})),
            "number"
        );
        assert_eq!(
            js_type_string(&serde_json::json!({"type": "boolean"})),
            "boolean"
        );
        assert_eq!(
            js_type_string(&serde_json::json!({"type": "object"})),
            "object"
        );
        assert_eq!(
            js_type_string(&serde_json::json!({"type": "undefined"})),
            "undefined"
        );
        assert_eq!(
            js_type_string(&serde_json::json!({"type": "function"})),
            "function"
        );
    }

    #[test]
    fn js_type_string_missing_defaults_to_undefined() {
        assert_eq!(js_type_string(&serde_json::json!({})), "undefined");
    }

    // =========================================================================
    // extract_result_value tests
    // =========================================================================

    #[test]
    fn extract_result_value_string() {
        let result = serde_json::json!({"type": "string", "value": "hello"});
        assert_eq!(
            extract_result_value(&result),
            serde_json::Value::String("hello".to_string())
        );
    }

    #[test]
    fn extract_result_value_number() {
        let result = serde_json::json!({"type": "number", "value": 42});
        assert_eq!(extract_result_value(&result), serde_json::json!(42));
    }

    #[test]
    fn extract_result_value_boolean() {
        let result = serde_json::json!({"type": "boolean", "value": true});
        assert_eq!(extract_result_value(&result), serde_json::json!(true));
    }

    #[test]
    fn extract_result_value_null() {
        let result = serde_json::json!({"type": "object", "subtype": "null", "value": null});
        assert_eq!(extract_result_value(&result), serde_json::Value::Null);
    }

    #[test]
    fn extract_result_value_undefined() {
        let result = serde_json::json!({"type": "undefined"});
        assert_eq!(extract_result_value(&result), serde_json::Value::Null);
    }

    #[test]
    fn extract_result_value_object_with_value() {
        let result = serde_json::json!({"type": "object", "value": {"key": "val"}});
        assert_eq!(
            extract_result_value(&result),
            serde_json::json!({"key": "val"})
        );
    }

    #[test]
    fn extract_result_value_object_without_value() {
        let result =
            serde_json::json!({"type": "object", "subtype": "promise", "description": "Promise"});
        assert_eq!(
            extract_result_value(&result),
            serde_json::Value::String("[promise: Promise]".to_string())
        );
    }

    // =========================================================================
    // apply_truncation tests
    // =========================================================================

    #[test]
    fn truncation_not_applied_when_none() {
        let value = serde_json::Value::String("hello".to_string());
        let (result, truncated) = apply_truncation(value.clone(), None);
        assert_eq!(result, value);
        assert!(!truncated);
    }

    #[test]
    fn truncation_not_applied_when_within_limit() {
        let value = serde_json::Value::String("hello".to_string());
        let (result, truncated) = apply_truncation(value.clone(), Some(1000));
        assert_eq!(result, value);
        assert!(!truncated);
    }

    #[test]
    fn truncation_applied_to_long_string() {
        let long_str: String = "x".repeat(10000);
        let value = serde_json::Value::String(long_str);
        let (result, truncated) = apply_truncation(value, Some(100));
        assert!(truncated);
        let s = result.as_str().unwrap();
        assert_eq!(s.len(), 100);
    }

    // =========================================================================
    // extract_console_entries tests
    // =========================================================================

    #[test]
    fn extract_console_entries_log() {
        let events = vec![serde_json::json!({
            "type": "log",
            "args": [{"type": "string", "value": "hello"}]
        })];
        let entries = extract_console_entries(&events);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, "log");
        assert_eq!(entries[0].text, "hello");
    }

    #[test]
    fn extract_console_entries_multiple_args() {
        let events = vec![serde_json::json!({
            "type": "log",
            "args": [
                {"type": "string", "value": "hello"},
                {"type": "string", "value": "world"}
            ]
        })];
        let entries = extract_console_entries(&events);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "hello world");
    }

    #[test]
    fn extract_console_entries_empty() {
        let events: Vec<serde_json::Value> = vec![];
        let entries = extract_console_entries(&events);
        assert!(entries.is_empty());
    }

    #[test]
    fn extract_console_entries_warn_level() {
        let events = vec![serde_json::json!({
            "type": "warning",
            "args": [{"type": "string", "value": "oops"}]
        })];
        let entries = extract_console_entries(&events);
        assert_eq!(entries[0].level, "warning");
    }

    // =========================================================================
    // resolve_code tests (basic — file and stdin can't easily be tested here)
    // =========================================================================

    #[test]
    fn resolve_code_inline() {
        let args = JsExecArgs {
            code: Some("document.title".to_string()),
            file: None,
            uid: None,
            no_await: false,
            timeout: None,
            max_size: None,
        };
        let code = resolve_code(&args).unwrap();
        assert_eq!(code, "document.title");
    }

    #[test]
    fn resolve_code_no_input() {
        let args = JsExecArgs {
            code: None,
            file: None,
            uid: None,
            no_await: false,
            timeout: None,
            max_size: None,
        };
        let err = resolve_code(&args).unwrap_err();
        assert!(err.message.contains("No JavaScript code provided"));
    }

    #[test]
    fn resolve_code_file_not_found() {
        let args = JsExecArgs {
            code: None,
            file: Some(std::path::PathBuf::from("/nonexistent/script.js")),
            uid: None,
            no_await: false,
            timeout: None,
            max_size: None,
        };
        let err = resolve_code(&args).unwrap_err();
        assert!(err.message.contains("Script file not found"));
    }

    #[test]
    fn resolve_code_from_file() {
        let dir = std::env::temp_dir().join("chrome-cli-test-js-resolve");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.js");
        std::fs::write(&path, "document.title").unwrap();

        let args = JsExecArgs {
            code: None,
            file: Some(path),
            uid: None,
            no_await: false,
            timeout: None,
            max_size: None,
        };
        let code = resolve_code(&args).unwrap();
        assert_eq!(code, "document.title");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
