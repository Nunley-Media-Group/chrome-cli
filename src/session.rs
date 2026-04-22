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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub active_tab_id: Option<String>,
    pub timestamp: String,
    /// ISO 8601 timestamp of the most recent auto-reconnect, or `None` if never.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_reconnect_at: Option<String>,
    /// Cumulative successful auto-reconnects for this session file.
    #[serde(default)]
    pub reconnect_count: u32,
}

/// Errors that can occur during session file operations.
#[derive(Debug)]
pub enum SessionError {
    /// Could not determine home directory. Carries a diagnostic listing the
    /// environment variables consulted so Windows misconfiguration is
    /// self-evident in the error message.
    NoHomeDir(String),
    /// I/O error reading/writing session file.
    Io(std::io::Error),
    /// Session file contains invalid JSON. Includes the resolved file path so
    /// users' first troubleshooting step is obvious.
    InvalidFormat(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHomeDir(diag) => {
                write!(f, "could not determine home directory ({diag})")
            }
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

/// Returns the path to the session file: `~/.agentchrome/session.json`.
///
/// Uses `$HOME` on Unix and `%USERPROFILE%` on Windows.
///
/// # Errors
///
/// Returns `SessionError::NoHomeDir` if the home directory cannot be determined.
pub fn session_file_path() -> Result<PathBuf, SessionError> {
    let home = home_dir()?;
    Ok(home.join(".agentchrome").join("session.json"))
}

fn home_dir() -> Result<PathBuf, SessionError> {
    #[cfg(unix)]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| SessionError::NoHomeDir("HOME env var is unset or invalid".to_string()))
    }

    #[cfg(windows)]
    {
        windows_home_chain(&|k| std::env::var(k).ok())
    }
}

/// Shared implementation of the Windows `%USERPROFILE%` → `%HOMEDRIVE%%HOMEPATH%`
/// fallback chain. Exposed for unit tests on all platforms so the resolution
/// contract is exercised regardless of the host OS.
#[cfg_attr(not(windows), allow(dead_code))]
fn windows_home_chain<F>(get: &F) -> Result<PathBuf, SessionError>
where
    F: Fn(&str) -> Option<String>,
{
    if let Some(v) = get("USERPROFILE").filter(|s| !s.is_empty()) {
        return Ok(PathBuf::from(v));
    }
    match (
        get("HOMEDRIVE").filter(|s| !s.is_empty()),
        get("HOMEPATH").filter(|s| !s.is_empty()),
    ) {
        (Some(drive), Some(path)) => Ok(PathBuf::from(format!("{drive}{path}"))),
        _ => Err(SessionError::NoHomeDir(
            "checked USERPROFILE (unset), HOMEDRIVE+HOMEPATH (unset)".to_string(),
        )),
    }
}

/// Write session data to the session file. Creates `~/.agentchrome/` if needed.
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
        set_owner_only_perms(parent, 0o700)?;
    }

    let json = serde_json::to_string_pretty(data)
        .map_err(|e| SessionError::InvalidFormat(e.to_string()))?;

    write_session_atomic(path, json.as_bytes())
}

#[cfg(unix)]
fn set_owner_only_perms(path: &std::path::Path, mode: u32) -> Result<(), SessionError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_perms(_path: &std::path::Path, _mode: u32) -> Result<(), SessionError> {
    Ok(())
}

/// Maximum retry attempts for the temp→final rename on Windows/NTFS. Antivirus
/// scanners occasionally hold the temp file open briefly after write, so a
/// bounded retry is more reliable than a single attempt.
const RENAME_RETRIES: u32 = 5;
/// Delay between rename retries. Short enough not to noticeably stall a
/// successful write; long enough to let an AV scanner finish a handle release.
const RENAME_RETRY_DELAY_MS: u64 = 10;

/// Atomic-write primitive: write to `{path}.tmp`, then rename into place.
///
/// On Windows, `std::fs::rename` can fail with `AccessDenied` /
/// `PermissionDenied` when an antivirus scanner is inspecting the temp file.
/// We retry a bounded number of times and, if the retries exhaust, fall back
/// to a direct non-atomic write with a `WARN` line on stderr so the user
/// knows the atomic guarantee was skipped.
fn write_session_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<(), SessionError> {
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, bytes)?;
    set_owner_only_perms(&tmp_path, 0o600)?;

    let mut last_err: Option<std::io::Error> = None;
    for attempt in 0..RENAME_RETRIES {
        match std::fs::rename(&tmp_path, path) {
            Ok(()) => return Ok(()),
            Err(e) if is_transient_rename_error(&e) => {
                last_err = Some(e);
                if attempt + 1 < RENAME_RETRIES {
                    std::thread::sleep(std::time::Duration::from_millis(RENAME_RETRY_DELAY_MS));
                }
            }
            Err(e) => {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(SessionError::Io(e));
            }
        }
    }

    // Retries exhausted — fall back to a direct (non-atomic) write so the user
    // does not lose the session. Warn on stderr per the design.
    let err_msg = last_err
        .as_ref()
        .map_or_else(|| "unknown rename error".to_string(), ToString::to_string);
    eprintln!(
        "warning: atomic rename of session file failed after {RENAME_RETRIES} retries ({err_msg}); \
         falling back to direct write"
    );
    let _ = std::fs::remove_file(&tmp_path);
    std::fs::write(path, bytes)?;
    set_owner_only_perms(path, 0o600)?;
    Ok(())
}

/// Whether a rename failure is worth retrying. Windows AV contention surfaces
/// as `PermissionDenied`; NTFS sharing violations also map here.
fn is_transient_rename_error(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::WouldBlock
    )
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
                .map_err(|e| SessionError::InvalidFormat(format!("{} at {}", e, path.display())))?;
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

/// Rewrite the session file with a new WebSocket URL, preserving `pid`, `port`,
/// and `active_tab_id` from `existing`. Bumps `reconnect_count` and refreshes
/// `timestamp` and `last_reconnect_at`.
///
/// Writes atomically and returns the newly persisted record so callers can use
/// the updated `ws_url` and telemetry fields.
///
/// # Errors
///
/// Returns `SessionError::Io` on I/O failure, or `SessionError::NoHomeDir` if
/// the home directory cannot be determined.
pub fn rewrite_preserving(
    existing: &SessionData,
    new_ws_url: String,
) -> Result<SessionData, SessionError> {
    let path = session_file_path()?;
    rewrite_preserving_to(&path, existing, new_ws_url)
}

/// Testable variant of [`rewrite_preserving`] that writes to a specific path.
///
/// When `new_ws_url` matches the existing URL, returns the existing record
/// unchanged without writing — this avoids inflating `reconnect_count` and
/// rewriting the file when rediscovery returned the same endpoint.
///
/// # Errors
///
/// Returns `SessionError::Io` on I/O failure.
pub fn rewrite_preserving_to(
    path: &std::path::Path,
    existing: &SessionData,
    new_ws_url: String,
) -> Result<SessionData, SessionError> {
    if new_ws_url == existing.ws_url {
        return Ok(existing.clone());
    }
    let now = now_iso8601();
    let updated = SessionData {
        ws_url: new_ws_url,
        port: existing.port,
        pid: existing.pid,
        active_tab_id: existing.active_tab_id.clone(),
        timestamp: now.clone(),
        last_reconnect_at: Some(now),
        reconnect_count: existing.reconnect_count.saturating_add(1),
    };
    write_session_to(path, &updated)?;
    Ok(updated)
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
        assert!(path.ends_with(".agentchrome/session.json"));
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
        let dir = std::env::temp_dir().join("agentchrome-test-session-rt");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        let data = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/abc".into(),
            port: 9222,
            pid: Some(1234),
            active_tab_id: None,
            timestamp: "2026-02-11T12:00:00Z".into(),
            last_reconnect_at: None,
            reconnect_count: 0,
        };

        write_session_to(&path, &data).unwrap();
        let read = read_session_from(&path).unwrap().unwrap();

        assert_eq!(read.ws_url, data.ws_url);
        assert_eq!(read.port, data.port);
        assert_eq!(read.pid, data.pid);
        assert_eq!(read.active_tab_id, data.active_tab_id);
        assert_eq!(read.timestamp, data.timestamp);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_read_round_trip_no_pid() {
        let dir = std::env::temp_dir().join("agentchrome-test-session-nopid");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        let data = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/xyz".into(),
            port: 9222,
            pid: None,
            active_tab_id: None,
            timestamp: "2026-02-11T12:00:00Z".into(),
            last_reconnect_at: None,
            reconnect_count: 0,
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
        let path = std::path::Path::new("/tmp/agentchrome-test-nonexistent/session.json");
        let result = read_session_from(path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_invalid_json_returns_error() {
        let dir = std::env::temp_dir().join("agentchrome-test-session-invalid");
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
        let path = std::path::Path::new("/tmp/agentchrome-test-del-nonexist/session.json");
        assert!(delete_session_from(path).is_ok());
    }

    #[test]
    fn delete_existing_removes_file() {
        let dir = std::env::temp_dir().join("agentchrome-test-session-del");
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
        let dir = std::env::temp_dir().join("agentchrome-test-pid-preserve");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        // Write initial session with PID (simulates --launch)
        let launch = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/aaa".into(),
            port: 9222,
            pid: Some(54321),
            active_tab_id: None,
            timestamp: "2026-02-15T00:00:00Z".into(),
            last_reconnect_at: None,
            reconnect_count: 0,
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
        let dir = std::env::temp_dir().join("agentchrome-test-pid-nopreserve");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        // Write initial session with PID on port 9222
        let launch = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/bbb".into(),
            port: 9222,
            pid: Some(99999),
            active_tab_id: None,
            timestamp: "2026-02-15T00:00:00Z".into(),
            last_reconnect_at: None,
            reconnect_count: 0,
        };
        write_session_to(&path, &launch).unwrap();

        // Simulate auto-discover on DIFFERENT port (pid: None)
        let pid = resolve_pid(&path, None, 9333);
        assert_eq!(pid, None, "PID should NOT be carried from a different port");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pid_not_injected_when_no_prior_session() {
        let dir = std::env::temp_dir().join("agentchrome-test-pid-noinject");
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
        let dir = std::env::temp_dir().join("agentchrome-test-pid-priority");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        // Write existing session with PID
        let existing = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/ccc".into(),
            port: 9222,
            pid: Some(11111),
            active_tab_id: None,
            timestamp: "2026-02-15T00:00:00Z".into(),
            last_reconnect_at: None,
            reconnect_count: 0,
        };
        write_session_to(&path, &existing).unwrap();

        // Incoming ConnectionInfo has its own PID (e.g. new --launch)
        let pid = resolve_pid(&path, Some(22222), 9222);
        assert_eq!(pid, Some(22222), "Incoming PID should take priority");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_read_round_trip_with_active_tab_id() {
        let dir = std::env::temp_dir().join("agentchrome-test-session-active-tab");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        let data = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/tab".into(),
            port: 9222,
            pid: Some(1234),
            active_tab_id: Some("ABCDEF123456".into()),
            timestamp: "2026-02-17T12:00:00Z".into(),
            last_reconnect_at: None,
            reconnect_count: 0,
        };

        write_session_to(&path, &data).unwrap();
        let read = read_session_from(&path).unwrap().unwrap();

        assert_eq!(read.active_tab_id, Some("ABCDEF123456".into()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn active_tab_id_skipped_when_none() {
        let dir = std::env::temp_dir().join("agentchrome-test-session-no-active-tab");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        let data = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/tab".into(),
            port: 9222,
            pid: None,
            active_tab_id: None,
            timestamp: "2026-02-17T12:00:00Z".into(),
            last_reconnect_at: None,
            reconnect_count: 0,
        };

        write_session_to(&path, &data).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(
            !contents.contains("active_tab_id"),
            "active_tab_id should be skipped when None"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn old_session_without_active_tab_id_deserializes() {
        let dir = std::env::temp_dir().join("agentchrome-test-session-compat");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.json");

        // Simulate an old session file that doesn't have active_tab_id
        let old_json = r#"{
            "ws_url": "ws://127.0.0.1:9222/devtools/browser/old",
            "port": 9222,
            "pid": 5678,
            "timestamp": "2026-02-17T12:00:00Z"
        }"#;
        std::fs::write(&path, old_json).unwrap();

        let read = read_session_from(&path).unwrap().unwrap();
        assert_eq!(read.active_tab_id, None);
        assert_eq!(read.pid, Some(5678));
        // Legacy files without reconnect telemetry deserialize with defaults.
        assert_eq!(read.last_reconnect_at, None);
        assert_eq!(read.reconnect_count, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_session_without_reconnect_fields_deserializes() {
        let dir = std::env::temp_dir().join("agentchrome-test-session-legacy-185");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.json");

        let legacy_json = r#"{
            "ws_url": "ws://127.0.0.1:9222/devtools/browser/legacy",
            "port": 9222,
            "pid": 4242,
            "active_tab_id": "TAB1",
            "timestamp": "2026-04-18T00:00:00Z"
        }"#;
        std::fs::write(&path, legacy_json).unwrap();

        let read = read_session_from(&path).unwrap().unwrap();
        assert_eq!(read.pid, Some(4242));
        assert_eq!(read.active_tab_id.as_deref(), Some("TAB1"));
        assert_eq!(read.last_reconnect_at, None);
        assert_eq!(read.reconnect_count, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rewrite_preserving_keeps_pid_and_bumps_count() {
        let dir = std::env::temp_dir().join("agentchrome-test-rewrite-preserving");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        let original = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/OLD".into(),
            port: 9222,
            pid: Some(12_345),
            active_tab_id: Some("TAB-A".into()),
            timestamp: "2026-04-18T00:00:00Z".into(),
            last_reconnect_at: None,
            reconnect_count: 2,
        };
        write_session_to(&path, &original).unwrap();

        let updated = rewrite_preserving_to(
            &path,
            &original,
            "ws://127.0.0.1:9222/devtools/browser/NEW".into(),
        )
        .unwrap();

        assert_eq!(updated.ws_url, "ws://127.0.0.1:9222/devtools/browser/NEW");
        assert_eq!(updated.port, 9222);
        assert_eq!(updated.pid, Some(12_345));
        assert_eq!(updated.active_tab_id.as_deref(), Some("TAB-A"));
        assert_eq!(updated.reconnect_count, 3);
        assert!(updated.last_reconnect_at.is_some());

        // The persisted file matches the returned record
        let on_disk = read_session_from(&path).unwrap().unwrap();
        assert_eq!(on_disk.ws_url, updated.ws_url);
        assert_eq!(on_disk.pid, Some(12_345));
        assert_eq!(on_disk.reconnect_count, 3);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn session_error_display() {
        let diag = "HOME env var is unset or invalid".to_string();
        assert_eq!(
            SessionError::NoHomeDir(diag.clone()).to_string(),
            format!("could not determine home directory ({diag})")
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

    #[test]
    fn windows_home_chain_prefers_userprofile() {
        let env = |k: &str| match k {
            "USERPROFILE" => Some("C:\\Users\\rich".to_string()),
            "HOMEDRIVE" => Some("C:".to_string()),
            "HOMEPATH" => Some("\\Users\\other".to_string()),
            _ => None,
        };
        let home = windows_home_chain(&env).unwrap();
        assert_eq!(home, PathBuf::from("C:\\Users\\rich"));
    }

    #[test]
    fn windows_home_chain_falls_back_to_homedrive_homepath() {
        let env = |k: &str| match k {
            "HOMEDRIVE" => Some("D:".to_string()),
            "HOMEPATH" => Some("\\Users\\fallback".to_string()),
            _ => None,
        };
        let home = windows_home_chain(&env).unwrap();
        assert_eq!(home, PathBuf::from("D:\\Users\\fallback"));
    }

    #[test]
    fn windows_home_chain_reports_diagnostic_when_unset() {
        let env = |_k: &str| None;
        let err = windows_home_chain(&env).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("USERPROFILE"),
            "diagnostic must name USERPROFILE: {msg}"
        );
        assert!(
            msg.contains("HOMEDRIVE"),
            "diagnostic must name HOMEDRIVE: {msg}"
        );
        assert!(
            msg.contains("HOMEPATH"),
            "diagnostic must name HOMEPATH: {msg}"
        );
    }

    #[test]
    fn read_invalid_json_error_includes_path() {
        let dir = std::env::temp_dir().join("agentchrome-test-read-err-path");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.json");
        std::fs::write(&path, "{ not json").unwrap();

        let err = read_session_from(&path).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains(&path.display().to_string()),
            "parse error must include resolved file path: {msg}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_read_roundtrip_with_non_ascii_and_spaces_in_path() {
        let dir = std::env::temp_dir().join("agentchrome test Björn O'Malley");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        let data = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/unicode".into(),
            port: 9222,
            pid: Some(4242),
            active_tab_id: Some("ünícødé-tab".into()),
            timestamp: "2026-04-21T00:00:00Z".into(),
            last_reconnect_at: None,
            reconnect_count: 0,
        };

        write_session_to(&path, &data).unwrap();
        let read = read_session_from(&path).unwrap().unwrap();
        assert_eq!(read.ws_url, data.ws_url);
        assert_eq!(read.active_tab_id.as_deref(), Some("ünícødé-tab"));

        let tmp = path.with_extension("json.tmp");
        assert!(
            !tmp.exists(),
            "temp file must not remain after atomic write"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_recovers_when_final_path_has_preexisting_file() {
        // Sanity: rename over an existing file is supported on all platforms we
        // target. This guards against a regression where the retry loop would
        // incorrectly classify EEXIST as a transient error.
        let dir = std::env::temp_dir().join("agentchrome-test-write-over-existing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.json");
        std::fs::write(&path, "stale contents").unwrap();

        let data = SessionData {
            ws_url: "ws://127.0.0.1:9222/devtools/browser/over".into(),
            port: 9222,
            pid: None,
            active_tab_id: None,
            timestamp: "2026-04-21T00:00:00Z".into(),
            last_reconnect_at: None,
            reconnect_count: 0,
        };
        write_session_to(&path, &data).unwrap();
        let read = read_session_from(&path).unwrap().unwrap();
        assert_eq!(read.ws_url, data.ws_url);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
