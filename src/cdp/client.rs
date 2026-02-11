use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant};

use super::error::CdpError;
use super::transport::{ReconnectConfig, TransportCommand, TransportHandle, spawn_transport};
use super::types::CdpEvent;

/// Configuration for a CDP client connection.
#[derive(Debug, Clone)]
pub struct CdpConfig {
    /// Timeout for the initial WebSocket connection (default: 10s).
    pub connect_timeout: Duration,
    /// Timeout for individual CDP commands (default: 30s).
    pub command_timeout: Duration,
    /// Capacity of the internal command channel (default: 256).
    pub channel_capacity: usize,
    /// Reconnection settings.
    pub reconnect: ReconnectConfig,
}

impl Default for CdpConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            command_timeout: Duration::from_secs(30),
            channel_capacity: 256,
            reconnect: ReconnectConfig::default(),
        }
    }
}

/// A CDP client connected to Chrome over WebSocket.
///
/// This is the main entry point for sending CDP commands and subscribing
/// to events. It communicates with a background transport task that owns
/// the WebSocket connection.
#[derive(Debug)]
pub struct CdpClient {
    handle: TransportHandle,
    config: CdpConfig,
    url: String,
}

impl CdpClient {
    /// Connect to a Chrome CDP WebSocket endpoint.
    ///
    /// # Errors
    ///
    /// Returns `CdpError::Connection` if the WebSocket handshake fails,
    /// or `CdpError::ConnectionTimeout` if the connection attempt exceeds
    /// the configured timeout.
    pub async fn connect(url: &str, config: CdpConfig) -> Result<Self, CdpError> {
        let handle = spawn_transport(
            url,
            config.channel_capacity,
            config.reconnect.clone(),
            config.connect_timeout,
        )
        .await?;

        Ok(Self {
            handle,
            config,
            url: url.to_owned(),
        })
    }

    /// Send a CDP command (browser-level, no session).
    ///
    /// # Errors
    ///
    /// Returns `CdpError::CommandTimeout` if Chrome does not respond within
    /// the configured timeout, `CdpError::Protocol` if Chrome returns an
    /// error, or `CdpError::Internal` if the transport task has exited.
    pub async fn send_command(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, CdpError> {
        send_command_impl(&self.handle, self.config.command_timeout, method, params, None).await
    }

    /// Subscribe to CDP events matching a method name.
    ///
    /// Returns a receiver that yields `CdpEvent` values. Events stop
    /// being delivered when the receiver is dropped.
    ///
    /// # Errors
    ///
    /// Returns `CdpError::Internal` if the transport task has exited.
    pub async fn subscribe(
        &self,
        method: &str,
    ) -> Result<mpsc::Receiver<CdpEvent>, CdpError> {
        subscribe_impl(&self.handle, self.config.channel_capacity, method, None).await
    }

    /// Create a CDP session attached to a specific target.
    ///
    /// Sends `Target.attachToTarget` and returns a `CdpSession` bound
    /// to the returned session ID.
    ///
    /// # Errors
    ///
    /// Returns `CdpError::Protocol` if the target cannot be attached,
    /// or any transport error.
    pub async fn create_session(&self, target_id: &str) -> Result<CdpSession, CdpError> {
        let params = serde_json::json!({
            "targetId": target_id,
            "flatten": true,
        });
        let result = self
            .send_command("Target.attachToTarget", Some(params))
            .await?;
        let session_id = result["sessionId"]
            .as_str()
            .ok_or_else(|| {
                CdpError::InvalidResponse(
                    "Target.attachToTarget response missing sessionId".into(),
                )
            })?
            .to_owned();

        Ok(CdpSession {
            session_id,
            handle: self.handle.clone(),
            config: self.config.clone(),
        })
    }

    /// Gracefully close the WebSocket connection.
    ///
    /// # Errors
    ///
    /// Returns `CdpError::Internal` if the transport task has already exited.
    pub async fn close(self) -> Result<(), CdpError> {
        self.handle.send(TransportCommand::Shutdown).await
    }

    /// Check if the client is currently connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.handle.is_connected()
    }

    /// Get the WebSocket URL this client is connected to.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }
}

/// A CDP session bound to a specific target (tab).
///
/// Sessions share the parent client's WebSocket connection but route
/// commands and events through a `sessionId`.
#[derive(Debug)]
pub struct CdpSession {
    session_id: String,
    handle: TransportHandle,
    config: CdpConfig,
}

impl CdpSession {
    /// Send a command within this session's context.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`CdpClient::send_command`].
    pub async fn send_command(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, CdpError> {
        send_command_impl(
            &self.handle,
            self.config.command_timeout,
            method,
            params,
            Some(self.session_id.clone()),
        )
        .await
    }

    /// Subscribe to events within this session.
    ///
    /// # Errors
    ///
    /// Returns `CdpError::Internal` if the transport task has exited.
    pub async fn subscribe(
        &self,
        method: &str,
    ) -> Result<mpsc::Receiver<CdpEvent>, CdpError> {
        subscribe_impl(
            &self.handle,
            self.config.channel_capacity,
            method,
            Some(self.session_id.clone()),
        )
        .await
    }

    /// Get the session ID.
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

// =============================================================================
// Shared helpers
// =============================================================================

/// Send a CDP command via the transport handle and await the response.
async fn send_command_impl(
    handle: &TransportHandle,
    command_timeout: Duration,
    method: &str,
    params: Option<serde_json::Value>,
    session_id: Option<String>,
) -> Result<serde_json::Value, CdpError> {
    let id = handle.next_message_id();
    let command = super::types::CdpCommand {
        id,
        method: method.to_owned(),
        params,
        session_id,
    };

    let (response_tx, response_rx) = oneshot::channel();
    let deadline = Instant::now() + command_timeout;

    handle
        .send(TransportCommand::SendCommand {
            command,
            response_tx,
            deadline,
        })
        .await?;

    response_rx
        .await
        .map_err(|_| CdpError::Internal("transport task exited before responding".into()))?
}

/// Register an event subscription via the transport handle.
async fn subscribe_impl(
    handle: &TransportHandle,
    channel_capacity: usize,
    method: &str,
    session_id: Option<String>,
) -> Result<mpsc::Receiver<CdpEvent>, CdpError> {
    let (event_tx, event_rx) = mpsc::channel(channel_capacity);
    handle
        .send(TransportCommand::Subscribe {
            method: method.to_owned(),
            session_id,
            event_tx,
        })
        .await?;
    Ok(event_rx)
}
