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

/// Check whether `buf` contains a complete HTTP response (headers + full body per Content-Length).
fn is_http_response_complete(buf: &[u8]) -> bool {
    let Some(header_end) = find_header_end(buf) else {
        return false;
    };
    let body_start = header_end + 4; // skip past \r\n\r\n
    let headers = &buf[..header_end];
    match parse_content_length(headers) {
        Some(cl) => buf.len() >= body_start + cl,
        None => true, // no Content-Length; headers are complete, assume body is too
    }
}

/// Find the byte offset of `\r\n\r\n` in `buf`, returning the position of the first `\r`.
fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Parse `Content-Length` from raw header bytes (case-insensitive).
fn parse_content_length(headers: &[u8]) -> Option<usize> {
    let header_str = std::str::from_utf8(headers).ok()?;
    for line in header_str.lines() {
        if let Some((key, value)) = line.split_once(':') {
            if key.trim().eq_ignore_ascii_case("content-length") {
                return value.trim().parse().ok();
            }
        }
    }
    None
}

/// Parse a raw HTTP response buffer into the body string.
///
/// Validates the status line is 200 OK and extracts the body after headers.
fn parse_http_response(buf: &[u8]) -> Result<String, ChromeError> {
    let header_end = find_header_end(buf)
        .ok_or_else(|| ChromeError::HttpError("malformed HTTP response".into()))?;
    let body_start = header_end + 4;

    let headers = std::str::from_utf8(&buf[..header_end])
        .map_err(|e| ChromeError::HttpError(format!("invalid UTF-8 in headers: {e}")))?;

    // Check for HTTP 200 status
    let status_line = headers
        .lines()
        .next()
        .ok_or_else(|| ChromeError::HttpError("empty response".into()))?;
    if !status_line.contains(" 200 ") {
        return Err(ChromeError::HttpError(format!(
            "unexpected HTTP status: {status_line}"
        )));
    }

    // Extract body: use Content-Length if available, otherwise take everything after headers
    let body_bytes = if let Some(cl) = parse_content_length(&buf[..header_end]) {
        let end = (body_start + cl).min(buf.len());
        &buf[body_start..end]
    } else {
        &buf[body_start..]
    };

    String::from_utf8(body_bytes.to_vec())
        .map_err(|e| ChromeError::HttpError(format!("invalid UTF-8 in body: {e}")))
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

        // Read response incrementally, stopping once we have Content-Length bytes
        // of body. This avoids blocking on EOF when Chrome keeps the connection open.
        let mut buf = Vec::with_capacity(4096);
        let mut tmp = [0u8; 4096];
        loop {
            match stream.read(&mut tmp) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                    if is_http_response_complete(&buf) {
                        break;
                    }
                }
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    // Timeout/EAGAIN: if we already have a complete response, use it
                    if is_http_response_complete(&buf) {
                        break;
                    }
                    return Err(ChromeError::HttpError(format!("read timed out: {e}")));
                }
                Err(e) => {
                    return Err(ChromeError::HttpError(format!("read failed: {e}")));
                }
            }
        }

        parse_http_response(&buf)
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

    #[test]
    fn parse_http_response_with_content_length() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, world!";
        let body = parse_http_response(raw).unwrap();
        assert_eq!(body, "Hello, world!");
    }

    #[test]
    fn parse_http_response_without_content_length() {
        let raw = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n{\"ok\":true}";
        let body = parse_http_response(raw).unwrap();
        assert_eq!(body, "{\"ok\":true}");
    }

    #[test]
    fn parse_http_response_content_length_zero() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        let body = parse_http_response(raw).unwrap();
        assert_eq!(body, "");
    }

    #[test]
    fn parse_http_response_malformed_no_separator() {
        let raw = b"HTTP/1.1 200 OK\nno double crlf here";
        let result = parse_http_response(raw);
        assert!(result.is_err());
    }

    #[test]
    fn parse_http_response_non_200_status() {
        let raw = b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
        let result = parse_http_response(raw);
        assert!(result.is_err());
    }

    #[test]
    fn is_http_response_complete_with_content_length() {
        let partial = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHe";
        assert!(!is_http_response_complete(partial));

        let complete = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello";
        assert!(is_http_response_complete(complete));
    }

    #[test]
    fn is_http_response_complete_no_headers_yet() {
        assert!(!is_http_response_complete(b"HTTP/1.1 200 OK\r\n"));
    }

    #[test]
    fn is_http_response_complete_without_content_length() {
        let response = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nbody";
        assert!(is_http_response_complete(response));
    }
}
