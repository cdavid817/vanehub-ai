use super::relay_failure::RelayFailure;
use super::relay_legacy_sse_io::{spawn_parent_pump, spawn_sse_pump, LegacyRelayEvent};
use super::relay_legacy_sse_session::RelaySession;
use super::relay_observer::RelayObserver;
use crate::contexts::tooling::mcp::application::McpCancellation;
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use reqwest::{Client, Method};
use std::collections::{BTreeMap, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;
use tokio::sync::mpsc;
use url::Url;

pub(super) fn run(
    stream_url: &str,
    headers: &BTreeMap<String, String>,
    traceparent: &str,
    request_timeout: Duration,
    cancellation: McpCancellation,
    observer: Option<RelayObserver>,
) -> Result<(), RelayFailure> {
    run_stream(
        stream_url,
        headers,
        traceparent,
        request_timeout,
        cancellation,
        observer,
        BufReader::new(std::io::stdin()),
        &mut std::io::stdout(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_stream<R, W>(
    stream_url: &str,
    headers: &BTreeMap<String, String>,
    traceparent: &str,
    request_timeout: Duration,
    cancellation: McpCancellation,
    observer: Option<RelayObserver>,
    input: R,
    output: &mut W,
) -> Result<(), RelayFailure>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| RelayFailure::new(McpFailureCode::Transport))?
        .block_on(run_async(
            stream_url,
            headers,
            traceparent,
            request_timeout.max(Duration::from_millis(1)),
            cancellation,
            observer,
            input,
            output,
        ))
}

#[allow(clippy::too_many_arguments)]
async fn run_async<R, W>(
    stream_url: &str,
    headers: &BTreeMap<String, String>,
    traceparent: &str,
    request_timeout: Duration,
    cancellation: McpCancellation,
    observer: Option<RelayObserver>,
    input: R,
    output: &mut W,
) -> Result<(), RelayFailure>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    let stream_url =
        Url::parse(stream_url).map_err(|_| RelayFailure::new(McpFailureCode::Validation))?;
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| RelayFailure::new(McpFailureCode::Transport))?;
    let deadline = tokio::time::Instant::now() + request_timeout;
    let response = tokio::time::timeout_at(
        deadline,
        request(
            &client,
            Method::GET,
            stream_url.clone(),
            headers,
            traceparent,
        )
        .header(ACCEPT, "text/event-stream")
        .send(),
    )
    .await
    .map_err(|_| RelayFailure::new(McpFailureCode::Timeout))?
    .map_err(|error| RelayFailure::from_reqwest(&error))?;
    validate_event_stream(&response)?;
    let (events, mut receiver) = mpsc::unbounded_channel();
    let sse_pump = spawn_sse_pump(response, events.clone());
    let negotiated = negotiate_endpoint(&stream_url, deadline, &mut receiver).await;
    let (endpoint, pending) = match negotiated {
        Ok(negotiated) => negotiated,
        Err(error) => {
            sse_pump.abort();
            let _ = sse_pump.await;
            return Err(error);
        }
    };
    let input_pump = spawn_parent_pump(input, events);
    let mut session = RelaySession::new(
        client,
        endpoint,
        headers,
        traceparent,
        request_timeout,
        observer,
        output,
    );
    let result = session.run(&mut receiver, pending, &cancellation).await;
    sse_pump.abort();
    let _ = sse_pump.await;
    if input_pump.is_finished() {
        let _ = input_pump.join();
    }
    session.finish(result.as_ref().err().copied()).await;
    result
}

async fn negotiate_endpoint(
    stream_url: &Url,
    deadline: tokio::time::Instant,
    receiver: &mut mpsc::UnboundedReceiver<LegacyRelayEvent>,
) -> Result<(Url, VecDeque<LegacyRelayEvent>), RelayFailure> {
    let mut pending = VecDeque::new();
    loop {
        let event = tokio::time::timeout_at(deadline, receiver.recv())
            .await
            .map_err(|_| RelayFailure::new(McpFailureCode::Timeout))?
            .ok_or_else(|| RelayFailure::new(McpFailureCode::Transport))?;
        match event {
            LegacyRelayEvent::Endpoint(bytes) => {
                let value = std::str::from_utf8(&bytes)
                    .map_err(|_| RelayFailure::new(McpFailureCode::Protocol))?;
                let endpoint = stream_url
                    .join(value)
                    .map_err(|_| RelayFailure::new(McpFailureCode::Protocol))?;
                if !same_origin(stream_url, &endpoint) {
                    return Err(RelayFailure::new(McpFailureCode::Protocol));
                }
                return Ok((endpoint, pending));
            }
            LegacyRelayEvent::SseFailure(error) => return Err(error),
            event => pending.push_back(event),
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

fn validate_event_stream(response: &reqwest::Response) -> Result<(), RelayFailure> {
    if response.status().is_redirection() || !response.status().is_success() {
        return Err(RelayFailure::new(McpFailureCode::UpstreamHttp));
    }
    let valid = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("text/event-stream"));
    if valid {
        Ok(())
    } else {
        Err(RelayFailure::new(McpFailureCode::Protocol))
    }
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

#[cfg(test)]
#[path = "relay_legacy_sse_tests.rs"]
mod tests;
