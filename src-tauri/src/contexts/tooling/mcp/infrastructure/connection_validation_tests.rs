use super::connection_adapter::RmcpConnectionAdapter;
use crate::contexts::tooling::mcp::application::{
    McpConnectionPort, McpExecutionControl, McpLimits,
};
use crate::contexts::tooling::mcp::domain::{
    McpFailureCode, Scope, ServerConfiguration, ServerConfigurationDraft, TransportType,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn unreachable_stdio_server() -> ServerConfiguration {
    ServerConfiguration::create(ServerConfigurationDraft {
        name: "validation-only".to_string(),
        transport_type: TransportType::Stdio,
        command: Some("this-binary-must-never-be-started".to_string()),
        args: None,
        env: None,
        url: None,
        headers: None,
        description: None,
        active: true,
        scope: Scope::User,
        project_path: None,
    })
    .expect("server")
}

fn control() -> McpExecutionControl {
    McpExecutionControl::with_timeout(Duration::from_secs(2))
}

#[tokio::test]
async fn malformed_arguments_are_rejected_before_stdio_validation_or_spawn() {
    let server = unreachable_stdio_server();
    let outcome = RmcpConnectionAdapter
        .call_tool(&server, "tool", serde_json::json!([]), &control())
        .await;

    assert!(outcome.is_error);
    assert_eq!(outcome.error_code, Some(McpFailureCode::Validation));
}

#[tokio::test]
async fn oversized_name_arguments_and_depth_are_rejected_before_spawn() {
    let server = unreachable_stdio_server();
    let limits = McpLimits::DEFAULT;
    let cases = [
        (
            "x".repeat(limits.tool_name_bytes + 1),
            serde_json::json!({}),
        ),
        (
            "tool".to_string(),
            serde_json::json!({ "value": "x".repeat(limits.tool_arguments_bytes) }),
        ),
        ("tool".to_string(), too_deep(limits.json_depth)),
    ];

    for (name, arguments) in cases {
        let outcome = RmcpConnectionAdapter
            .call_tool(&server, &name, arguments, &control())
            .await;
        assert_eq!(outcome.error_code, Some(McpFailureCode::LimitExceeded));
    }
}

#[tokio::test]
async fn call_timeout_is_returned_after_managed_cleanup() {
    let server = ServerConfiguration::create(ServerConfigurationDraft {
        name: "slow-tool".to_string(),
        transport_type: TransportType::Stdio,
        command: Some("node".to_string()),
        args: Some(vec![
            "-e".to_string(),
            "setTimeout(() => {}, 5000)".to_string(),
        ]),
        env: None,
        url: None,
        headers: None,
        description: None,
        active: true,
        scope: Scope::User,
        project_path: None,
    })
    .expect("server");
    let outcome = RmcpConnectionAdapter
        .call_tool(
            &server,
            "fixture_echo",
            serde_json::json!({}),
            &McpExecutionControl::with_timeout(Duration::from_millis(100)),
        )
        .await;
    assert_eq!(outcome.error_code, Some(McpFailureCode::Timeout));
}

#[tokio::test]
async fn streamable_http_cancellation_closes_the_in_flight_request_before_returning() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    let (request_seen_tx, request_seen_rx) = tokio::sync::oneshot::channel();
    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut received = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = socket.read(&mut buffer).await.expect("request read");
            assert_ne!(read, 0, "request closed before headers");
            received.extend_from_slice(&buffer[..read]);
            if request_is_complete(&received) {
                break;
            }
        }
        let _ = request_seen_tx.send(());
        loop {
            let read = tokio::time::timeout(Duration::from_secs(2), socket.read(&mut buffer))
                .await
                .expect("request close deadline")
                .expect("request close read");
            if read == 0 {
                break;
            }
        }
        let _ = socket.shutdown().await;
    });
    let server = ServerConfiguration::create(ServerConfigurationDraft {
        name: "cancel-http".to_string(),
        transport_type: TransportType::StreamableHttp,
        command: None,
        args: None,
        env: None,
        url: Some(format!("http://{address}/mcp")),
        headers: None,
        description: None,
        active: true,
        scope: Scope::User,
        project_path: None,
    })
    .expect("server");
    let control = McpExecutionControl::with_timeout(Duration::from_secs(2));
    let cancellation = control.cancellation();
    let running =
        tokio::spawn(async move { RmcpConnectionAdapter.test(&server, &control, None).await });

    request_seen_rx.await.expect("request observed");
    cancellation.cancel();
    let outcome = running.await.expect("adapter join");
    server_task.await.expect("server join");

    assert_eq!(outcome.error_code(), Some(McpFailureCode::Cancelled));
}

fn request_is_complete(request: &[u8]) -> bool {
    let Some(headers_end) = request.windows(4).position(|part| part == b"\r\n\r\n") else {
        return false;
    };
    let headers = String::from_utf8_lossy(&request[..headers_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    request.len() >= headers_end + 4 + content_length
}

fn too_deep(maximum: usize) -> serde_json::Value {
    let mut value = serde_json::Value::Null;
    for _ in 0..maximum {
        value = serde_json::json!({ "nested": value });
    }
    value
}
