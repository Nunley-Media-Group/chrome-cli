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
    fn no_chrome_found_error() {
        let err = AppError::no_chrome_found();
        assert!(err.message.contains("No Chrome instance found"));
        assert!(matches!(err.code, ExitCode::ConnectionError));
    }
}
