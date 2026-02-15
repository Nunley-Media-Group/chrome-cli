use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::ChromeError;
use super::discovery::query_version;

/// Configuration for launching a Chrome process.
pub struct LaunchConfig {
    /// Path to the Chrome executable.
    pub executable: PathBuf,
    /// Port for Chrome's remote debugging protocol.
    pub port: u16,
    /// Whether to launch in headless mode.
    pub headless: bool,
    /// Additional command-line arguments for Chrome.
    pub extra_args: Vec<String>,
    /// User data directory. If `None`, a temporary directory is created.
    pub user_data_dir: Option<PathBuf>,
}

/// A handle to a running Chrome process.
pub struct ChromeProcess {
    child: Option<std::process::Child>,
    port: u16,
    temp_dir: Option<TempDir>,
}

/// A temporary directory that is removed on drop.
struct TempDir {
    path: PathBuf,
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

impl ChromeProcess {
    /// Returns the PID of the Chrome process.
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.child.as_ref().map_or(0, std::process::Child::id)
    }

    /// Returns the remote debugging port.
    #[must_use]
    #[allow(dead_code)]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Kill the Chrome process and clean up.
    pub fn kill(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Detach the Chrome process so it keeps running after this handle is dropped.
    ///
    /// Returns `(pid, port)`. The caller is responsible for the process lifetime.
    #[must_use]
    pub fn detach(mut self) -> (u32, u16) {
        let pid = self.pid();
        let port = self.port;
        // Take ownership to prevent Drop from killing the process
        self.child = None;
        // Prevent temp dir cleanup — Chrome still needs it
        self.temp_dir = None;
        (pid, port)
    }
}

impl Drop for ChromeProcess {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Generate a random hex suffix for temporary directory names.
///
/// Reads 8 bytes from `/dev/urandom` on Unix, falling back to a PID + address
/// combination when that is not available.
fn random_suffix() -> String {
    use std::io::Read;
    let mut buf = [0u8; 8];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        if f.read_exact(&mut buf).is_ok() {
            return hex_encode(&buf);
        }
    }
    // Fallback: combine PID and a stack address for uniqueness
    let pid = std::process::id();
    let addr = &raw const buf as usize;
    format!("{pid:x}-{addr:x}")
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Find an available TCP port on localhost.
///
/// # Errors
///
/// Returns `ChromeError::LaunchFailed` if binding fails.
pub fn find_available_port() -> Result<u16, ChromeError> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| {
        ChromeError::LaunchFailed(format!("could not bind to find a free port: {e}"))
    })?;
    let port = listener
        .local_addr()
        .map_err(|e| ChromeError::LaunchFailed(format!("could not get local address: {e}")))?
        .port();
    drop(listener);
    Ok(port)
}

/// Build the Chrome command-line arguments from a launch configuration.
fn build_chrome_args(config: &LaunchConfig, data_dir: &Path) -> Vec<String> {
    let mut args = vec![
        format!("--remote-debugging-port={}", config.port),
        format!("--user-data-dir={}", data_dir.display()),
        "--no-first-run".to_string(),
        "--no-default-browser-check".to_string(),
        "--enable-automation".to_string(),
    ];

    if config.headless {
        args.push("--headless=new".to_string());
    }

    for arg in &config.extra_args {
        args.push(arg.clone());
    }

    args
}

/// Launch a Chrome process with the given configuration.
///
/// Polls the Chrome debug endpoint until it responds or the timeout expires.
///
/// # Errors
///
/// Returns `ChromeError::LaunchFailed` if the process cannot be spawned,
/// or `ChromeError::StartupTimeout` if Chrome does not become ready in time.
pub async fn launch_chrome(
    config: LaunchConfig,
    timeout: Duration,
) -> Result<ChromeProcess, ChromeError> {
    let (data_dir, temp_dir) = if let Some(ref dir) = config.user_data_dir {
        (dir.clone(), None)
    } else {
        let dir = std::env::temp_dir().join(format!("chrome-cli-{}", random_suffix()));
        std::fs::create_dir_all(&dir)?;
        let td = TempDir { path: dir.clone() };
        (dir, Some(td))
    };

    let args = build_chrome_args(&config, &data_dir);

    let mut cmd = Command::new(&config.executable);
    for arg in &args {
        cmd.arg(arg);
    }

    cmd.stdout(Stdio::null()).stderr(Stdio::null());

    let child = cmd.spawn().map_err(|e| {
        ChromeError::LaunchFailed(format!(
            "failed to spawn {}: {e}",
            config.executable.display()
        ))
    })?;

    let mut process = ChromeProcess {
        child: Some(child),
        port: config.port,
        temp_dir,
    };

    // Poll until Chrome is ready or timeout
    let start = tokio::time::Instant::now();
    let poll_interval = Duration::from_millis(100);

    loop {
        if start.elapsed() > timeout {
            // Kill the process since we're giving up
            process.kill();
            return Err(ChromeError::StartupTimeout { port: config.port });
        }

        // Check if the child has exited unexpectedly
        if let Some(child) = process.child.as_mut() {
            if let Ok(Some(status)) = child.try_wait() {
                return Err(ChromeError::LaunchFailed(format!(
                    "Chrome exited with status {status} before becoming ready"
                )));
            }
        }

        if query_version("127.0.0.1", config.port).await.is_ok() {
            return Ok(process);
        }

        tokio::time::sleep(poll_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_available_port_returns_valid_port() {
        let port = find_available_port().unwrap();
        assert!(port > 0, "Expected a positive port number, got {port}");
    }

    fn default_launch_config(port: u16) -> LaunchConfig {
        LaunchConfig {
            executable: PathBuf::from("/usr/bin/chrome"),
            port,
            headless: false,
            extra_args: vec![],
            user_data_dir: None,
        }
    }

    #[test]
    fn automation_flag_is_included_on_launch() {
        let config = default_launch_config(9222);
        let data_dir = PathBuf::from("/tmp/test-data");
        let args = build_chrome_args(&config, &data_dir);
        assert!(
            args.iter().any(|a| a == "--enable-automation"),
            "Expected --enable-automation in args: {args:?}"
        );
    }

    #[test]
    fn headless_mode_includes_automation_flag() {
        let mut config = default_launch_config(9222);
        config.headless = true;
        let data_dir = PathBuf::from("/tmp/test-data");
        let args = build_chrome_args(&config, &data_dir);
        assert!(
            args.iter().any(|a| a == "--enable-automation"),
            "Expected --enable-automation in args: {args:?}"
        );
        assert!(
            args.iter().any(|a| a == "--headless=new"),
            "Expected --headless=new in args: {args:?}"
        );
    }

    #[test]
    fn extra_args_do_not_conflict_with_automation_flag() {
        let mut config = default_launch_config(9222);
        config.extra_args = vec!["--enable-automation".to_string()];
        let data_dir = PathBuf::from("/tmp/test-data");
        let args = build_chrome_args(&config, &data_dir);
        // Should contain --enable-automation (at least once) without error
        assert!(
            args.iter().any(|a| a == "--enable-automation"),
            "Expected --enable-automation in args: {args:?}"
        );
    }

    #[test]
    fn temp_dir_cleanup_on_drop() {
        let path = std::env::temp_dir().join("chrome-cli-test-cleanup");
        std::fs::create_dir_all(&path).unwrap();
        assert!(path.exists());

        let td = TempDir { path: path.clone() };
        drop(td);

        assert!(!path.exists(), "TempDir should have been cleaned up");
    }
}
