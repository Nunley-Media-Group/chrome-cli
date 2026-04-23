use std::time::Duration;

use serde::Serialize;

use agentchrome::cdp::{CdpClient, KeepAliveConfig};
use agentchrome::connection::{ManagedSession, ReconnectPolicy, resolve_target};
use agentchrome::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, OutputFormat};
use crate::emulate::apply_emulate_state;
use crate::snapshot;

// =============================================================================
// Constants
// =============================================================================

/// Default large-response threshold in bytes (16 KB).
pub const DEFAULT_THRESHOLD: usize = 16_384;

// =============================================================================
// Temp file output struct
// =============================================================================

#[derive(Serialize)]
pub struct TempFileOutput {
    pub output_file: String,
    pub size_bytes: u64,
    pub command: String,
    pub summary: serde_json::Value,
}

// =============================================================================
// Helpers
// =============================================================================

/// Write content to a UUID-named temp file and return the file path.
pub fn write_temp_file(content: &str, extension: &str) -> Result<String, AppError> {
    let id = uuid::Uuid::new_v4();
    let filename = format!("agentchrome-{id}.{extension}");
    let path = std::env::temp_dir().join(filename);
    std::fs::write(&path, content).map_err(|e| AppError {
        message: format!("failed to write temp file {}: {e}", path.display()),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;
    Ok(path.to_string_lossy().into_owned())
}

#[allow(clippy::needless_pass_by_value)]
fn serialization_error(e: serde_json::Error) -> AppError {
    AppError {
        message: format!("serialization error: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    }
}

/// Format a byte count as a human-readable string.
#[cfg(test)]
#[allow(clippy::cast_precision_loss)]
pub fn format_human_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{} KB", bytes / 1024)
    } else {
        format!("{bytes} bytes")
    }
}

// =============================================================================
// Shared output helpers
// =============================================================================

/// Serialize a value to JSON and print to stdout.
///
/// Uses compact or pretty formatting based on `OutputFormat` flags.
pub fn print_output(value: &impl Serialize, output: &OutputFormat) -> Result<(), AppError> {
    let json = if output.pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };
    let json = json.map_err(serialization_error)?;
    println!("{json}");
    Ok(())
}

// =============================================================================
// CDP config helpers
// =============================================================================

/// Resolve the effective keep-alive config from `GlobalOpts`.
///
/// Precedence: `--no-keepalive` > `--keepalive-interval 0` > `--keepalive-interval N`
/// > `AGENTCHROME_KEEPALIVE_INTERVAL` (handled by clap `env`) > built-in default.
pub fn build_keepalive(global: &GlobalOpts) -> KeepAliveConfig {
    if global.no_keepalive {
        return KeepAliveConfig {
            interval: None,
            ..KeepAliveConfig::default()
        };
    }
    match global.keepalive_interval {
        Some(0) => KeepAliveConfig {
            interval: None,
            ..KeepAliveConfig::default()
        },
        Some(ms) => KeepAliveConfig {
            interval: Some(Duration::from_millis(ms)),
            ..KeepAliveConfig::default()
        },
        None => KeepAliveConfig::default(),
    }
}

/// Open a CDP connection for the current command, applying invocation-level
/// auto-reconnect and keep-alive defaults derived from `global`.
///
/// This collapses the `policy + keepalive + connect_for_command` boilerplate
/// that every command would otherwise repeat.
///
/// # Errors
///
/// Propagates `AppError` from the resolve/connect pipeline.
pub async fn connect_from_global(
    global: &GlobalOpts,
) -> Result<agentchrome::connection::CommandConnection, AppError> {
    connect_from_global_with_timeout(global, global.timeout).await
}

/// Like [`connect_from_global`], but lets a subcommand override the command
/// timeout (e.g. `js exec --timeout`).
///
/// # Errors
///
/// Propagates `AppError` from the resolve/connect pipeline.
pub async fn connect_from_global_with_timeout(
    global: &GlobalOpts,
    timeout_ms: Option<u64>,
) -> Result<agentchrome::connection::CommandConnection, AppError> {
    let policy = ReconnectPolicy::default();
    let keepalive = build_keepalive(global);
    agentchrome::connection::connect_for_command(
        &global.host,
        global.port,
        global.ws_url.as_deref(),
        timeout_ms,
        keepalive,
        &policy,
    )
    .await
}

// =============================================================================
// Session setup
// =============================================================================

/// Connect to Chrome, attach to a target, and apply emulation state.
pub async fn setup_session(global: &GlobalOpts) -> Result<(CdpClient, ManagedSession), AppError> {
    let conn = connect_from_global(global).await?;
    let target = resolve_target(
        &conn.resolved.host,
        conn.resolved.port,
        global.tab.as_deref(),
        global.page_id.as_deref(),
    )
    .await?;

    let session = conn.client.create_session(&target.id).await?;
    let mut managed = ManagedSession::new(session);
    apply_emulate_state(&mut managed).await?;

    Ok((conn.client, managed))
}

/// Like `setup_session`, but without applying emulation state.
///
/// Used by emulate commands that set (rather than apply) emulation state.
pub async fn setup_session_bare(
    global: &GlobalOpts,
) -> Result<(CdpClient, ManagedSession), AppError> {
    let conn = connect_from_global(global).await?;
    let target = resolve_target(
        &conn.resolved.host,
        conn.resolved.port,
        global.tab.as_deref(),
        global.page_id.as_deref(),
    )
    .await?;

    let session = conn.client.create_session(&target.id).await?;
    let managed = ManagedSession::new(session);

    Ok((conn.client, managed))
}

/// Like `setup_session`, but also installs dialog interceptors.
pub async fn setup_session_with_interceptors(
    global: &GlobalOpts,
) -> Result<(CdpClient, ManagedSession), AppError> {
    let (client, managed) = setup_session(global).await?;
    managed.install_dialog_interceptors().await;
    Ok((client, managed))
}

// =============================================================================
// Frame resolution
// =============================================================================

/// Resolve the optional `--frame` argument and return a `FrameContext`.
///
/// `uid` is the target element identifier; it is required when `frame` is
/// `"auto"` so that `resolve_frame_auto` can locate the correct frame.
pub async fn resolve_optional_frame(
    client: &CdpClient,
    managed: &mut ManagedSession,
    frame: Option<&str>,
    uid: Option<&str>,
) -> Result<Option<agentchrome::frame::FrameContext>, AppError> {
    // Aggregate-snapshot UID auto-routing (T029). When a UID-based command is
    // invoked against a snapshot produced by `page snapshot --include-iframes`,
    // UIDs can point into any frame. Consult the aggregate UID ranges to pick
    // the right frame automatically, and verify the recorded frame_id is still
    // attached to the live frame tree.
    if let Some(uid_str) = uid {
        if snapshot::is_uid(uid_str) {
            if let Some(state) = snapshot::read_snapshot_state().ok().flatten() {
                if let Some(uid_frame_idx) = snapshot::aggregate_frame_for_uid(&state, uid_str) {
                    let recorded_frame_id =
                        snapshot::aggregate_frame_id(&state, uid_frame_idx).map(str::to_string);

                    // Explicit --frame that names a specific index: warn on mismatch.
                    if let Some(frame_str) = frame {
                        if let Ok(agentchrome::frame::FrameArg::Index(n)) =
                            agentchrome::frame::parse_frame_arg(frame_str)
                        {
                            if n != uid_frame_idx {
                                eprintln!(
                                    "warning: UID {uid_str} was recorded in frame {uid_frame_idx} but --frame {n} was supplied; proceeding with explicit frame"
                                );
                            }
                        }
                        // Fall through to the standard resolution path below.
                    } else {
                        // No --frame given: route to the UID's originating frame.
                        verify_frame_still_attached(
                            managed,
                            uid_frame_idx,
                            recorded_frame_id.as_deref(),
                        )
                        .await?;
                        if uid_frame_idx == 0 {
                            return Ok(None);
                        }
                        let arg = agentchrome::frame::FrameArg::Index(uid_frame_idx);
                        let ctx = agentchrome::frame::resolve_frame(client, managed, &arg).await?;
                        return Ok(Some(ctx));
                    }
                }
            }
        }
    }

    if let Some(frame_str) = frame {
        let arg = agentchrome::frame::parse_frame_arg(frame_str)?;
        if matches!(arg, agentchrome::frame::FrameArg::Auto) {
            let target_uid = uid.unwrap_or_default();
            let state = snapshot::read_snapshot_state().ok().flatten();
            let hint = state
                .as_ref()
                .and_then(|s| s.frame_index.map(|idx| (idx, &s.uid_map)));
            let (ctx, _frame_idx) =
                agentchrome::frame::resolve_frame_auto(client, managed, target_uid, hint).await?;
            Ok(Some(ctx))
        } else {
            let ctx = agentchrome::frame::resolve_frame(client, managed, &arg).await?;
            Ok(Some(ctx))
        }
    } else {
        Ok(None)
    }
}

/// Confirm that a frame previously recorded in an aggregate snapshot is still
/// present at the same index with the same CDP frame id.
async fn verify_frame_still_attached(
    managed: &mut ManagedSession,
    frame_index: u32,
    recorded_frame_id: Option<&str>,
) -> Result<(), AppError> {
    let Some(expected_id) = recorded_frame_id else {
        return Ok(());
    };
    let frames = agentchrome::frame::list_frames(managed).await?;
    let Some(current) = frames.iter().find(|f| f.index == frame_index) else {
        return Err(AppError::frame_detached());
    };
    if current.id != expected_id {
        return Err(AppError::frame_detached());
    }
    Ok(())
}

// =============================================================================
// Emit functions
// =============================================================================

/// Emit plain text through the large-response gate.
///
/// If the text exceeds the threshold, it is written to a temp file and
/// the file path is printed on stdout instead.
pub fn emit_plain(text: &str, output: &OutputFormat) -> Result<(), AppError> {
    let threshold = output.large_response_threshold.unwrap_or(DEFAULT_THRESHOLD);

    if text.len() <= threshold {
        print!("{text}");
        return Ok(());
    }

    let path = write_temp_file(text, "txt")?;
    println!("{path}");
    Ok(())
}

/// Emit a serializable value through the large-response gate.
///
/// If the serialized JSON exceeds the threshold, the full output is written
/// to a UUID-named temp file and a `TempFileOutput` object is printed instead.
pub fn emit<T, F>(
    value: &T,
    output: &OutputFormat,
    command_name: &str,
    summary_fn: F,
) -> Result<(), AppError>
where
    T: Serialize,
    F: FnOnce(&T) -> serde_json::Value,
{
    // 1. Serialize to JSON string (once)
    let json_string = if output.pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .map_err(serialization_error)?;

    // 2. Determine effective threshold
    let threshold = output.large_response_threshold.unwrap_or(DEFAULT_THRESHOLD);

    // 3. If under threshold, print and return
    if json_string.len() <= threshold {
        println!("{json_string}");
        return Ok(());
    }

    // 4. Write to temp file
    let path = write_temp_file(&json_string, "json")?;

    // 5. Build and print TempFileOutput
    let summary = summary_fn(value);
    #[allow(clippy::cast_possible_truncation)]
    let size_bytes = json_string.len() as u64;

    let temp_output = TempFileOutput {
        output_file: path,
        size_bytes,
        command: command_name.to_string(),
        summary,
    };

    let output_json = serde_json::to_string(&temp_output).map_err(serialization_error)?;
    println!("{output_json}");
    Ok(())
}

/// Emit a compound result that carries a small interaction-confirmation payload
/// plus a potentially large `snapshot` field.
///
/// If the serialized form of the whole value fits under the threshold, the full
/// inline JSON is printed (identical to `emit`).
///
/// Otherwise, the `snapshot_field` key is extracted from the serialized value,
/// written to a UUID-named temp file, and replaced with a `TempFileOutput`
/// whose `summary` is produced by `snapshot_summary_fn`.  The
/// interaction-confirmation fields (everything other than `snapshot_field`)
/// remain inline.
///
/// If the `snapshot_field` key is absent after serialization, this falls back
/// to `emit` transparently (never panics).
///
/// # Errors
///
/// Propagates `AppError` from serialization and temp-file writes.
pub fn emit_with_snapshot<T, F>(
    value: &T,
    output: &OutputFormat,
    command_name: &str,
    snapshot_field: &'static str,
    snapshot_summary_fn: F,
) -> Result<(), AppError>
where
    T: Serialize,
    F: FnOnce(&serde_json::Value) -> serde_json::Value,
{
    // 1. Serialize `value` to a serde_json::Value (no string yet).
    let mut outer: serde_json::Value = serde_json::to_value(value).map_err(serialization_error)?;

    // 2. Compute total size by serializing to a string.
    let full_json = if output.pretty {
        serde_json::to_string_pretty(&outer)
    } else {
        serde_json::to_string(&outer)
    }
    .map_err(serialization_error)?;

    let threshold = output.large_response_threshold.unwrap_or(DEFAULT_THRESHOLD);

    // 3. Under threshold → print inline (identical to emit's small-path).
    if full_json.len() <= threshold {
        println!("{full_json}");
        return Ok(());
    }

    // 4. Extract the snapshot field.  If absent, fall back to emit.
    let Some(map) = outer.as_object_mut() else {
        // Value is not an object; fall back to regular emit.
        println!("{full_json}");
        return Ok(());
    };

    let Some(snapshot_value) = map.remove(snapshot_field) else {
        // Snapshot field absent — fall back to regular emit.
        println!("{full_json}");
        return Ok(());
    };

    // 5. Serialize the snapshot alone and write it to a temp file.
    let snapshot_json = serde_json::to_string(&snapshot_value).map_err(serialization_error)?;
    let path = write_temp_file(&snapshot_json, "json")?;

    // 6. Build a TempFileOutput for the snapshot.
    let summary = snapshot_summary_fn(&snapshot_value);
    #[allow(clippy::cast_possible_truncation)]
    let size_bytes = snapshot_json.len() as u64;

    let temp_output = TempFileOutput {
        output_file: path,
        size_bytes,
        command: command_name.to_string(),
        summary,
    };

    // 7. Replace the snapshot field in the outer object with the TempFileOutput.
    let temp_value = serde_json::to_value(&temp_output).map_err(serialization_error)?;
    map.insert(snapshot_field.to_string(), temp_value);

    // 8. Print the modified outer object.
    let result_json = if output.pretty {
        serde_json::to_string_pretty(&outer)
    } else {
        serde_json::to_string(&outer)
    }
    .map_err(serialization_error)?;
    println!("{result_json}");
    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_human_size_bytes() {
        assert_eq!(format_human_size(500), "500 bytes");
        assert_eq!(format_human_size(0), "0 bytes");
        assert_eq!(format_human_size(1023), "1023 bytes");
    }

    #[test]
    fn format_human_size_kb() {
        assert_eq!(format_human_size(1024), "1 KB");
        assert_eq!(format_human_size(16_384), "16 KB");
        assert_eq!(format_human_size(1_048_575), "1023 KB");
    }

    #[test]
    fn format_human_size_mb() {
        assert_eq!(format_human_size(1_048_576), "1.0 MB");
        assert_eq!(format_human_size(5_242_880), "5.0 MB");
    }

    #[test]
    fn temp_file_output_serialization() {
        let output = TempFileOutput {
            output_file: "/tmp/agentchrome-abc.json".to_string(),
            size_bytes: 32_768,
            command: "page snapshot".to_string(),
            summary: serde_json::json!({"total_nodes": 5000}),
        };
        let json: serde_json::Value = serde_json::to_value(&output).unwrap();
        assert_eq!(json["output_file"], "/tmp/agentchrome-abc.json");
        assert_eq!(json["size_bytes"], 32_768);
        assert_eq!(json["command"], "page snapshot");
        assert_eq!(json["summary"]["total_nodes"], 5000);
    }

    #[test]
    fn temp_file_output_has_exactly_four_keys() {
        let output = TempFileOutput {
            output_file: "/tmp/test.json".to_string(),
            size_bytes: 100,
            command: "test".to_string(),
            summary: serde_json::json!({}),
        };
        let json: serde_json::Value = serde_json::to_value(&output).unwrap();
        let keys = json.as_object().unwrap();
        assert_eq!(keys.len(), 4);
        assert!(keys.contains_key("output_file"));
        assert!(keys.contains_key("size_bytes"));
        assert!(keys.contains_key("command"));
        assert!(keys.contains_key("summary"));
    }

    #[test]
    fn write_temp_file_creates_readable_file() {
        let content = "hello temp file";
        let path = write_temp_file(content, "txt").unwrap();
        let read_back = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read_back, content);
        assert!(path.contains("agentchrome-"));
        assert!(
            std::path::Path::new(&path)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_temp_file_json_extension() {
        let content = r#"{"key":"value"}"#;
        let path = write_temp_file(content, "json").unwrap();
        assert!(
            std::path::Path::new(&path)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_temp_file_uuid_uniqueness() {
        let path1 = write_temp_file("a", "txt").unwrap();
        let path2 = write_temp_file("b", "txt").unwrap();
        assert_ne!(path1, path2);
        let _ = std::fs::remove_file(&path1);
        let _ = std::fs::remove_file(&path2);
    }

    #[test]
    fn emit_below_threshold_prints_json() {
        let value = serde_json::json!({"key": "value"});
        let output = OutputFormat {
            json: true,
            pretty: false,
            plain: false,
            large_response_threshold: Some(1_000_000),
        };
        let result = emit(&value, &output, "test", |_| serde_json::json!({}));
        assert!(result.is_ok());
    }

    #[test]
    fn emit_above_threshold_creates_temp_file() {
        // Create a large value that exceeds the threshold
        let large_string: String = "x".repeat(1000);
        let value = serde_json::json!({"data": large_string});
        let output = OutputFormat {
            json: true,
            pretty: false,
            plain: false,
            large_response_threshold: Some(10), // Very low threshold
        };
        let result = emit(
            &value,
            &output,
            "test cmd",
            |_| serde_json::json!({"test": true}),
        );
        assert!(result.is_ok());
        // The function prints to stdout; we verify no error occurred.
        // Full integration behavior is tested via BDD.
    }

    #[test]
    fn emit_plain_below_threshold() {
        let output = OutputFormat {
            json: false,
            pretty: false,
            plain: true,
            large_response_threshold: Some(1_000_000),
        };
        let result = emit_plain("short text", &output);
        assert!(result.is_ok());
    }

    #[test]
    fn emit_plain_above_threshold() {
        let output = OutputFormat {
            json: false,
            pretty: false,
            plain: true,
            large_response_threshold: Some(5), // Very low
        };
        let result = emit_plain("this text is longer than 5 bytes", &output);
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------------
    // emit_with_snapshot tests
    // -------------------------------------------------------------------------

    /// Helper: `OutputFormat` with a very large threshold so nothing is offloaded.
    fn unlimited_output() -> OutputFormat {
        OutputFormat {
            json: true,
            pretty: false,
            plain: false,
            large_response_threshold: Some(10_000_000),
        }
    }

    /// Helper: `OutputFormat` with threshold = 1 so everything is offloaded.
    fn tiny_threshold_output() -> OutputFormat {
        OutputFormat {
            json: true,
            pretty: false,
            plain: false,
            large_response_threshold: Some(1),
        }
    }

    #[test]
    fn emit_with_snapshot_below_threshold_is_inline() {
        let value = serde_json::json!({
            "success": true,
            "uid": "s1",
            "snapshot": {"total_nodes": 10, "nodes": []}
        });
        let result = emit_with_snapshot(
            &value,
            &unlimited_output(),
            "interact click",
            "snapshot",
            |_| serde_json::json!({"total_nodes": 10}),
        );
        // Should succeed (prints inline JSON to stdout — no temp file).
        assert!(result.is_ok());
    }

    #[test]
    fn emit_with_snapshot_above_threshold_offloads_snapshot_only() {
        let large_snapshot: Vec<serde_json::Value> = (0..500)
            .map(|i| serde_json::json!({"id": i, "role": "generic", "name": format!("node-{i}")}))
            .collect();
        let value = serde_json::json!({
            "success": true,
            "uid": "s12",
            "navigation": {"url": "https://example.com", "committed": true},
            "snapshot": {"total_nodes": 500, "nodes": large_snapshot}
        });
        let result = emit_with_snapshot(
            &value,
            &tiny_threshold_output(),
            "interact click",
            "snapshot",
            |v| {
                let count = v["total_nodes"].as_u64().unwrap_or(0);
                serde_json::json!({"total_nodes": count, "top_roles": ["generic"]})
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn emit_with_snapshot_missing_field_falls_back_to_emit() {
        // Value has no "snapshot" key — should fall back without panicking.
        let value = serde_json::json!({"success": true, "uid": "s5"});
        let result = emit_with_snapshot(
            &value,
            &tiny_threshold_output(),
            "interact click",
            "snapshot",
            |_| serde_json::json!({}),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn emit_with_snapshot_non_object_falls_back() {
        // A plain array — not an object.
        let value = serde_json::json!([1, 2, 3]);
        let result =
            emit_with_snapshot(&value, &tiny_threshold_output(), "test", "snapshot", |_| {
                serde_json::json!({})
            });
        assert!(result.is_ok());
    }
}
