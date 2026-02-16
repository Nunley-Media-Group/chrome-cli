use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Session file content persisted between CLI invocations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub ws_url: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub timestamp: String,
}

/// Errors that can occur during session file operations.
#[derive(Debug)]
pub enum SessionError {
    /// Could not determine home directory.
    NoHomeDir,
    /// I/O error reading/writing session file.
    Io(std::io::Error),
    /// Session file contains invalid JSON.
    InvalidFormat(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHomeDir => write!(f, "could not determine home directory"),
            Self::Io(e) => write!(f, "session file error: {e}"),
            Self::InvalidFormat(e) => write!(f, "invalid session file: {e}"),
        }
    }
}

impl std::error::Error for SessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SessionError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<SessionError> for crate::error::AppError {
    fn from(e: SessionError) -> Self {
        use crate::error::ExitCode;
        Self {
            message: e.to_string(),
            code: ExitCode::GeneralError,
            custom_json: None,
        }
    }
}

/// Returns the path to the session file: `~/.chrome-cli/session.json`.
///
/// Uses `$HOME` on Unix and `%USERPROFILE%` on Windows.
///
/// # Errors
///
/// Returns `SessionError::NoHomeDir` if the home directory cannot be determined.
pub fn session_file_path() -> Result<PathBuf, SessionError> {
    let home = home_dir()?;
    Ok(home.join(".chrome-cli").join("session.json"))
}

fn home_dir() -> Result<PathBuf, SessionError> {
    #[cfg(unix)]
    let key = "HOME";
    #[cfg(windows)]
    let key = "USERPROFILE";

    std::env::var(key)
        .map(PathBuf::from)
        .map_err(|_| SessionError::NoHomeDir)
}

/// Write session data to the session file. Creates `~/.chrome-cli/` if needed.
///
/// Uses atomic write (write to temp file then rename) and sets file permissions
/// to `0o600` on Unix.
///
/// # Errors
///
/// Returns `SessionError::Io` on I/O failure or `SessionError::NoHomeDir` if the
/// home directory cannot be determined.
pub fn write_session(data: &SessionData) -> Result<(), SessionError> {
    let path = session_file_path()?;
    write_session_to(&path, data)
}

/// Write session data to a specific path. Testable variant of [`write_session`].
///
/// # Errors
///
/// Returns `SessionError::Io` on I/O failure.
pub fn write_session_to(path: &std::path::Path, data: &SessionData) -> Result<(), SessionError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
    }

    let json = serde_json::to_string_pretty(data)
        .map_err(|e| SessionError::InvalidFormat(e.to_string()))?;

    // Atomic write: write to temp file, then rename
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, &json)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600))?;
    }

    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Read session data from the session file.
///
/// Returns `Ok(None)` if the file does not exist.
///
/// # Errors
///
/// Returns `SessionError::InvalidFormat` if the file contains invalid JSON,
/// or `SessionError::Io` on other I/O errors.
pub fn read_session() -> Result<Option<SessionData>, SessionError> {
    let path = session_file_path()?;
    read_session_from(&path)
}

/// Read session data from a specific path. Testable variant of [`read_session`].
///
/// # Errors
///
/// Returns `SessionError::InvalidFormat` if the file contains invalid JSON,
/// or `SessionError::Io` on other I/O errors.
pub fn read_session_from(path: &std::path::Path) -> Result<Option<SessionData>, SessionError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let data: SessionData = serde_json::from_str(&contents)
                .map_err(|e| SessionError::InvalidFormat(e.to_string()))?;
            Ok(Some(data))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(SessionError::Io(e)),
    }
}

/// Delete the session file. Returns `Ok(())` even if the file doesn't exist.
///
/// # Errors
///
/// Returns `SessionError::Io` on I/O errors other than "not found".
pub fn delete_session() -> Result<(), SessionError> {
    let path = session_file_path()?;
    delete_session_from(&path)
}

/// Delete a session file at a specific path. Testable variant of [`delete_session`].
///
/// # Errors
///
/// Returns `SessionError::Io` on I/O errors other than "not found".
pub fn delete_session_from(path: &std::path::Path) -> Result<(), SessionError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(SessionError::Io(e)),
    }
}

/// Format the current time as a simplified ISO 8601 string (e.g., `"2026-02-11T12:00:00Z"`).
///
/// Uses the Howard Hinnant algorithm for civil date computation from Unix timestamp.
#[must_use]
pub fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    format_unix_secs(secs)
}

#[allow(
    clippy::similar_names,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn format_unix_secs(secs: u64) -> String {
    let day_secs = secs % 86_400;
    let hours = day_secs / 3_600;
    let minutes = (day_secs % 3_600) / 60;
    let seconds = day_secs % 60;

    // Howard Hinnant's algorithm for civil date from days since epoch
    let mut days = (secs / 86_400) as i64;
    days += 719_468; // shift epoch from 1970-01-01 to 0000-03-01
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = (days - era * 146_097) as u32; // [0, 146096]
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
    let y = i64::from(year_of_era) + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100); // [0, 365]
    let mp = (5 * day_of_year + 2) / 153; // month index [0, 11]
    let d = day_of_year - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_file_path_ends_with_expected_suffix() {
        let path = session_file_path().unwrap();
        assert!(path.ends_with(".chrome-cli/session.json"));
    }

    #[test]
    fn format_unix_epoch() {
        assert_eq!(format_unix_secs(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn format_known_timestamp() {
        // 2001-09-09T01:46:40Z = 1_000_000_000 seconds since epoch (well-known)
        assert_eq!(format_unix_secs(1_000_000_000), "2001-09-09T01:46:40Z");
    }

    #[test]
    fn now_iso8601_produces_valid_format() {
        let ts = now_iso8601();
        // Basic format validation: YYYY-MM-DDTHH:MM:SSZ
        assert_eq!(ts.len(), 20);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
        assert_eq!(&ts[19..20], "Z");
    }

    #[test]
    fn write_read_round_trip() {
        let dir = std::env::temp_dir().join("chrome-cli-test-session-rt");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        let data = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/abc".into(),
            port: 9222,
            pid: Some(1234),
            timestamp: "2026-02-11T12:00:00Z".into(),
        };

        write_session_to(&path, &data).unwrap();
        let read = read_session_from(&path).unwrap().unwrap();

        assert_eq!(read.ws_url, data.ws_url);
        assert_eq!(read.port, data.port);
        assert_eq!(read.pid, data.pid);
        assert_eq!(read.timestamp, data.timestamp);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_read_round_trip_no_pid() {
        let dir = std::env::temp_dir().join("chrome-cli-test-session-nopid");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        let data = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/xyz".into(),
            port: 9222,
            pid: None,
            timestamp: "2026-02-11T12:00:00Z".into(),
        };

        write_session_to(&path, &data).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(!contents.contains("pid"), "pid should be skipped when None");

        let read = read_session_from(&path).unwrap().unwrap();
        assert_eq!(read.pid, None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_nonexistent_returns_none() {
        let path = std::path::Path::new("/tmp/chrome-cli-test-nonexistent/session.json");
        let result = read_session_from(path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_invalid_json_returns_error() {
        let dir = std::env::temp_dir().join("chrome-cli-test-session-invalid");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.json");
        std::fs::write(&path, "not valid json").unwrap();

        let result = read_session_from(&path);
        assert!(matches!(result, Err(SessionError::InvalidFormat(_))));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_nonexistent_returns_ok() {
        let path = std::path::Path::new("/tmp/chrome-cli-test-del-nonexist/session.json");
        assert!(delete_session_from(path).is_ok());
    }

    #[test]
    fn delete_existing_removes_file() {
        let dir = std::env::temp_dir().join("chrome-cli-test-session-del");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.json");
        std::fs::write(&path, "{}").unwrap();
        assert!(path.exists());

        delete_session_from(&path).unwrap();
        assert!(!path.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Simulate the PID-preservation logic from `save_session()`: read existing
    /// session, carry PID forward if ports match and incoming PID is None.
    fn resolve_pid(
        path: &std::path::Path,
        incoming_pid: Option<u32>,
        incoming_port: u16,
    ) -> Option<u32> {
        incoming_pid.or_else(|| {
            read_session_from(path)
                .ok()
                .flatten()
                .filter(|existing| existing.port == incoming_port)
                .and_then(|existing| existing.pid)
        })
    }

    #[test]
    fn pid_preserved_when_ports_match() {
        let dir = std::env::temp_dir().join("chrome-cli-test-pid-preserve");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        // Write initial session with PID (simulates --launch)
        let launch = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/aaa".into(),
            port: 9222,
            pid: Some(54321),
            timestamp: "2026-02-15T00:00:00Z".into(),
        };
        write_session_to(&path, &launch).unwrap();

        // Simulate auto-discover on same port (pid: None)
        let pid = resolve_pid(&path, None, 9222);
        assert_eq!(
            pid,
            Some(54321),
            "PID should be preserved from existing session"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pid_not_preserved_when_ports_differ() {
        let dir = std::env::temp_dir().join("chrome-cli-test-pid-nopreserve");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        // Write initial session with PID on port 9222
        let launch = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/bbb".into(),
            port: 9222,
            pid: Some(99999),
            timestamp: "2026-02-15T00:00:00Z".into(),
        };
        write_session_to(&path, &launch).unwrap();

        // Simulate auto-discover on DIFFERENT port (pid: None)
        let pid = resolve_pid(&path, None, 9333);
        assert_eq!(pid, None, "PID should NOT be carried from a different port");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pid_not_injected_when_no_prior_session() {
        let dir = std::env::temp_dir().join("chrome-cli-test-pid-noinject");
        let _ = std::fs::remove_dir_all(&dir);
        // Do NOT create the session file

        let path = dir.join("session.json");
        let pid = resolve_pid(&path, None, 9222);
        assert_eq!(
            pid, None,
            "No PID should be injected when no prior session exists"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn incoming_pid_takes_priority_over_existing() {
        let dir = std::env::temp_dir().join("chrome-cli-test-pid-priority");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        // Write existing session with PID
        let existing = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/ccc".into(),
            port: 9222,
            pid: Some(11111),
            timestamp: "2026-02-15T00:00:00Z".into(),
        };
        write_session_to(&path, &existing).unwrap();

        // Incoming ConnectionInfo has its own PID (e.g. new --launch)
        let pid = resolve_pid(&path, Some(22222), 9222);
        assert_eq!(pid, Some(22222), "Incoming PID should take priority");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn session_error_display() {
        assert_eq!(
            SessionError::NoHomeDir.to_string(),
            "could not determine home directory"
        );
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        assert_eq!(
            SessionError::Io(io_err).to_string(),
            "session file error: denied"
        );
        assert_eq!(
            SessionError::InvalidFormat("bad json".into()).to_string(),
            "invalid session file: bad json"
        );
    }
}
