use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use chrome_cli::cdp::{CdpClient, CdpConfig};
use chrome_cli::connection::{ManagedSession, resolve_connection, resolve_target};
use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{
    FormArgs, FormClearArgs, FormCommand, FormFillArgs, FormFillManyArgs, FormUploadArgs,
    GlobalOpts,
};
use crate::emulate::apply_emulate_state;
use crate::snapshot;

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct FillResult {
    filled: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum FillManyOutput {
    Plain(Vec<FillResult>),
    WithSnapshot {
        results: Vec<FillResult>,
        snapshot: serde_json::Value,
    },
}

#[derive(Serialize)]
struct ClearResult {
    cleared: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct UploadResult {
    uploaded: String,
    files: Vec<String>,
    size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

/// JSON input for fill-many: each entry has a uid and value.
#[derive(Deserialize)]
struct FillEntry {
    uid: String,
    value: String,
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

fn print_fill_plain(result: &FillResult) {
    println!("Filled {} = {}", result.filled, result.value);
}

fn print_fill_many_plain(results: &[FillResult]) {
    for r in results {
        println!("Filled {} = {}", r.filled, r.value);
    }
}

fn print_clear_plain(result: &ClearResult) {
    println!("Cleared {}", result.cleared);
}

fn print_upload_plain(result: &UploadResult) {
    let file_list = result.files.join(", ");
    println!(
        "Uploaded {} ({} bytes): {}",
        result.uploaded, result.size, file_list
    );
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
// Target resolution helpers
// =============================================================================

/// Check if a target string is a UID (matches pattern: 's' + digits).
fn is_uid(target: &str) -> bool {
    if !target.starts_with('s') {
        return false;
    }
    let rest = &target[1..];
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

/// Check if a target string is a CSS selector (starts with 'css:').
fn is_css_selector(target: &str) -> bool {
    target.starts_with("css:")
}

/// Resolve a target (UID or CSS selector) to a backend DOM node ID.
async fn resolve_target_to_backend_node_id(
    session: &mut ManagedSession,
    target: &str,
) -> Result<i64, AppError> {
    if is_uid(target) {
        let state = snapshot::read_snapshot_state()?.ok_or_else(AppError::no_snapshot_state)?;
        let backend_node_id = state
            .uid_map
            .get(target)
            .copied()
            .ok_or_else(|| AppError::uid_not_found(target))?;
        Ok(backend_node_id)
    } else if is_css_selector(target) {
        let selector = &target[4..];

        let doc_response = session.send_command("DOM.getDocument", None).await?;
        let root_node_id = doc_response["root"]["nodeId"]
            .as_i64()
            .ok_or_else(|| AppError::element_not_found(selector))?;

        let query_params = serde_json::json!({
            "nodeId": root_node_id,
            "selector": selector,
        });
        let query_response = session
            .send_command("DOM.querySelector", Some(query_params))
            .await?;

        let node_id = query_response["nodeId"].as_i64().unwrap_or(0);
        if node_id == 0 {
            return Err(AppError::element_not_found(selector));
        }

        let describe_params = serde_json::json!({ "nodeId": node_id });
        let describe_response = session
            .send_command("DOM.describeNode", Some(describe_params))
            .await?;

        let backend_node_id = describe_response["node"]["backendNodeId"]
            .as_i64()
            .ok_or_else(|| AppError::element_not_found(selector))?;

        Ok(backend_node_id)
    } else {
        Err(AppError::element_not_found(target))
    }
}

// =============================================================================
// Snapshot refresh helper
// =============================================================================

/// Take a fresh snapshot and write it to snapshot state.
async fn take_snapshot(
    session: &mut ManagedSession,
    url: &str,
) -> Result<serde_json::Value, AppError> {
    session.ensure_domain("Accessibility").await?;

    let response = session
        .send_command("Accessibility.getFullAXTree", None)
        .await?;

    let nodes = response["nodes"]
        .as_array()
        .ok_or_else(|| AppError::snapshot_failed("missing nodes array"))?;

    let build_result = snapshot::build_tree(nodes, false);

    let state = snapshot::SnapshotState {
        url: url.to_string(),
        timestamp: chrome_cli::session::now_iso8601(),
        uid_map: build_result.uid_map,
    };
    snapshot::write_snapshot_state(&state)?;

    let snapshot_json = serde_json::to_value(&build_result.root)
        .map_err(|e| AppError::snapshot_failed(&format!("failed to serialize snapshot: {e}")))?;

    Ok(snapshot_json)
}

// =============================================================================
// Fill JavaScript
// =============================================================================

/// JavaScript function to set a form field's value and dispatch events.
///
/// Handles input/textarea (text value), select (option matching), and checkbox/radio (checked).
const FILL_JS: &str = r"
function(value) {
    const el = this;
    const tag = el.tagName.toLowerCase();

    if (tag === 'select') {
        const options = Array.from(el.options);
        const idx = options.findIndex(o => o.value === value || o.textContent.trim() === value);
        if (idx >= 0) {
            el.selectedIndex = idx;
            el.value = options[idx].value;
        }
    } else if (el.type === 'checkbox' || el.type === 'radio') {
        el.checked = value === 'true' || value === 'checked';
    } else {
        // text, password, email, number, textarea, date, tel, url, etc.
        const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
            window.HTMLInputElement.prototype, 'value'
        )?.set || Object.getOwnPropertyDescriptor(
            window.HTMLTextAreaElement.prototype, 'value'
        )?.set;
        if (nativeInputValueSetter) {
            nativeInputValueSetter.call(el, value);
        } else {
            el.value = value;
        }
    }

    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
}
";

/// JavaScript function to clear a form field's value and dispatch events.
const CLEAR_JS: &str = r"
function() {
    const el = this;
    const tag = el.tagName.toLowerCase();

    if (el.type === 'checkbox' || el.type === 'radio') {
        el.checked = false;
    } else if (tag === 'select') {
        el.selectedIndex = 0;
    } else {
        const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
            window.HTMLInputElement.prototype, 'value'
        )?.set || Object.getOwnPropertyDescriptor(
            window.HTMLTextAreaElement.prototype, 'value'
        )?.set;
        if (nativeInputValueSetter) {
            nativeInputValueSetter.call(el, '');
        } else {
            el.value = '';
        }
    }

    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
}
";

// =============================================================================
// Core fill helper
// =============================================================================

/// Resolve a target to a Runtime object ID via DOM.resolveNode.
async fn resolve_to_object_id(
    session: &mut ManagedSession,
    target: &str,
) -> Result<String, AppError> {
    let backend_node_id = resolve_target_to_backend_node_id(session, target).await?;

    let resolve_params = serde_json::json!({ "backendNodeId": backend_node_id });
    let resolve_response = session
        .send_command("DOM.resolveNode", Some(resolve_params))
        .await
        .map_err(|e| AppError::interaction_failed("resolve_node", &e.to_string()))?;

    resolve_response["object"]["objectId"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| AppError::interaction_failed("resolve_node", "no objectId returned"))
}

/// Resolve a target to a Runtime object and call a JS function on it.
async fn fill_element(
    session: &mut ManagedSession,
    target: &str,
    value: &str,
) -> Result<(), AppError> {
    let object_id = resolve_to_object_id(session, target).await?;

    let call_params = serde_json::json!({
        "objectId": object_id,
        "functionDeclaration": FILL_JS,
        "arguments": [{ "value": value }],
    });
    session
        .send_command("Runtime.callFunctionOn", Some(call_params))
        .await
        .map_err(|e| AppError::interaction_failed("fill", &e.to_string()))?;

    Ok(())
}

/// Clear an element's value via Runtime.callFunctionOn.
async fn clear_element(session: &mut ManagedSession, target: &str) -> Result<(), AppError> {
    let object_id = resolve_to_object_id(session, target).await?;

    let call_params = serde_json::json!({
        "objectId": object_id,
        "functionDeclaration": CLEAR_JS,
        "arguments": [],
    });
    session
        .send_command("Runtime.callFunctionOn", Some(call_params))
        .await
        .map_err(|e| AppError::interaction_failed("clear", &e.to_string()))?;

    Ok(())
}

// =============================================================================
// Get current URL helper
// =============================================================================

async fn get_current_url(session: &mut ManagedSession) -> Result<String, AppError> {
    session.ensure_domain("Runtime").await?;
    let url_response = session
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": "window.location.href" })),
        )
        .await?;
    Ok(url_response["result"]["value"]
        .as_str()
        .unwrap_or("")
        .to_string())
}

// =============================================================================
// Command implementations
// =============================================================================

/// Execute the `form fill` command.
async fn execute_fill(global: &GlobalOpts, args: &FormFillArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Enable required domains
    managed.ensure_domain("DOM").await?;
    managed.ensure_domain("Runtime").await?;

    // Fill the element
    fill_element(&mut managed, &args.target, &args.value).await?;

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        let url = get_current_url(&mut managed).await?;
        Some(take_snapshot(&mut managed, &url).await?)
    } else {
        None
    };

    let result = FillResult {
        filled: args.target.clone(),
        value: args.value.clone(),
        snapshot,
    };

    if global.output.plain {
        print_fill_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

/// Execute the `form fill-many` command.
async fn execute_fill_many(global: &GlobalOpts, args: &FormFillManyArgs) -> Result<(), AppError> {
    // Parse the JSON input (inline or from file)
    let json_str = if let Some(file_path) = &args.file {
        read_json_file(file_path)?
    } else if let Some(json) = &args.input {
        json.clone()
    } else {
        return Err(AppError {
            message: "Either inline JSON or --file must be provided".to_string(),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    };

    let entries: Vec<FillEntry> = serde_json::from_str(&json_str).map_err(|e| AppError {
        message: format!("Invalid JSON: expected array of {{uid, value}} objects: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;

    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Enable required domains
    managed.ensure_domain("DOM").await?;
    managed.ensure_domain("Runtime").await?;

    // Fill each element
    let mut results = Vec::with_capacity(entries.len());
    for entry in &entries {
        fill_element(&mut managed, &entry.uid, &entry.value).await?;
        results.push(FillResult {
            filled: entry.uid.clone(),
            value: entry.value.clone(),
            snapshot: None,
        });
    }

    // Take snapshot once after all fills if requested
    if args.include_snapshot {
        let url = get_current_url(&mut managed).await?;
        let snapshot = take_snapshot(&mut managed, &url).await?;
        let output = FillManyOutput::WithSnapshot { results, snapshot };
        if global.output.plain {
            if let FillManyOutput::WithSnapshot { results, .. } = &output {
                print_fill_many_plain(results);
            }
            Ok(())
        } else {
            print_output(&output, &global.output)
        }
    } else {
        let output = FillManyOutput::Plain(results);
        if global.output.plain {
            if let FillManyOutput::Plain(results) = &output {
                print_fill_many_plain(results);
            }
            Ok(())
        } else {
            print_output(&output, &global.output)
        }
    }
}

/// Execute the `form clear` command.
async fn execute_clear(global: &GlobalOpts, args: &FormClearArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Enable required domains
    managed.ensure_domain("DOM").await?;
    managed.ensure_domain("Runtime").await?;

    // Clear the element
    clear_element(&mut managed, &args.target).await?;

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        let url = get_current_url(&mut managed).await?;
        Some(take_snapshot(&mut managed, &url).await?)
    } else {
        None
    };

    let result = ClearResult {
        cleared: args.target.clone(),
        snapshot,
    };

    if global.output.plain {
        print_clear_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

// =============================================================================
// File upload constants
// =============================================================================

/// JavaScript function to check if an element is a file input.
const IS_FILE_INPUT_JS: &str = r"
function() {
    return this.tagName === 'INPUT' && this.type === 'file';
}
";

/// JavaScript function to dispatch a change event on a file input.
const DISPATCH_CHANGE_JS: &str = r"
function() {
    this.dispatchEvent(new Event('change', { bubbles: true }));
}
";

/// Size threshold in bytes above which a warning is emitted (100 MB).
const LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024;

// =============================================================================
// File upload implementation
// =============================================================================

/// Execute the `form upload` command.
async fn execute_upload(global: &GlobalOpts, args: &FormUploadArgs) -> Result<(), AppError> {
    // --- Validate files before connecting to Chrome ---
    let mut total_size: u64 = 0;
    let mut resolved_paths: Vec<String> = Vec::with_capacity(args.files.len());

    for path in &args.files {
        if !path.exists() {
            return Err(AppError::file_not_found(&path.display().to_string()));
        }
        if !path.is_file() {
            return Err(AppError::file_not_found(&path.display().to_string()));
        }
        let metadata = std::fs::metadata(path)
            .map_err(|_| AppError::file_not_readable(&path.display().to_string()))?;
        let file_size = metadata.len();
        if file_size > LARGE_FILE_THRESHOLD {
            eprintln!(
                "warning: file is large ({} bytes): {}",
                file_size,
                path.display()
            );
        }
        total_size += file_size;

        // Canonicalize the path for CDP
        let canonical = path
            .canonicalize()
            .map_err(|_| AppError::file_not_readable(&path.display().to_string()))?;
        resolved_paths.push(canonical.to_string_lossy().to_string());
    }

    // --- Setup CDP session ---
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("DOM").await?;
    managed.ensure_domain("Runtime").await?;

    // --- Resolve target to backend node ID and object ID ---
    let backend_node_id = resolve_target_to_backend_node_id(&mut managed, &args.target).await?;
    let object_id = resolve_to_object_id(&mut managed, &args.target).await?;

    // --- Validate element is a file input ---
    let check_params = serde_json::json!({
        "objectId": object_id,
        "functionDeclaration": IS_FILE_INPUT_JS,
        "returnByValue": true,
    });
    let check_response = managed
        .send_command("Runtime.callFunctionOn", Some(check_params))
        .await
        .map_err(|e| AppError::interaction_failed("validate_file_input", &e.to_string()))?;

    let is_file_input = check_response["result"]["value"].as_bool().unwrap_or(false);
    if !is_file_input {
        return Err(AppError::not_file_input(&args.target));
    }

    // --- Call DOM.setFileInputFiles ---
    let set_files_params = serde_json::json!({
        "files": resolved_paths,
        "backendNodeId": backend_node_id,
    });
    managed
        .send_command("DOM.setFileInputFiles", Some(set_files_params))
        .await
        .map_err(|e| AppError::interaction_failed("setFileInputFiles", &e.to_string()))?;

    // --- Dispatch change event ---
    let change_params = serde_json::json!({
        "objectId": object_id,
        "functionDeclaration": DISPATCH_CHANGE_JS,
        "arguments": [],
    });
    managed
        .send_command("Runtime.callFunctionOn", Some(change_params))
        .await
        .map_err(|e| AppError::interaction_failed("dispatch_change", &e.to_string()))?;

    // --- Optionally take snapshot ---
    let snapshot = if args.include_snapshot {
        let url = get_current_url(&mut managed).await?;
        Some(take_snapshot(&mut managed, &url).await?)
    } else {
        None
    };

    // --- Build and print result ---
    let result = UploadResult {
        uploaded: args.target.clone(),
        files: resolved_paths,
        size: total_size,
        snapshot,
    };

    if global.output.plain {
        print_upload_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

// =============================================================================
// File reading helper
// =============================================================================

fn read_json_file(path: &Path) -> Result<String, AppError> {
    std::fs::read_to_string(path).map_err(|e| AppError {
        message: format!("File not found: {}: {e}", path.display()),
        code: ExitCode::GeneralError,
        custom_json: None,
    })
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Execute the `form` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_form(global: &GlobalOpts, args: &FormArgs) -> Result<(), AppError> {
    match &args.command {
        FormCommand::Fill(fill_args) => execute_fill(global, fill_args).await,
        FormCommand::FillMany(fill_many_args) => execute_fill_many(global, fill_many_args).await,
        FormCommand::Clear(clear_args) => execute_clear(global, clear_args).await,
        FormCommand::Upload(upload_args) => execute_upload(global, upload_args).await,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Target validation tests
    // =========================================================================

    #[test]
    fn is_uid_valid() {
        assert!(is_uid("s1"));
        assert!(is_uid("s42"));
        assert!(is_uid("s999"));
    }

    #[test]
    fn is_uid_invalid() {
        assert!(!is_uid("s"));
        assert!(!is_uid("s0a"));
        assert!(!is_uid("css:button"));
        assert!(!is_uid("button"));
        assert!(!is_uid("1s"));
    }

    #[test]
    fn is_css_selector_valid() {
        assert!(is_css_selector("css:#button"));
        assert!(is_css_selector("css:.class"));
        assert!(is_css_selector("css:div > p"));
    }

    #[test]
    fn is_css_selector_invalid() {
        assert!(!is_css_selector("#button"));
        assert!(!is_css_selector("s1"));
        assert!(!is_css_selector("button"));
    }

    // =========================================================================
    // FillResult serialization tests
    // =========================================================================

    #[test]
    fn fill_result_serialization() {
        let result = FillResult {
            filled: "s1".to_string(),
            value: "John".to_string(),
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["filled"], "s1");
        assert_eq!(json["value"], "John");
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn fill_result_serialization_with_snapshot() {
        let result = FillResult {
            filled: "s1".to_string(),
            value: "John".to_string(),
            snapshot: Some(serde_json::json!({"role": "document"})),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["filled"], "s1");
        assert_eq!(json["value"], "John");
        assert!(json.get("snapshot").is_some());
    }

    #[test]
    fn fill_result_css_selector_target() {
        let result = FillResult {
            filled: "css:#email".to_string(),
            value: "user@example.com".to_string(),
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["filled"], "css:#email");
        assert_eq!(json["value"], "user@example.com");
    }

    // =========================================================================
    // FillManyOutput serialization tests
    // =========================================================================

    #[test]
    fn fill_many_output_plain_serialization() {
        let output = FillManyOutput::Plain(vec![
            FillResult {
                filled: "s1".to_string(),
                value: "John".to_string(),
                snapshot: None,
            },
            FillResult {
                filled: "s2".to_string(),
                value: "Doe".to_string(),
                snapshot: None,
            },
        ]);
        let json: serde_json::Value = serde_json::to_value(&output).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["filled"], "s1");
        assert_eq!(arr[0]["value"], "John");
        assert_eq!(arr[1]["filled"], "s2");
        assert_eq!(arr[1]["value"], "Doe");
    }

    #[test]
    fn fill_many_output_with_snapshot_serialization() {
        let output = FillManyOutput::WithSnapshot {
            results: vec![FillResult {
                filled: "s1".to_string(),
                value: "John".to_string(),
                snapshot: None,
            }],
            snapshot: serde_json::json!({"role": "document"}),
        };
        let json: serde_json::Value = serde_json::to_value(&output).unwrap();
        assert!(json.get("results").is_some());
        assert!(json.get("snapshot").is_some());
        let results = json["results"].as_array().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["filled"], "s1");
    }

    // =========================================================================
    // ClearResult serialization tests
    // =========================================================================

    #[test]
    fn clear_result_serialization() {
        let result = ClearResult {
            cleared: "s1".to_string(),
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["cleared"], "s1");
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn clear_result_serialization_with_snapshot() {
        let result = ClearResult {
            cleared: "s1".to_string(),
            snapshot: Some(serde_json::json!({"role": "document"})),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["cleared"], "s1");
        assert!(json.get("snapshot").is_some());
    }

    // =========================================================================
    // FillEntry deserialization tests
    // =========================================================================

    #[test]
    fn fill_entry_deserialization() {
        let json = r#"[{"uid":"s1","value":"John"},{"uid":"s2","value":"Doe"}]"#;
        let entries: Vec<FillEntry> = serde_json::from_str(json).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].uid, "s1");
        assert_eq!(entries[0].value, "John");
        assert_eq!(entries[1].uid, "s2");
        assert_eq!(entries[1].value, "Doe");
    }

    #[test]
    fn fill_entry_invalid_json() {
        let json = r#"[{"uid":"s1"}]"#; // missing "value"
        let result: Result<Vec<FillEntry>, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn fill_entry_not_array() {
        let json = r#"{"uid":"s1","value":"John"}"#;
        let result: Result<Vec<FillEntry>, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    // =========================================================================
    // Plain text output tests
    // =========================================================================

    #[test]
    fn fill_plain_output_format() {
        // Just verify it doesn't panic
        let result = FillResult {
            filled: "s1".to_string(),
            value: "test".to_string(),
            snapshot: None,
        };
        // Would print "Filled s1 = test"
        print_fill_plain(&result);
    }

    #[test]
    fn clear_plain_output_format() {
        let result = ClearResult {
            cleared: "s1".to_string(),
            snapshot: None,
        };
        // Would print "Cleared s1"
        print_clear_plain(&result);
    }

    // =========================================================================
    // UploadResult serialization tests
    // =========================================================================

    #[test]
    fn upload_result_serialization() {
        let result = UploadResult {
            uploaded: "s5".to_string(),
            files: vec!["/tmp/photo.jpg".to_string()],
            size: 24576,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["uploaded"], "s5");
        assert_eq!(json["files"].as_array().unwrap().len(), 1);
        assert_eq!(json["files"][0], "/tmp/photo.jpg");
        assert_eq!(json["size"], 24576);
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn upload_result_serialization_with_snapshot() {
        let result = UploadResult {
            uploaded: "s5".to_string(),
            files: vec!["/tmp/photo.jpg".to_string()],
            size: 24576,
            snapshot: Some(serde_json::json!({"role": "document"})),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["uploaded"], "s5");
        assert_eq!(json["size"], 24576);
        assert!(json.get("snapshot").is_some());
    }

    #[test]
    fn upload_result_multiple_files() {
        let result = UploadResult {
            uploaded: "s3".to_string(),
            files: vec!["/tmp/doc1.pdf".to_string(), "/tmp/doc2.pdf".to_string()],
            size: 102_400,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["uploaded"], "s3");
        assert_eq!(json["files"].as_array().unwrap().len(), 2);
        assert_eq!(json["files"][0], "/tmp/doc1.pdf");
        assert_eq!(json["files"][1], "/tmp/doc2.pdf");
        assert_eq!(json["size"], 102_400);
    }

    #[test]
    fn upload_result_css_selector_target() {
        let result = UploadResult {
            uploaded: "css:#file-upload".to_string(),
            files: vec!["/tmp/document.pdf".to_string()],
            size: 51200,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["uploaded"], "css:#file-upload");
    }

    #[test]
    fn upload_plain_output_format() {
        let result = UploadResult {
            uploaded: "s5".to_string(),
            files: vec!["/tmp/photo.jpg".to_string()],
            size: 24576,
            snapshot: None,
        };
        // Would print "Uploaded s5 (24576 bytes): /tmp/photo.jpg"
        print_upload_plain(&result);
    }
}
