use std::collections::HashSet;
use std::time::Duration;

use crate::cdp::{CdpError, CdpEvent, CdpSession};
use crate::chrome::{TargetInfo, discover_chrome, query_targets, query_version};
use crate::error::AppError;
use crate::session;

/// Default Chrome `DevTools` Protocol port.
pub const DEFAULT_CDP_PORT: u16 = 9222;

/// Resolved connection info ready for use by a command.
#[derive(Debug)]
pub struct ResolvedConnection {
    pub ws_url: String,
    pub host: String,
    pub port: u16,
}

/// Health-check a connection by querying `/json/version`.
///
/// Returns `Ok(())` if Chrome responds, or `Err(AppError::stale_session())` if not.
///
/// # Errors
///
/// Returns `AppError` with `ConnectionError` exit code if Chrome is unreachable.
pub async fn health_check(host: &str, port: u16) -> Result<(), AppError> {
    query_version(host, port)
        .await
        .map(|_| ())
        .map_err(|_| AppError::stale_session())
}

/// Resolve a Chrome connection using the priority chain:
///
/// 1. Explicit `--ws-url`
/// 2. Explicit `--port` (user provided, not the default)
/// 3. Session file (with health check)
/// 4. Auto-discover (default host:port 9222)
/// 5. Error with suggestion
///
/// # Errors
///
/// Returns `AppError` if no Chrome connection can be resolved.
pub async fn resolve_connection(
    host: &str,
    port: Option<u16>,
    ws_url: Option<&str>,
) -> Result<ResolvedConnection, AppError> {
    let default_port = DEFAULT_CDP_PORT;

    // 1. Explicit --ws-url
    if let Some(ws_url) = ws_url {
        let resolved_port =
            extract_port_from_ws_url(ws_url).unwrap_or(port.unwrap_or(default_port));
        return Ok(ResolvedConnection {
            ws_url: ws_url.to_string(),
            host: host.to_string(),
            port: resolved_port,
        });
    }

    // 2. Explicit --port (user provided) — try only this port, no DevToolsActivePort fallback
    if let Some(explicit_port) = port {
        match query_version(host, explicit_port).await {
            Ok(version) => {
                return Ok(ResolvedConnection {
                    ws_url: version.ws_debugger_url,
                    host: host.to_string(),
                    port: explicit_port,
                });
            }
            Err(_) => return Err(AppError::no_chrome_found()),
        }
    }

    // 3. Session file
    if let Some(session_data) = session::read_session()? {
        health_check(host, session_data.port).await?;
        return Ok(ResolvedConnection {
            ws_url: session_data.ws_url,
            host: host.to_string(),
            port: session_data.port,
        });
    }

    // 4. Auto-discover on default port
    match discover_chrome(host, default_port).await {
        Ok((ws_url, p)) => Ok(ResolvedConnection {
            ws_url,
            host: host.to_string(),
            port: p,
        }),
        Err(_) => Err(AppError::no_chrome_found()),
    }
}

/// Extract port from a WebSocket URL like `ws://host:port/path`.
#[must_use]
pub fn extract_port_from_ws_url(url: &str) -> Option<u16> {
    let without_scheme = url
        .strip_prefix("ws://")
        .or_else(|| url.strip_prefix("wss://"))?;
    let host_port = without_scheme.split('/').next()?;
    let port_str = host_port.rsplit(':').next()?;
    port_str.parse().ok()
}

/// Select a target from a list based on the `--tab` option.
///
/// - `None` → first target with `target_type == "page"`
/// - `Some(value)` → try as numeric index, then as target ID
///
/// This is a pure function for testability.
///
/// # Errors
///
/// Returns `AppError::no_page_targets()` if no page-type target exists,
/// or `AppError::target_not_found()` if the specified tab cannot be matched.
pub fn select_target<'a>(
    targets: &'a [TargetInfo],
    tab: Option<&str>,
) -> Result<&'a TargetInfo, AppError> {
    match tab {
        None => targets
            .iter()
            .find(|t| t.target_type == "page")
            .ok_or_else(AppError::no_page_targets),
        Some(value) => {
            // Try as numeric index first
            if let Ok(index) = value.parse::<usize>() {
                return targets
                    .get(index)
                    .ok_or_else(|| AppError::target_not_found(value));
            }
            // Try as target ID
            targets
                .iter()
                .find(|t| t.id == value)
                .ok_or_else(|| AppError::target_not_found(value))
        }
    }
}

/// Resolve the target tab from the `--tab` option by querying Chrome for targets.
///
/// # Errors
///
/// Returns `AppError` if targets cannot be queried or the specified tab is not found.
pub async fn resolve_target(
    host: &str,
    port: u16,
    tab: Option<&str>,
) -> Result<TargetInfo, AppError> {
    let targets = query_targets(host, port).await?;
    select_target(&targets, tab).cloned()
}

/// Timeout for `Page.enable` during auto-dismiss setup (milliseconds).
///
/// Chrome re-emits `Page.javascriptDialogOpening` to newly-attached sessions
/// when `Page.enable` is sent, but `Page.enable` itself blocks when a dialog
/// is already open. We use a short timeout so auto-dismiss can proceed.
const PAGE_ENABLE_TIMEOUT_MS: u64 = 300;

/// A CDP session wrapper that tracks which domains have been enabled,
/// ensuring each domain is only enabled once (lazy domain enabling).
///
/// This fulfills AC13: "only the required domains are enabled" per command.
#[derive(Debug)]
pub struct ManagedSession {
    session: CdpSession,
    enabled_domains: HashSet<String>,
}

impl ManagedSession {
    /// Wrap a [`CdpSession`] with domain tracking.
    #[must_use]
    pub fn new(session: CdpSession) -> Self {
        Self {
            session,
            enabled_domains: HashSet::new(),
        }
    }

    /// Ensure a CDP domain is enabled. Sends `{domain}.enable` only if
    /// the domain has not already been enabled in this session.
    ///
    /// # Errors
    ///
    /// Returns `CdpError` if the enable command fails.
    pub async fn ensure_domain(&mut self, domain: &str) -> Result<(), CdpError> {
        if self.enabled_domains.contains(domain) {
            return Ok(());
        }
        let method = format!("{domain}.enable");
        self.session.send_command(&method, None).await?;
        self.enabled_domains.insert(domain.to_string());
        Ok(())
    }

    /// Send a command within this session.
    ///
    /// # Errors
    ///
    /// Returns `CdpError` if the command fails.
    pub async fn send_command(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, CdpError> {
        self.session.send_command(method, params).await
    }

    /// Get the underlying session ID.
    #[must_use]
    pub fn session_id(&self) -> &str {
        self.session.session_id()
    }

    /// Subscribe to CDP events matching a method name within this session.
    ///
    /// # Errors
    ///
    /// Returns `CdpError` if the transport task has exited.
    pub async fn subscribe(
        &self,
        method: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<CdpEvent>, CdpError> {
        self.session.subscribe(method).await
    }

    /// Returns the set of currently enabled domains.
    #[must_use]
    pub fn enabled_domains(&self) -> &HashSet<String> {
        &self.enabled_domains
    }

    /// Install dialog interceptor scripts that override `window.alert`,
    /// `window.confirm`, and `window.prompt` to store dialog metadata in a
    /// cookie named `__chrome_cli_dialog` before calling the original function.
    ///
    /// This enables `dialog info` and `dialog handle` to retrieve dialog type,
    /// message, and default value via `Network.getCookies` even when the dialog
    /// was opened before the current CDP session was created.
    ///
    /// This method is best-effort: errors are silently ignored so that failure
    /// to install interceptors never breaks the calling command.
    pub async fn install_dialog_interceptors(&self) {
        let script = r"(function(){
if(window.__chrome_cli_intercepted)return;
window.__chrome_cli_intercepted=true;
var oA=window.alert,oC=window.confirm,oP=window.prompt;
function s(t,m,d){try{document.cookie='__chrome_cli_dialog='+
encodeURIComponent(JSON.stringify({type:t,message:String(m||''),
defaultValue:String(d||''),timestamp:Date.now()}))+
'; path=/; max-age=300';}catch(e){}}
window.alert=function(m){s('alert',m);return oA.apply(this,arguments);};
window.confirm=function(m){s('confirm',m);return oC.apply(this,arguments);};
window.prompt=function(m,d){s('prompt',m,d);return oP.apply(this,arguments);};
})();";

        // Install on current page via Runtime.evaluate (best-effort)
        let _ = self
            .session
            .send_command(
                "Runtime.evaluate",
                Some(serde_json::json!({ "expression": script })),
            )
            .await;

        // Register for future navigations (best-effort)
        let _ = self
            .session
            .send_command(
                "Page.addScriptToEvaluateOnNewDocument",
                Some(serde_json::json!({ "source": script })),
            )
            .await;
    }

    /// Spawn a background task that automatically dismisses JavaScript dialogs.
    ///
    /// Subscribes to dialog events and sends `Page.enable` with a short
    /// timeout. If a dialog is already open, `Page.enable` will block, but
    /// Chrome re-emits the `Page.javascriptDialogOpening` event before
    /// blocking, so the pre-existing dialog is captured and dismissed.
    /// Returns a `JoinHandle` whose `abort()` method can be called to stop
    /// the task (or it stops naturally when the session is dropped).
    ///
    /// # Errors
    ///
    /// Returns `CdpError` if the event subscription fails.
    pub async fn spawn_auto_dismiss(&mut self) -> Result<tokio::task::JoinHandle<()>, CdpError> {
        // Subscribe BEFORE Page.enable so we capture re-emitted dialog events.
        let mut dialog_rx = self
            .session
            .subscribe("Page.javascriptDialogOpening")
            .await?;

        // Send Page.enable with a timeout. If a dialog is already open,
        // Page.enable blocks but the dialog event is delivered before the
        // block. We accept the timeout and proceed.
        let page_enable = self.session.send_command("Page.enable", None);
        let enable_result =
            tokio::time::timeout(Duration::from_millis(PAGE_ENABLE_TIMEOUT_MS), page_enable).await;
        if matches!(enable_result, Ok(Ok(_))) {
            self.enabled_domains.insert("Page".to_string());
        }

        let session = self.session.clone();

        Ok(tokio::spawn(async move {
            while let Some(_event) = dialog_rx.recv().await {
                let params = serde_json::json!({ "accept": false });
                // Best-effort dismiss; ignore errors (session may have closed).
                let _ = session
                    .send_command("Page.handleJavaScriptDialog", Some(params))
                    .await;
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_target(id: &str, target_type: &str) -> TargetInfo {
        TargetInfo {
            id: id.to_string(),
            target_type: target_type.to_string(),
            title: format!("Title {id}"),
            url: format!("https://example.com/{id}"),
            ws_debugger_url: Some(format!("ws://127.0.0.1:9222/devtools/page/{id}")),
        }
    }

    #[test]
    fn extract_port_ws() {
        assert_eq!(
            extract_port_from_ws_url("ws://127.0.0.1:9222/devtools/browser/abc"),
            Some(9222)
        );
    }

    #[test]
    fn extract_port_wss() {
        assert_eq!(
            extract_port_from_ws_url("wss://localhost:9333/devtools/browser/abc"),
            Some(9333)
        );
    }

    #[test]
    fn extract_port_no_scheme() {
        assert_eq!(extract_port_from_ws_url("http://localhost:9222"), None);
    }

    #[test]
    fn select_target_default_picks_first_page() {
        let targets = vec![
            make_target("bg1", "background_page"),
            make_target("page1", "page"),
            make_target("page2", "page"),
        ];
        let result = select_target(&targets, None).unwrap();
        assert_eq!(result.id, "page1");
    }

    #[test]
    fn select_target_default_skips_non_page() {
        let targets = vec![
            make_target("sw1", "service_worker"),
            make_target("p1", "page"),
        ];
        let result = select_target(&targets, None).unwrap();
        assert_eq!(result.id, "p1");
    }

    #[test]
    fn select_target_by_index() {
        let targets = vec![
            make_target("a", "page"),
            make_target("b", "page"),
            make_target("c", "page"),
        ];
        let result = select_target(&targets, Some("1")).unwrap();
        assert_eq!(result.id, "b");
    }

    #[test]
    fn select_target_by_id() {
        let targets = vec![make_target("ABCDEF", "page"), make_target("GHIJKL", "page")];
        let result = select_target(&targets, Some("GHIJKL")).unwrap();
        assert_eq!(result.id, "GHIJKL");
    }

    #[test]
    fn select_target_invalid_tab() {
        let targets = vec![make_target("a", "page")];
        let result = select_target(&targets, Some("nonexistent"));
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("not found"));
    }

    #[test]
    fn select_target_index_out_of_bounds() {
        let targets = vec![make_target("a", "page")];
        let result = select_target(&targets, Some("5"));
        assert!(result.is_err());
    }

    #[test]
    fn select_target_empty_list_no_tab() {
        let targets: Vec<TargetInfo> = vec![];
        let result = select_target(&targets, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("No page targets"));
    }

    #[test]
    fn select_target_no_page_targets() {
        let targets = vec![
            make_target("sw1", "service_worker"),
            make_target("bg1", "background_page"),
        ];
        let result = select_target(&targets, None);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn managed_session_enables_domain_once() {
        use crate::cdp::{CdpClient, CdpConfig, ReconnectConfig};
        use futures_util::{SinkExt, StreamExt};
        use std::time::Duration;
        use tokio::net::TcpListener;
        use tokio::sync::mpsc;
        use tokio_tungstenite::tungstenite::Message;

        // Start mock CDP server that echoes responses and records messages
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (record_tx, mut record_rx) = mpsc::channel::<serde_json::Value>(32);

        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let (mut sink, mut source) = ws.split();
                while let Some(Ok(Message::Text(text))) = source.next().await {
                    let cmd: serde_json::Value = serde_json::from_str(&text).unwrap();
                    let _ = record_tx.send(cmd.clone()).await;

                    if cmd["method"] == "Target.attachToTarget" {
                        let tid = cmd["params"]["targetId"].as_str().unwrap_or("test");
                        let resp = serde_json::json!({
                            "id": cmd["id"],
                            "result": {"sessionId": tid}
                        });
                        let _ = sink.send(Message::Text(resp.to_string().into())).await;
                    } else {
                        let mut resp = serde_json::json!({"id": cmd["id"], "result": {}});
                        if let Some(sid) = cmd.get("sessionId") {
                            resp["sessionId"] = sid.clone();
                        }
                        let _ = sink.send(Message::Text(resp.to_string().into())).await;
                    }
                }
            }
        });

        // Connect and create session
        let url = format!("ws://{addr}");
        let config = CdpConfig {
            connect_timeout: Duration::from_secs(5),
            command_timeout: Duration::from_secs(5),
            channel_capacity: 256,
            reconnect: ReconnectConfig {
                max_retries: 0,
                ..ReconnectConfig::default()
            },
        };
        let client = CdpClient::connect(&url, config).await.unwrap();
        let session = client.create_session("test-target").await.unwrap();
        // Drain the attachToTarget message
        let _ = tokio::time::timeout(Duration::from_millis(200), record_rx.recv()).await;

        let mut managed = ManagedSession::new(session);
        assert!(managed.enabled_domains().is_empty());

        // First enable: should send Page.enable
        managed.ensure_domain("Page").await.unwrap();
        let msg = tokio::time::timeout(Duration::from_millis(200), record_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(msg["method"], "Page.enable");
        assert!(managed.enabled_domains().contains("Page"));

        // Second enable of same domain: should NOT send anything
        managed.ensure_domain("Page").await.unwrap();
        let no_msg = tokio::time::timeout(Duration::from_millis(100), record_rx.recv()).await;
        assert!(
            no_msg.is_err(),
            "No message should be sent for already-enabled domain"
        );

        // Enable a different domain
        managed.ensure_domain("Runtime").await.unwrap();
        let msg2 = tokio::time::timeout(Duration::from_millis(200), record_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(msg2["method"], "Runtime.enable");

        // Verify final state
        let domains = managed.enabled_domains();
        assert!(domains.contains("Page"));
        assert!(domains.contains("Runtime"));
        assert_eq!(domains.len(), 2);
    }
}
