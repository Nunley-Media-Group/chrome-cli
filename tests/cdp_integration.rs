//! Integration tests for the CDP WebSocket client.
//!
//! Each test spins up a mock WebSocket server with configurable behavior,
//! connects a `CdpClient`, and verifies the expected interactions.

#![allow(clippy::needless_pass_by_value)]

use std::net::SocketAddr;
use std::time::Duration;

use chrome_cli::cdp::{CdpClient, CdpConfig, CdpError, ReconnectConfig};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

// =============================================================================
// Mock server helpers
// =============================================================================

/// Start a mock CDP server that echoes `{"id": N, "result": {}}` for each command.
async fn start_echo_server() -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let (mut sink, mut source) = ws.split();
                while let Some(Ok(msg)) = source.next().await {
                    if let Message::Text(text) = msg {
                        let cmd: Value = serde_json::from_str(&text).unwrap();
                        let response = json!({"id": cmd["id"], "result": {}});
                        sink.send(Message::Text(response.to_string().into()))
                            .await
                            .unwrap();
                    }
                }
            });
        }
    });
    (addr, handle)
}

/// Start a mock server that responds with a custom result for each command.
async fn start_custom_result_server(
    result_fn: fn(&Value) -> Value,
) -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let (mut sink, mut source) = ws.split();
                while let Some(Ok(msg)) = source.next().await {
                    if let Message::Text(text) = msg {
                        let cmd: Value = serde_json::from_str(&text).unwrap();
                        let result = result_fn(&cmd);
                        let response = json!({"id": cmd["id"], "result": result});
                        sink.send(Message::Text(response.to_string().into()))
                            .await
                            .unwrap();
                    }
                }
            });
        }
    });
    (addr, handle)
}

/// Start a mock server that never responds to commands (for timeout tests).
async fn start_silent_server() -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let (_sink, mut source) = ws.split();
                // Accept commands but never respond
                while source.next().await.is_some() {}
            });
        }
    });
    (addr, handle)
}

/// Start a mock server that returns a CDP protocol error for each command.
async fn start_protocol_error_server(code: i64, message: &str) -> (SocketAddr, JoinHandle<()>) {
    let message = message.to_owned();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let message = message.clone();
            tokio::spawn(async move {
                let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let (mut sink, mut source) = ws.split();
                while let Some(Ok(msg)) = source.next().await {
                    if let Message::Text(text) = msg {
                        let cmd: Value = serde_json::from_str(&text).unwrap();
                        let response = json!({
                            "id": cmd["id"],
                            "error": {"code": code, "message": message}
                        });
                        sink.send(Message::Text(response.to_string().into()))
                            .await
                            .unwrap();
                    }
                }
            });
        }
    });
    (addr, handle)
}

/// Start a mock server that drops connection after N messages.
async fn start_drop_after_server(n: usize) -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let (mut sink, mut source) = ws.split();
                let mut count = 0;
                while let Some(Ok(msg)) = source.next().await {
                    if let Message::Text(text) = msg {
                        let cmd: Value = serde_json::from_str(&text).unwrap();
                        let response = json!({"id": cmd["id"], "result": {}});
                        sink.send(Message::Text(response.to_string().into()))
                            .await
                            .unwrap();
                        count += 1;
                        if count >= n {
                            // Close connection
                            drop(sink);
                            return;
                        }
                    }
                }
            });
        }
    });
    (addr, handle)
}

/// Start a mock server that emits events on demand via a channel.
async fn start_event_server() -> (SocketAddr, mpsc::Sender<Value>, JoinHandle<()>) {
    let (event_tx, mut event_rx) = mpsc::channel::<Value>(32);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let (mut sink, mut source) = ws.split();

            loop {
                tokio::select! {
                    // Handle incoming commands (echo response)
                    msg = source.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                let cmd: Value = serde_json::from_str(&text).unwrap();
                                let response = json!({"id": cmd["id"], "result": {}});
                                sink.send(Message::Text(response.to_string().into()))
                                    .await
                                    .unwrap();
                            }
                            None | Some(Err(_)) => break,
                            _ => {}
                        }
                    }
                    // Send events requested by tests
                    event = event_rx.recv() => {
                        if let Some(event) = event {
                            sink.send(Message::Text(event.to_string().into()))
                                .await
                                .unwrap();
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    });
    (addr, event_tx, handle)
}

/// Start a mock server that sends malformed JSON, then continues serving normally.
async fn start_malformed_then_echo_server() -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let (mut sink, mut source) = ws.split();
                let mut first = true;
                while let Some(Ok(msg)) = source.next().await {
                    if let Message::Text(text) = msg {
                        let cmd: Value = serde_json::from_str(&text).unwrap();
                        if first {
                            // Send malformed JSON first
                            sink.send(Message::Text(r"this is not json{".into()))
                                .await
                                .unwrap();
                            first = false;
                        }
                        // Then send a proper response
                        let response = json!({"id": cmd["id"], "result": {}});
                        sink.send(Message::Text(response.to_string().into()))
                            .await
                            .unwrap();
                    }
                }
            });
        }
    });
    (addr, handle)
}

/// Start a mock server that records all received messages including sessionId.
async fn start_recording_server() -> (SocketAddr, mpsc::Receiver<Value>, JoinHandle<()>) {
    let (record_tx, record_rx) = mpsc::channel::<Value>(64);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let record_tx = record_tx.clone();
            tokio::spawn(async move {
                let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let (mut sink, mut source) = ws.split();
                while let Some(Ok(msg)) = source.next().await {
                    if let Message::Text(text) = msg {
                        let cmd: Value = serde_json::from_str(&text).unwrap();
                        let _ = record_tx.send(cmd.clone()).await;

                        // If it's Target.attachToTarget, return a sessionId
                        if cmd["method"] == "Target.attachToTarget" {
                            let target_id = cmd["params"]["targetId"].as_str().unwrap_or("unknown");
                            let session_id = format!("session-for-{target_id}");
                            let response = json!({
                                "id": cmd["id"],
                                "result": {"sessionId": session_id}
                            });
                            sink.send(Message::Text(response.to_string().into()))
                                .await
                                .unwrap();
                        } else {
                            // Normal echo with sessionId preserved if present
                            let mut response = json!({"id": cmd["id"], "result": {}});
                            if let Some(sid) = cmd.get("sessionId") {
                                response["sessionId"] = sid.clone();
                            }
                            sink.send(Message::Text(response.to_string().into()))
                                .await
                                .unwrap();
                        }
                    }
                }
            });
        }
    });
    (addr, record_rx, handle)
}

fn ws_url(addr: SocketAddr) -> String {
    format!("ws://{addr}")
}

fn quick_config() -> CdpConfig {
    CdpConfig {
        connect_timeout: Duration::from_secs(5),
        command_timeout: Duration::from_secs(5),
        channel_capacity: 256,
        reconnect: ReconnectConfig {
            max_retries: 0,
            initial_backoff: Duration::from_millis(50),
            max_backoff: Duration::from_millis(200),
        },
    }
}

// =============================================================================
// Tests
// =============================================================================

/// AC1: Connect to Chrome CDP endpoint
#[tokio::test]
async fn connect_to_mock_server() {
    let (addr, _handle) = start_echo_server().await;
    let client = CdpClient::connect(&ws_url(addr), quick_config()).await;
    assert!(client.is_ok());
    let client = client.unwrap();
    assert!(client.is_connected());
}

/// AC2: Send command and receive response
#[tokio::test]
async fn send_command_and_receive_response() {
    let (addr, _handle) = start_echo_server().await;
    let client = CdpClient::connect(&ws_url(addr), quick_config())
        .await
        .unwrap();

    let result = client
        .send_command("Page.navigate", Some(json!({"url": "https://example.com"})))
        .await;

    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value.is_object());
}

/// AC3: Concurrent command correlation
#[tokio::test]
async fn concurrent_command_correlation() {
    // Server returns the command's id as a result value
    let (addr, _handle) = start_custom_result_server(|cmd| json!({"echo_id": cmd["id"]})).await;
    let client = CdpClient::connect(&ws_url(addr), quick_config())
        .await
        .unwrap();

    // Send 10 commands concurrently using join_all
    let client_ref = &client;
    let futures: Vec<_> = (0..10)
        .map(|i| async move {
            let method = format!("Test.method{i}");
            client_ref.send_command(&method, None).await
        })
        .collect();

    let results = futures_util::future::join_all(futures).await;

    // All 10 should succeed with distinct echo_ids
    let ids: std::collections::HashSet<u64> = results
        .iter()
        .map(|r| {
            let value = r.as_ref().expect("command failed");
            value["echo_id"].as_u64().unwrap()
        })
        .collect();
    assert_eq!(ids.len(), 10, "expected 10 unique response IDs");
}

/// AC4: Receive CDP events
#[tokio::test]
async fn receive_cdp_events() {
    let (addr, event_tx, _handle) = start_event_server().await;
    let client = CdpClient::connect(&ws_url(addr), quick_config())
        .await
        .unwrap();

    let mut rx = client.subscribe("Page.loadEventFired").await.unwrap();

    // Server emits the event
    event_tx
        .send(json!({
            "method": "Page.loadEventFired",
            "params": {"timestamp": 123.456}
        }))
        .await
        .unwrap();

    let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for event")
        .expect("channel closed");

    assert_eq!(event.method, "Page.loadEventFired");
    assert!(event.params["timestamp"].as_f64().is_some());
}

/// AC5: Event subscription and unsubscription
#[tokio::test]
async fn event_unsubscription_on_drop() {
    let (addr, event_tx, _handle) = start_event_server().await;
    let client = CdpClient::connect(&ws_url(addr), quick_config())
        .await
        .unwrap();

    let rx = client.subscribe("Console.messageAdded").await.unwrap();
    // Drop the receiver
    drop(rx);

    // Give transport time to notice the drop
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Server sends event — should not cause issues
    event_tx
        .send(json!({
            "method": "Console.messageAdded",
            "params": {"text": "hello"}
        }))
        .await
        .unwrap();

    // Client should still be functional
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(client.is_connected());
}

/// AC6: Session multiplexing over single WebSocket
#[tokio::test]
async fn session_multiplexing() {
    let (addr, mut record_rx, _handle) = start_recording_server().await;
    let client = CdpClient::connect(&ws_url(addr), quick_config())
        .await
        .unwrap();

    let session1 = client.create_session("target-1").await.unwrap();
    let session2 = client.create_session("target-2").await.unwrap();

    // Drain the two Target.attachToTarget records
    let _ = record_rx.recv().await;
    let _ = record_rx.recv().await;

    // Send commands on each session
    let _r1 = session1.send_command("Runtime.evaluate", None).await;
    let _r2 = session2.send_command("DOM.getDocument", None).await;

    let msg1 = record_rx.recv().await.unwrap();
    let msg2 = record_rx.recv().await.unwrap();

    assert_eq!(msg1["sessionId"].as_str().unwrap(), session1.session_id());
    assert_eq!(msg2["sessionId"].as_str().unwrap(), session2.session_id());
}

/// AC7: Flatten session protocol support
#[tokio::test]
async fn flatten_session_protocol() {
    let (addr, mut record_rx, _handle) = start_recording_server().await;
    let client = CdpClient::connect(&ws_url(addr), quick_config())
        .await
        .unwrap();

    let session = client.create_session("target-abc").await.unwrap();
    let _ = record_rx.recv().await; // drain attach command

    let result = session
        .send_command("Runtime.evaluate", Some(json!({"expression": "1+1"})))
        .await;
    assert!(result.is_ok());

    let recorded = record_rx.recv().await.unwrap();
    assert!(
        recorded["sessionId"]
            .as_str()
            .unwrap()
            .contains("target-abc"),
        "outgoing message should contain sessionId"
    );
}

/// AC8: Connection timeout
#[tokio::test]
async fn connection_timeout() {
    // Connect to a port that's unlikely to be listening, with a very short timeout
    let config = CdpConfig {
        connect_timeout: Duration::from_secs(1),
        command_timeout: Duration::from_secs(1),
        channel_capacity: 16,
        reconnect: ReconnectConfig {
            max_retries: 0,
            ..ReconnectConfig::default()
        },
    };

    let start = std::time::Instant::now();
    let result = CdpClient::connect("ws://192.0.2.1:9999", config).await;
    let elapsed = start.elapsed();

    assert!(result.is_err());
    assert!(
        elapsed < Duration::from_secs(3),
        "should timeout quickly, took {elapsed:?}"
    );

    match result {
        Err(CdpError::ConnectionTimeout | CdpError::Connection(_)) => {}
        Err(other) => panic!("expected ConnectionTimeout or Connection, got: {other}"),
        Ok(_) => panic!("expected connection error, but connection succeeded"),
    }
}

/// AC9: Command timeout
#[tokio::test]
async fn command_timeout() {
    let (addr, _handle) = start_silent_server().await;
    let config = CdpConfig {
        connect_timeout: Duration::from_secs(5),
        command_timeout: Duration::from_secs(1),
        channel_capacity: 256,
        reconnect: ReconnectConfig {
            max_retries: 0,
            ..ReconnectConfig::default()
        },
    };
    let client = CdpClient::connect(&ws_url(addr), config).await.unwrap();

    let result = client.send_command("Slow.method", None).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, CdpError::CommandTimeout { .. }),
        "expected CommandTimeout, got: {err}"
    );
}

/// AC10: WebSocket close handling
#[tokio::test]
async fn websocket_close_handling() {
    // Server drops connection after 1 message
    let (addr, _handle) = start_drop_after_server(1).await;
    let config = CdpConfig {
        connect_timeout: Duration::from_secs(5),
        command_timeout: Duration::from_secs(2),
        channel_capacity: 256,
        reconnect: ReconnectConfig {
            max_retries: 0,
            ..ReconnectConfig::default()
        },
    };
    let client = CdpClient::connect(&ws_url(addr), config).await.unwrap();

    // First command succeeds (server responds then drops)
    let r1 = client.send_command("First.command", None).await;
    assert!(r1.is_ok());

    // Give transport time to notice the close
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Client should report disconnected
    assert!(
        !client.is_connected(),
        "client should report disconnected after server drops"
    );
}

/// AC11: CDP protocol error handling
#[tokio::test]
async fn protocol_error_handling() {
    let (addr, _handle) = start_protocol_error_server(-32000, "Not found").await;
    let client = CdpClient::connect(&ws_url(addr), quick_config())
        .await
        .unwrap();

    let result = client.send_command("Unknown.method", None).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        CdpError::Protocol { code, message } => {
            assert_eq!(code, -32000);
            assert_eq!(message, "Not found");
        }
        other => panic!("expected Protocol error, got: {other}"),
    }
}

/// AC13: Reconnection after disconnection
#[tokio::test]
async fn reconnection_after_disconnection() {
    // Server that drops after 1 message but keeps listening for new connections
    let (addr, _handle) = start_drop_after_server(1).await;

    let config = CdpConfig {
        connect_timeout: Duration::from_secs(5),
        command_timeout: Duration::from_secs(5),
        channel_capacity: 256,
        reconnect: ReconnectConfig {
            max_retries: 5,
            initial_backoff: Duration::from_millis(50),
            max_backoff: Duration::from_millis(500),
        },
    };

    let client = CdpClient::connect(&ws_url(addr), config).await.unwrap();

    // First command triggers the drop
    let _ = client.send_command("First.command", None).await;

    // Wait for reconnection
    tokio::time::sleep(Duration::from_secs(1)).await;

    // After reconnection, client should be connected again
    assert!(
        client.is_connected(),
        "client should reconnect after server restarts"
    );

    // Should be able to send commands again
    let result = client.send_command("After.reconnect", None).await;
    assert!(result.is_ok(), "command after reconnect failed: {result:?}");
}

/// AC13b: Reconnection failure after retries exhausted
#[tokio::test]
async fn reconnection_failure() {
    // Start a server, connect, then stop it
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Accept one connection, respond once, then close everything
    let server_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut sink, mut source) = ws.split();
        if let Some(Ok(Message::Text(text))) = source.next().await {
            let cmd: Value = serde_json::from_str(&text).unwrap();
            let response = json!({"id": cmd["id"], "result": {}});
            sink.send(Message::Text(response.to_string().into()))
                .await
                .unwrap();
        }
        // Drop ws — close connection. Listener is also dropped, so reconnection will fail.
    });

    let config = CdpConfig {
        connect_timeout: Duration::from_secs(1),
        command_timeout: Duration::from_secs(2),
        channel_capacity: 256,
        reconnect: ReconnectConfig {
            max_retries: 2,
            initial_backoff: Duration::from_millis(50),
            max_backoff: Duration::from_millis(100),
        },
    };

    let client = CdpClient::connect(&ws_url(addr), config).await.unwrap();
    let _ = client.send_command("Test.command", None).await;

    // Wait for the server task to finish (it drops the listener)
    server_handle.await.unwrap();

    // Wait for reconnection attempts to exhaust
    tokio::time::sleep(Duration::from_secs(2)).await;

    assert!(
        !client.is_connected(),
        "client should be disconnected after retries exhausted"
    );
}

/// AC14: Invalid JSON handling
#[tokio::test]
async fn invalid_json_handling() {
    let (addr, _handle) = start_malformed_then_echo_server().await;
    let client = CdpClient::connect(&ws_url(addr), quick_config())
        .await
        .unwrap();

    // First command triggers malformed JSON, but should still get a response
    let result = client.send_command("Test.first", None).await;
    assert!(
        result.is_ok(),
        "client should handle malformed JSON gracefully"
    );

    // Second command should work normally
    let result = client.send_command("Test.second", None).await;
    assert!(result.is_ok(), "subsequent commands should still work");
}
