use serde_json::Value;
use std::collections::HashMap;
use std::hash::Hash;
use std::io::{self, BufRead};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RelayDirection {
    ParentToUpstream,
    UpstreamToParent,
}

impl RelayDirection {
    fn opposite(self) -> Self {
        match self {
            Self::ParentToUpstream => Self::UpstreamToParent,
            Self::UpstreamToParent => Self::ParentToUpstream,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) enum JsonRpcId {
    String(String),
    Number(String),
    Null,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum JsonRpcFrame {
    Request { id: JsonRpcId, method: String },
    Notification { method: String },
    Response { id: JsonRpcId, success: bool },
}

#[derive(Debug)]
pub(super) struct PendingRequest<T> {
    started_at: Instant,
    pub(super) token: T,
}

pub(super) struct CorrelatedRequest<T> {
    pub(super) direction: RelayDirection,
    pub(super) id: JsonRpcId,
    pub(super) pending: PendingRequest<T>,
}

pub(super) struct JsonRpcCorrelation<T> {
    parent_to_upstream: HashMap<JsonRpcId, PendingRequest<T>>,
    upstream_to_parent: HashMap<JsonRpcId, PendingRequest<T>>,
    closed: bool,
}

impl<T> Default for JsonRpcCorrelation<T> {
    fn default() -> Self {
        Self {
            parent_to_upstream: HashMap::new(),
            upstream_to_parent: HashMap::new(),
            closed: false,
        }
    }
}

impl<T> JsonRpcCorrelation<T> {
    pub(super) fn insert_request(
        &mut self,
        direction: RelayDirection,
        id: JsonRpcId,
        token: T,
    ) -> Result<(), PendingRequest<T>> {
        let pending = PendingRequest {
            started_at: Instant::now(),
            token,
        };
        if self.closed {
            return Err(pending);
        }
        let requests = self.requests_mut(direction);
        if requests.contains_key(&id) {
            return Err(pending);
        }
        requests.insert(id, pending);
        Ok(())
    }

    pub(super) fn complete_response(
        &mut self,
        response_direction: RelayDirection,
        id: &JsonRpcId,
    ) -> Option<PendingRequest<T>> {
        self.requests_mut(response_direction.opposite()).remove(id)
    }

    pub(super) fn abort_request(
        &mut self,
        direction: RelayDirection,
        id: &JsonRpcId,
    ) -> Option<PendingRequest<T>> {
        self.requests_mut(direction).remove(id)
    }

    pub(super) fn oldest_deadline(&self, timeout: Duration) -> Option<Instant> {
        self.oldest_key()
            .map(|(_, _, started_at)| started_at + timeout)
    }

    pub(super) fn take_expired(
        &mut self,
        now: Instant,
        timeout: Duration,
    ) -> Option<CorrelatedRequest<T>> {
        let (direction, id, started_at) = self.oldest_key()?;
        if now < started_at + timeout {
            return None;
        }
        let pending = self.requests_mut(direction).remove(&id)?;
        Some(CorrelatedRequest {
            direction,
            id,
            pending,
        })
    }

    #[cfg(test)]
    pub(super) fn close_and_drain(&mut self) -> Vec<PendingRequest<T>> {
        self.closed = true;
        self.parent_to_upstream
            .drain()
            .chain(self.upstream_to_parent.drain())
            .map(|(_, pending)| pending)
            .collect()
    }

    pub(super) fn close_and_drain_correlated(&mut self) -> Vec<CorrelatedRequest<T>> {
        self.closed = true;
        self.parent_to_upstream
            .drain()
            .map(|(id, pending)| CorrelatedRequest {
                direction: RelayDirection::ParentToUpstream,
                id,
                pending,
            })
            .chain(
                self.upstream_to_parent
                    .drain()
                    .map(|(id, pending)| CorrelatedRequest {
                        direction: RelayDirection::UpstreamToParent,
                        id,
                        pending,
                    }),
            )
            .collect()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.parent_to_upstream.is_empty() && self.upstream_to_parent.is_empty()
    }

    #[cfg(test)]
    pub(super) fn pending_count(&self) -> usize {
        self.parent_to_upstream.len() + self.upstream_to_parent.len()
    }

    fn requests_mut(
        &mut self,
        direction: RelayDirection,
    ) -> &mut HashMap<JsonRpcId, PendingRequest<T>> {
        match direction {
            RelayDirection::ParentToUpstream => &mut self.parent_to_upstream,
            RelayDirection::UpstreamToParent => &mut self.upstream_to_parent,
        }
    }

    fn oldest_key(&self) -> Option<(RelayDirection, JsonRpcId, Instant)> {
        let parent = self
            .parent_to_upstream
            .iter()
            .map(|(id, pending)| (RelayDirection::ParentToUpstream, id, pending.started_at));
        let upstream = self
            .upstream_to_parent
            .iter()
            .map(|(id, pending)| (RelayDirection::UpstreamToParent, id, pending.started_at));
        parent
            .chain(upstream)
            .min_by_key(|(_, _, started_at)| *started_at)
            .map(|(direction, id, started_at)| (direction, id.clone(), started_at))
    }
}

impl JsonRpcId {
    pub(super) fn to_value(&self) -> Value {
        match self {
            Self::String(value) => Value::String(value.clone()),
            Self::Number(value) => value
                .parse::<serde_json::Number>()
                .map(Value::Number)
                .unwrap_or(Value::Null),
            Self::Null => Value::Null,
        }
    }
}

pub(super) fn parse_json_rpc_frame(bytes: &[u8]) -> Result<JsonRpcFrame, String> {
    let value = serde_json::from_slice::<Value>(bytes)
        .map_err(|_| "relay received invalid JSON-RPC".to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "relay received a non-object JSON-RPC frame".to_string())?;
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err("relay received an unsupported JSON-RPC version".to_string());
    }

    let method = match object.get("method") {
        Some(Value::String(method)) => Some(method.clone()),
        Some(_) => return Err("relay received an invalid JSON-RPC method".to_string()),
        None => None,
    };
    let id = object.get("id").map(parse_id).transpose()?;
    match (method, id) {
        (Some(method), Some(id)) => Ok(JsonRpcFrame::Request { id, method }),
        (Some(method), None) => Ok(JsonRpcFrame::Notification { method }),
        (None, Some(id)) => {
            let has_result = object.contains_key("result");
            let has_error = object.contains_key("error");
            if has_result == has_error {
                return Err("relay received an invalid JSON-RPC response".to_string());
            }
            Ok(JsonRpcFrame::Response {
                id,
                success: has_result,
            })
        }
        (None, None) => Err("relay received an unclassified JSON-RPC frame".to_string()),
    }
}

pub(super) fn read_bounded_frame(
    source: &mut impl BufRead,
    frame: &mut Vec<u8>,
    maximum: usize,
) -> io::Result<usize> {
    frame.clear();
    loop {
        let available = source.fill_buf()?;
        if available.is_empty() {
            return Ok(frame.len());
        }
        let remaining = maximum.saturating_sub(frame.len());
        let inspected = available.len().min(remaining.saturating_add(1));
        let chunk = &available[..inspected];
        if let Some(index) = chunk.iter().position(|byte| *byte == b'\n') {
            frame.extend_from_slice(&chunk[..=index]);
            source.consume(index + 1);
            return Ok(frame.len());
        }
        frame.extend_from_slice(chunk);
        source.consume(inspected);
        if frame.len() > maximum {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "MCP stdio frame exceeded byte limit",
            ));
        }
    }
}

fn parse_id(value: &Value) -> Result<JsonRpcId, String> {
    match value {
        Value::String(value) => Ok(JsonRpcId::String(value.clone())),
        Value::Number(value) => Ok(JsonRpcId::Number(value.to_string())),
        Value::Null => Ok(JsonRpcId::Null),
        _ => Err("relay received an invalid JSON-RPC id".to_string()),
    }
}

#[cfg(test)]
#[path = "relay_jsonrpc_tests.rs"]
mod tests;
