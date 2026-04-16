use std::time::Duration;

use serde::Serialize;

use agentchrome::cdp::{CdpClient, CdpConfig};
use agentchrome::connection::{ManagedSession, resolve_connection, resolve_target};
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
// CDP config helper
// =============================================================================

/// Build a `CdpConfig` from the global CLI options.
pub fn cdp_config(global: &GlobalOpts) -> CdpConfig {
    let mut config = CdpConfig::default();
    if let Some(timeout_ms) = global.timeout {
        config.command_timeout = Duration::from_millis(timeout_ms);
    }
    config
}

// =============================================================================
// Session setup
// =============================================================================

/// Connect to Chrome, attach to a target, and apply emulation state.
pub async fn setup_session(global: &GlobalOpts) -> Result<(CdpClient, ManagedSession), AppError> {
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

    Ok((client, managed))
}

/// Like `setup_session`, but without applying emulation state.
///
/// Used by emulate commands that set (rather than apply) emulation state.
pub async fn setup_session_bare(
    global: &GlobalOpts,
) -> Result<(CdpClient, ManagedSession), AppError> {
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
    let managed = ManagedSession::new(session);

    Ok((client, managed))
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
}
