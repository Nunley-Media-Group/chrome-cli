use std::path::PathBuf;

use super::ChromeError;

/// Chrome release channel.
#[derive(Debug, Clone, Copy)]
pub enum Channel {
    Stable,
    Canary,
    Beta,
    Dev,
}

/// Result of a process liveness probe.
///
/// Used to classify connection losses as `chrome_terminated` (definitively
/// dead) versus `transient` (alive or unknown).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeResult {
    /// The process exists and accepts signals from the current user.
    Alive,
    /// The process does not exist (`ESRCH` on Unix, missing in `tasklist` on Windows).
    Dead,
    /// We could not determine liveness (e.g. permission denied, OS error).
    Unknown,
}

/// Probe whether a process with the given PID is currently alive.
///
/// On Unix, sends signal `0` via `libc::kill`, which performs an existence/permission
/// check without actually delivering a signal. On Windows, shells out to `tasklist`.
/// When the result is ambiguous we return [`ProbeResult::Unknown`], so callers can
/// fall back to the conservative "transient" classification.
#[must_use]
pub fn is_process_alive(pid: u32) -> ProbeResult {
    #[cfg(unix)]
    {
        // PID values fit in i32 on all supported platforms.
        #[allow(clippy::cast_possible_wrap)]
        let pid_i32 = pid as i32;
        // SAFETY: signal 0 is the documented null-signal; it never delivers a
        // signal, only validates existence + permission for the target PID.
        let rc = unsafe { libc::kill(pid_i32, 0) };
        if rc == 0 {
            return ProbeResult::Alive;
        }
        // SAFETY: libc::__errno_location / __error are sound to read after a
        // failing libc call. std::io::Error::last_os_error wraps that for us.
        let err = std::io::Error::last_os_error();
        match err.raw_os_error() {
            Some(libc::ESRCH) => ProbeResult::Dead,
            Some(libc::EPERM) => ProbeResult::Alive, // EPERM means the process exists
            _ => ProbeResult::Unknown,
        }
    }
    #[cfg(windows)]
    {
        let output = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output();
        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                // tasklist prints "INFO: No tasks are running..." when no match;
                // otherwise the PID appears in the output.
                if stdout.contains(&pid.to_string()) {
                    ProbeResult::Alive
                } else {
                    ProbeResult::Dead
                }
            }
            _ => ProbeResult::Unknown,
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        ProbeResult::Unknown
    }
}

/// Find a Chrome executable for the given release channel.
///
/// Checks the `CHROME_PATH` environment variable first, then falls back
/// to platform-specific well-known paths.
///
/// # Errors
///
/// Returns `ChromeError::NotFound` if no Chrome executable can be located.
pub fn find_chrome_executable(channel: Channel) -> Result<PathBuf, ChromeError> {
    let env_override = std::env::var("CHROME_PATH").ok().map(PathBuf::from);
    find_chrome_from(channel, env_override.as_deref())
}

/// Find a Chrome executable, optionally checking an explicit override path first.
///
/// This is the testable core of [`find_chrome_executable`]: it accepts the
/// environment override as a parameter instead of reading `CHROME_PATH` directly.
fn find_chrome_from(
    channel: Channel,
    env_override: Option<&std::path::Path>,
) -> Result<PathBuf, ChromeError> {
    if let Some(p) = env_override
        && p.exists()
    {
        return Ok(p.to_path_buf());
    }

    for candidate in chrome_candidates(channel) {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(ChromeError::NotFound(format!(
        "could not find Chrome ({channel:?} channel). Use --chrome-path to specify the executable"
    )))
}

/// Returns the default Chrome user data directory for the current platform.
#[must_use]
pub fn default_user_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        home_dir().map(|h| h.join("Library/Application Support/Google/Chrome"))
    }

    #[cfg(target_os = "linux")]
    {
        home_dir().map(|h| h.join(".config/google-chrome"))
    }

    #[cfg(target_os = "windows")]
    {
        std::env::var("LOCALAPPDATA").ok().map(|d| {
            PathBuf::from(d)
                .join("Google")
                .join("Chrome")
                .join("User Data")
        })
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

/// Returns all candidate executable paths for the given channel on the current platform.
fn chrome_candidates(channel: Channel) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        macos_candidates(channel)
    }

    #[cfg(target_os = "linux")]
    {
        linux_candidates(channel)
    }

    #[cfg(target_os = "windows")]
    {
        windows_candidates(channel)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = channel;
        vec![]
    }
}

#[cfg(target_os = "macos")]
fn macos_candidates(channel: Channel) -> Vec<PathBuf> {
    match channel {
        Channel::Stable => vec![
            PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
            PathBuf::from("/Applications/Chromium.app/Contents/MacOS/Chromium"),
        ],
        Channel::Canary => vec![PathBuf::from(
            "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
        )],
        Channel::Beta => vec![PathBuf::from(
            "/Applications/Google Chrome Beta.app/Contents/MacOS/Google Chrome Beta",
        )],
        Channel::Dev => vec![PathBuf::from(
            "/Applications/Google Chrome Dev.app/Contents/MacOS/Google Chrome Dev",
        )],
    }
}

#[cfg(target_os = "linux")]
fn linux_candidates(channel: Channel) -> Vec<PathBuf> {
    let path_dirs: Vec<PathBuf> = std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .map(PathBuf::from)
        .collect();

    let names: &[&str] = match channel {
        Channel::Stable => &[
            "google-chrome",
            "google-chrome-stable",
            "chromium-browser",
            "chromium",
        ],
        Channel::Canary => &["google-chrome-canary"],
        Channel::Beta => &["google-chrome-beta"],
        Channel::Dev => &["google-chrome-unstable"],
    };

    let mut candidates = Vec::new();
    for name in names {
        for dir in &path_dirs {
            candidates.push(dir.join(name));
        }
    }
    candidates
}

#[cfg(target_os = "windows")]
fn windows_candidates(channel: Channel) -> Vec<PathBuf> {
    let program_files = std::env::var("ProgramFiles").unwrap_or_default();
    let program_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();

    match channel {
        Channel::Stable => vec![
            PathBuf::from(&program_files).join("Google/Chrome/Application/chrome.exe"),
            PathBuf::from(&program_files_x86).join("Google/Chrome/Application/chrome.exe"),
        ],
        Channel::Canary => {
            vec![PathBuf::from(&local_app_data).join("Google/Chrome SxS/Application/chrome.exe")]
        }
        Channel::Beta => vec![
            PathBuf::from(&program_files).join("Google/Chrome Beta/Application/chrome.exe"),
            PathBuf::from(&program_files_x86).join("Google/Chrome Beta/Application/chrome.exe"),
        ],
        Channel::Dev => vec![
            PathBuf::from(&program_files).join("Google/Chrome Dev/Application/chrome.exe"),
            PathBuf::from(&program_files_x86).join("Google/Chrome Dev/Application/chrome.exe"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_user_data_dir_returns_some() {
        // On CI or dev machines, home dir should exist
        let dir = default_user_data_dir();
        assert!(dir.is_some(), "Expected a default user data directory");
    }

    #[test]
    fn chrome_candidates_is_not_empty() {
        let candidates = chrome_candidates(Channel::Stable);
        assert!(
            !candidates.is_empty(),
            "Expected at least one candidate path"
        );
    }

    #[test]
    fn chrome_path_override_existing_file() {
        // Use the test binary itself as a known-existing file
        let exe = std::env::current_exe().unwrap();
        let result = find_chrome_from(Channel::Stable, Some(&exe));
        assert_eq!(result.unwrap(), exe);
    }

    #[test]
    fn is_process_alive_self_is_alive() {
        let me = std::process::id();
        assert_eq!(is_process_alive(me), ProbeResult::Alive);
    }

    #[cfg(unix)]
    #[test]
    fn is_process_alive_high_pid_is_dead() {
        // u32::MAX wraps to -1 as i32, which targets a process group instead
        // of a single process; use a high but valid-i32 PID so libc::kill
        // returns ESRCH.
        assert_eq!(is_process_alive(999_999_999), ProbeResult::Dead);
    }

    #[test]
    fn chrome_path_override_nonexistent_is_skipped() {
        let fake = std::path::Path::new("/nonexistent/chrome-test-binary");
        let result = find_chrome_from(Channel::Stable, Some(fake));
        // Should fall through to candidates (which may or may not find Chrome)
        // — the point is that the nonexistent override is skipped, not returned.
        if let Ok(path) = &result {
            assert_ne!(path.as_path(), fake);
        }
    }
}
