use super::sse_parser::{BoundedSseParser, SseEvent};
use super::streamable_http::StreamableHttpLease;
use super::streamable_http_model::{reqwest_error, StreamableHttpError};
use crate::contexts::tooling::mcp::application::McpExecutionControl;
use rmcp::model::ServerJsonRpcMessage;
use rmcp::service::TxJsonRpcMessage;
use rmcp::RoleClient;
use std::future::Future;
use std::time::Duration;
use tokio::sync::mpsc;

const CONTROL_POLL: Duration = Duration::from_millis(10);

pub(super) async fn read_sse_response(
    mut response: reqwest::Response,
    control: &McpExecutionControl,
    maximum: usize,
    request_id: Option<serde_json::Value>,
    lease: &StreamableHttpLease,
    sender: mpsc::Sender<ServerJsonRpcMessage>,
) -> Result<(), StreamableHttpError> {
    let mut parser = BoundedSseParser::new(maximum);
    let mut observed = 0_usize;
    while let Some(chunk) = wait_controlled(control, response.chunk())
        .await?
        .map_err(reqwest_error)?
    {
        if observed.saturating_add(chunk.len()) > maximum {
            return Err(StreamableHttpError::LimitExceeded);
        }
        observed += chunk.len();
        for event in parser.feed(&chunk).map_err(|error| match error.code() {
            crate::contexts::tooling::mcp::domain::McpFailureCode::LimitExceeded => {
                StreamableHttpError::LimitExceeded
            }
            _ => StreamableHttpError::Protocol,
        })? {
            let Some(message) = decode_event(event)? else {
                continue;
            };
            let matched = request_id
                .as_ref()
                .is_some_and(|request_id| message_id(&message).as_ref() == Some(request_id));
            lease.observe_protocol_version(&message).await?;
            sender
                .send(message)
                .await
                .map_err(|_| StreamableHttpError::Transport)?;
            if matched {
                return Ok(());
            }
        }
    }
    if request_id.is_some() {
        Err(StreamableHttpError::Protocol)
    } else {
        Ok(())
    }
}

fn decode_event(event: SseEvent) -> Result<Option<ServerJsonRpcMessage>, StreamableHttpError> {
    if event.data.is_empty()
        || event
            .event_type
            .as_deref()
            .is_some_and(|event_type| event_type != "message")
    {
        return Ok(None);
    }
    serde_json::from_slice(&event.data)
        .map(Some)
        .map_err(|_| StreamableHttpError::Protocol)
}

pub(super) fn request_id(item: &TxJsonRpcMessage<RoleClient>) -> Option<serde_json::Value> {
    message_id(item)
}

fn message_id(message: &impl serde::Serialize) -> Option<serde_json::Value> {
    serde_json::to_value(message).ok()?.get("id").cloned()
}

pub(super) async fn wait_controlled<F: Future>(
    control: &McpExecutionControl,
    future: F,
) -> Result<F::Output, StreamableHttpError> {
    tokio::pin!(future);
    loop {
        if control.is_cancelled() {
            return Err(StreamableHttpError::Cancelled);
        }
        let remaining = control
            .deadline_remaining()
            .map_err(|_| StreamableHttpError::Timeout)?;
        match tokio::time::timeout(CONTROL_POLL.min(remaining), &mut future).await {
            Ok(result) => return Ok(result),
            Err(_) => continue,
        }
    }
}
