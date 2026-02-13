use std::fmt;

use serde::Serialize;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ExitCode {
    Success = 0,
    GeneralError = 1,
    ConnectionError = 2,
    TargetError = 3,
    TimeoutError = 4,
    ProtocolError = 5,
}

impl fmt::Display for ExitCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::GeneralError => write!(f, "general error"),
            Self::ConnectionError => write!(f, "connection error"),
            Self::TargetError => write!(f, "target error"),
            Self::TimeoutError => write!(f, "timeout error"),
            Self::ProtocolError => write!(f, "protocol error"),
        }
    }
}

#[derive(Debug)]
pub struct AppError {
    pub message: String,
    pub code: ExitCode,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl AppError {
    #[must_use]
    pub fn not_implemented(command: &str) -> Self {
        Self {
            message: format!("{command}: not yet implemented"),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn stale_session() -> Self {
        Self {
            message: "Session is stale: Chrome is not reachable at the stored address. \
                      Run 'chrome-cli connect' to establish a new connection."
                .into(),
            code: ExitCode::ConnectionError,
        }
    }

    #[must_use]
    pub fn no_session() -> Self {
        Self {
            message: "No active session. Run 'chrome-cli connect' or \
                      'chrome-cli connect --launch' to establish a connection."
                .into(),
            code: ExitCode::ConnectionError,
        }
    }

    #[must_use]
    pub fn target_not_found(tab: &str) -> Self {
        Self {
            message: format!(
                "Tab '{tab}' not found. Run 'chrome-cli tabs list' to see available tabs."
            ),
            code: ExitCode::TargetError,
        }
    }

    #[must_use]
    pub fn no_page_targets() -> Self {
        Self {
            message: "No page targets found in Chrome. Open a tab first.".into(),
            code: ExitCode::TargetError,
        }
    }

    #[must_use]
    pub fn last_tab() -> Self {
        Self {
            message: "Cannot close the last tab. Chrome requires at least one open tab.".into(),
            code: ExitCode::TargetError,
        }
    }

    #[must_use]
    pub fn navigation_failed(error_text: &str) -> Self {
        Self {
            message: format!("Navigation failed: {error_text}"),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn navigation_timeout(timeout_ms: u64, strategy: &str) -> Self {
        Self {
            message: format!("Navigation timed out after {timeout_ms}ms waiting for {strategy}"),
            code: ExitCode::TimeoutError,
        }
    }

    #[must_use]
    pub fn element_not_found(selector: &str) -> Self {
        Self {
            message: format!("Element not found for selector: {selector}"),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn evaluation_failed(description: &str) -> Self {
        Self {
            message: format!("Text extraction failed: {description}"),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn snapshot_failed(description: &str) -> Self {
        Self {
            message: format!("Accessibility tree capture failed: {description}"),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn file_write_failed(path: &str, error: &str) -> Self {
        Self {
            message: format!("Failed to write snapshot to file: {path}: {error}"),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn screenshot_failed(description: &str) -> Self {
        Self {
            message: format!("Screenshot capture failed: {description}"),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn uid_not_found(uid: &str) -> Self {
        Self {
            message: format!("UID '{uid}' not found. Run 'chrome-cli page snapshot' first."),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn invalid_clip(input: &str) -> Self {
        Self {
            message: format!(
                "Invalid clip format: expected X,Y,WIDTH,HEIGHT (e.g. 10,20,200,100): {input}"
            ),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn no_active_trace() -> Self {
        Self {
            message: "No active trace. Run 'chrome-cli perf start' first.".into(),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn unknown_insight(name: &str) -> Self {
        Self {
            message: format!(
                "Unknown insight: '{name}'. Available: DocumentLatency, LCPBreakdown, \
                 RenderBlocking, LongTasks"
            ),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn trace_file_not_found(path: &str) -> Self {
        Self {
            message: format!("Trace file not found: {path}"),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn trace_parse_failed(error: &str) -> Self {
        Self {
            message: format!("Failed to parse trace file: {error}"),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn trace_timeout(timeout_ms: u64) -> Self {
        Self {
            message: format!("Trace timed out after {timeout_ms}ms"),
            code: ExitCode::TimeoutError,
        }
    }

    #[must_use]
    pub fn js_execution_failed(description: &str) -> Self {
        Self {
            message: format!("JavaScript execution failed: {description}"),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn script_file_not_found(path: &str) -> Self {
        Self {
            message: format!("Script file not found: {path}"),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn script_file_read_failed(path: &str, error: &str) -> Self {
        Self {
            message: format!("Failed to read script file: {path}: {error}"),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn no_js_code() -> Self {
        Self {
            message:
                "No JavaScript code provided. Specify code as argument, --file, or pipe via stdin."
                    .into(),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn no_dialog_open() -> Self {
        Self {
            message: "No dialog is currently open. A dialog must be open before it can be handled."
                .into(),
            code: ExitCode::GeneralError,
        }
    }

    #[must_use]
    pub fn dialog_handle_failed(reason: &str) -> Self {
        Self {
            message: format!("Dialog handling failed: {reason}"),
            code: ExitCode::ProtocolError,
        }
    }

    #[must_use]
    pub fn no_chrome_found() -> Self {
        Self {
            message: "No Chrome instance found. Run 'chrome-cli connect' or \
                      'chrome-cli connect --launch' to establish a connection."
                .into(),
            code: ExitCode::ConnectionError,
        }
    }

    #[must_use]
    pub fn to_json(&self) -> String {
        let output = ErrorOutput {
            error: &self.message,
            code: self.code as u8,
        };
        serde_json::to_string(&output).unwrap_or_else(|_| {
            format!(
                r#"{{"error":"{}","code":{}}}"#,
                self.message, self.code as u8
            )
        })
    }

    pub fn print_json_stderr(&self) {
        eprintln!("{}", self.to_json());
    }
}

#[derive(Serialize)]
struct ErrorOutput<'a> {
    error: &'a str,
    code: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_implemented_produces_json_with_error_and_code() {
        let err = AppError::not_implemented("tabs");
        let json = err.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["error"], "tabs: not yet implemented");
        assert_eq!(parsed["code"], 1);
    }

    #[test]
    fn exit_code_display() {
        assert_eq!(ExitCode::Success.to_string(), "success");
        assert_eq!(ExitCode::GeneralError.to_string(), "general error");
        assert_eq!(ExitCode::ConnectionError.to_string(), "connection error");
    }

    #[test]
    fn app_error_display() {
        let err = AppError::not_implemented("connect");
        assert_eq!(
            err.to_string(),
            "general error: connect: not yet implemented"
        );
    }

    #[test]
    fn stale_session_error() {
        let err = AppError::stale_session();
        assert!(err.message.contains("stale"));
        assert!(err.message.contains("chrome-cli connect"));
        assert!(matches!(err.code, ExitCode::ConnectionError));
    }

    #[test]
    fn no_session_error() {
        let err = AppError::no_session();
        assert!(err.message.contains("No active session"));
        assert!(matches!(err.code, ExitCode::ConnectionError));
    }

    #[test]
    fn target_not_found_error() {
        let err = AppError::target_not_found("ABCDEF");
        assert!(err.message.contains("ABCDEF"));
        assert!(err.message.contains("tabs list"));
        assert!(matches!(err.code, ExitCode::TargetError));
    }

    #[test]
    fn no_page_targets_error() {
        let err = AppError::no_page_targets();
        assert!(err.message.contains("No page targets"));
        assert!(matches!(err.code, ExitCode::TargetError));
    }

    #[test]
    fn last_tab_error() {
        let err = AppError::last_tab();
        assert!(err.message.contains("Cannot close the last tab"));
        assert!(err.message.contains("at least one open tab"));
        assert!(matches!(err.code, ExitCode::TargetError));
    }

    #[test]
    fn navigation_failed_error() {
        let err = AppError::navigation_failed("net::ERR_NAME_NOT_RESOLVED");
        assert!(err.message.contains("Navigation failed"));
        assert!(err.message.contains("ERR_NAME_NOT_RESOLVED"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn navigation_timeout_error() {
        let err = AppError::navigation_timeout(30000, "load");
        assert!(err.message.contains("timed out"));
        assert!(err.message.contains("30000ms"));
        assert!(err.message.contains("load"));
        assert!(matches!(err.code, ExitCode::TimeoutError));
    }

    #[test]
    fn element_not_found_error() {
        let err = AppError::element_not_found("#missing");
        assert!(err.message.contains("Element not found"));
        assert!(err.message.contains("#missing"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn evaluation_failed_error() {
        let err = AppError::evaluation_failed("script threw an exception");
        assert!(err.message.contains("Text extraction failed"));
        assert!(err.message.contains("script threw an exception"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn snapshot_failed_error() {
        let err = AppError::snapshot_failed("domain not enabled");
        assert!(err.message.contains("Accessibility tree capture failed"));
        assert!(err.message.contains("domain not enabled"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn file_write_failed_error() {
        let err = AppError::file_write_failed("/tmp/out.txt", "permission denied");
        assert!(err.message.contains("Failed to write snapshot to file"));
        assert!(err.message.contains("/tmp/out.txt"));
        assert!(err.message.contains("permission denied"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn no_chrome_found_error() {
        let err = AppError::no_chrome_found();
        assert!(err.message.contains("No Chrome instance found"));
        assert!(matches!(err.code, ExitCode::ConnectionError));
    }

    #[test]
    fn screenshot_failed_error() {
        let err = AppError::screenshot_failed("timeout waiting for capture");
        assert!(err.message.contains("Screenshot capture failed"));
        assert!(err.message.contains("timeout waiting for capture"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn uid_not_found_error() {
        let err = AppError::uid_not_found("s99");
        assert!(err.message.contains("s99"));
        assert!(err.message.contains("page snapshot"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn invalid_clip_error() {
        let err = AppError::invalid_clip("abc");
        assert!(err.message.contains("Invalid clip format"));
        assert!(err.message.contains("X,Y,WIDTH,HEIGHT"));
        assert!(err.message.contains("abc"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn no_active_trace_error() {
        let err = AppError::no_active_trace();
        assert!(err.message.contains("No active trace"));
        assert!(err.message.contains("perf start"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn unknown_insight_error() {
        let err = AppError::unknown_insight("BadInsight");
        assert!(err.message.contains("Unknown insight"));
        assert!(err.message.contains("BadInsight"));
        assert!(err.message.contains("DocumentLatency"));
        assert!(err.message.contains("LCPBreakdown"));
        assert!(err.message.contains("RenderBlocking"));
        assert!(err.message.contains("LongTasks"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn trace_file_not_found_error() {
        let err = AppError::trace_file_not_found("/tmp/missing.json");
        assert!(err.message.contains("Trace file not found"));
        assert!(err.message.contains("/tmp/missing.json"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn trace_parse_failed_error() {
        let err = AppError::trace_parse_failed("unexpected EOF");
        assert!(err.message.contains("Failed to parse trace file"));
        assert!(err.message.contains("unexpected EOF"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn trace_timeout_error() {
        let err = AppError::trace_timeout(30000);
        assert!(err.message.contains("Trace timed out"));
        assert!(err.message.contains("30000ms"));
        assert!(matches!(err.code, ExitCode::TimeoutError));
    }

    #[test]
    fn js_execution_failed_error() {
        let err = AppError::js_execution_failed("ReferenceError: foo is not defined");
        assert!(err.message.contains("JavaScript execution failed"));
        assert!(err.message.contains("ReferenceError: foo is not defined"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn script_file_not_found_error() {
        let err = AppError::script_file_not_found("/tmp/missing.js");
        assert!(err.message.contains("Script file not found"));
        assert!(err.message.contains("/tmp/missing.js"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn script_file_read_failed_error() {
        let err = AppError::script_file_read_failed("/tmp/bad.js", "permission denied");
        assert!(err.message.contains("Failed to read script file"));
        assert!(err.message.contains("/tmp/bad.js"));
        assert!(err.message.contains("permission denied"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn no_dialog_open_error() {
        let err = AppError::no_dialog_open();
        assert!(err.message.contains("No dialog is currently open"));
        assert!(err.message.contains("must be open"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }

    #[test]
    fn dialog_handle_failed_error() {
        let err = AppError::dialog_handle_failed("could not dismiss");
        assert!(err.message.contains("Dialog handling failed"));
        assert!(err.message.contains("could not dismiss"));
        assert!(matches!(err.code, ExitCode::ProtocolError));
    }

    #[test]
    fn no_js_code_error() {
        let err = AppError::no_js_code();
        assert!(err.message.contains("No JavaScript code provided"));
        assert!(err.message.contains("--file"));
        assert!(err.message.contains("stdin"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }
}
