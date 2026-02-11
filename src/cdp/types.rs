use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Outgoing CDP command (client to Chrome).
#[derive(Debug, Serialize)]
pub struct CdpCommand {
    /// Unique message ID for response correlation.
    pub id: u64,
    /// CDP method name (e.g., `Page.navigate`).
    pub method: String,
    /// Optional parameters for the command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    /// Optional session ID for session-scoped commands.
    #[serde(rename = "sessionId", skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Raw incoming CDP message before classification.
///
/// This is the union of response and event fields — every incoming
/// WebSocket message is deserialized into this type first, then
/// classified via [`classify`](Self::classify).
#[derive(Debug, Deserialize)]
pub struct RawCdpMessage {
    /// Present for responses; absent for events.
    pub id: Option<u64>,
    /// Present for events (and some responses with `method`).
    pub method: Option<String>,
    /// Event parameters or additional response data.
    pub params: Option<Value>,
    /// Successful response payload.
    pub result: Option<Value>,
    /// Protocol error payload.
    pub error: Option<CdpProtocolError>,
    /// Session ID for session-scoped messages.
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

/// CDP protocol error payload returned by Chrome.
#[derive(Debug, Clone, Deserialize)]
pub struct CdpProtocolError {
    /// The CDP error code (e.g., -32000).
    pub code: i64,
    /// Human-readable error description.
    pub message: String,
}

/// Parsed CDP response (has an `id`).
#[derive(Debug)]
pub struct CdpResponse {
    /// The message ID that correlates to the sent command.
    pub id: u64,
    /// The result: either a successful value or a protocol error.
    pub result: Result<Value, CdpProtocolError>,
    /// Session ID if this response is session-scoped.
    pub session_id: Option<String>,
}

/// Parsed CDP event (no `id`, has `method`).
#[derive(Debug, Clone)]
pub struct CdpEvent {
    /// The CDP event method name (e.g., `Page.loadEventFired`).
    pub method: String,
    /// Event parameters.
    pub params: Value,
    /// Session ID if this event is session-scoped.
    pub session_id: Option<String>,
}

/// Classification of a raw CDP message.
pub enum MessageKind {
    /// A response to a previously sent command.
    Response(CdpResponse),
    /// An asynchronous event from Chrome.
    Event(CdpEvent),
}

impl RawCdpMessage {
    /// Classify this raw message as either a response or an event.
    ///
    /// Messages with an `id` field are responses; messages with a `method`
    /// field but no `id` are events. Returns `None` if the message cannot
    /// be classified (neither `id` nor `method` present).
    #[must_use]
    pub fn classify(self) -> Option<MessageKind> {
        if let Some(id) = self.id {
            let result = if let Some(error) = self.error {
                Err(error)
            } else {
                Ok(self.result.unwrap_or(Value::Null))
            };
            Some(MessageKind::Response(CdpResponse {
                id,
                result,
                session_id: self.session_id,
            }))
        } else if let Some(method) = self.method {
            Some(MessageKind::Event(CdpEvent {
                method,
                params: self.params.unwrap_or(Value::Null),
                session_id: self.session_id,
            }))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- CdpCommand serialization ---

    #[test]
    fn serialize_command_without_params_or_session() {
        let cmd = CdpCommand {
            id: 1,
            method: "Browser.getVersion".into(),
            params: None,
            session_id: None,
        };
        let json: Value = serde_json::to_value(&cmd).unwrap();
        assert_eq!(json["id"], 1);
        assert_eq!(json["method"], "Browser.getVersion");
        assert!(json.get("params").is_none());
        assert!(json.get("sessionId").is_none());
    }

    #[test]
    fn serialize_command_with_params() {
        let cmd = CdpCommand {
            id: 2,
            method: "Page.navigate".into(),
            params: Some(json!({"url": "https://example.com"})),
            session_id: None,
        };
        let json: Value = serde_json::to_value(&cmd).unwrap();
        assert_eq!(json["id"], 2);
        assert_eq!(json["params"]["url"], "https://example.com");
        assert!(json.get("sessionId").is_none());
    }

    #[test]
    fn serialize_command_with_session_id() {
        let cmd = CdpCommand {
            id: 3,
            method: "Runtime.evaluate".into(),
            params: Some(json!({"expression": "1+1"})),
            session_id: Some("session-abc".into()),
        };
        let json: Value = serde_json::to_value(&cmd).unwrap();
        assert_eq!(json["sessionId"], "session-abc");
    }

    // --- RawCdpMessage deserialization ---

    #[test]
    fn deserialize_success_response() {
        let raw: RawCdpMessage = serde_json::from_str(
            r#"{"id": 1, "result": {"frameId": "abc"}}"#,
        )
        .unwrap();
        assert_eq!(raw.id, Some(1));
        assert!(raw.result.is_some());
        assert!(raw.error.is_none());
        assert!(raw.method.is_none());
    }

    #[test]
    fn deserialize_error_response() {
        let raw: RawCdpMessage = serde_json::from_str(
            r#"{"id": 2, "error": {"code": -32000, "message": "Not found"}}"#,
        )
        .unwrap();
        assert_eq!(raw.id, Some(2));
        assert!(raw.error.is_some());
        let err = raw.error.unwrap();
        assert_eq!(err.code, -32000);
        assert_eq!(err.message, "Not found");
    }

    #[test]
    fn deserialize_event() {
        let raw: RawCdpMessage = serde_json::from_str(
            r#"{"method": "Page.loadEventFired", "params": {"timestamp": 123.456}}"#,
        )
        .unwrap();
        assert!(raw.id.is_none());
        assert_eq!(raw.method.as_deref(), Some("Page.loadEventFired"));
        assert!(raw.params.is_some());
    }

    #[test]
    fn deserialize_session_scoped_event() {
        let raw: RawCdpMessage = serde_json::from_str(
            r#"{"method": "DOM.documentUpdated", "params": {}, "sessionId": "sess-1"}"#,
        )
        .unwrap();
        assert_eq!(raw.session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn deserialize_session_scoped_response() {
        let raw: RawCdpMessage = serde_json::from_str(
            r#"{"id": 5, "result": {}, "sessionId": "sess-2"}"#,
        )
        .unwrap();
        assert_eq!(raw.id, Some(5));
        assert_eq!(raw.session_id.as_deref(), Some("sess-2"));
    }

    // --- classify() ---

    #[test]
    fn classify_response() {
        let raw: RawCdpMessage =
            serde_json::from_str(r#"{"id": 1, "result": {"ok": true}}"#).unwrap();
        let kind = raw.classify();
        assert!(matches!(kind, Some(MessageKind::Response(_))));
        if let Some(MessageKind::Response(resp)) = kind {
            assert_eq!(resp.id, 1);
            assert!(resp.result.is_ok());
        }
    }

    #[test]
    fn classify_error_response() {
        let raw: RawCdpMessage = serde_json::from_str(
            r#"{"id": 2, "error": {"code": -32600, "message": "Invalid request"}}"#,
        )
        .unwrap();
        let kind = raw.classify();
        assert!(matches!(kind, Some(MessageKind::Response(_))));
        if let Some(MessageKind::Response(resp)) = kind {
            assert_eq!(resp.id, 2);
            assert!(resp.result.is_err());
            let err = resp.result.unwrap_err();
            assert_eq!(err.code, -32600);
        }
    }

    #[test]
    fn classify_event() {
        let raw: RawCdpMessage = serde_json::from_str(
            r#"{"method": "Network.requestWillBeSent", "params": {"requestId": "r1"}}"#,
        )
        .unwrap();
        let kind = raw.classify();
        assert!(matches!(kind, Some(MessageKind::Event(_))));
        if let Some(MessageKind::Event(event)) = kind {
            assert_eq!(event.method, "Network.requestWillBeSent");
            assert_eq!(event.params["requestId"], "r1");
        }
    }

    #[test]
    fn classify_unclassifiable_returns_none() {
        let raw: RawCdpMessage = serde_json::from_str(r"{}").unwrap();
        assert!(raw.classify().is_none());
    }

    #[test]
    fn classify_response_without_result_yields_null() {
        let raw: RawCdpMessage = serde_json::from_str(r#"{"id": 10}"#).unwrap();
        if let Some(MessageKind::Response(resp)) = raw.classify() {
            assert_eq!(resp.result.unwrap(), Value::Null);
        } else {
            panic!("expected response");
        }
    }

    #[test]
    fn classify_event_without_params_yields_null() {
        let raw: RawCdpMessage =
            serde_json::from_str(r#"{"method": "Page.frameNavigated"}"#).unwrap();
        if let Some(MessageKind::Event(event)) = raw.classify() {
            assert_eq!(event.params, Value::Null);
        } else {
            panic!("expected event");
        }
    }

    // --- Message ID ---

    #[test]
    fn message_ids_are_unique_and_monotonic() {
        use std::sync::atomic::{AtomicU64, Ordering};
        // Mirrors the pattern used by TransportHandle::next_message_id
        let counter = AtomicU64::new(1);
        let id1 = counter.fetch_add(1, Ordering::Relaxed);
        let id2 = counter.fetch_add(1, Ordering::Relaxed);
        let id3 = counter.fetch_add(1, Ordering::Relaxed);
        assert!(id2 > id1);
        assert!(id3 > id2);
    }
}
