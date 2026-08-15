use super::connection_adapter::RmcpConnectionAdapter;
use super::connection_support::stdio_process_parts;
use crate::contexts::tooling::mcp::application::{McpConnectionPort, McpExecutionControl};
use crate::contexts::tooling::mcp::domain::{
    McpFailureCode, Scope, ServerConfiguration, ServerConfigurationDraft, TransportType,
};
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

fn fixture_path(name: &str) -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .to_string()
}

fn server(
    transport_type: TransportType,
    command: Option<String>,
    args: Option<Vec<String>>,
    url: Option<String>,
) -> ServerConfiguration {
    ServerConfiguration::create(ServerConfigurationDraft {
        name: "fixture-tools".to_string(),
        transport_type,
        command,
        args,
        env: None,
        url,
        headers: None,
        description: None,
        active: true,
        scope: Scope::User,
        project_path: None,
    })
    .expect("server")
}

fn control() -> McpExecutionControl {
    McpExecutionControl::with_timeout(Duration::from_secs(15))
}

fn start_http_fixture(name: &str) -> (Child, String) {
    start_http_fixture_with_mode(name, None)
}

#[cfg(test)]
fn start_http_fixture_with_mode(name: &str, mode: Option<&str>) -> (Child, String) {
    let mut command = Command::new("node");
    command.arg(fixture_path(name)).arg("0");
    if let Some(mode) = mode {
        command.arg(mode);
    }
    let mut child = command
        .stdout(Stdio::piped())
        .spawn()
        .expect("start fixture");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("ready line");
    let url = line
        .trim()
        .strip_prefix("READY ")
        .expect("ready prefix")
        .to_string();
    (child, url)
}

fn stop_fixture(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

async fn wait_for_fixture_phase(url: &str, expected: &str) {
    let phase_url = format!("{}/phase", url.trim_end_matches("/mcp"));
    // The fixture is on loopback; a default client would route this through the host's proxy and
    // the phase would never be observed, so the wait would only ever time out.
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("loopback client");
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Ok(response) = client.get(&phase_url).send().await {
                if let Ok(phase) = response.text().await {
                    if phase == expected {
                        return;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("fixture did not enter {expected} phase"));
}

async fn cancel_during_http_phase(
    fixture_mode: &str,
    expected_phase: &str,
    call_tool: bool,
) -> Option<McpFailureCode> {
    let (child, url) = start_http_fixture_with_mode("mcp_http_server.cjs", Some(fixture_mode));
    let server = server(TransportType::StreamableHttp, None, None, Some(url.clone()));
    let control = McpExecutionControl::with_timeout(Duration::from_secs(3));
    let cancellation = control.cancellation();
    let running = tokio::spawn(async move {
        if call_tool {
            RmcpConnectionAdapter
                .call_tool(
                    &server,
                    "fixture_http_echo",
                    serde_json::json!({"text": "hi"}),
                    &control,
                )
                .await
                .error_code
        } else {
            RmcpConnectionAdapter
                .test(&server, &control, None)
                .await
                .error_code()
        }
    });

    wait_for_fixture_phase(&url, expected_phase).await;
    cancellation.cancel();
    let code = running.await.expect("adapter join");
    stop_fixture(child);
    code
}

#[tokio::test]
async fn stdio_fixture_initializes_lists_and_calls_through_a_managed_session() {
    let server = server(
        TransportType::Stdio,
        Some("node".to_string()),
        Some(vec![fixture_path("mcp_stdio_server.cjs")]),
        None,
    );

    let outcome = RmcpConnectionAdapter.test(&server, &control(), None).await;
    assert!(outcome.is_success(), "{:?}", outcome.error());
    assert_eq!(outcome.tools()[0].name, "fixture_echo");

    let outcome = RmcpConnectionAdapter
        .call_tool(
            &server,
            "fixture_echo",
            serde_json::json!({"text": "hi"}),
            &control(),
        )
        .await;
    assert!(!outcome.is_error, "{}", outcome.content);
    assert_eq!(outcome.content, "echo: hi");
}

#[tokio::test]
async fn streamable_http_fixture_lists_calls_and_closes_the_session() {
    let (child, url) = start_http_fixture("mcp_http_server.cjs");
    let server = server(TransportType::StreamableHttp, None, None, Some(url));

    let outcome = RmcpConnectionAdapter.test(&server, &control(), None).await;
    assert!(outcome.is_success(), "{:?}", outcome.error());
    assert_eq!(outcome.tools()[0].name, "fixture_http_echo");

    let outcome = RmcpConnectionAdapter
        .call_tool(
            &server,
            "fixture_http_echo",
            serde_json::json!({"text": "hi"}),
            &control(),
        )
        .await;
    stop_fixture(child);
    assert!(!outcome.is_error, "{}", outcome.content);
    assert_eq!(outcome.content, "http echo: hi");
}

#[tokio::test]
async fn streamable_http_cleanup_failure_replaces_remote_success() {
    let (child, url) = start_http_fixture_with_mode("mcp_http_server.cjs", Some("fail-delete"));
    let server = server(TransportType::StreamableHttp, None, None, Some(url));

    let outcome = RmcpConnectionAdapter.test(&server, &control(), None).await;
    stop_fixture(child);

    assert!(!outcome.is_success());
    assert_eq!(outcome.error_code(), Some(McpFailureCode::Cleanup));
}

#[tokio::test]
async fn agent_cancellation_interrupts_initialization_discovery_and_invocation() {
    for (mode, phase, call_tool) in [
        ("hang-initialize", "initialize", false),
        ("hang-discovery", "discovery", false),
        ("hang-invocation", "invocation", true),
    ] {
        assert_eq!(
            cancel_during_http_phase(mode, phase, call_tool).await,
            Some(McpFailureCode::Cancelled),
            "cancellation was not preserved during {phase}"
        );
    }
}

#[tokio::test]
async fn agent_cancellation_during_cleanup_waits_for_cleanup_and_returns_cancelled() {
    let (child, url) = start_http_fixture_with_mode("mcp_http_server.cjs", Some("delay-delete"));
    let server = server(TransportType::StreamableHttp, None, None, Some(url.clone()));
    let control = McpExecutionControl::with_timeout(Duration::from_secs(3));
    let cancellation = control.cancellation();
    let running = tokio::spawn(async move {
        RmcpConnectionAdapter
            .call_tool(
                &server,
                "fixture_http_echo",
                serde_json::json!({"text": "hi"}),
                &control,
            )
            .await
    });

    wait_for_fixture_phase(&url, "cleanup").await;
    cancellation.cancel();
    let outcome = running.await.expect("adapter join");
    stop_fixture(child);

    assert!(outcome.is_error);
    assert_eq!(outcome.error_code, Some(McpFailureCode::Cancelled));
}

#[tokio::test]
async fn legacy_sse_fixture_negotiates_lists_and_calls() {
    let (child, url) = start_http_fixture("mcp_legacy_sse_server.cjs");
    let server = server(TransportType::Sse, None, None, Some(url));

    let outcome = RmcpConnectionAdapter.test(&server, &control(), None).await;
    assert!(outcome.is_success(), "{:?}", outcome.error());
    assert_eq!(outcome.tools()[0].name, "fixture_sse_echo");

    let outcome = RmcpConnectionAdapter
        .call_tool(
            &server,
            "fixture_sse_echo",
            serde_json::json!({"text": "hi"}),
            &control(),
        )
        .await;
    stop_fixture(child);
    assert!(!outcome.is_error, "{}", outcome.content);
    assert_eq!(outcome.content, "sse echo: hi");
}

#[tokio::test]
async fn managed_stdio_timeout_returns_only_after_the_child_is_reaped() {
    let server = server(
        TransportType::Stdio,
        Some("node".to_string()),
        Some(vec![
            "-e".to_string(),
            "setTimeout(() => {}, 5000)".to_string(),
        ]),
        None,
    );

    let outcome = RmcpConnectionAdapter
        .test(
            &server,
            &McpExecutionControl::with_timeout(Duration::from_millis(100)),
            None,
        )
        .await;

    assert!(!outcome.is_success());
    assert_eq!(outcome.error_code(), Some(McpFailureCode::Timeout));
}

#[test]
fn stdio_arguments_remain_literal_and_are_not_interpreted_by_a_shell() {
    let server = server(
        TransportType::Stdio,
        Some("node".to_string()),
        Some(vec![
            "literal; echo should-not-run".to_string(),
            "$(also-literal)".to_string(),
        ]),
        None,
    );

    let (_, args, _) = stdio_process_parts(&server).expect("command");
    assert_eq!(args, ["literal; echo should-not-run", "$(also-literal)"]);
}

#[tokio::test]
async fn invalid_http_headers_return_a_safe_validation_failure() {
    let server = ServerConfiguration::create(ServerConfigurationDraft {
        name: "http-tools".to_string(),
        transport_type: TransportType::Sse,
        command: None,
        args: None,
        env: None,
        url: Some("http://localhost:1/mcp".to_string()),
        headers: Some([("bad header".to_string(), "value".to_string())].into()),
        description: None,
        active: true,
        scope: Scope::User,
        project_path: None,
    })
    .expect("server");

    let outcome = RmcpConnectionAdapter.test(&server, &control(), None).await;
    assert_eq!(outcome.error_code(), Some(McpFailureCode::Validation));
}

#[tokio::test]
async fn tool_level_error_remains_tool_execution_data() {
    let server = server(
        TransportType::Stdio,
        Some("node".to_string()),
        Some(vec![fixture_path("mcp_stdio_server.cjs")]),
        None,
    );
    let outcome = RmcpConnectionAdapter
        .call_tool(&server, "does_not_exist", serde_json::json!({}), &control())
        .await;
    assert!(outcome.is_error);
    assert_eq!(outcome.error_code, None);
    assert_eq!(outcome.content, "Unknown tool \"does_not_exist\".");
}
