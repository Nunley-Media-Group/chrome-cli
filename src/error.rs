use std::fmt;

use serde::Serialize;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
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
    pub fn not_implemented(command: &str) -> Self {
        Self {
            message: format!("{command}: not yet implemented"),
            code: ExitCode::GeneralError,
        }
    }

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
}
