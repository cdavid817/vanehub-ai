use super::streamable_http_model::{reqwest_error, StreamableHttpError, StreamableHttpStatus};
use super::streamable_http_response::{read_sse_response, request_id, wait_controlled};
use crate::contexts::tooling::mcp::application::{McpExecutionControl, McpRuntimeError};
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use http::header::{ACCEPT, CONTENT_LENGTH, CONTENT_TYPE};
use http::{HeaderMap, HeaderName, HeaderValue};
use rmcp::model::ServerJsonRpcMessage;
use rmcp::service::{RxJsonRpcMessage, TxJsonRpcMessage};
use rmcp::transport::Transport;
use rmcp::RoleClient;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};
use url::Url;

const RESPONSE_CHANNEL_CAPACITY: usize = 32;

struct SessionState {
    session_id: Mutex<Option<HeaderValue>>,
    protocol_version: Mutex<Option<HeaderValue>>,
    closed: AtomicBool,
    cleanup_failed: AtomicBool,
}

#[derive(Clone)]
pub(super) struct StreamableHttpLease {
    client: reqwest::Client,
    url: Url,
    headers: HeaderMap,
    state: Arc<SessionState>,
    status: StreamableHttpStatus,
}

impl StreamableHttpLease {
    async fn request_headers(&self) -> HeaderMap {
        let mut headers = self.headers.clone();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/event-stream"),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(session_id) = self.state.session_id.lock().await.clone() {
            headers.insert(session_header(), session_id);
        }
        if let Some(version) = self.state.protocol_version.lock().await.clone() {
            headers.insert(protocol_header(), version);
        }
        headers
    }

    async fn observe_response(&self, response: &reqwest::Response) {
        if let Some(session_id) = response.headers().get(session_header()).cloned() {
            *self.state.session_id.lock().await = Some(session_id);
        }
    }

    pub(super) async fn observe_protocol_version(
        &self,
        message: &ServerJsonRpcMessage,
    ) -> Result<(), StreamableHttpError> {
        let value = serde_json::to_value(message).map_err(|_| StreamableHttpError::Protocol)?;
        let Some(version) = value
            .pointer("/result/protocolVersion")
            .and_then(serde_json::Value::as_str)
        else {
            return Ok(());
        };
        let version = HeaderValue::from_str(version).map_err(|_| StreamableHttpError::Protocol)?;
        *self.state.protocol_version.lock().await = Some(version);
        Ok(())
    }

    pub(super) async fn shutdown(&self, deadline: Instant) -> Result<(), McpRuntimeError> {
        if self.state.closed.swap(true, Ordering::AcqRel) {
            return if self.state.cleanup_failed.load(Ordering::Acquire) {
                Err(McpRuntimeError::new(McpFailureCode::Cleanup))
            } else {
                Ok(())
            };
        }
        let Some(session_id) = self.state.session_id.lock().await.clone() else {
            return Ok(());
        };
        let mut headers = self.headers.clone();
        headers.insert(session_header(), session_id);
        let control = McpExecutionControl::with_deadline(deadline);
        let result = wait_controlled(
            &control,
            self.client.delete(self.url.clone()).headers(headers).send(),
        )
        .await
        .and_then(|response| response.map_err(reqwest_error))
        .and_then(|response| {
            if response.status().is_success() {
                Ok(())
            } else {
                Err(StreamableHttpError::Cleanup)
            }
        });
        if result.is_err() {
            self.state.cleanup_failed.store(true, Ordering::Release);
            self.status.record(&StreamableHttpError::Cleanup);
            return Err(McpRuntimeError::new(McpFailureCode::Cleanup));
        }
        Ok(())
    }
}

pub(super) struct BoundedStreamableHttpTransport {
    lease: StreamableHttpLease,
    control: McpExecutionControl,
    maximum_body_bytes: usize,
    sender: mpsc::Sender<ServerJsonRpcMessage>,
    receiver: mpsc::Receiver<ServerJsonRpcMessage>,
}

impl BoundedStreamableHttpTransport {
    pub(super) fn new(
        client: reqwest::Client,
        url: Url,
        headers: HeaderMap,
        control: McpExecutionControl,
        maximum_body_bytes: usize,
    ) -> (Self, StreamableHttpStatus, StreamableHttpLease) {
        let status = StreamableHttpStatus::default();
        let lease = StreamableHttpLease {
            client,
            url,
            headers,
            state: Arc::new(SessionState {
                session_id: Mutex::new(None),
                protocol_version: Mutex::new(None),
                closed: AtomicBool::new(false),
                cleanup_failed: AtomicBool::new(false),
            }),
            status: status.clone(),
        };
        let (sender, receiver) = mpsc::channel(RESPONSE_CHANNEL_CAPACITY);
        (
            Self {
                lease: lease.clone(),
                control,
                maximum_body_bytes,
                sender,
                receiver,
            },
            status,
            lease,
        )
    }
}

impl Transport<RoleClient> for BoundedStreamableHttpTransport {
    type Error = StreamableHttpError;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleClient>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let lease = self.lease.clone();
        let control = self.control.clone();
        let maximum = self.maximum_body_bytes;
        let sender = self.sender.clone();
        async move {
            let result = post_message(&lease, &control, maximum, sender, item).await;
            if let Err(error) = &result {
                lease.status.record(error);
            }
            result
        }
    }

    async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleClient>> {
        self.receiver.recv().await
    }

    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let lease = self.lease.clone();
        let deadline = Instant::now() + self.control.deadline_remaining().unwrap_or(Duration::ZERO);
        async move {
            lease
                .shutdown(deadline)
                .await
                .map_err(|_| StreamableHttpError::Cleanup)
        }
    }
}

async fn post_message(
    lease: &StreamableHttpLease,
    control: &McpExecutionControl,
    maximum: usize,
    sender: mpsc::Sender<ServerJsonRpcMessage>,
    item: TxJsonRpcMessage<RoleClient>,
) -> Result<(), StreamableHttpError> {
    let request_id = request_id(&item);
    let response = wait_controlled(
        control,
        lease
            .client
            .post(lease.url.clone())
            .headers(lease.request_headers().await)
            .json(&item)
            .send(),
    )
    .await?
    .map_err(reqwest_error)?;
    lease.observe_response(&response).await;
    if response.status() == reqwest::StatusCode::ACCEPTED {
        return Ok(());
    }
    if !response.status().is_success() {
        return Err(StreamableHttpError::UpstreamHttp);
    }
    reject_oversized_content_length(&response, maximum)?;
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if content_type.starts_with("application/json") {
        let body = read_bounded_body(response, control, maximum).await?;
        let message = serde_json::from_slice(&body).map_err(|_| StreamableHttpError::Protocol)?;
        lease.observe_protocol_version(&message).await?;
        sender
            .send(message)
            .await
            .map_err(|_| StreamableHttpError::Transport)
    } else if content_type.starts_with("text/event-stream") {
        read_sse_response(response, control, maximum, request_id, lease, sender).await
    } else {
        Err(StreamableHttpError::Protocol)
    }
}

fn reject_oversized_content_length(
    response: &reqwest::Response,
    maximum: usize,
) -> Result<(), StreamableHttpError> {
    let oversized = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > maximum as u64);
    if oversized {
        Err(StreamableHttpError::LimitExceeded)
    } else {
        Ok(())
    }
}

async fn read_bounded_body(
    mut response: reqwest::Response,
    control: &McpExecutionControl,
    maximum: usize,
) -> Result<Vec<u8>, StreamableHttpError> {
    let mut body = Vec::new();
    while let Some(chunk) = wait_controlled(control, response.chunk())
        .await?
        .map_err(reqwest_error)?
    {
        if body.len().saturating_add(chunk.len()) > maximum {
            return Err(StreamableHttpError::LimitExceeded);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn session_header() -> HeaderName {
    HeaderName::from_static("mcp-session-id")
}

fn protocol_header() -> HeaderName {
    HeaderName::from_static("mcp-protocol-version")
}
