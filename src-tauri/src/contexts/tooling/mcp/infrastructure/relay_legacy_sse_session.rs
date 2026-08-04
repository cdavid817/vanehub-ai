use super::relay_failure::RelayFailure;
use super::relay_jsonrpc::{
    CorrelatedRequest, JsonRpcCorrelation, JsonRpcFrame, PendingRequest, RelayDirection,
};
use super::relay_legacy_sse_io::LegacyRelayEvent;
use super::relay_observer::{RelayObserver, RelayRequest};
use super::relay_streamable_http_protocol::write_json_line;
use crate::contexts::tooling::mcp::application::McpCancellation;
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use reqwest::header::CONTENT_TYPE;
use reqwest::{Client, Method};
use std::collections::{BTreeMap, VecDeque};
use std::io::Write;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use url::Url;

const CONTROL_POLL: Duration = Duration::from_millis(10);
type Correlation = JsonRpcCorrelation<Option<RelayRequest>>;

pub(super) struct RelaySession<'a, W> {
    client: Client,
    endpoint: Url,
    headers: &'a BTreeMap<String, String>,
    traceparent: &'a str,
    request_timeout: Duration,
    correlation: Correlation,
    observer: Option<RelayObserver>,
    output: &'a mut W,
    parent_closed: bool,
}

impl<'a, W: Write> RelaySession<'a, W> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        client: Client,
        endpoint: Url,
        headers: &'a BTreeMap<String, String>,
        traceparent: &'a str,
        request_timeout: Duration,
        observer: Option<RelayObserver>,
        output: &'a mut W,
    ) -> Self {
        Self {
            client,
            endpoint,
            headers,
            traceparent,
            request_timeout,
            correlation: JsonRpcCorrelation::default(),
            observer,
            output,
            parent_closed: false,
        }
    }

    pub(super) async fn run(
        &mut self,
        receiver: &mut mpsc::UnboundedReceiver<LegacyRelayEvent>,
        mut pending: VecDeque<LegacyRelayEvent>,
        cancellation: &McpCancellation,
    ) -> Result<(), RelayFailure> {
        loop {
            if self.parent_closed && self.correlation.is_empty() {
                return Ok(());
            }
            if cancellation.is_cancelled() {
                return Err(RelayFailure::new(McpFailureCode::Cancelled));
            }
            if let Some(expired) = self
                .correlation
                .take_expired(Instant::now(), self.request_timeout)
            {
                return self.handle_timeout(expired).await;
            }
            let event = if let Some(event) = pending.pop_front() {
                event
            } else {
                match tokio::time::timeout(CONTROL_POLL, receiver.recv()).await {
                    Ok(Some(event)) => event,
                    Ok(None) => return Err(RelayFailure::new(McpFailureCode::Transport)),
                    Err(_) => continue,
                }
            };
            self.handle_event(event).await?;
        }
    }

    pub(super) async fn finish(&mut self, failure: Option<RelayFailure>) {
        let pending = self.correlation.close_and_drain_correlated();
        for request in pending {
            if let Some(failure) = failure {
                let _ = self.emit_failure(&request, failure).await;
            }
            finish_pending(
                self.observer.as_ref(),
                request.pending,
                failure.map_or("transport", RelayFailure::classification),
            );
        }
    }

    async fn handle_event(&mut self, event: LegacyRelayEvent) -> Result<(), RelayFailure> {
        match event {
            LegacyRelayEvent::ParentFrame(bytes, frame) => {
                self.handle_parent_frame(bytes, frame).await
            }
            LegacyRelayEvent::Message(bytes, frame) => self.handle_server_frame(bytes, frame),
            LegacyRelayEvent::ParentEof => {
                self.parent_closed = true;
                Ok(())
            }
            LegacyRelayEvent::ParentFailure(error) | LegacyRelayEvent::SseFailure(error) => {
                Err(error)
            }
            LegacyRelayEvent::Endpoint(_) => Err(RelayFailure::new(McpFailureCode::Protocol)),
        }
    }

    async fn handle_parent_frame(
        &mut self,
        bytes: Vec<u8>,
        frame: JsonRpcFrame,
    ) -> Result<(), RelayFailure> {
        if let JsonRpcFrame::Request { id, method } = &frame {
            let token = self
                .observer
                .as_ref()
                .and_then(|observer| observer.start_request("sse", Some(method)));
            self.correlation
                .insert_request(RelayDirection::ParentToUpstream, id.clone(), token)
                .map_err(|_| RelayFailure::new(McpFailureCode::Protocol))?;
        }
        self.post(&bytes).await?;
        if let JsonRpcFrame::Response { id, success } = frame {
            if let Some(pending) = self
                .correlation
                .complete_response(RelayDirection::ParentToUpstream, &id)
            {
                finish_response(self.observer.as_ref(), pending, success);
            }
        }
        Ok(())
    }

    fn handle_server_frame(
        &mut self,
        bytes: Vec<u8>,
        frame: JsonRpcFrame,
    ) -> Result<(), RelayFailure> {
        if let JsonRpcFrame::Request { id, method } = &frame {
            let token = self
                .observer
                .as_ref()
                .and_then(|observer| observer.start_request("sse", Some(method)));
            self.correlation
                .insert_request(RelayDirection::UpstreamToParent, id.clone(), token)
                .map_err(|_| RelayFailure::new(McpFailureCode::Protocol))?;
        }
        write_json_line(self.output, &bytes)?;
        if let JsonRpcFrame::Response { id, success } = frame {
            if let Some(pending) = self
                .correlation
                .complete_response(RelayDirection::UpstreamToParent, &id)
            {
                finish_response(self.observer.as_ref(), pending, success);
            }
        }
        Ok(())
    }

    async fn post(&self, bytes: &[u8]) -> Result<(), RelayFailure> {
        let response = tokio::time::timeout(
            self.request_timeout,
            request(
                &self.client,
                Method::POST,
                self.endpoint.clone(),
                self.headers,
                self.traceparent,
            )
            .header(CONTENT_TYPE, "application/json")
            .body(bytes.to_vec())
            .send(),
        )
        .await
        .map_err(|_| RelayFailure::new(McpFailureCode::Timeout))?
        .map_err(|error| RelayFailure::from_reqwest(&error))?;
        if response.status().is_redirection() || !response.status().is_success() {
            Err(RelayFailure::new(McpFailureCode::UpstreamHttp))
        } else {
            Ok(())
        }
    }

    async fn handle_timeout(
        &mut self,
        expired: CorrelatedRequest<Option<RelayRequest>>,
    ) -> Result<(), RelayFailure> {
        let failure = RelayFailure::new(McpFailureCode::Timeout);
        self.emit_failure(&expired, failure).await?;
        finish_pending(
            self.observer.as_ref(),
            expired.pending,
            failure.classification(),
        );
        Err(failure)
    }

    async fn emit_failure(
        &mut self,
        request: &CorrelatedRequest<Option<RelayRequest>>,
        failure: RelayFailure,
    ) -> Result<(), RelayFailure> {
        match request.direction {
            RelayDirection::ParentToUpstream => failure.write_response(self.output, &request.id),
            RelayDirection::UpstreamToParent => {
                let mut bytes = Vec::new();
                failure.write_response(&mut bytes, &request.id)?;
                self.post(&bytes).await
            }
        }
    }
}

fn request(
    client: &Client,
    method: Method,
    url: Url,
    headers: &BTreeMap<String, String>,
    traceparent: &str,
) -> reqwest::RequestBuilder {
    let mut request = client
        .request(method, url)
        .header("traceparent", traceparent);
    for (name, value) in headers {
        request = request.header(name, value);
    }
    request
}

fn finish_response(
    observer: Option<&RelayObserver>,
    pending: PendingRequest<Option<RelayRequest>>,
    success: bool,
) {
    if let (Some(observer), Some(request)) = (observer, pending.token) {
        observer.finish_request(
            request,
            success,
            (!success).then_some("mcp_legacy_sse_json_rpc_error"),
        );
    }
}

fn finish_pending(
    observer: Option<&RelayObserver>,
    pending: PendingRequest<Option<RelayRequest>>,
    classification: &'static str,
) {
    if let (Some(observer), Some(request)) = (observer, pending.token) {
        observer.finish_request(request, false, Some(classification));
    }
}
