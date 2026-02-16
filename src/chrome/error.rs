use std::fmt;

/// Errors that can occur during Chrome discovery and launch.
#[derive(Debug)]
pub enum ChromeError {
    /// Chrome executable was not found on the system.
    NotFound(String),

    /// Chrome process failed to launch.
    LaunchFailed(String),

    /// Chrome did not start accepting connections within the timeout.
    StartupTimeout {
        /// The port Chrome was expected to listen on.
        port: u16,
    },

    /// HTTP request to Chrome's debug endpoint failed.
    HttpError(String),

    /// Failed to parse a response from Chrome.
    ParseError(String),

    /// The `DevToolsActivePort` file was not found.
    NoActivePort,

    /// No running Chrome instance could be discovered.
    NotRunning(String),

    /// An I/O error occurred.
    Io(std::io::Error),
}

impl fmt::Display for ChromeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Chrome not found: {msg}"),
            Self::LaunchFailed(msg) => write!(f, "Chrome launch failed: {msg}"),
            Self::StartupTimeout { port } => {
                write!(
                    f,
                    "Chrome startup timed out on port {port}. Try --timeout to increase the wait time, or --headless for headless mode"
                )
            }
            Self::HttpError(msg) => write!(f, "Chrome HTTP error: {msg}"),
            Self::ParseError(msg) => write!(f, "Chrome parse error: {msg}"),
            Self::NoActivePort => write!(f, "DevToolsActivePort file not found"),
            Self::NotRunning(detail) => {
                write!(
                    f,
                    "no running Chrome instance found with remote debugging: {detail}"
                )
            }
            Self::Io(e) => write!(f, "Chrome I/O error: {e}"),
        }
    }
}

impl std::error::Error for ChromeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ChromeError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ChromeError> for crate::error::AppError {
    fn from(e: ChromeError) -> Self {
        use crate::error::ExitCode;
        let code = match &e {
            ChromeError::NotFound(_) | ChromeError::ParseError(_) | ChromeError::Io(_) => {
                ExitCode::GeneralError
            }
            ChromeError::LaunchFailed(_)
            | ChromeError::HttpError(_)
            | ChromeError::NotRunning(_)
            | ChromeError::NoActivePort => ExitCode::ConnectionError,
            ChromeError::StartupTimeout { .. } => ExitCode::TimeoutError,
        };
        Self {
            message: e.to_string(),
            code,
            custom_json: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_not_found() {
        let err = ChromeError::NotFound("try --chrome-path".into());
        assert_eq!(err.to_string(), "Chrome not found: try --chrome-path");
    }

    #[test]
    fn display_launch_failed() {
        let err = ChromeError::LaunchFailed("permission denied".into());
        assert_eq!(err.to_string(), "Chrome launch failed: permission denied");
    }

    #[test]
    fn display_startup_timeout() {
        let err = ChromeError::StartupTimeout { port: 9222 };
        assert_eq!(
            err.to_string(),
            "Chrome startup timed out on port 9222. Try --timeout to increase the wait time, or --headless for headless mode"
        );
    }

    #[test]
    fn display_http_error() {
        let err = ChromeError::HttpError("connection refused".into());
        assert_eq!(err.to_string(), "Chrome HTTP error: connection refused");
    }

    #[test]
    fn display_parse_error() {
        let err = ChromeError::ParseError("invalid JSON".into());
        assert_eq!(err.to_string(), "Chrome parse error: invalid JSON");
    }

    #[test]
    fn display_no_active_port() {
        let err = ChromeError::NoActivePort;
        assert_eq!(err.to_string(), "DevToolsActivePort file not found");
    }

    #[test]
    fn display_not_running() {
        let err = ChromeError::NotRunning("port 9222 refused".into());
        assert_eq!(
            err.to_string(),
            "no running Chrome instance found with remote debugging: port 9222 refused"
        );
    }

    #[test]
    fn display_io() {
        let err = ChromeError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file gone",
        ));
        assert_eq!(err.to_string(), "Chrome I/O error: file gone");
    }

    #[test]
    fn error_source_is_none_for_non_io() {
        let err: &dyn std::error::Error = &ChromeError::NotRunning("no instance".into());
        assert!(err.source().is_none());
    }

    #[test]
    fn error_source_returns_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file gone");
        let err: &dyn std::error::Error = &ChromeError::Io(io_err);
        assert!(err.source().is_some());
    }
}
