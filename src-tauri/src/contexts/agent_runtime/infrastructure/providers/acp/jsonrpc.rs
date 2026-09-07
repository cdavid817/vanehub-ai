//! JSON-RPC 2.0 message model for the ACP stdio transport.
//!
//! Deliberately a strict minimum: request, notification, response, error. Wire payloads are kept
//! as `serde_json::Value` because the manifest's `deny_unknown_fields` discipline is the wrong
//! tool here -- an agent may legitimately attach `_meta` or vendor fields, and refusing them would
//! be a compatibility bug, not a safety gain.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;

#[cfg(test)]
pub(crate) const PARSE_ERROR: i64 = -32700;
pub(crate) const INVALID_REQUEST: i64 = -32600;
pub(crate) const METHOD_NOT_FOUND: i64 = -32601;
pub(crate) const INVALID_PARAMS: i64 = -32602;
#[cfg(test)]
pub(crate) const INTERNAL_ERROR: i64 = -32603;
/// ACP's cancelled-request error, sent when a pending request is answered by cancellation.
pub(crate) const REQUEST_CANCELLED: i64 = -32800;
/// Host refusal of an authorized operation: policy denied, path outside roots, and the like.
pub(crate) const HOST_REFUSED: i64 = -32000;

/// A JSON-RPC id. Numbers and strings are both legal; the host only ever mints numbers, but an
/// agent may use either and the reply must echo exactly what arrived.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum RpcId {
    Number(u64),
    Text(String),
}

impl fmt::Display for RpcId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(value) => write!(formatter, "{value}"),
            Self::Text(value) => formatter.write_str(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RpcError {
    pub(crate) code: i64,
    pub(crate) message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<Value>,
}

impl RpcError {
    pub(crate) fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub(crate) fn method_not_found(method: &str) -> Self {
        Self::new(METHOD_NOT_FOUND, format!("method not found: {method}"))
    }

    pub(crate) fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, message)
    }

    pub(crate) fn refused(message: impl Into<String>) -> Self {
        Self::new(HOST_REFUSED, message)
    }

    pub(crate) fn cancelled() -> Self {
        Self::new(REQUEST_CANCELLED, "request cancelled")
    }
}

impl fmt::Display for RpcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({})", self.message, self.code)
    }
}

/// One decoded inbound frame.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InboundMessage {
    Request {
        id: RpcId,
        method: String,
        params: Value,
    },
    Notification {
        method: String,
        params: Value,
    },
    Response {
        id: RpcId,
        outcome: Result<Value, RpcError>,
    },
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    jsonrpc: Option<String>,
    #[serde(default)]
    id: Option<Value>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    params: Option<Value>,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<RpcError>,
}

/// Classifies one JSON document as a request, notification, or response.
///
/// A frame that is valid JSON but not a JSON-RPC message is a protocol violation: the transport
/// contract says nothing but ACP messages may appear on stdout, so a banner or a stray log line
/// fails the connection rather than becoming assistant text.
pub(crate) fn classify(document: &Value) -> Result<InboundMessage, RpcError> {
    let raw: RawMessage = serde_json::from_value(document.clone())
        .map_err(|error| RpcError::new(INVALID_REQUEST, format!("malformed message: {error}")))?;
    if raw.jsonrpc.as_deref() != Some("2.0") {
        return Err(RpcError::new(INVALID_REQUEST, "missing jsonrpc 2.0 marker"));
    }
    let id = match raw.id {
        None | Some(Value::Null) => None,
        Some(Value::Number(number)) => Some(RpcId::Number(number.as_u64().ok_or_else(|| {
            RpcError::new(INVALID_REQUEST, "request id must be a non-negative integer")
        })?)),
        Some(Value::String(text)) => Some(RpcId::Text(text)),
        Some(_) => {
            return Err(RpcError::new(
                INVALID_REQUEST,
                "request id must be a number or string",
            ))
        }
    };
    match (raw.method, id) {
        (Some(method), Some(id)) => Ok(InboundMessage::Request {
            id,
            method,
            params: raw.params.unwrap_or(Value::Null),
        }),
        (Some(method), None) => Ok(InboundMessage::Notification {
            method,
            params: raw.params.unwrap_or(Value::Null),
        }),
        (None, Some(id)) => {
            let outcome = match (raw.result, raw.error) {
                (_, Some(error)) => Err(error),
                (Some(result), None) => Ok(result),
                (None, None) => Ok(Value::Null),
            };
            Ok(InboundMessage::Response { id, outcome })
        }
        (None, None) => Err(RpcError::new(
            INVALID_REQUEST,
            "message has neither method nor id",
        )),
    }
}

pub(crate) fn request_document(id: u64, method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

pub(crate) fn notification_document(method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "method": method, "params": params })
}

pub(crate) fn response_document(id: &RpcId, outcome: Result<Value, RpcError>) -> Value {
    match outcome {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_requests_notifications_and_responses() {
        let request = classify(
            &json!({"jsonrpc":"2.0","id":7,"method":"session/request_permission","params":{"a":1}}),
        )
        .expect("request");
        assert_eq!(
            request,
            InboundMessage::Request {
                id: RpcId::Number(7),
                method: "session/request_permission".to_string(),
                params: json!({"a":1}),
            }
        );
        let notification =
            classify(&json!({"jsonrpc":"2.0","method":"session/update","params":{}})).expect("n");
        assert!(
            matches!(notification, InboundMessage::Notification { method, .. } if method == "session/update")
        );
        let response =
            classify(&json!({"jsonrpc":"2.0","id":"abc","result":{"stopReason":"end_turn"}}))
                .expect("response");
        assert!(matches!(
            response,
            InboundMessage::Response { id: RpcId::Text(id), outcome: Ok(_) } if id == "abc"
        ));
        let error =
            classify(&json!({"jsonrpc":"2.0","id":3,"error":{"code":-32601,"message":"nope"}}))
                .expect("error response");
        assert!(matches!(
            error,
            InboundMessage::Response { outcome: Err(error), .. } if error.code == METHOD_NOT_FOUND
        ));
    }

    #[test]
    fn rejects_non_rpc_documents() {
        assert!(classify(&json!({"type":"banner","text":"Welcome"})).is_err());
        assert!(classify(&json!({"jsonrpc":"1.0","id":1,"method":"x"})).is_err());
        assert!(classify(&json!({"jsonrpc":"2.0","id":-1,"method":"x"})).is_err());
        assert!(classify(&json!({"jsonrpc":"2.0","id":{"a":1},"method":"x"})).is_err());
        assert!(classify(&json!({"jsonrpc":"2.0"})).is_err());
        assert!(classify(&json!("just a string")).is_err());
    }

    #[test]
    fn documents_round_trip_with_exact_ids() {
        let text = response_document(&RpcId::Text("q-1".to_string()), Ok(json!({"ok": true})));
        assert_eq!(text["id"], json!("q-1"));
        let number = response_document(&RpcId::Number(4), Err(RpcError::method_not_found("x/y")));
        assert_eq!(number["id"], json!(4));
        assert_eq!(number["error"]["code"], json!(METHOD_NOT_FOUND));
        assert!(number.get("result").is_none());
        let request = request_document(9, "session/prompt", json!({"sessionId":"s"}));
        assert_eq!(request["jsonrpc"], json!("2.0"));
        let notification = notification_document("session/cancel", json!({"sessionId":"s"}));
        assert!(notification.get("id").is_none());
        assert_eq!(RpcId::Number(3).to_string(), "3");
        assert_eq!(RpcError::cancelled().code, REQUEST_CANCELLED);
        assert_eq!(RpcError::refused("x").code, HOST_REFUSED);
        assert_eq!(RpcError::invalid_params("p").code, INVALID_PARAMS);
        assert_eq!(PARSE_ERROR, -32700);
        assert_eq!(INTERNAL_ERROR, -32603);
    }
}
