use super::relay_observer::RelayObserver;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

const RELAY_FLAG: &str = "--vanehub-mcp-relay";
const DEFAULT_TIMEOUT_MS: u64 = 30_000;

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
    Http {
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
    directory: &Path,
    configuration: &RelayConfiguration,
) -> Result<PathBuf, String> {
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let path = directory.join(format!("relay-{}.json", Uuid::new_v4()));
    let bytes = serde_json::to_vec(configuration).map_err(|error| error.to_string())?;
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path)
}

pub(crate) fn try_run_from_process_args() -> bool {
    match run_from_process_args(std::env::args_os()) {
        Ok(is_relay) => is_relay,
        Err(_) => std::process::exit(2),
    }
}

fn run_from_process_args(
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<bool, String> {
    let mut args = args.into_iter();
    let _ = args.next();
    if args.next().as_deref() != Some(std::ffi::OsStr::new(RELAY_FLAG)) {
        return Ok(false);
    }
    args.next()
        .ok_or_else(|| "relay configuration path is required".to_string())
        .and_then(|path| run_configuration(Path::new(&path)))?;
    Ok(true)
}

fn run_configuration(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let _ = fs::remove_file(path);
    let configuration =
        serde_json::from_slice::<RelayConfiguration>(&bytes).map_err(|error| error.to_string())?;
    let observer = RelayObserver::new(configuration.observation.as_ref());
    match configuration.target {
        RelayTarget::Stdio { command, args, env } => relay_stdio(
            &command,
            &args,
            &env,
            Duration::from_millis(configuration.timeout_ms.max(1)),
            observer,
        ),
        RelayTarget::Http { url, headers } => relay_http(
            &url,
            &headers,
            &configuration.traceparent,
            Duration::from_millis(configuration.timeout_ms.max(1)),
            observer,
        ),
    }
}

fn relay_stdio(
    executable: &str,
    args: &[String],
    env: &BTreeMap<String, String>,
    timeout: Duration,
    observer: Option<RelayObserver>,
) -> Result<(), String> {
    let mut child = crate::platform::process::spawn_piped(executable, args, env)
        .map_err(|error| error.to_string())?;
    let mut child_stdin = child
        .stdin
        .take()
        .ok_or_else(|| "relay child stdin unavailable".to_string())?;
    let mut child_stdout = child
        .stdout
        .take()
        .ok_or_else(|| "relay child stdout unavailable".to_string())?;
    let input = thread::spawn(move || {
        let source = BufReader::new(std::io::stdin().lock());
        forward_stdio_input(source, &mut child_stdin, observer.as_ref())?;
        drop(child_stdin);
        Ok::<(), String>(())
    });
    let output =
        thread::spawn(move || std::io::copy(&mut child_stdout, &mut std::io::stdout().lock()));
    let started = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            break;
        }
        if input.is_finished() && started.elapsed() >= timeout {
            let _ = child.kill();
            return Err("MCP stdio relay shutdown timed out".to_string());
        }
        thread::sleep(Duration::from_millis(10));
    }
    input
        .join()
        .map_err(|_| "relay input thread failed".to_string())??;
    output
        .join()
        .map_err(|_| "relay output thread failed".to_string())?
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn forward_stdio_input(
    mut source: impl BufRead,
    target: &mut impl Write,
    observer: Option<&RelayObserver>,
) -> Result<(), String> {
    let mut line = Vec::new();
    loop {
        line.clear();
        let count = source
            .read_until(b'\n', &mut line)
            .map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        let method = request_method(&line);
        let request =
            observer.and_then(|observer| observer.start_request("stdio", method.as_deref()));
        let result = target
            .write_all(&line)
            .and_then(|()| target.flush())
            .map_err(|error| error.to_string());
        if let (Some(observer), Some(request)) = (observer, request) {
            observer.finish_request(
                &request,
                result.is_ok(),
                result.as_ref().err().map(|_| "mcp_stdio_forward_failed"),
            );
        }
        result?;
    }
    Ok(())
}

fn relay_http(
    url: &str,
    headers: &BTreeMap<String, String>,
    traceparent: &str,
    timeout: Duration,
    observer: Option<RelayObserver>,
) -> Result<(), String> {
    relay_http_stream(
        url,
        headers,
        traceparent,
        timeout,
        observer,
        BufReader::new(std::io::stdin().lock()),
        &mut std::io::stdout().lock(),
    )
}

fn relay_http_stream(
    url: &str,
    headers: &BTreeMap<String, String>,
    traceparent: &str,
    timeout: Duration,
    observer: Option<RelayObserver>,
    input: impl BufRead,
    output: &mut impl Write,
) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
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
                        &observed_request,
                        false,
                        Some("mcp_http_request_failed"),
                    );
                }
                return Err(error.to_string());
            }
        };
        if response.status().is_redirection() {
            if let (Some(observer), Some(observed_request)) = (&observer, observed_request) {
                observer.finish_request(
                    &observed_request,
                    false,
                    Some("mcp_http_redirect_refused"),
                );
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
                &observed_request,
                success,
                (!success).then_some("mcp_http_error_status"),
            );
        }
    }
    Ok(())
}

fn request_method(bytes: &[u8]) -> Option<String> {
    let value = serde_json::from_slice::<serde_json::Value>(bytes).ok()?;
    json_rpc_method(&value).map(str::to_string)
}

fn json_rpc_method(value: &serde_json::Value) -> Option<&str> {
    value.get("method").and_then(serde_json::Value::as_str)
}

fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

#[cfg(test)]
#[path = "relay_tests.rs"]
mod tests;
