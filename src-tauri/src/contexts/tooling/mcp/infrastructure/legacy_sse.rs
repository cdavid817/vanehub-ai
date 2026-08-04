use super::legacy_sse_model::{reqwest_error, runtime_error, LegacySseError, LegacySseStatus};
use super::sse_parser::{BoundedSseParser, SseEvent};
use crate::contexts::tooling::mcp::application::{McpExecutionControl, McpRuntimeError};
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use http::header::{ACCEPT, CONTENT_TYPE};
use http::{HeaderMap, HeaderValue};
use rmcp::model::ServerJsonRpcMessage;
use rmcp::service::{RxJsonRpcMessage, TxJsonRpcMessage};
use rmcp::transport::Transport;
use rmcp::RoleClient;
use std::collections::VecDeque;
use std::future::Future;
use std::time::Duration;
use url::Url;

const CONTROL_POLL: Duration = Duration::from_millis(10);

pub(super) struct LegacySseTransport {
    client: reqwest::Client,
    headers: HeaderMap,
    endpoint: Url,
    response: Option<reqwest::Response>,
    parser: BoundedSseParser,
    pending: VecDeque<ServerJsonRpcMessage>,
    control: McpExecutionControl,
    status: LegacySseStatus,
}

impl LegacySseTransport {
    pub(super) async fn connect(
        client: reqwest::Client,
        stream_url: Url,
        mut headers: HeaderMap,
        control: McpExecutionControl,
        maximum_event_bytes: usize,
    ) -> Result<(Self, LegacySseStatus), McpRuntimeError> {
        headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));
        let response = wait_controlled(
            &control,
            client
                .get(stream_url.clone())
                .headers(headers.clone())
                .send(),
        )
        .await
        .map_err(runtime_error)?
        .map_err(|error| runtime_error(reqwest_error(error)))?;
        validate_event_stream_response(&response)?;
        let status = LegacySseStatus::default();
        let mut transport = Self {
            client,
            headers,
            endpoint: stream_url.clone(),
            response: Some(response),
            parser: BoundedSseParser::new(maximum_event_bytes),
            pending: VecDeque::new(),
            control,
            status: status.clone(),
        };
        transport.endpoint = transport.negotiate_endpoint(&stream_url).await?;
        Ok((transport, status))
    }

    async fn negotiate_endpoint(&mut self, stream_url: &Url) -> Result<Url, McpRuntimeError> {
        loop {
            let events = self.next_events().await.map_err(runtime_error)?;
            for event in events {
                if event.event_type.as_deref() == Some("endpoint") {
                    let endpoint = std::str::from_utf8(&event.data)
                        .map_err(|_| McpRuntimeError::new(McpFailureCode::Protocol))?;
                    let endpoint = stream_url
                        .join(endpoint)
                        .map_err(|_| McpRuntimeError::new(McpFailureCode::Protocol))?;
                    if !same_origin(stream_url, &endpoint) {
                        return Err(McpRuntimeError::new(McpFailureCode::Protocol));
                    }
                    return Ok(endpoint);
                }
                if let Some(message) = decode_message(event)? {
                    self.pending.push_back(message);
                }
            }
        }
    }

    async fn next_events(&mut self) -> Result<Vec<SseEvent>, LegacySseError> {
        let control = self.control.clone();
        let response = self.response.as_mut().ok_or(LegacySseError::Transport)?;
        let chunk = wait_controlled(&control, response.chunk())
            .await?
            .map_err(reqwest_error)?
            .ok_or(LegacySseError::Transport)?;
        self.parser
            .feed(&chunk)
            .map_err(|error| match error.code() {
                McpFailureCode::LimitExceeded => LegacySseError::LimitExceeded,
                _ => LegacySseError::Protocol,
            })
    }
}

impl Transport<RoleClient> for LegacySseTransport {
    type Error = LegacySseError;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleClient>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let client = self.client.clone();
        let endpoint = self.endpoint.clone();
        let mut headers = self.headers.clone();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let control = self.control.clone();
        let status = self.status.clone();
        async move {
            let result = async {
                let response = wait_controlled(
                    &control,
                    client.post(endpoint).headers(headers).json(&item).send(),
                )
                .await?
                .map_err(reqwest_error)?;
                if response.status().is_success() {
                    Ok(())
                } else {
                    Err(LegacySseError::UpstreamHttp)
                }
            }
            .await;
            if let Err(error) = &result {
                status.record(error);
            }
            result
        }
    }

    async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleClient>> {
        if let Some(message) = self.pending.pop_front() {
            return Some(message);
        }
        loop {
            let events = match self.next_events().await {
                Ok(events) => events,
                Err(error) => {
                    self.status.record(&error);
                    return None;
                }
            };
            for event in events {
                match decode_message(event) {
                    Ok(Some(message)) => return Some(message),
                    Ok(None) => {}
                    Err(error) => {
                        self.status.record(&LegacySseError::Protocol);
                        let _ = error;
                        return None;
                    }
                }
            }
        }
    }

    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.response.take();
        std::future::ready(Ok(()))
    }
}

fn decode_message(event: SseEvent) -> Result<Option<ServerJsonRpcMessage>, McpRuntimeError> {
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
        .map_err(|_| McpRuntimeError::new(McpFailureCode::Protocol))
}

fn validate_event_stream_response(response: &reqwest::Response) -> Result<(), McpRuntimeError> {
    if !response.status().is_success() {
        return Err(McpRuntimeError::new(McpFailureCode::UpstreamHttp));
    }
    let valid = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("text/event-stream"));
    if valid {
        Ok(())
    } else {
        Err(McpRuntimeError::new(McpFailureCode::Protocol))
    }
}

async fn wait_controlled<F: Future>(
    control: &McpExecutionControl,
    future: F,
) -> Result<F::Output, LegacySseError> {
    tokio::pin!(future);
    loop {
        if control.is_cancelled() {
            return Err(LegacySseError::Cancelled);
        }
        let remaining = control
            .deadline_remaining()
            .map_err(|_| LegacySseError::Timeout)?;
        match tokio::time::timeout(CONTROL_POLL.min(remaining), &mut future).await {
            Ok(result) => return Ok(result),
            Err(_) => continue,
        }
    }
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}
