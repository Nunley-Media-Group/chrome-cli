use serde::Serialize;

use agentchrome::error::{AppError, ExitCode};

use crate::cli::OutputFormat;

// =============================================================================
// Constants
// =============================================================================

/// Default large-response threshold in bytes (16 KB).
pub const DEFAULT_THRESHOLD: usize = 16_384;

// =============================================================================
// Guidance struct
// =============================================================================

#[derive(Serialize)]
pub struct LargeResponseGuidance {
    pub large_response: bool,
    pub size_bytes: u64,
    pub command: String,
    pub summary: serde_json::Value,
    pub guidance: String,
}

// =============================================================================
// Emit functions
// =============================================================================

/// Emit a serializable value through the large-response gate.
///
/// If the serialized JSON exceeds the threshold (and `--full-response` is not set),
/// a structured guidance object is printed instead.
///
/// Plain mode is handled by callers before reaching this function.
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

    // 2. If full_response, print and return
    if output.full_response {
        println!("{json_string}");
        return Ok(());
    }

    // 3. Determine effective threshold
    let threshold = output.large_response_threshold.unwrap_or(DEFAULT_THRESHOLD);

    // 4. If under threshold, print and return
    if json_string.len() <= threshold {
        println!("{json_string}");
        return Ok(());
    }

    // 5. Build guidance object
    let summary = summary_fn(value);
    #[allow(clippy::cast_possible_truncation)]
    let size_bytes = json_string.len() as u64;
    let guidance_text = build_guidance_text(command_name, size_bytes, &summary, threshold);

    let guidance = LargeResponseGuidance {
        large_response: true,
        size_bytes,
        command: command_name.to_string(),
        summary,
        guidance: guidance_text,
    };

    // 6. Serialize and print guidance (always compact JSON, even with --pretty)
    let guidance_json = serde_json::to_string(&guidance).map_err(serialization_error)?;
    println!("{guidance_json}");
    Ok(())
}

/// Emit a serializable value that was already filtered by `--search`.
///
/// Search results always bypass the large-response gate.
pub fn emit_searched<T: Serialize>(value: &T, output: &OutputFormat) -> Result<(), AppError> {
    let json_string = if output.pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .map_err(serialization_error)?;
    println!("{json_string}");
    Ok(())
}

// =============================================================================
// Helpers
// =============================================================================

#[allow(clippy::needless_pass_by_value)]
fn serialization_error(e: serde_json::Error) -> AppError {
    AppError {
        message: format!("serialization error: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    }
}

/// Format a byte count as a human-readable string.
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

fn build_guidance_text(
    command_name: &str,
    size_bytes: u64,
    summary: &serde_json::Value,
    threshold: usize,
) -> String {
    let human_size = format_human_size(size_bytes);
    let threshold_str = format_human_size(threshold as u64);

    let (summary_sentence, search_example, full_response_reasons) =
        command_specific_guidance(command_name, summary);

    format!(
        "Response is {human_size} (above {threshold_str} threshold). \
         {summary_sentence} \
         Options: (1) Use --search \"<query>\" to retrieve matching content only. \
         Example: {search_example}. \
         (2) Use --full-response to retrieve the complete response. \
         Use --full-response when: {full_response_reasons}."
    )
}

fn command_specific_guidance(
    command_name: &str,
    summary: &serde_json::Value,
) -> (String, String, String) {
    match command_name {
        "page snapshot" => {
            let total_nodes = summary["total_nodes"].as_u64().unwrap_or(0);
            let top_roles = summary["top_roles"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            (
                format!(
                    "Summary: accessibility tree with {total_nodes} nodes (top roles: {top_roles})."
                ),
                format!("{command_name} --search \"login\""),
                "you need to inspect all interactive elements, \
                 --search doesn't narrow results sufficiently, \
                 or you are performing a comprehensive page audit"
                    .to_string(),
            )
        }
        "page text" => {
            let char_count = summary["character_count"].as_u64().unwrap_or(0);
            let line_count = summary["line_count"].as_u64().unwrap_or(0);
            (
                format!("Summary: page text with {char_count} characters, {line_count} lines."),
                format!("{command_name} --search \"error\""),
                "you need the complete page text for analysis, \
                 or --search doesn't capture the content you need"
                    .to_string(),
            )
        }
        "js exec" => {
            let result_type = summary["result_type"].as_str().unwrap_or("unknown");
            let size = summary["size_bytes"].as_u64().unwrap_or(0);
            (
                format!(
                    "Summary: JavaScript result of type \"{result_type}\" ({} serialized).",
                    format_human_size(size)
                ),
                format!("{command_name} \"expr\" --search \"key\""),
                "you need the complete result for processing, \
                 or --search doesn't isolate the data you need"
                    .to_string(),
            )
        }
        "network list" => {
            let count = summary["request_count"].as_u64().unwrap_or(0);
            let methods = summary["methods"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            (
                format!("Summary: {count} network requests (methods: {methods})."),
                format!("{command_name} --search \"api\""),
                "you need the complete request list, \
                 or --search doesn't narrow results sufficiently"
                    .to_string(),
            )
        }
        "network get" => {
            let url = summary["url"].as_str().unwrap_or("unknown");
            let status = summary["status"].as_u64().unwrap_or(0);
            (
                format!("Summary: response from {url} (status {status})."),
                format!("{command_name} <id> --search \"token\""),
                "you need the complete response body and headers, \
                 or --search doesn't isolate the data you need"
                    .to_string(),
            )
        }
        _ => (
            "Summary: large response.".to_string(),
            format!("{command_name} --search \"query\""),
            "you need the complete response".to_string(),
        ),
    }
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
    fn guidance_text_contains_key_elements() {
        let summary = serde_json::json!({
            "total_nodes": 5000,
            "top_roles": ["main", "navigation"],
        });
        let text = build_guidance_text("page snapshot", 32_768, &summary, DEFAULT_THRESHOLD);
        assert!(text.contains("32 KB"));
        assert!(text.contains("16 KB threshold"));
        assert!(text.contains("--search"));
        assert!(text.contains("--full-response"));
        assert!(text.contains("page snapshot --search"));
        assert!(text.contains("5000 nodes"));
    }

    #[test]
    fn guidance_text_page_text() {
        let summary = serde_json::json!({
            "character_count": 45000,
            "line_count": 1200,
        });
        let text = build_guidance_text("page text", 45_000, &summary, DEFAULT_THRESHOLD);
        assert!(text.contains("45000 characters"));
        assert!(text.contains("1200 lines"));
        assert!(text.contains("page text --search"));
    }

    #[test]
    fn guidance_text_js_exec() {
        let summary = serde_json::json!({
            "result_type": "object",
            "size_bytes": 32000,
        });
        let text = build_guidance_text("js exec", 32_000, &summary, DEFAULT_THRESHOLD);
        assert!(text.contains("object"));
        assert!(text.contains("js exec"));
    }

    #[test]
    fn guidance_text_network_list() {
        let summary = serde_json::json!({
            "request_count": 150,
            "methods": ["GET", "POST"],
            "domains": ["api.example.com"],
        });
        let text = build_guidance_text("network list", 50_000, &summary, DEFAULT_THRESHOLD);
        assert!(text.contains("150 network requests"));
        assert!(text.contains("GET, POST"));
    }

    #[test]
    fn guidance_text_network_get() {
        let summary = serde_json::json!({
            "url": "https://api.example.com/data",
            "status": 200,
        });
        let text = build_guidance_text("network get", 50_000, &summary, DEFAULT_THRESHOLD);
        assert!(text.contains("api.example.com"));
        assert!(text.contains("200"));
    }

    #[test]
    fn emit_below_threshold_prints_json() {
        // Construct a small value that's below the threshold
        let value = serde_json::json!({"key": "value"});
        let output = OutputFormat {
            json: true,
            pretty: false,
            plain: false,
            full_response: false,
            large_response_threshold: Some(1_000_000),
        };
        // emit() prints to stdout; we just verify it doesn't error
        let result = emit(&value, &output, "test", |_| serde_json::json!({}));
        assert!(result.is_ok());
    }

    #[test]
    fn emit_full_response_bypasses_gate() {
        let value = serde_json::json!({"key": "value"});
        let output = OutputFormat {
            json: true,
            pretty: false,
            plain: false,
            full_response: true,
            large_response_threshold: Some(1), // Very low threshold
        };
        let result = emit(&value, &output, "test", |_| serde_json::json!({}));
        assert!(result.is_ok());
    }

    #[test]
    fn emit_searched_always_prints() {
        let value = serde_json::json!({"key": "value"});
        let output = OutputFormat {
            json: true,
            pretty: false,
            plain: false,
            full_response: false,
            large_response_threshold: Some(1),
        };
        let result = emit_searched(&value, &output);
        assert!(result.is_ok());
    }

    #[test]
    fn large_response_guidance_serialization() {
        let guidance = LargeResponseGuidance {
            large_response: true,
            size_bytes: 32_768,
            command: "page snapshot".to_string(),
            summary: serde_json::json!({"total_nodes": 5000}),
            guidance: "test guidance".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&guidance).unwrap();
        assert_eq!(json["large_response"], true);
        assert_eq!(json["size_bytes"], 32_768);
        assert_eq!(json["command"], "page snapshot");
        assert_eq!(json["summary"]["total_nodes"], 5000);
        assert_eq!(json["guidance"], "test guidance");
    }

    #[test]
    fn command_specific_guidance_unknown_command() {
        let summary = serde_json::json!({});
        let (sentence, _example, _reasons) = command_specific_guidance("unknown", &summary);
        assert!(sentence.contains("large response"));
    }
}
