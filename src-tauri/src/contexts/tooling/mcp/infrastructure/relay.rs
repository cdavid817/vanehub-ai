use super::relay_legacy_sse;
use super::relay_observer::RelayObserver;
use super::relay_stdio;
use super::relay_streamable_http;
use super::runtime_logging::{self, McpRuntimeLogContext};
use crate::contexts::execution_observability::api::ExecutionTelemetryPort;
use crate::contexts::tooling::mcp::application::{McpExecutionControl, McpLimits};
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use crate::platform::private_relay_fs::PrivateRelayDirectory;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
#[cfg(test)]
use std::io::BufRead;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const RELAY_FLAG: &str = "--vanehub-mcp-relay";
const DEFAULT_TIMEOUT_MS: u64 = 30_000;

pub(crate) type RelayTelemetryFactory = fn(&Path) -> Option<Arc<dyn ExecutionTelemetryPort>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub(crate) enum RelayTarget {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
    },
    LegacySse {
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
    StreamableHttp {
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RelayObservation {
    pub(crate) database_path: PathBuf,
    pub(crate) run_id: String,
    pub(crate) trace_id: String,
    pub(crate) parent_span_id: String,
    pub(crate) capture_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RelayConfiguration {
    pub(crate) target: RelayTarget,
    pub(crate) traceparent: String,
    pub(crate) observation: Option<RelayObservation>,
    #[serde(default = "default_timeout_ms")]
    pub(crate) timeout_ms: u64,
}

pub(crate) fn write_configuration(
    directory: &PrivateRelayDirectory,
    configuration: &RelayConfiguration,
) -> Result<PathBuf, String> {
    let bytes = serde_json::to_vec(configuration).map_err(|error| error.to_string())?;
    McpLimits::DEFAULT
        .validate_bytes(
            "MCP relay configuration",
            bytes.len(),
            McpLimits::DEFAULT.configuration_serialized_bytes,
        )
        .map_err(|error| error.to_string())?;
    let name = format!("relay-{}.json", Uuid::new_v4());
    let path = directory.path().join(&name);
    let mut file = directory
        .create_file(&name)
        .map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())?;
    Ok(path)
}

pub(crate) fn try_run_from_process_args(telemetry_factory: RelayTelemetryFactory) -> bool {
    match run_from_process_args(std::env::args_os(), telemetry_factory) {
        Ok(is_relay) => is_relay,
        Err(_) => std::process::exit(2),
    }
}

fn run_from_process_args(
    args: impl IntoIterator<Item = std::ffi::OsString>,
    telemetry_factory: RelayTelemetryFactory,
) -> Result<bool, String> {
    let mut args = args.into_iter();
    let _ = args.next();
    if args.next().as_deref() != Some(std::ffi::OsStr::new(RELAY_FLAG)) {
        return Ok(false);
    }
    args.next()
        .ok_or_else(|| "relay configuration path is required".to_string())
        .and_then(|path| run_configuration(Path::new(&path), telemetry_factory))?;
    Ok(true)
}

fn run_configuration(path: &Path, telemetry_factory: RelayTelemetryFactory) -> Result<(), String> {
    let file = fs::File::open(path).map_err(|error| error.to_string())?;
    fs::remove_file(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.take((McpLimits::DEFAULT.configuration_serialized_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    McpLimits::DEFAULT
        .validate_bytes(
            "MCP relay configuration",
            bytes.len(),
            McpLimits::DEFAULT.configuration_serialized_bytes,
        )
        .map_err(|error| error.to_string())?;
    let configuration =
        serde_json::from_slice::<RelayConfiguration>(&bytes).map_err(|error| error.to_string())?;
    let control =
        McpExecutionControl::with_timeout(Duration::from_millis(configuration.timeout_ms.max(1)));
    let telemetry = configuration
        .observation
        .as_ref()
        .and_then(|value| value.database_path.parent())
        .and_then(telemetry_factory);
    let observer = RelayObserver::new(configuration.observation.as_ref(), telemetry);
    let transport = match &configuration.target {
        RelayTarget::Stdio { .. } => "stdio",
        RelayTarget::LegacySse { .. } => "sse",
        RelayTarget::StreamableHttp { .. } => "streamable_http",
    };
    let log_context = McpRuntimeLogContext::for_relay(
        transport,
        configuration
            .observation
            .as_ref()
            .map(|value| value.run_id.as_str()),
        configuration
            .observation
            .as_ref()
            .map(|value| value.trace_id.as_str()),
        configuration
            .observation
            .as_ref()
            .map(|value| value.parent_span_id.as_str()),
    );
    let (result, failure_code) = match configuration.target {
        RelayTarget::Stdio { command, args, env } => {
            let command_context = log_context.clone().with_command(&command, args.len());
            let result = relay_stdio::run(
                &command,
                &args,
                &env,
                Duration::from_millis(configuration.timeout_ms.max(1)),
                control.cancellation(),
                observer,
                &command_context,
            );
            let failure_code = result.is_err().then_some(McpFailureCode::Transport);
            (result, failure_code)
        }
        RelayTarget::LegacySse { url, headers } => {
            let result = relay_legacy_sse::run(
                &url,
                &headers,
                &configuration.traceparent,
                Duration::from_millis(configuration.timeout_ms.max(1)),
                control.cancellation(),
                observer,
            );
            let failure_code = result.as_ref().err().map(|error| error.code());
            (result.map_err(|error| error.to_string()), failure_code)
        }
        RelayTarget::StreamableHttp { url, headers } => {
            let result = relay_streamable_http::run(
                &url,
                &headers,
                &configuration.traceparent,
                Duration::from_millis(configuration.timeout_ms.max(1)),
                observer,
            );
            let failure_code = result.as_ref().err().map(|error| error.code());
            (result.map_err(|error| error.to_string()), failure_code)
        }
    };
    runtime_logging::record_relay_terminal(&log_context, failure_code);
    result
}

#[cfg(test)]
fn relay_http_stream(
    url: &str,
    headers: &BTreeMap<String, String>,
    traceparent: &str,
    control: McpExecutionControl,
    observer: Option<RelayObserver>,
    input: impl BufRead,
    output: &mut impl Write,
) -> Result<(), String> {
    let client = crate::platform::network::apply_proxy_routing(
        reqwest::blocking::Client::builder().redirect(reqwest::redirect::Policy::none()),
    )
    .map_err(|error| error.to_string())?
    .build()
    .map_err(|error| error.to_string())?;
    let mut session_id: Option<String> = None;
    for line in input.lines() {
        let line = line.map_err(|error| error.to_string())?;
        let parsed = serde_json::from_str::<serde_json::Value>(&line)
            .map_err(|_| "relay received invalid JSON-RPC".to_string())?;
        let observed_request = observer
            .as_ref()
            .and_then(|observer| observer.start_request("http", json_rpc_method(&parsed)));
        let mut request = client
            .post(url)
            .timeout(control.remaining().map_err(|error| error.to_string())?)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("traceparent", traceparent)
            .body(line.into_bytes());
        for (name, value) in headers {
            request = request.header(name, value);
        }
        if let Some(value) = &session_id {
            request = request.header("mcp-session-id", value);
        }
        let response = match request.send() {
            Ok(response) => response,
            Err(error) => {
                if let (Some(observer), Some(observed_request)) = (&observer, observed_request) {
                    observer.finish_request(
                        observed_request,
                        false,
                        Some("mcp_http_request_failed"),
                    );
                }
                return Err(error.to_string());
            }
        };
        if response.status().is_redirection() {
            if let (Some(observer), Some(observed_request)) = (&observer, observed_request) {
                observer.finish_request(observed_request, false, Some("mcp_http_redirect_refused"));
            }
            return Err("MCP HTTP relay refused a redirect".to_string());
        }
        let success = response.status().is_success();
        if let Some(value) = response.headers().get("mcp-session-id") {
            session_id = value.to_str().ok().map(str::to_string);
        }
        let bytes = response.bytes().map_err(|error| error.to_string())?;
        output
            .write_all(&bytes)
            .map_err(|error| error.to_string())?;
        output.write_all(b"\n").map_err(|error| error.to_string())?;
        output.flush().map_err(|error| error.to_string())?;
        if let (Some(observer), Some(observed_request)) = (&observer, observed_request) {
            observer.finish_request(
                observed_request,
                success,
                (!success).then_some("mcp_http_error_status"),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
fn json_rpc_method(value: &serde_json::Value) -> Option<&str> {
    value.get("method").and_then(serde_json::Value::as_str)
}

fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

#[cfg(test)]
#[path = "relay_tests.rs"]
mod tests;
