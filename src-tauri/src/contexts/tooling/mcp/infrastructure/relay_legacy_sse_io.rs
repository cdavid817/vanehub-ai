use super::relay_failure::RelayFailure;
use super::relay_jsonrpc::{parse_json_rpc_frame, read_bounded_frame, JsonRpcFrame};
use super::sse_parser::BoundedSseParser;
use crate::contexts::tooling::mcp::application::McpLimits;
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use std::io::BufRead;
use std::thread::{self, JoinHandle};
use tokio::sync::mpsc;

pub(super) enum LegacyRelayEvent {
    Endpoint(Vec<u8>),
    Message(Vec<u8>, JsonRpcFrame),
    SseFailure(RelayFailure),
    ParentFrame(Vec<u8>, JsonRpcFrame),
    ParentEof,
    ParentFailure(RelayFailure),
}

pub(super) fn spawn_sse_pump(
    mut response: reqwest::Response,
    events: mpsc::UnboundedSender<LegacyRelayEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut parser = BoundedSseParser::new(McpLimits::DEFAULT.protocol_message_bytes);
        loop {
            let chunk = match response.chunk().await {
                Ok(Some(chunk)) => chunk,
                Ok(None) => {
                    let _ = events.send(LegacyRelayEvent::SseFailure(RelayFailure::new(
                        McpFailureCode::Transport,
                    )));
                    return;
                }
                Err(error) => {
                    let _ = events.send(LegacyRelayEvent::SseFailure(RelayFailure::from_reqwest(
                        &error,
                    )));
                    return;
                }
            };
            let parsed = match parser.feed(&chunk) {
                Ok(parsed) => parsed,
                Err(error) => {
                    let _ = events.send(LegacyRelayEvent::SseFailure(error.into()));
                    return;
                }
            };
            for event in parsed {
                if event.event_type.as_deref() == Some("endpoint") {
                    let _ = events.send(LegacyRelayEvent::Endpoint(event.data));
                    continue;
                }
                if event.data.is_empty()
                    || event
                        .event_type
                        .as_deref()
                        .is_some_and(|event_type| event_type != "message")
                {
                    continue;
                }
                let frame = match parse_json_rpc_frame(&event.data) {
                    Ok(frame) => frame,
                    Err(_) => {
                        let _ = events.send(LegacyRelayEvent::SseFailure(RelayFailure::new(
                            McpFailureCode::Protocol,
                        )));
                        return;
                    }
                };
                let _ = events.send(LegacyRelayEvent::Message(event.data, frame));
            }
        }
    })
}

pub(super) fn spawn_parent_pump<R>(
    mut input: R,
    events: mpsc::UnboundedSender<LegacyRelayEvent>,
) -> JoinHandle<()>
where
    R: BufRead + Send + 'static,
{
    thread::spawn(move || {
        let mut bytes = Vec::new();
        loop {
            let count = match read_bounded_frame(
                &mut input,
                &mut bytes,
                McpLimits::DEFAULT.protocol_message_bytes,
            ) {
                Ok(count) => count,
                Err(error) => {
                    let code = if error.kind() == std::io::ErrorKind::InvalidData {
                        McpFailureCode::LimitExceeded
                    } else {
                        McpFailureCode::Transport
                    };
                    let _ = events.send(LegacyRelayEvent::ParentFailure(RelayFailure::new(code)));
                    return;
                }
            };
            if count == 0 {
                let _ = events.send(LegacyRelayEvent::ParentEof);
                return;
            }
            let frame = match parse_json_rpc_frame(&bytes) {
                Ok(frame) => frame,
                Err(_) => {
                    let _ = events.send(LegacyRelayEvent::ParentFailure(RelayFailure::new(
                        McpFailureCode::Protocol,
                    )));
                    return;
                }
            };
            if events
                .send(LegacyRelayEvent::ParentFrame(bytes.clone(), frame))
                .is_err()
            {
                return;
            }
        }
    })
}
