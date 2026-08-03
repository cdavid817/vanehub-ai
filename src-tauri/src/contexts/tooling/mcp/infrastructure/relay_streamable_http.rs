use super::relay_failure::RelayFailure;
use super::relay_jsonrpc::{parse_json_rpc_frame, read_bounded_frame, JsonRpcFrame};
use super::relay_observer::RelayObserver;
use super::relay_streamable_http_observer::{finish_observed, observed_request};
use super::relay_streamable_http_protocol::{
    decode_sse_event, expects_response, read_bounded_body, reject_oversized_content_length,
    request_id, require_matching_response, response_matches, write_json_line,
};
use super::sse_parser::BoundedSseParser;
use crate::contexts::tooling::mcp::application::McpLimits;
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use reqwest::blocking::{Client, Response};
use reqwest::header::CONTENT_TYPE;
use reqwest::{Method, StatusCode};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::time::Duration;

const READ_CHUNK_BYTES: usize = 8 * 1024;
const DELETE_TIMEOUT: Duration = Duration::from_secs(1);

struct RelaySession<'a> {
    client: Client,
    url: &'a str,
    headers: &'a BTreeMap<String, String>,
    traceparent: &'a str,
    request_timeout: Duration,
    session_id: Option<String>,
    protocol_version: Option<String>,
    observer: Option<RelayObserver>,
}

pub(super) fn run(
    url: &str,
    headers: &BTreeMap<String, String>,
    traceparent: &str,
    request_timeout: Duration,
    observer: Option<RelayObserver>,
) -> Result<(), RelayFailure> {
    run_stream(
        url,
        headers,
        traceparent,
        request_timeout,
        observer,
        BufReader::new(std::io::stdin()),
        &mut std::io::stdout(),
    )
}

pub(super) fn run_stream(
    url: &str,
    headers: &BTreeMap<String, String>,
    traceparent: &str,
    request_timeout: Duration,
    observer: Option<RelayObserver>,
    mut input: impl BufRead,
    output: &mut impl Write,
) -> Result<(), RelayFailure> {
    let mut session = RelaySession {
        client: Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| RelayFailure::new(McpFailureCode::Transport))?,
        url,
        headers,
        traceparent,
        request_timeout: request_timeout.max(Duration::from_millis(1)),
        session_id: None,
        protocol_version: None,
        observer,
    };
    let result = forward_requests(&mut session, &mut input, output);
    let cleanup = session.delete_session();
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(error)) => Err(error),
        (Err(error), _) => Err(error),
    }
}

fn forward_requests(
    session: &mut RelaySession<'_>,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<(), RelayFailure> {
    let mut bytes = Vec::new();
    loop {
        let count =
            read_bounded_frame(input, &mut bytes, McpLimits::DEFAULT.protocol_message_bytes)
                .map_err(|error| {
                    RelayFailure::new(if error.kind() == std::io::ErrorKind::InvalidData {
                        McpFailureCode::LimitExceeded
                    } else {
                        McpFailureCode::Transport
                    })
                })?;
        if count == 0 {
            return Ok(());
        }
        let frame = parse_json_rpc_frame(&bytes)
            .map_err(|_| RelayFailure::new(McpFailureCode::Protocol))?;
        let observed = observed_request(session.observer.as_ref(), &frame);
        let result = session.post(&bytes, &frame, output);
        match result {
            Ok(()) => finish_observed(session.observer.as_ref(), observed, None),
            Err(failure) => {
                finish_observed(session.observer.as_ref(), observed, Some(failure));
                if let Some(id) = request_id(&frame) {
                    failure.write_response(output, &id)?;
                }
                return Err(failure);
            }
        }
    }
}

impl RelaySession<'_> {
    fn post(
        &mut self,
        body: &[u8],
        outbound: &JsonRpcFrame,
        output: &mut impl Write,
    ) -> Result<(), RelayFailure> {
        let mut request = self
            .request(Method::POST, self.request_timeout)
            .body(body.to_vec());
        request = request
            .header(CONTENT_TYPE, "application/json")
            .header("accept", "application/json, text/event-stream");
        let mut response = request
            .send()
            .map_err(|error| RelayFailure::from_reqwest(&error))?;
        self.observe_response(&response);
        if response.status() == StatusCode::ACCEPTED {
            return if expects_response(outbound) {
                Err(RelayFailure::new(McpFailureCode::Protocol))
            } else {
                Ok(())
            };
        }
        if response.status().is_redirection() {
            return Err(RelayFailure::new(McpFailureCode::UpstreamHttp));
        }
        if !response.status().is_success() {
            return Err(RelayFailure::new(McpFailureCode::UpstreamHttp));
        }
        reject_oversized_content_length(&response)?;
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if content_type.starts_with("application/json") {
            self.forward_json(response, outbound, output)
        } else if content_type.starts_with("text/event-stream") {
            self.forward_sse(&mut response, outbound, output)
        } else {
            Err(RelayFailure::new(McpFailureCode::Protocol))
        }
    }

    fn forward_json(
        &mut self,
        response: Response,
        outbound: &JsonRpcFrame,
        output: &mut impl Write,
    ) -> Result<(), RelayFailure> {
        let body = read_bounded_body(response)?;
        let frame =
            parse_json_rpc_frame(&body).map_err(|_| RelayFailure::new(McpFailureCode::Protocol))?;
        require_matching_response(outbound, &frame)?;
        self.observe_protocol_version(&body);
        write_json_line(output, &body)
    }

    fn forward_sse(
        &mut self,
        response: &mut Response,
        outbound: &JsonRpcFrame,
        output: &mut impl Write,
    ) -> Result<(), RelayFailure> {
        let expected = request_id(outbound);
        let mut parser = BoundedSseParser::new(McpLimits::DEFAULT.protocol_message_bytes);
        let mut observed = 0_usize;
        let mut chunk = [0_u8; READ_CHUNK_BYTES];
        loop {
            let count = response.read(&mut chunk).map_err(|error| {
                RelayFailure::new(if error.kind() == std::io::ErrorKind::TimedOut {
                    McpFailureCode::Timeout
                } else {
                    McpFailureCode::Transport
                })
            })?;
            if count == 0 {
                return if expected.is_some() {
                    Err(RelayFailure::new(McpFailureCode::Transport))
                } else {
                    Ok(())
                };
            }
            observed = observed.saturating_add(count);
            if observed > McpLimits::DEFAULT.protocol_message_bytes {
                return Err(RelayFailure::new(McpFailureCode::LimitExceeded));
            }
            for event in parser.feed(&chunk[..count]).map_err(RelayFailure::from)? {
                let Some((frame, data)) = decode_sse_event(event)? else {
                    continue;
                };
                self.observe_protocol_version(&data);
                write_json_line(output, &data)?;
                if response_matches(expected.as_ref(), &frame) {
                    return Ok(());
                }
            }
        }
    }

    fn request(&self, method: Method, timeout: Duration) -> reqwest::blocking::RequestBuilder {
        let mut request = self
            .client
            .request(method, self.url)
            .timeout(timeout)
            .header("traceparent", self.traceparent);
        for (name, value) in self.headers {
            request = request.header(name, value);
        }
        if let Some(value) = &self.session_id {
            request = request.header("mcp-session-id", value);
        }
        if let Some(value) = &self.protocol_version {
            request = request.header("mcp-protocol-version", value);
        }
        request
    }

    fn observe_response(&mut self, response: &Response) {
        if let Some(value) = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
        {
            self.session_id = Some(value.to_string());
        }
    }

    fn observe_protocol_version(&mut self, bytes: &[u8]) {
        let observed = serde_json::from_slice::<Value>(bytes)
            .ok()
            .and_then(|value| {
                value
                    .pointer("/result/protocolVersion")?
                    .as_str()
                    .map(str::to_string)
            });
        if observed.is_some() {
            self.protocol_version = observed;
        }
    }

    fn delete_session(&self) -> Result<(), RelayFailure> {
        if self.session_id.is_none() {
            return Ok(());
        }
        let response = self
            .request(Method::DELETE, DELETE_TIMEOUT.min(self.request_timeout))
            .send()
            .map_err(|_| RelayFailure::new(McpFailureCode::Cleanup))?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(RelayFailure::new(McpFailureCode::Cleanup))
        }
    }
}

#[cfg(test)]
#[path = "relay_streamable_http_tests.rs"]
mod tests;
