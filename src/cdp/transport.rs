use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::error::CdpError;
use super::types::{CdpCommand, CdpEvent, MessageKind, RawCdpMessage};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Key for the subscriber map: (`method_name`, `session_id`).
type SubscriberKey = (String, Option<String>);

/// Command sent from the client handle to the transport task.
pub enum TransportCommand {
    /// Send a CDP command and deliver the response via the oneshot channel.
    SendCommand {
        command: CdpCommand,
        response_tx: oneshot::Sender<Result<serde_json::Value, CdpError>>,
        deadline: Instant,
    },
    /// Subscribe to events matching a method name (and optional session).
    Subscribe {
        method: String,
        session_id: Option<String>,
        event_tx: mpsc::Sender<CdpEvent>,
    },
    /// Shut down the transport gracefully.
    Shutdown,
}

/// Tracks an in-flight command awaiting its response.
struct PendingRequest {
    response_tx: oneshot::Sender<Result<serde_json::Value, CdpError>>,
    method: String,
    deadline: Instant,
}

/// Reconnection configuration.
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Maximum number of reconnection attempts (default: 5).
    pub max_retries: u32,
    /// Initial backoff delay (default: 100ms).
    pub initial_backoff: Duration,
    /// Maximum backoff delay (default: 5s).
    pub max_backoff: Duration,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(5),
        }
    }
}

/// Transport-level WebSocket keep-alive configuration.
///
/// When `interval` is `Some(d)`, the transport task sends a `Ping` frame
/// after `d` of outbound silence and treats a missing `Pong` within
/// `pong_timeout` as a dead connection. When `interval` is `None`, the
/// keep-alive loop is disabled entirely.
#[derive(Debug, Clone, Copy)]
pub struct KeepAliveConfig {
    /// Interval between keep-alive pings. `None` disables keep-alive.
    pub interval: Option<Duration>,
    /// Time to wait for a `Pong` response before treating the link as dead.
    pub pong_timeout: Duration,
}

impl Default for KeepAliveConfig {
    fn default() -> Self {
        Self {
            interval: Some(Duration::from_secs(30)),
            pong_timeout: Duration::from_secs(10),
        }
    }
}

/// Clonable handle for communicating with the transport task.
#[derive(Debug, Clone)]
pub struct TransportHandle {
    command_tx: mpsc::Sender<TransportCommand>,
    connected: Arc<AtomicBool>,
    next_id: Arc<AtomicU64>,
}

impl TransportHandle {
    /// Send a transport command to the background task.
    ///
    /// # Errors
    ///
    /// Returns `CdpError::Internal` if the transport task has exited.
    pub async fn send(&self, cmd: TransportCommand) -> Result<(), CdpError> {
        self.command_tx
            .send(cmd)
            .await
            .map_err(|_| CdpError::Internal("transport task is not running".into()))
    }

    /// Check whether the transport is currently connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Generate the next unique message ID for this connection.
    pub fn next_message_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}

/// Spawn the transport background task.
///
/// Returns a `TransportHandle` for sending commands to the task.
///
/// # Errors
///
/// Returns `CdpError::Connection` or `CdpError::ConnectionTimeout` if the
/// initial WebSocket connection cannot be established.
pub async fn spawn_transport(
    url: &str,
    channel_capacity: usize,
    reconnect_config: ReconnectConfig,
    connect_timeout: Duration,
    keepalive: KeepAliveConfig,
) -> Result<TransportHandle, CdpError> {
    let ws_stream = connect_ws(url, connect_timeout).await?;
    let connected = Arc::new(AtomicBool::new(true));
    let next_id = Arc::new(AtomicU64::new(1));
    let (command_tx, command_rx) = mpsc::channel(channel_capacity);

    let handle = TransportHandle {
        command_tx,
        connected: Arc::clone(&connected),
        next_id,
    };

    let url_owned = url.to_owned();
    tokio::spawn(async move {
        let mut task = TransportTask {
            ws_stream,
            command_rx,
            pending: HashMap::new(),
            subscribers: HashMap::new(),
            connected,
            url: url_owned,
            reconnect_config,
            connect_timeout,
            reconnect_failure: None,
            keepalive,
            // Initialize inside the task so scheduler delay between spawn and
            // first poll isn't counted against the first keep-alive interval.
            last_outbound: Instant::now(),
            last_ping_at: None,
        };
        task.run().await;
    });

    Ok(handle)
}

/// Establish a WebSocket connection with a timeout.
async fn connect_ws(url: &str, timeout: Duration) -> Result<WsStream, CdpError> {
    match tokio::time::timeout(timeout, tokio_tungstenite::connect_async(url)).await {
        Ok(Ok((stream, _response))) => Ok(stream),
        Ok(Err(e)) => Err(CdpError::Connection(e.to_string())),
        Err(_) => Err(CdpError::ConnectionTimeout),
    }
}

/// The background transport task that owns the WebSocket connection.
struct TransportTask {
    ws_stream: WsStream,
    command_rx: mpsc::Receiver<TransportCommand>,
    pending: HashMap<u64, PendingRequest>,
    subscribers: HashMap<SubscriberKey, Vec<mpsc::Sender<CdpEvent>>>,
    connected: Arc<AtomicBool>,
    url: String,
    reconnect_config: ReconnectConfig,
    connect_timeout: Duration,
    reconnect_failure: Option<(u32, String)>,
    keepalive: KeepAliveConfig,
    /// Timestamp of the most recent outbound WebSocket frame, used to gate
    /// the keep-alive timer against steady command traffic.
    last_outbound: Instant,
    /// `Some(when_sent)` while a Ping is awaiting a Pong; `None` otherwise.
    last_ping_at: Option<Instant>,
}

impl TransportTask {
    #[allow(clippy::too_many_lines)]
    async fn run(&mut self) {
        loop {
            // If reconnection has permanently failed, drain remaining
            // commands with ReconnectFailed errors until shutdown.
            if let Some((attempts, ref last_error)) = self.reconnect_failure {
                match self.command_rx.recv().await {
                    Some(TransportCommand::SendCommand { response_tx, .. }) => {
                        let _ = response_tx.send(Err(CdpError::ReconnectFailed {
                            attempts,
                            last_error: last_error.clone(),
                        }));
                        continue;
                    }
                    Some(TransportCommand::Subscribe { .. }) => continue,
                    Some(TransportCommand::Shutdown) | None => return,
                }
            }

            let next_deadline = self.earliest_deadline();
            let timeout_sleep = async {
                if let Some(deadline) = next_deadline {
                    tokio::time::sleep_until(deadline).await;
                } else {
                    // No pending requests — sleep forever (will be cancelled by select)
                    std::future::pending::<()>().await;
                }
            };

            // When keep-alive is disabled, this resolves to a never-firing
            // future so the select! branch is effectively inert.
            let keepalive_interval = self.keepalive.interval;
            let last_outbound = self.last_outbound;
            let keepalive_tick = async move {
                match keepalive_interval {
                    Some(interval) => {
                        tokio::time::sleep_until(last_outbound + interval).await;
                    }
                    None => std::future::pending::<()>().await,
                }
            };

            // Inert unless a Ping is outstanding.
            let pong_timeout = self.keepalive.pong_timeout;
            let last_ping = self.last_ping_at;
            let pong_deadline = async move {
                match last_ping {
                    Some(sent_at) => {
                        tokio::time::sleep_until(sent_at + pong_timeout).await;
                    }
                    None => std::future::pending::<()>().await,
                }
            };

            tokio::select! {
                ws_msg = self.ws_stream.next() => {
                    match ws_msg {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_text_message(&text);
                        }
                        Some(Ok(Message::Pong(_))) => {
                            self.last_ping_at = None;
                        }
                        Some(Ok(Message::Close(_)) | Err(_)) | None => {
                            self.handle_disconnect().await;
                            self.last_ping_at = None;
                            self.last_outbound = Instant::now();
                        }
                        Some(Ok(_)) => {
                            // Binary, Ping, Frame — ignore
                        }
                    }
                }

                cmd = self.command_rx.recv() => {
                    match cmd {
                        Some(TransportCommand::SendCommand { command, response_tx, deadline }) => {
                            self.handle_send_command(command, response_tx, deadline).await;
                        }
                        Some(TransportCommand::Subscribe { method, session_id, event_tx }) => {
                            self.subscribers
                                .entry((method, session_id))
                                .or_default()
                                .push(event_tx);
                        }
                        Some(TransportCommand::Shutdown) | None => {
                            self.drain_pending();
                            let _ = self.ws_stream.close(None).await;
                            self.connected.store(false, Ordering::Relaxed);
                            return;
                        }
                    }
                }

                () = timeout_sleep => {
                    self.sweep_timeouts();
                }

                () = keepalive_tick => {
                    if self.keepalive.interval.is_some() {
                        let _ = self.ws_stream.send(Message::Ping(Vec::new().into())).await;
                        let now = Instant::now();
                        self.last_outbound = now;
                        self.last_ping_at = Some(now);
                    }
                }

                () = pong_deadline => {
                    if self.last_ping_at.is_some() {
                        // Missed Pong — treat as dead and let reconnect recover.
                        self.handle_disconnect().await;
                        self.last_ping_at = None;
                        self.last_outbound = Instant::now();
                    }
                }
            }
        }
    }

    fn handle_text_message(&mut self, text: &str) {
        let raw: RawCdpMessage = match serde_json::from_str(text) {
            Ok(msg) => msg,
            Err(_) => {
                // Malformed JSON — ignore and continue
                return;
            }
        };

        let Some(kind) = raw.classify() else {
            // Unclassifiable message — ignore
            return;
        };

        match kind {
            MessageKind::Response(response) => {
                if let Some(pending) = self.pending.remove(&response.id) {
                    let result = match response.result {
                        Ok(value) => Ok(value),
                        Err(proto_err) => Err(CdpError::Protocol {
                            code: proto_err.code,
                            message: proto_err.message,
                        }),
                    };
                    let _ = pending.response_tx.send(result);
                }
            }
            MessageKind::Event(event) => {
                self.dispatch_event(&event);
            }
        }
    }

    fn dispatch_event(&mut self, event: &CdpEvent) {
        let key = (event.method.clone(), event.session_id.clone());
        if let Some(senders) = self.subscribers.get_mut(&key) {
            // Remove senders whose receiver has been dropped
            senders.retain(|tx| tx.try_send(event.clone()).is_ok() || !tx.is_closed());
            if senders.is_empty() {
                self.subscribers.remove(&key);
            }
        }
    }

    async fn handle_send_command(
        &mut self,
        command: CdpCommand,
        response_tx: oneshot::Sender<Result<serde_json::Value, CdpError>>,
        deadline: Instant,
    ) {
        let id = command.id;
        let method = command.method.clone();

        let json = match serde_json::to_string(&command) {
            Ok(j) => j,
            Err(e) => {
                let _ =
                    response_tx.send(Err(CdpError::Internal(format!("serialization error: {e}"))));
                return;
            }
        };

        if let Err(e) = self.ws_stream.send(Message::Text(json.into())).await {
            let _ = response_tx.send(Err(CdpError::Connection(format!(
                "WebSocket write error: {e}"
            ))));
            return;
        }

        // Resets the keep-alive timer so pings don't fire during steady traffic.
        self.last_outbound = Instant::now();

        self.pending.insert(
            id,
            PendingRequest {
                response_tx,
                method,
                deadline,
            },
        );
    }

    fn earliest_deadline(&self) -> Option<Instant> {
        self.pending.values().map(|p| p.deadline).min()
    }

    fn sweep_timeouts(&mut self) {
        let now = Instant::now();
        let timed_out: Vec<u64> = self
            .pending
            .iter()
            .filter(|(_, p)| p.deadline <= now)
            .map(|(&id, _)| id)
            .collect();

        for id in timed_out {
            if let Some(pending) = self.pending.remove(&id) {
                let _ = pending.response_tx.send(Err(CdpError::CommandTimeout {
                    method: pending.method,
                }));
            }
        }
    }

    fn drain_pending(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        for (_, req) in pending {
            let _ = req.response_tx.send(Err(CdpError::ConnectionClosed));
        }
    }

    async fn handle_disconnect(&mut self) {
        self.connected.store(false, Ordering::Relaxed);
        self.drain_pending();

        let mut backoff = self.reconnect_config.initial_backoff;
        let mut last_error_msg = String::from("no retries configured");

        for attempt in 1..=self.reconnect_config.max_retries {
            tokio::time::sleep(backoff).await;

            match connect_ws(&self.url, self.connect_timeout).await {
                Ok(new_stream) => {
                    self.ws_stream = new_stream;
                    self.connected.store(true, Ordering::Relaxed);
                    return;
                }
                Err(e) => {
                    last_error_msg = e.to_string();
                    if attempt < self.reconnect_config.max_retries {
                        backoff = (backoff * 2).min(self.reconnect_config.max_backoff);
                    }
                }
            }
        }

        // All retries exhausted — store failure and let the run loop
        // drain remaining commands with ReconnectFailed errors.
        self.reconnect_failure = Some((self.reconnect_config.max_retries, last_error_msg));
    }
}
