use std::io::Write;
use std::time::Duration;

use serde::Serialize;

use chrome_cli::cdp::{CdpClient, CdpConfig};
use chrome_cli::connection::{ManagedSession, resolve_connection, resolve_target};
use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{ConsoleArgs, ConsoleCommand, ConsoleFollowArgs, ConsoleReadArgs, GlobalOpts};
use crate::emulate::apply_emulate_state;

// =============================================================================
// Output types
// =============================================================================

/// A console message in list mode.
#[derive(Clone, Debug, Serialize)]
pub struct ConsoleMessage {
    id: usize,
    #[serde(rename = "type")]
    msg_type: String,
    text: String,
    timestamp: String,
    url: String,
    line: u64,
    column: u64,
}

/// A console message in detail mode (single message with full args and stack trace).
#[derive(Debug, Serialize)]
struct ConsoleMessageDetail {
    id: usize,
    #[serde(rename = "type")]
    msg_type: String,
    text: String,
    timestamp: String,
    url: String,
    line: u64,
    column: u64,
    args: Vec<serde_json::Value>,
    #[serde(rename = "stackTrace")]
    stack_trace: Vec<StackFrame>,
}

/// A single stack frame in a console message detail.
#[derive(Clone, Debug, Serialize)]
struct StackFrame {
    file: String,
    line: u64,
    column: u64,
    #[serde(rename = "functionName")]
    function_name: String,
}

/// A console message emitted by `console follow` (one JSON line per message).
#[derive(Debug, Serialize)]
struct StreamMessage {
    #[serde(rename = "type")]
    msg_type: String,
    text: String,
    timestamp: String,
}

/// Raw collected event data before filtering.
struct RawConsoleEvent {
    params: serde_json::Value,
    navigation_id: u32,
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

fn print_read_plain(messages: &[ConsoleMessage]) {
    for msg in messages {
        let prefix = match msg.msg_type.as_str() {
            "error" | "assert" => "ERR",
            "warn" => "WRN",
            "info" => "INF",
            "debug" => "DBG",
            _ => "LOG",
        };
        println!("[{prefix}] {}", msg.text);
    }
}

fn print_detail_plain(detail: &ConsoleMessageDetail) {
    let prefix = match detail.msg_type.as_str() {
        "error" | "assert" => "ERR",
        "warn" => "WRN",
        "info" => "INF",
        "debug" => "DBG",
        _ => "LOG",
    };
    println!("[{prefix}] {}", detail.text);
    println!("  Source: {}:{}:{}", detail.url, detail.line, detail.column);
    println!("  Timestamp: {}", detail.timestamp);
    if !detail.stack_trace.is_empty() {
        println!("  Stack trace:");
        for frame in &detail.stack_trace {
            let func = if frame.function_name.is_empty() {
                "<anonymous>"
            } else {
                &frame.function_name
            };
            println!(
                "    at {func} ({}:{}:{})",
                frame.file, frame.line, frame.column
            );
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
// Helpers
// =============================================================================

/// Maximum number of stack frames to include in detail output.
const MAX_STACK_FRAMES: usize = 50;

/// Map CDP console type names to our simplified type names.
fn map_cdp_type(cdp_type: &str) -> &str {
    match cdp_type {
        "warning" => "warn",
        other => other,
    }
}

/// Format CDP `RemoteObject` args into a single text string.
fn format_console_args(args: &[serde_json::Value]) -> String {
    args.iter()
        .filter_map(|arg| {
            arg["value"]
                .as_str()
                .map(String::from)
                .or_else(|| {
                    // For non-string primitive values, convert directly
                    if arg["type"].as_str() == Some("string") {
                        arg["value"].as_str().map(String::from)
                    } else if let Some(val) = arg.get("value") {
                        if !val.is_null() {
                            return serde_json::to_string(val).ok();
                        }
                        arg["description"].as_str().map(String::from)
                    } else {
                        None
                    }
                })
                .or_else(|| arg["description"].as_str().map(String::from))
                .or_else(|| {
                    let val = &arg["value"];
                    if val.is_null() && arg["type"].as_str() == Some("undefined") {
                        Some("undefined".to_string())
                    } else {
                        serde_json::to_string(val).ok()
                    }
                })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract stack trace frames from a CDP event's stackTrace field.
fn extract_stack_trace(stack_trace: &serde_json::Value, max_frames: usize) -> Vec<StackFrame> {
    let Some(call_frames) = stack_trace["callFrames"].as_array() else {
        return Vec::new();
    };

    call_frames
        .iter()
        .take(max_frames)
        .map(|f| StackFrame {
            file: f["url"].as_str().unwrap_or("").to_string(),
            line: f["lineNumber"].as_u64().unwrap_or(0),
            column: f["columnNumber"].as_u64().unwrap_or(0),
            function_name: f["functionName"].as_str().unwrap_or("").to_string(),
        })
        .collect()
}

/// Convert a CDP timestamp (epoch milliseconds, floating point) to ISO 8601 string.
///
/// CDP `Runtime.consoleAPICalled` provides timestamps as milliseconds since epoch.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::similar_names
)]
fn timestamp_to_iso(ts: f64) -> String {
    // CDP timestamps are in milliseconds since epoch
    let millis = ts as u64;
    let secs = millis / 1000;
    let ms_part = millis % 1000;

    // Civil date/time from epoch seconds (Howard Hinnant's algorithm)
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    let z = days_since_epoch as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}.{ms_part:03}Z")
}

/// Parse a `Runtime.consoleAPICalled` event into a `ConsoleMessage` for list mode.
fn parse_console_event(event_params: &serde_json::Value, id: usize) -> Option<ConsoleMessage> {
    let raw_type = event_params["type"].as_str().unwrap_or("log");
    let msg_type = map_cdp_type(raw_type).to_string();
    let args = event_params["args"].as_array()?;
    let text = format_console_args(args);
    let timestamp = event_params["timestamp"]
        .as_f64()
        .map_or_else(String::new, timestamp_to_iso);

    let stack = &event_params["stackTrace"];
    let url = stack["callFrames"]
        .as_array()
        .and_then(|frames| frames.first())
        .and_then(|f| f["url"].as_str())
        .unwrap_or("")
        .to_string();
    let line = stack["callFrames"]
        .as_array()
        .and_then(|frames| frames.first())
        .and_then(|f| f["lineNumber"].as_u64())
        .unwrap_or(0);
    let column = stack["callFrames"]
        .as_array()
        .and_then(|frames| frames.first())
        .and_then(|f| f["columnNumber"].as_u64())
        .unwrap_or(0);

    Some(ConsoleMessage {
        id,
        msg_type,
        text,
        timestamp,
        url,
        line,
        column,
    })
}

/// Parse a `Runtime.consoleAPICalled` event into a `ConsoleMessageDetail` for detail mode.
fn parse_console_event_detail(
    event_params: &serde_json::Value,
    id: usize,
) -> Option<ConsoleMessageDetail> {
    let raw_type = event_params["type"].as_str().unwrap_or("log");
    let msg_type = map_cdp_type(raw_type).to_string();
    let raw_args = event_params["args"].as_array()?;
    let text = format_console_args(raw_args);
    let timestamp = event_params["timestamp"]
        .as_f64()
        .map_or_else(String::new, timestamp_to_iso);

    let stack = &event_params["stackTrace"];
    let stack_trace = extract_stack_trace(stack, MAX_STACK_FRAMES);

    let url = stack_trace
        .first()
        .map_or_else(String::new, |f| f.file.clone());
    let line = stack_trace.first().map_or(0, |f| f.line);
    let column = stack_trace.first().map_or(0, |f| f.column);

    let args: Vec<serde_json::Value> = raw_args.clone();

    Some(ConsoleMessageDetail {
        id,
        msg_type,
        text,
        timestamp,
        url,
        line,
        column,
        args,
        stack_trace,
    })
}

/// Resolve `--type` / `--errors-only` into an optional type filter list.
fn resolve_type_filter(type_arg: Option<&str>, errors_only: bool) -> Option<Vec<String>> {
    if errors_only {
        return Some(vec!["error".to_string(), "assert".to_string()]);
    }
    type_arg.map(|types| types.split(',').map(|t| t.trim().to_string()).collect())
}

/// Filter messages by type list.
fn filter_by_type(messages: Vec<ConsoleMessage>, types: &[String]) -> Vec<ConsoleMessage> {
    messages
        .into_iter()
        .filter(|m| types.iter().any(|t| t == &m.msg_type))
        .collect()
}

/// Apply pagination (limit + page offset).
fn paginate(messages: Vec<ConsoleMessage>, limit: usize, page: usize) -> Vec<ConsoleMessage> {
    let offset = page * limit;
    messages.into_iter().skip(offset).take(limit).collect()
}

/// Check if a message type is error-level (for follow exit code tracking).
fn is_error_level(msg_type: &str) -> bool {
    matches!(msg_type, "error" | "assert")
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Execute the `console` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_console(global: &GlobalOpts, args: &ConsoleArgs) -> Result<(), AppError> {
    match &args.command {
        ConsoleCommand::Read(read_args) => execute_read(global, read_args).await,
        ConsoleCommand::Follow(follow_args) => execute_follow(global, follow_args).await,
    }
}

// =============================================================================
// Read: list and detail modes
// =============================================================================

/// Default timeout for the reload+drain cycle in milliseconds.
const DEFAULT_RELOAD_TIMEOUT_MS: u64 = 5000;

/// Idle window after page load event to catch trailing console messages from deferred scripts (ms).
const POST_LOAD_IDLE_MS: u64 = 200;

#[allow(clippy::too_many_lines)]
async fn execute_read(global: &GlobalOpts, args: &ConsoleReadArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;

    let total_timeout = Duration::from_millis(global.timeout.unwrap_or(DEFAULT_RELOAD_TIMEOUT_MS));

    // Enable Runtime domain for console events
    managed.ensure_domain("Runtime").await?;

    // Enable Page domain for reload and navigation tracking
    managed.ensure_domain("Page").await?;

    // Subscribe to console events
    let mut console_rx = managed
        .subscribe("Runtime.consoleAPICalled")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to console events: {e}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

    // Subscribe to navigation events for tracking
    let mut nav_rx = managed
        .subscribe("Page.frameNavigated")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to navigation events: {e}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

    // Subscribe to page load event to know when reload completes
    let mut load_event_rx = managed
        .subscribe("Page.loadEventFired")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to page load events: {e}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

    // Trigger a page reload to replay page scripts and regenerate console events
    managed
        .send_command("Page.reload", Some(serde_json::json!({})))
        .await
        .map_err(|e| AppError {
            message: format!("Failed to reload page: {e}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

    // Collect events until page load completes + idle window, with total timeout
    let mut raw_events = Vec::new();
    let mut current_nav_id: u32 = 0;
    let mut page_loaded = false;
    let absolute_deadline = tokio::time::Instant::now() + total_timeout;
    let mut idle_deadline: Option<tokio::time::Instant> = None;

    loop {
        let effective_deadline = match idle_deadline {
            Some(idle) => idle.min(absolute_deadline),
            None => absolute_deadline,
        };
        let remaining = effective_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        tokio::select! {
            event = console_rx.recv() => {
                match event {
                    Some(ev) => {
                        raw_events.push(RawConsoleEvent {
                            params: ev.params,
                            navigation_id: current_nav_id,
                        });
                    }
                    None => break,
                }
            }
            event = nav_rx.recv() => {
                match event {
                    Some(_) => current_nav_id += 1,
                    None => break,
                }
            }
            event = load_event_rx.recv() => {
                match event {
                    Some(_) => {
                        if !page_loaded {
                            page_loaded = true;
                            idle_deadline = Some(
                                tokio::time::Instant::now()
                                    + tokio::time::Duration::from_millis(POST_LOAD_IDLE_MS),
                            );
                        }
                    }
                    None => break,
                }
            }
            () = tokio::time::sleep(remaining) => break,
        }
    }

    // Navigation-aware filtering: keep events from the last 3 navigations
    let events_to_process = if args.include_preserved {
        // Include up to last 3 navigations
        let min_nav_id = current_nav_id.saturating_sub(2);
        raw_events
            .into_iter()
            .filter(|e| e.navigation_id >= min_nav_id)
            .collect::<Vec<_>>()
    } else {
        // Only current navigation
        raw_events
            .into_iter()
            .filter(|e| e.navigation_id == current_nav_id)
            .collect::<Vec<_>>()
    };

    // Handle detail mode (MSG_ID provided)
    if let Some(msg_id) = args.msg_id {
        #[allow(clippy::cast_possible_truncation)]
        let id = msg_id as usize;
        if id >= events_to_process.len() {
            return Err(AppError {
                message: format!("Message ID {id} not found"),
                code: ExitCode::GeneralError,
                custom_json: None,
            });
        }
        let detail =
            parse_console_event_detail(&events_to_process[id].params, id).ok_or_else(|| {
                AppError {
                    message: format!("Failed to parse message ID {id}"),
                    code: ExitCode::GeneralError,
                    custom_json: None,
                }
            })?;

        if global.output.plain {
            print_detail_plain(&detail);
            return Ok(());
        }
        return print_output(&detail, &global.output);
    }

    // List mode: parse all events into ConsoleMessages
    let messages: Vec<ConsoleMessage> = events_to_process
        .iter()
        .enumerate()
        .filter_map(|(i, e)| parse_console_event(&e.params, i))
        .collect();

    // Apply type filter
    let type_filter = resolve_type_filter(args.r#type.as_deref(), args.errors_only);
    let messages = if let Some(ref types) = type_filter {
        filter_by_type(messages, types)
    } else {
        messages
    };

    // Apply pagination
    let messages = paginate(messages, args.limit, args.page);

    // Output
    if global.output.plain {
        print_read_plain(&messages);
        return Ok(());
    }
    print_output(&messages, &global.output)
}

// =============================================================================
// Follow: streaming mode
// =============================================================================

async fn execute_follow(global: &GlobalOpts, args: &ConsoleFollowArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;

    // Enable Runtime domain
    managed.ensure_domain("Runtime").await?;

    // Subscribe to console events
    let mut console_rx = managed
        .subscribe("Runtime.consoleAPICalled")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to console events: {e}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

    let type_filter = resolve_type_filter(args.r#type.as_deref(), args.errors_only);
    let mut saw_errors = false;

    let timeout_duration = args.timeout.map(Duration::from_millis);
    let deadline = timeout_duration.map(|d| tokio::time::Instant::now() + d);

    loop {
        tokio::select! {
            event = console_rx.recv() => {
                match event {
                    Some(ev) => {
                        let raw_type = ev.params["type"].as_str().unwrap_or("log");
                        let msg_type = map_cdp_type(raw_type).to_string();

                        // Track error-level messages
                        if is_error_level(&msg_type) {
                            saw_errors = true;
                        }

                        // Apply type filter
                        if let Some(ref types) = type_filter {
                            if !types.iter().any(|t| t == &msg_type) {
                                continue;
                            }
                        }

                        let args_arr = ev.params["args"].as_array();
                        let text = args_arr
                            .map(|a| format_console_args(a))
                            .unwrap_or_default();
                        let timestamp = ev.params["timestamp"]
                            .as_f64()
                            .map_or_else(String::new, timestamp_to_iso);

                        let stream_msg = StreamMessage {
                            msg_type,
                            text,
                            timestamp,
                        };

                        let json = serde_json::to_string(&stream_msg).unwrap_or_default();
                        println!("{json}");
                        let _ = std::io::stdout().flush();
                    }
                    None => {
                        // Connection closed
                        return Err(AppError {
                            message: "CDP connection closed".to_string(),
                            code: ExitCode::ConnectionError,
                            custom_json: None,
                        });
                    }
                }
            }
            () = async {
                if let Some(d) = deadline {
                    tokio::time::sleep_until(d).await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                // Timeout expired
                break;
            }
            _ = tokio::signal::ctrl_c() => {
                // Ctrl+C
                break;
            }
        }
    }

    if saw_errors {
        Err(AppError {
            message: "Error-level console messages were seen".to_string(),
            code: ExitCode::GeneralError,
            custom_json: None,
        })
    } else {
        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ConsoleMessage serialization
    // =========================================================================

    #[test]
    fn console_message_serialization() {
        let msg = ConsoleMessage {
            id: 0,
            msg_type: "log".to_string(),
            text: "hello".to_string(),
            timestamp: "2026-02-14T12:00:00.000Z".to_string(),
            url: "https://example.com/script.js".to_string(),
            line: 42,
            column: 5,
        };
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["id"], 0);
        assert_eq!(json["type"], "log");
        assert_eq!(json["text"], "hello");
        assert_eq!(json["timestamp"], "2026-02-14T12:00:00.000Z");
        assert_eq!(json["url"], "https://example.com/script.js");
        assert_eq!(json["line"], 42);
        assert_eq!(json["column"], 5);
        // Verify "type" field, not "msg_type"
        assert!(json.get("msg_type").is_none());
    }

    // =========================================================================
    // ConsoleMessageDetail serialization
    // =========================================================================

    #[test]
    fn console_message_detail_serialization() {
        let detail = ConsoleMessageDetail {
            id: 1,
            msg_type: "error".to_string(),
            text: "fail".to_string(),
            timestamp: "2026-02-14T12:00:01.000Z".to_string(),
            url: "https://example.com/app.js".to_string(),
            line: 10,
            column: 3,
            args: vec![serde_json::json!({"type": "string", "value": "fail"})],
            stack_trace: vec![StackFrame {
                file: "https://example.com/app.js".to_string(),
                line: 10,
                column: 3,
                function_name: "handleClick".to_string(),
            }],
        };
        let json: serde_json::Value = serde_json::to_value(&detail).unwrap();
        assert_eq!(json["type"], "error");
        assert!(json.get("args").is_some());
        assert!(json.get("stackTrace").is_some());
        assert!(json.get("msg_type").is_none());
        assert!(json.get("stack_trace").is_none());
        let frames = json["stackTrace"].as_array().unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0]["functionName"], "handleClick");
        assert!(frames[0].get("function_name").is_none());
    }

    // =========================================================================
    // StackFrame serialization
    // =========================================================================

    #[test]
    fn stack_frame_serialization() {
        let frame = StackFrame {
            file: "script.js".to_string(),
            line: 1,
            column: 2,
            function_name: "main".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&frame).unwrap();
        assert_eq!(json["file"], "script.js");
        assert_eq!(json["line"], 1);
        assert_eq!(json["column"], 2);
        assert_eq!(json["functionName"], "main");
        assert!(json.get("function_name").is_none());
    }

    // =========================================================================
    // format_console_args
    // =========================================================================

    #[test]
    fn format_args_string() {
        let args = vec![serde_json::json!({"type": "string", "value": "hello"})];
        assert_eq!(format_console_args(&args), "hello");
    }

    #[test]
    fn format_args_number() {
        let args = vec![serde_json::json!({"type": "number", "value": 42, "description": "42"})];
        assert_eq!(format_console_args(&args), "42");
    }

    #[test]
    fn format_args_object() {
        let args = vec![
            serde_json::json!({"type": "object", "className": "Object", "description": "Object"}),
        ];
        assert_eq!(format_console_args(&args), "Object");
    }

    #[test]
    fn format_args_undefined() {
        let args = vec![serde_json::json!({"type": "undefined"})];
        assert_eq!(format_console_args(&args), "undefined");
    }

    #[test]
    fn format_args_multiple() {
        let args = vec![
            serde_json::json!({"type": "string", "value": "hello"}),
            serde_json::json!({"type": "string", "value": "world"}),
        ];
        assert_eq!(format_console_args(&args), "hello world");
    }

    // =========================================================================
    // filter_by_type
    // =========================================================================

    #[test]
    fn filter_by_type_single() {
        let messages = vec![
            ConsoleMessage {
                id: 0,
                msg_type: "log".to_string(),
                text: "a".to_string(),
                timestamp: String::new(),
                url: String::new(),
                line: 0,
                column: 0,
            },
            ConsoleMessage {
                id: 1,
                msg_type: "error".to_string(),
                text: "b".to_string(),
                timestamp: String::new(),
                url: String::new(),
                line: 0,
                column: 0,
            },
        ];
        let filtered = filter_by_type(messages, &["error".to_string()]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].msg_type, "error");
    }

    #[test]
    fn filter_by_type_multiple() {
        let messages = vec![
            ConsoleMessage {
                id: 0,
                msg_type: "log".to_string(),
                text: "a".to_string(),
                timestamp: String::new(),
                url: String::new(),
                line: 0,
                column: 0,
            },
            ConsoleMessage {
                id: 1,
                msg_type: "error".to_string(),
                text: "b".to_string(),
                timestamp: String::new(),
                url: String::new(),
                line: 0,
                column: 0,
            },
            ConsoleMessage {
                id: 2,
                msg_type: "warn".to_string(),
                text: "c".to_string(),
                timestamp: String::new(),
                url: String::new(),
                line: 0,
                column: 0,
            },
        ];
        let filtered = filter_by_type(messages, &["error".to_string(), "warn".to_string()]);
        assert_eq!(filtered.len(), 2);
        assert!(
            filtered
                .iter()
                .all(|m| m.msg_type == "error" || m.msg_type == "warn")
        );
    }

    // =========================================================================
    // resolve_type_filter
    // =========================================================================

    #[test]
    fn resolve_type_filter_errors_only() {
        let result = resolve_type_filter(None, true);
        let types = result.unwrap();
        assert!(types.contains(&"error".to_string()));
        assert!(types.contains(&"assert".to_string()));
        assert_eq!(types.len(), 2);
    }

    #[test]
    fn resolve_type_filter_custom() {
        let result = resolve_type_filter(Some("log,warn"), false);
        let types = result.unwrap();
        assert!(types.contains(&"log".to_string()));
        assert!(types.contains(&"warn".to_string()));
        assert_eq!(types.len(), 2);
    }

    #[test]
    fn resolve_type_filter_none() {
        let result = resolve_type_filter(None, false);
        assert!(result.is_none());
    }

    // =========================================================================
    // paginate
    // =========================================================================

    fn make_messages(count: usize) -> Vec<ConsoleMessage> {
        (0..count)
            .map(|i| ConsoleMessage {
                id: i,
                msg_type: "log".to_string(),
                text: format!("msg {i}"),
                timestamp: String::new(),
                url: String::new(),
                line: 0,
                column: 0,
            })
            .collect()
    }

    #[test]
    fn paginate_page_0() {
        let messages = make_messages(20);
        let result = paginate(messages, 10, 0);
        assert_eq!(result.len(), 10);
        assert_eq!(result[0].id, 0);
        assert_eq!(result[9].id, 9);
    }

    #[test]
    fn paginate_page_1() {
        let messages = make_messages(20);
        let result = paginate(messages, 10, 1);
        assert_eq!(result.len(), 10);
        assert_eq!(result[0].id, 10);
        assert_eq!(result[9].id, 19);
    }

    #[test]
    fn paginate_beyond_available() {
        let messages = make_messages(5);
        let result = paginate(messages, 10, 1);
        assert!(result.is_empty());
    }

    #[test]
    fn paginate_partial_last_page() {
        let messages = make_messages(15);
        let result = paginate(messages, 10, 1);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0].id, 10);
    }

    // =========================================================================
    // extract_stack_trace
    // =========================================================================

    #[test]
    fn extract_stack_trace_basic() {
        let trace = serde_json::json!({
            "callFrames": [
                {
                    "url": "script.js",
                    "lineNumber": 10,
                    "columnNumber": 5,
                    "functionName": "main"
                }
            ]
        });
        let frames = extract_stack_trace(&trace, MAX_STACK_FRAMES);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].file, "script.js");
        assert_eq!(frames[0].line, 10);
        assert_eq!(frames[0].column, 5);
        assert_eq!(frames[0].function_name, "main");
    }

    #[test]
    fn extract_stack_trace_limits_to_max() {
        let frames_data: Vec<serde_json::Value> = (0..60)
            .map(|i| {
                serde_json::json!({
                    "url": format!("script{i}.js"),
                    "lineNumber": i,
                    "columnNumber": 0,
                    "functionName": format!("fn{i}")
                })
            })
            .collect();
        let trace = serde_json::json!({ "callFrames": frames_data });
        let frames = extract_stack_trace(&trace, MAX_STACK_FRAMES);
        assert_eq!(frames.len(), 50);
    }

    #[test]
    fn extract_stack_trace_empty() {
        let trace = serde_json::json!({});
        let frames = extract_stack_trace(&trace, MAX_STACK_FRAMES);
        assert!(frames.is_empty());
    }

    // =========================================================================
    // map_cdp_type
    // =========================================================================

    #[test]
    fn map_cdp_type_warning_to_warn() {
        assert_eq!(map_cdp_type("warning"), "warn");
    }

    #[test]
    fn map_cdp_type_passthrough() {
        assert_eq!(map_cdp_type("log"), "log");
        assert_eq!(map_cdp_type("error"), "error");
        assert_eq!(map_cdp_type("info"), "info");
        assert_eq!(map_cdp_type("debug"), "debug");
    }

    // =========================================================================
    // parse_console_event
    // =========================================================================

    #[test]
    fn parse_console_event_basic() {
        let params = serde_json::json!({
            "type": "log",
            "args": [{"type": "string", "value": "hello"}],
            "timestamp": 1_707_900_000.123_f64,
            "stackTrace": {
                "callFrames": [{
                    "url": "script.js",
                    "lineNumber": 1,
                    "columnNumber": 2,
                    "functionName": "test"
                }]
            }
        });
        let msg = parse_console_event(&params, 0).unwrap();
        assert_eq!(msg.id, 0);
        assert_eq!(msg.msg_type, "log");
        assert_eq!(msg.text, "hello");
        assert_eq!(msg.url, "script.js");
        assert_eq!(msg.line, 1);
        assert_eq!(msg.column, 2);
    }

    #[test]
    fn parse_console_event_warning_mapped() {
        let params = serde_json::json!({
            "type": "warning",
            "args": [{"type": "string", "value": "oops"}],
            "timestamp": 1_707_900_000.0,
            "stackTrace": {"callFrames": []}
        });
        let msg = parse_console_event(&params, 0).unwrap();
        assert_eq!(msg.msg_type, "warn");
    }

    // =========================================================================
    // is_error_level
    // =========================================================================

    #[test]
    fn is_error_level_checks() {
        assert!(is_error_level("error"));
        assert!(is_error_level("assert"));
        assert!(!is_error_level("log"));
        assert!(!is_error_level("warn"));
    }

    // =========================================================================
    // StreamMessage serialization
    // =========================================================================

    #[test]
    fn stream_message_serialization() {
        let msg = StreamMessage {
            msg_type: "log".to_string(),
            text: "hello".to_string(),
            timestamp: "2026-02-14T12:00:00.000Z".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "log");
        assert_eq!(json["text"], "hello");
        assert_eq!(json["timestamp"], "2026-02-14T12:00:00.000Z");
        assert!(json.get("msg_type").is_none());
    }

    // =========================================================================
    // Plain text output (no panics)
    // =========================================================================

    #[test]
    fn plain_text_read_empty() {
        print_read_plain(&[]);
    }

    #[test]
    fn plain_text_read_messages() {
        let messages = vec![
            ConsoleMessage {
                id: 0,
                msg_type: "log".to_string(),
                text: "hello".to_string(),
                timestamp: String::new(),
                url: String::new(),
                line: 0,
                column: 0,
            },
            ConsoleMessage {
                id: 1,
                msg_type: "error".to_string(),
                text: "fail".to_string(),
                timestamp: String::new(),
                url: String::new(),
                line: 0,
                column: 0,
            },
        ];
        print_read_plain(&messages);
    }

    #[test]
    fn plain_text_detail() {
        let detail = ConsoleMessageDetail {
            id: 0,
            msg_type: "warn".to_string(),
            text: "warning".to_string(),
            timestamp: "2026-02-14T12:00:00.000Z".to_string(),
            url: "script.js".to_string(),
            line: 5,
            column: 10,
            args: vec![],
            stack_trace: vec![StackFrame {
                file: "script.js".to_string(),
                line: 5,
                column: 10,
                function_name: "handleClick".to_string(),
            }],
        };
        print_detail_plain(&detail);
    }

    // =========================================================================
    // timestamp_to_iso
    // =========================================================================

    #[test]
    fn timestamp_to_iso_epoch_zero() {
        assert_eq!(timestamp_to_iso(0.0), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn timestamp_to_iso_known_value() {
        // 2024-02-14T12:00:00.000Z = 1707912000000 ms since epoch
        assert_eq!(
            timestamp_to_iso(1_707_912_000_000.0),
            "2024-02-14T12:00:00.000Z"
        );
    }

    #[test]
    fn timestamp_to_iso_with_milliseconds() {
        // 2024-02-14T12:00:00.123Z = 1707912000123 ms since epoch
        assert_eq!(
            timestamp_to_iso(1_707_912_000_123.0),
            "2024-02-14T12:00:00.123Z"
        );
    }

    #[test]
    fn timestamp_to_iso_year_2026() {
        // 2026-01-01T00:00:00.000Z = 1767225600000 ms since epoch
        assert_eq!(
            timestamp_to_iso(1_767_225_600_000.0),
            "2026-01-01T00:00:00.000Z"
        );
    }

    // =========================================================================
    // parse_console_event_detail
    // =========================================================================

    #[test]
    fn parse_console_event_detail_basic() {
        let params = serde_json::json!({
            "type": "error",
            "args": [
                {"type": "string", "value": "something failed"}
            ],
            "timestamp": 1_707_912_000_000.0_f64,
            "stackTrace": {
                "callFrames": [
                    {
                        "url": "https://example.com/app.js",
                        "lineNumber": 42,
                        "columnNumber": 5,
                        "functionName": "handleClick"
                    },
                    {
                        "url": "https://example.com/lib.js",
                        "lineNumber": 100,
                        "columnNumber": 10,
                        "functionName": ""
                    }
                ]
            }
        });
        let detail = parse_console_event_detail(&params, 3).unwrap();
        assert_eq!(detail.id, 3);
        assert_eq!(detail.msg_type, "error");
        assert_eq!(detail.text, "something failed");
        assert_eq!(detail.url, "https://example.com/app.js");
        assert_eq!(detail.line, 42);
        assert_eq!(detail.column, 5);
        assert_eq!(detail.args.len(), 1);
        assert_eq!(detail.stack_trace.len(), 2);
        assert_eq!(detail.stack_trace[0].function_name, "handleClick");
        assert_eq!(detail.stack_trace[1].function_name, "");
    }

    #[test]
    fn parse_console_event_detail_no_stack_trace() {
        let params = serde_json::json!({
            "type": "log",
            "args": [{"type": "string", "value": "hello"}],
            "timestamp": 1_707_912_000_000.0_f64,
            "stackTrace": {"callFrames": []}
        });
        let detail = parse_console_event_detail(&params, 0).unwrap();
        assert_eq!(detail.msg_type, "log");
        assert!(detail.stack_trace.is_empty());
        assert_eq!(detail.url, "");
        assert_eq!(detail.line, 0);
        assert_eq!(detail.column, 0);
    }
}
