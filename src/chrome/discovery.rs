use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde::Deserialize;

use super::ChromeError;
use super::platform;

/// Browser version information returned by `/json/version`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct BrowserVersion {
    /// The browser name and version (e.g. "Chrome/120.0.6099.71").
    #[serde(rename = "Browser")]
    pub browser: String,

    /// The CDP protocol version (e.g. "1.3").
    #[serde(rename = "Protocol-Version")]
    pub protocol_version: String,

    /// The browser-level WebSocket debugger URL.
    #[serde(rename = "webSocketDebuggerUrl")]
    pub ws_debugger_url: String,
}

/// Information about a single debuggable target (tab, service worker, etc.).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct TargetInfo {
    /// Unique target identifier.
    pub id: String,

    /// Target type (e.g. "page", "`background_page`").
    #[serde(rename = "type")]
    pub target_type: String,

    /// Page title.
    pub title: String,

    /// Current URL.
    pub url: String,

    /// WebSocket URL to debug this specific target.
    #[serde(rename = "webSocketDebuggerUrl")]
    pub ws_debugger_url: Option<String>,
}

/// Query Chrome's `/json/version` endpoint.
///
/// # Errors
///
/// Returns `ChromeError::HttpError` on connection failure or `ChromeError::ParseError`
/// if the response cannot be deserialized.
pub async fn query_version(host: &str, port: u16) -> Result<BrowserVersion, ChromeError> {
    let body = http_get(host, port, "/json/version").await?;
    serde_json::from_str(&body).map_err(|e| ChromeError::ParseError(e.to_string()))
}

/// Query Chrome's `/json/list` endpoint for debuggable targets.
///
/// # Errors
///
/// Returns `ChromeError::HttpError` on connection failure or `ChromeError::ParseError`
/// if the response cannot be deserialized.
#[allow(dead_code)]
pub async fn query_targets(host: &str, port: u16) -> Result<Vec<TargetInfo>, ChromeError> {
    let body = http_get(host, port, "/json/list").await?;
    serde_json::from_str(&body).map_err(|e| ChromeError::ParseError(e.to_string()))
}

/// Read the `DevToolsActivePort` file from the default user data directory.
///
/// Returns `(port, ws_path)` on success.
///
/// # Errors
///
/// Returns `ChromeError::NoActivePort` if the file is missing or unreadable,
/// or `ChromeError::ParseError` if the contents are malformed.
pub fn read_devtools_active_port() -> Result<(u16, String), ChromeError> {
    let data_dir = platform::default_user_data_dir().ok_or(ChromeError::NoActivePort)?;
    read_devtools_active_port_from(&data_dir)
}

/// Read the `DevToolsActivePort` file from a specific directory.
///
/// This is the parameterized version of [`read_devtools_active_port`] that accepts
/// an explicit data directory, enabling unit testing without relying on
/// platform-specific defaults.
///
/// # Errors
///
/// Returns `ChromeError::NoActivePort` if the file is missing or unreadable,
/// or `ChromeError::ParseError` if the contents are malformed.
pub fn read_devtools_active_port_from(
    data_dir: &std::path::Path,
) -> Result<(u16, String), ChromeError> {
    let path = data_dir.join("DevToolsActivePort");
    let contents = std::fs::read_to_string(&path).map_err(|_| ChromeError::NoActivePort)?;
    parse_devtools_active_port(&contents)
}

/// Parse the contents of a `DevToolsActivePort` file.
///
/// The file has two lines: a port number and a WebSocket path.
fn parse_devtools_active_port(contents: &str) -> Result<(u16, String), ChromeError> {
    let mut lines = contents.lines();
    let port_str = lines.next().ok_or(ChromeError::NoActivePort)?;
    let port: u16 = port_str.trim().parse().map_err(|_| {
        ChromeError::ParseError(format!("invalid port in DevToolsActivePort: {port_str}"))
    })?;
    let ws_path = lines
        .next()
        .ok_or(ChromeError::NoActivePort)?
        .trim()
        .to_string();
    Ok((port, ws_path))
}

/// Attempt to discover a running Chrome instance.
///
/// Tries `DevToolsActivePort` file first, then falls back to the given host/port.
/// Returns the WebSocket URL and port on success.
///
/// # Errors
///
/// Returns `ChromeError::NotRunning` if no Chrome instance can be discovered.
pub async fn discover_chrome(host: &str, port: u16) -> Result<(String, u16), ChromeError> {
    // Try DevToolsActivePort file first
    if let Ok((file_port, _ws_path)) = read_devtools_active_port() {
        if let Ok(version) = query_version("127.0.0.1", file_port).await {
            return Ok((version.ws_debugger_url, file_port));
        }
    }

    // Fall back to the explicitly given host/port
    query_version(host, port)
        .await
        .map(|version| (version.ws_debugger_url, port))
        .map_err(|e| ChromeError::NotRunning(format!("discovery failed on {host}:{port}: {e}")))
}

/// Perform a simple HTTP GET request using blocking I/O in a `spawn_blocking` context.
async fn http_get(host: &str, port: u16, path: &str) -> Result<String, ChromeError> {
    let addr = format!("{host}:{port}");
    let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");

    let (addr_clone, request_clone) = (addr.clone(), request);
    tokio::task::spawn_blocking(move || {
        let mut stream = TcpStream::connect_timeout(
            &addr_clone
                .parse()
                .map_err(|e| ChromeError::HttpError(format!("invalid address: {e}")))?,
            Duration::from_secs(2),
        )
        .map_err(|e| ChromeError::HttpError(format!("connection failed to {addr_clone}: {e}")))?;

        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();

        stream
            .write_all(request_clone.as_bytes())
            .map_err(|e| ChromeError::HttpError(format!("write failed: {e}")))?;

        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .map_err(|e| ChromeError::HttpError(format!("read failed: {e}")))?;

        // Check for HTTP 200 status
        let status_line = response
            .lines()
            .next()
            .ok_or_else(|| ChromeError::HttpError("empty response".into()))?;
        if !status_line.contains("200") {
            return Err(ChromeError::HttpError(format!(
                "unexpected HTTP status: {status_line}"
            )));
        }

        // Extract body after \r\n\r\n
        let body = response
            .split_once("\r\n\r\n")
            .map(|(_, b)| b.to_string())
            .ok_or_else(|| ChromeError::HttpError("malformed HTTP response".into()))?;

        Ok(body)
    })
    .await
    .map_err(|e| ChromeError::HttpError(format!("task join failed: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_browser_version() {
        let json = r#"{
            "Browser": "Chrome/120.0.6099.71",
            "Protocol-Version": "1.3",
            "User-Agent": "Mozilla/5.0",
            "V8-Version": "12.0.267.8",
            "WebKit-Version": "537.36",
            "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/browser/abc-123"
        }"#;
        let v: BrowserVersion = serde_json::from_str(json).unwrap();
        assert_eq!(v.browser, "Chrome/120.0.6099.71");
        assert_eq!(v.protocol_version, "1.3");
        assert!(v.ws_debugger_url.contains("ws://"));
    }

    #[test]
    fn parse_target_info() {
        let json = r#"[{
            "description": "",
            "devtoolsFrontendUrl": "/devtools/inspector.html",
            "id": "ABCDEF",
            "title": "New Tab",
            "type": "page",
            "url": "chrome://newtab/",
            "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/ABCDEF"
        }]"#;
        let targets: Vec<TargetInfo> = serde_json::from_str(json).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, "ABCDEF");
        assert_eq!(targets[0].target_type, "page");
        assert_eq!(targets[0].title, "New Tab");
        assert!(targets[0].ws_debugger_url.is_some());
    }

    #[test]
    fn parse_devtools_active_port_valid() {
        let contents = "9222\n/devtools/browser/abc-123\n";
        let (port, path) = parse_devtools_active_port(contents).unwrap();
        assert_eq!(port, 9222);
        assert_eq!(path, "/devtools/browser/abc-123");
    }

    #[test]
    fn parse_devtools_active_port_empty() {
        let result = parse_devtools_active_port("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_devtools_active_port_invalid_port() {
        let result = parse_devtools_active_port("notaport\n/ws/path\n");
        assert!(result.is_err());
    }

    #[test]
    fn read_devtools_active_port_from_dir() {
        let dir = std::env::temp_dir().join("chrome-cli-test-devtools-port");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("DevToolsActivePort");
        std::fs::write(&file, "9333\n/devtools/browser/xyz-789\n").unwrap();

        let (port, path) = read_devtools_active_port_from(&dir).unwrap();
        assert_eq!(port, 9333);
        assert_eq!(path, "/devtools/browser/xyz-789");

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_devtools_active_port_from_missing_dir() {
        let dir = std::path::Path::new("/nonexistent/chrome-cli-test");
        let result = read_devtools_active_port_from(dir);
        assert!(result.is_err());
    }
}
