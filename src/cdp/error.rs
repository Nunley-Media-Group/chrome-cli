use std::fmt;

/// Errors that can occur during CDP communication.
#[derive(Debug)]
pub enum CdpError {
    /// WebSocket connection could not be established.
    Connection(String),

    /// Connection attempt exceeded the configured timeout.
    ConnectionTimeout,

    /// A command did not receive a response within the configured timeout.
    CommandTimeout {
        /// The CDP method that timed out.
        method: String,
    },

    /// Chrome returned a CDP protocol-level error.
    Protocol {
        /// The CDP error code (e.g., -32000).
        code: i64,
        /// The CDP error message.
        message: String,
    },

    /// The WebSocket connection was closed unexpectedly.
    ConnectionClosed,

    /// Failed to parse a message received from Chrome.
    InvalidResponse(String),

    /// Reconnection failed after all retry attempts were exhausted.
    ReconnectFailed {
        /// Number of reconnection attempts made.
        attempts: u32,
        /// The error from the last reconnection attempt.
        last_error: String,
    },

    /// Internal error (e.g., transport task died or channel closed).
    Internal(String),
}

impl fmt::Display for CdpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection(msg) => write!(f, "CDP connection error: {msg}"),
            Self::ConnectionTimeout => write!(f, "CDP connection timed out"),
            Self::CommandTimeout { method } => {
                write!(f, "CDP command timed out: {method}")
            }
            Self::Protocol { code, message } => {
                write!(f, "CDP protocol error ({code}): {message}")
            }
            Self::ConnectionClosed => write!(f, "CDP connection closed"),
            Self::InvalidResponse(msg) => {
                write!(f, "CDP invalid response: {msg}")
            }
            Self::ReconnectFailed {
                attempts,
                last_error,
            } => {
                write!(
                    f,
                    "CDP reconnection failed after {attempts} attempts: {last_error}"
                )
            }
            Self::Internal(msg) => write!(f, "CDP internal error: {msg}"),
        }
    }
}

impl std::error::Error for CdpError {}

impl From<CdpError> for crate::error::AppError {
    fn from(e: CdpError) -> Self {
        use crate::error::ExitCode;
        let code = match &e {
            CdpError::Connection(_)
            | CdpError::ConnectionClosed
            | CdpError::ReconnectFailed { .. } => ExitCode::ConnectionError,
            CdpError::ConnectionTimeout | CdpError::CommandTimeout { .. } => ExitCode::TimeoutError,
            CdpError::Protocol { .. } => ExitCode::ProtocolError,
            CdpError::InvalidResponse(_) | CdpError::Internal(_) => ExitCode::GeneralError,
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
    fn display_connection() {
        let err = CdpError::Connection("refused".into());
        assert_eq!(err.to_string(), "CDP connection error: refused");
    }

    #[test]
    fn display_connection_timeout() {
        let err = CdpError::ConnectionTimeout;
        assert_eq!(err.to_string(), "CDP connection timed out");
    }

    #[test]
    fn display_command_timeout() {
        let err = CdpError::CommandTimeout {
            method: "Page.navigate".into(),
        };
        assert_eq!(err.to_string(), "CDP command timed out: Page.navigate");
    }

    #[test]
    fn display_protocol() {
        let err = CdpError::Protocol {
            code: -32000,
            message: "Not found".into(),
        };
        assert_eq!(err.to_string(), "CDP protocol error (-32000): Not found");
    }

    #[test]
    fn display_connection_closed() {
        let err = CdpError::ConnectionClosed;
        assert_eq!(err.to_string(), "CDP connection closed");
    }

    #[test]
    fn display_invalid_response() {
        let err = CdpError::InvalidResponse("bad json".into());
        assert_eq!(err.to_string(), "CDP invalid response: bad json");
    }

    #[test]
    fn display_reconnect_failed() {
        let err = CdpError::ReconnectFailed {
            attempts: 3,
            last_error: "connection refused".into(),
        };
        assert_eq!(
            err.to_string(),
            "CDP reconnection failed after 3 attempts: connection refused"
        );
    }

    #[test]
    fn display_internal() {
        let err = CdpError::Internal("channel closed".into());
        assert_eq!(err.to_string(), "CDP internal error: channel closed");
    }

    #[test]
    fn error_trait_is_implemented() {
        let err: &dyn std::error::Error = &CdpError::ConnectionClosed;
        // Ensure we can call source() without panic
        assert!(err.source().is_none());
    }
}
