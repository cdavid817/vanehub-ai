use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const RELAY_FLAG: &str = "--vanehub-mcp-relay";
const PROCESS_GUARD: Duration = Duration::from_secs(12);

#[derive(Clone, Copy, Debug)]
enum ProviderShape {
    Claude,
    Codex,
}

#[derive(Clone, Copy, Debug)]
enum UpstreamTransport {
    Stdio,
    LegacySse,
    StreamableHttp,
}

struct HelperInvocation {
    command: PathBuf,
    args: Vec<String>,
    provider_file: Option<PathBuf>,
}

struct HttpFixture {
    child: Child,
    url: String,
}

impl Drop for HttpFixture {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

struct LineReceiver {
    lines: Receiver<String>,
    reader: JoinHandle<()>,
}

impl LineReceiver {
    fn next(&self, label: &str) -> String {
        self.lines
            .recv_timeout(PROCESS_GUARD)
            .unwrap_or_else(|error| panic!("timed out waiting for {label}: {error}"))
    }

    fn finish(self) {
        self.reader.join().expect("join helper stdout reader");
    }
}

#[test]
fn claude_provider_configuration_runs_the_complete_relay_matrix() {
    run_provider_matrix(ProviderShape::Claude);
}

#[test]
fn codex_provider_configuration_runs_the_complete_relay_matrix() {
    run_provider_matrix(ProviderShape::Codex);
}

#[test]
fn injected_protocol_failure_reaps_descendants_and_leaves_no_raw_secret_artifact() {
    let root = TempDir::new().expect("create relay failure root");
    let invocation_dir = root.path().join("failure-injection");
    fs::create_dir(&invocation_dir).expect("create failure invocation directory");
    let descendant_pid_file = invocation_dir.join("descendant.pid");
    let relay_file = invocation_dir.join("relay.json");
    let configuration = json!({
        "target": {
            "transport": "stdio",
            "command": "node",
            "args": [fixture_path("mcp_stdio_server.cjs"), "failure-secret-descendant"],
            "env": {
                "VANEHUB_MCP_FIXTURE_DESCENDANT_PID_FILE": descendant_pid_file,
                "FIXTURE_CONFIG_SECRET": "raw-relay-config-secret"
            }
        },
        "traceparent": "00-11111111111111111111111111111111-2222222222222222-01",
        "observation": null,
        "timeout_ms": 1_000
    });
    fs::write(
        &relay_file,
        serde_json::to_vec(&configuration).expect("serialize failure configuration"),
    )
    .expect("write failure configuration");
    let helper = provider_invocation(ProviderShape::Claude, &invocation_dir, &relay_file);
    let mut child = Command::new(&helper.command)
        .args(&helper.args)
        .env("VANEHUB_APP_DATA_DIR", &invocation_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start failure-injected relay helper");
    let mut input = child.stdin.take().expect("helper stdin");
    let output = collect_lines(child.stdout.take().expect("helper stdout"));
    let stderr = collect_bytes(child.stderr.take().expect("helper stderr"));

    send_request(
        &mut input,
        json!({"jsonrpc": "2.0", "id": "failure-correlation", "method": "initialize"}),
    );
    let response_line = output.next("safe correlated failure response");
    let response = parse_response(response_line.clone());
    assert_eq!(response["id"], json!("failure-correlation"));
    assert!(response.get("error").is_some());
    drop(input);

    let status = wait_with_guard(&mut child, ProviderShape::Claude, UpstreamTransport::Stdio);
    let stderr = String::from_utf8(stderr.join().expect("join helper stderr reader"))
        .expect("UTF-8 helper stderr");
    output.finish();
    assert!(!status.success(), "protocol failure unexpectedly succeeded");
    assert!(!relay_file.exists(), "secret relay configuration remained");
    let descendant_pid = wait_for_descendant_pid(&descendant_pid_file);
    wait_for_process_exit(descendant_pid);

    for secret in [
        "fixture-stderr-secret",
        "raw-relay-config-secret",
        "Authorization: Bearer",
    ] {
        assert!(!response_line.contains(secret));
        assert!(!stderr.contains(secret));
        assert_directory_does_not_contain(&invocation_dir, secret);
    }
    fs::remove_dir_all(&invocation_dir).expect("cleanup failure invocation artifacts");
    assert!(!invocation_dir.exists());
}

fn run_provider_matrix(provider: ProviderShape) {
    for transport in [
        UpstreamTransport::Stdio,
        UpstreamTransport::LegacySse,
        UpstreamTransport::StreamableHttp,
    ] {
        run_invocation(provider, transport);
    }
}

fn run_invocation(provider: ProviderShape, transport: UpstreamTransport) {
    let root = TempDir::new().expect("create relay integration root");
    let invocation_dir = root.path().join(format!("{provider:?}-{transport:?}"));
    fs::create_dir(&invocation_dir).expect("create invocation directory");

    let fixture = match transport {
        UpstreamTransport::Stdio => None,
        UpstreamTransport::LegacySse => Some(start_http_fixture("mcp_legacy_sse_server.cjs")),
        UpstreamTransport::StreamableHttp => Some(start_http_fixture("mcp_http_server.cjs")),
    };
    let descendant_pid_file = invocation_dir.join("descendant.pid");
    let relay_file = invocation_dir.join("relay.json");
    write_relay_configuration(
        &relay_file,
        transport,
        fixture.as_ref().map(|fixture| fixture.url.as_str()),
        &descendant_pid_file,
    );

    let helper = provider_invocation(provider, &invocation_dir, &relay_file);
    assert_provider_shape(provider, &helper, &relay_file);
    let mut child = Command::new(&helper.command)
        .args(&helper.args)
        .env("VANEHUB_APP_DATA_DIR", &invocation_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start VaneHub MCP relay helper");
    let mut input = child.stdin.take().expect("helper stdin");
    let output = collect_lines(child.stdout.take().expect("helper stdout"));
    let stderr = collect_bytes(child.stderr.take().expect("helper stderr"));

    send_request(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 101,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "vanehub-relay-integration", "version": "1"}
            }
        }),
    );
    let initialized = parse_response(output.next("initialize response"));
    assert_eq!(initialized["id"], json!(101), "{provider:?} {transport:?}");

    send_request(
        &mut input,
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    );
    send_request(
        &mut input,
        json!({"jsonrpc": "2.0", "id": "list-correlation", "method": "tools/list"}),
    );
    let listed = parse_response(output.next("tools/list response"));
    assert_eq!(
        listed["id"],
        json!("list-correlation"),
        "notification emitted output or correlation was lost for {provider:?} {transport:?}"
    );

    let (tool_name, expected_text) = match transport {
        UpstreamTransport::Stdio => ("fixture_echo", "echo: correlated"),
        UpstreamTransport::LegacySse => ("fixture_sse_echo", "sse echo: correlated"),
        UpstreamTransport::StreamableHttp => ("fixture_http_echo", "http echo: correlated"),
    };
    assert_eq!(listed["result"]["tools"][0]["name"], json!(tool_name));
    send_request(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 303,
            "method": "tools/call",
            "params": {"name": tool_name, "arguments": {"text": "correlated"}}
        }),
    );
    let called = parse_response(output.next("tools/call response"));
    assert_eq!(called["id"], json!(303), "{provider:?} {transport:?}");
    assert_eq!(
        called["result"]["content"][0]["text"],
        json!(expected_text),
        "{provider:?} {transport:?}"
    );

    drop(input);
    let status = wait_with_guard(&mut child, provider, transport);
    let stderr = stderr.join().expect("join helper stderr reader");
    output.finish();
    assert!(
        status.success(),
        "relay helper failed for {provider:?} {transport:?}: {}",
        String::from_utf8_lossy(&stderr)
    );
    assert!(
        !relay_file.exists(),
        "relay helper did not consume its invocation-scoped configuration"
    );

    if matches!(transport, UpstreamTransport::Stdio) {
        let descendant_pid = wait_for_descendant_pid(&descendant_pid_file);
        wait_for_process_exit(descendant_pid);
    }
    if let Some(fixture) = &fixture {
        assert_http_fixture_state(transport, &fixture.url);
    }

    if let Some(provider_file) = &helper.provider_file {
        assert!(provider_file.is_file());
    }
    fs::remove_dir_all(&invocation_dir).expect("finalize invocation artifact guard");
    assert!(
        !invocation_dir.exists(),
        "invocation-scoped provider artifacts were not cleaned"
    );
}

fn write_relay_configuration(
    path: &Path,
    transport: UpstreamTransport,
    url: Option<&str>,
    descendant_pid_file: &Path,
) {
    let target = match transport {
        UpstreamTransport::Stdio => json!({
            "transport": "stdio",
            "command": "node",
            "args": [fixture_path("mcp_stdio_server.cjs"), "spawn-descendant"],
            "env": {
                "VANEHUB_MCP_FIXTURE_DESCENDANT_PID_FILE": descendant_pid_file
            }
        }),
        UpstreamTransport::LegacySse => json!({
            "transport": "legacy_sse",
            "url": url.expect("legacy SSE fixture URL"),
            "headers": {"x-fixture-provider": "vanehub"}
        }),
        UpstreamTransport::StreamableHttp => json!({
            "transport": "streamable_http",
            "url": url.expect("Streamable HTTP fixture URL"),
            "headers": {"x-fixture-provider": "vanehub"}
        }),
    };
    let configuration = json!({
        "target": target,
        "traceparent": "00-11111111111111111111111111111111-2222222222222222-01",
        "observation": null,
        "timeout_ms": 5_000
    });
    fs::write(
        path,
        serde_json::to_vec(&configuration).expect("serialize relay configuration"),
    )
    .expect("write relay configuration");
}

fn provider_invocation(
    provider: ProviderShape,
    invocation_dir: &Path,
    relay_file: &Path,
) -> HelperInvocation {
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_vanehub-ai"));
    match provider {
        ProviderShape::Claude => {
            let provider_file = invocation_dir.join("claude.json");
            let configuration = json!({
                "mcpServers": {
                    "fixture": {
                        "command": executable,
                        "args": [RELAY_FLAG, relay_file]
                    }
                }
            });
            fs::write(
                &provider_file,
                serde_json::to_vec(&configuration).expect("serialize Claude configuration"),
            )
            .expect("write Claude provider configuration");
            let parsed: Value = serde_json::from_slice(
                &fs::read(&provider_file).expect("read Claude provider configuration"),
            )
            .expect("parse Claude provider configuration");
            let server = &parsed["mcpServers"]["fixture"];
            HelperInvocation {
                command: PathBuf::from(server["command"].as_str().expect("Claude command")),
                args: serde_json::from_value(server["args"].clone()).expect("Claude args"),
                provider_file: Some(provider_file),
            }
        }
        ProviderShape::Codex => {
            let command = serde_json::to_string(&executable.to_string_lossy().to_string())
                .expect("serialize Codex command");
            let relay_args = serde_json::to_string(&vec![
                RELAY_FLAG.to_string(),
                relay_file.to_string_lossy().to_string(),
            ])
            .expect("serialize Codex args");
            let overrides = [
                "-c".to_string(),
                format!("mcp_servers.\"fixture\".command={command}"),
                "-c".to_string(),
                format!("mcp_servers.\"fixture\".args={relay_args}"),
            ];
            let command: String = serde_json::from_str(
                overrides[1]
                    .split_once('=')
                    .expect("Codex command override")
                    .1,
            )
            .expect("parse Codex command");
            let args: Vec<String> =
                serde_json::from_str(overrides[3].split_once('=').expect("Codex args override").1)
                    .expect("parse Codex args");
            HelperInvocation {
                command: PathBuf::from(command),
                args,
                provider_file: None,
            }
        }
    }
}

fn assert_provider_shape(provider: ProviderShape, helper: &HelperInvocation, relay_file: &Path) {
    assert_eq!(
        helper.command,
        PathBuf::from(env!("CARGO_BIN_EXE_vanehub-ai"))
    );
    assert_eq!(helper.args[0], RELAY_FLAG);
    assert_eq!(Path::new(&helper.args[1]), relay_file);
    assert_eq!(
        helper.provider_file.is_some(),
        matches!(provider, ProviderShape::Claude)
    );
}

fn start_http_fixture(script: &str) -> HttpFixture {
    let mut child = Command::new("node")
        .arg(fixture_path(script))
        .arg("0")
        .arg("normal")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("start HTTP MCP fixture");
    let mut stdout = BufReader::new(child.stdout.take().expect("fixture stdout"));
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut ready = String::new();
        let result = stdout.read_line(&mut ready).map(|_| ready);
        let _ = sender.send(result);
    });
    let ready = receiver
        .recv_timeout(PROCESS_GUARD)
        .expect("HTTP fixture readiness timed out")
        .expect("read HTTP fixture readiness");
    let url = ready
        .trim()
        .strip_prefix("READY ")
        .unwrap_or_else(|| panic!("unexpected HTTP fixture readiness: {ready}"))
        .to_string();
    HttpFixture { child, url }
}

fn send_request(input: &mut impl Write, request: Value) {
    serde_json::to_writer(&mut *input, &request).expect("write JSON-RPC request");
    input.write_all(b"\n").expect("finish JSON-RPC frame");
    input.flush().expect("flush JSON-RPC request");
}

fn parse_response(line: String) -> Value {
    serde_json::from_str(&line)
        .unwrap_or_else(|error| panic!("invalid relay output {line:?}: {error}"))
}

fn collect_lines(reader: impl Read + Send + 'static) -> LineReceiver {
    let (sender, lines) = mpsc::channel();
    let reader = thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            match line {
                Ok(line) => {
                    if sender.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    LineReceiver { lines, reader }
}

fn collect_bytes(mut reader: impl Read + Send + 'static) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).expect("read helper stderr");
        bytes
    })
}

fn wait_with_guard(
    child: &mut Child,
    provider: ProviderShape,
    transport: UpstreamTransport,
) -> std::process::ExitStatus {
    let deadline = Instant::now() + PROCESS_GUARD;
    loop {
        if let Some(status) = child.try_wait().expect("poll relay helper") {
            return status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("relay helper exceeded wall-clock guard for {provider:?} {transport:?}");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn wait_for_descendant_pid(path: &Path) -> u32 {
    let deadline = Instant::now() + PROCESS_GUARD;
    while Instant::now() < deadline {
        if let Ok(value) = fs::read_to_string(path) {
            return value.trim().parse().expect("descendant PID");
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("stdio fixture did not report its descendant PID");
}

fn wait_for_process_exit(pid: u32) {
    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline && process_is_alive(pid) {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !process_is_alive(pid),
        "MCP descendant {pid} remained alive"
    );
}

#[cfg(windows)]
fn process_is_alive(pid: u32) -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .expect("query Windows process list");
    String::from_utf8_lossy(&output.stdout)
        .split(',')
        .any(|field| field.trim().trim_matches('"') == pid.to_string())
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as i32, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}

fn assert_http_fixture_state(transport: UpstreamTransport, url: &str) {
    let state_url = reqwest::Url::parse(url)
        .expect("fixture URL")
        .join("/state")
        .expect("fixture state URL");
    let state: Value = reqwest::blocking::get(state_url)
        .expect("read fixture state")
        .json()
        .expect("parse fixture state");
    let requests = state["requests"].as_array().expect("fixture requests");
    assert!(requests
        .iter()
        .any(|request| request["rpcMethod"] == "initialize"));
    assert!(requests
        .iter()
        .any(|request| request["rpcMethod"] == "tools/list"));
    assert!(requests
        .iter()
        .any(|request| request["rpcMethod"] == "tools/call"));
    assert!(requests.iter().any(|request| request["hasId"] == false));
    if matches!(transport, UpstreamTransport::StreamableHttp) {
        assert_eq!(state["deleteCount"], json!(1));
    }
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn assert_directory_does_not_contain(directory: &Path, forbidden: &str) {
    for entry in fs::read_dir(directory)
        .expect("scan invocation directory")
        .collect::<Result<Vec<_>, _>>()
        .expect("read invocation artifacts")
    {
        assert!(!entry.file_name().to_string_lossy().contains(forbidden));
        if entry.path().is_dir() {
            assert_directory_does_not_contain(&entry.path(), forbidden);
        } else {
            let bytes = fs::read(entry.path()).expect("read invocation artifact");
            assert!(!String::from_utf8_lossy(&bytes).contains(forbidden));
        }
    }
}
