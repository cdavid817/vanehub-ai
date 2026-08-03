use super::relay_failure::RelayFailure;
use super::relay_jsonrpc::{parse_json_rpc_frame, JsonRpcFrame, JsonRpcId};
use super::sse_parser::SseEvent;
use crate::contexts::tooling::mcp::application::McpLimits;
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use reqwest::blocking::Response;
use reqwest::header::CONTENT_LENGTH;
use serde_json::Value;
use std::io::{Read, Write};

pub(super) fn read_bounded_body(response: Response) -> Result<Vec<u8>, RelayFailure> {
    let maximum = McpLimits::DEFAULT.protocol_message_bytes;
    let mut body = Vec::new();
    response
        .take((maximum + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|error| {
            RelayFailure::new(if error.kind() == std::io::ErrorKind::TimedOut {
                McpFailureCode::Timeout
            } else {
                McpFailureCode::Transport
            })
        })?;
    if body.len() > maximum {
        Err(RelayFailure::new(McpFailureCode::LimitExceeded))
    } else {
        Ok(body)
    }
}

pub(super) fn reject_oversized_content_length(response: &Response) -> Result<(), RelayFailure> {
    let oversized = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|length| length > McpLimits::DEFAULT.protocol_message_bytes);
    if oversized {
        Err(RelayFailure::new(McpFailureCode::LimitExceeded))
    } else {
        Ok(())
    }
}

pub(super) fn decode_sse_event(
    event: SseEvent,
) -> Result<Option<(JsonRpcFrame, Vec<u8>)>, RelayFailure> {
    if event.data.is_empty()
        || event
            .event_type
            .as_deref()
            .is_some_and(|event_type| event_type != "message")
    {
        return Ok(None);
    }
    let frame = parse_json_rpc_frame(&event.data)
        .map_err(|_| RelayFailure::new(McpFailureCode::Protocol))?;
    Ok(Some((frame, event.data)))
}

pub(super) fn write_json_line(output: &mut impl Write, bytes: &[u8]) -> Result<(), RelayFailure> {
    let value = serde_json::from_slice::<Value>(bytes)
        .map_err(|_| RelayFailure::new(McpFailureCode::Protocol))?;
    serde_json::to_writer(&mut *output, &value)
        .map_err(|_| RelayFailure::new(McpFailureCode::Transport))?;
    output
        .write_all(b"\n")
        .and_then(|()| output.flush())
        .map_err(|_| RelayFailure::new(McpFailureCode::Transport))
}

pub(super) fn expects_response(frame: &JsonRpcFrame) -> bool {
    matches!(frame, JsonRpcFrame::Request { .. })
}

pub(super) fn request_id(frame: &JsonRpcFrame) -> Option<JsonRpcId> {
    match frame {
        JsonRpcFrame::Request { id, .. } => Some(id.clone()),
        _ => None,
    }
}

pub(super) fn response_matches(expected: Option<&JsonRpcId>, frame: &JsonRpcFrame) -> bool {
    matches!((expected, frame), (Some(expected), JsonRpcFrame::Response { id, .. }) if expected == id)
}

pub(super) fn require_matching_response(
    outbound: &JsonRpcFrame,
    inbound: &JsonRpcFrame,
) -> Result<(), RelayFailure> {
    let Some(expected) = request_id(outbound) else {
        return Ok(());
    };
    if response_matches(Some(&expected), inbound) {
        Ok(())
    } else {
        Err(RelayFailure::new(McpFailureCode::Protocol))
    }
}
