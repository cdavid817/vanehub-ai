use super::legacy_sse::LegacySseTransport;
use crate::contexts::tooling::mcp::application::{McpExecutionControl, McpLimits};
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use crate::platform::network;
use http::{HeaderMap, HeaderValue};
use rmcp::service::TxJsonRpcMessage;
use rmcp::transport::Transport;
use rmcp::RoleClient;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

async fn read_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut byte = [0_u8; 1];
    while !request.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).await.expect("request header");
        request.push(byte[0]);
    }
    let header = String::from_utf8_lossy(&request);
    let content_length = header
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    let mut body = vec![0_u8; content_length];
    stream.read_exact(&mut body).await.expect("request body");
    request.extend(body);
    String::from_utf8(request).expect("HTTP request")
}

async fn fixture_listener() -> (TcpListener, Url) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
    let url = Url::parse(&format!(
        "http://{}/sse",
        listener.local_addr().expect("address")
    ))
    .expect("URL");
    (listener, url)
}

#[tokio::test]
async fn transport_negotiates_endpoint_posts_and_receives_incremental_message() {
    let (listener, url) = fixture_listener().await;
    let fixture = tokio::spawn(async move {
        let (mut event_stream, _) = listener.accept().await.expect("SSE connection");
        let get = read_request(&mut event_stream).await;
        event_stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\nevent: end",
            )
            .await
            .expect("SSE response");
        event_stream
            .write_all(b"point\r\ndata: /messages?session=test\r\n\r\n")
            .await
            .expect("endpoint event");

        let (mut post_stream, _) = listener.accept().await.expect("POST connection");
        let post = read_request(&mut post_stream).await;
        post_stream
            .write_all(b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n")
            .await
            .expect("POST response");
        event_stream
            .write_all(b"event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n")
            .await
            .expect("message event");
        (get, post)
    });
    let mut headers = HeaderMap::new();
    headers.insert("x-test-token", HeaderValue::from_static("fixture-secret"));
    let client = network::no_redirect_http_client(Duration::from_secs(5)).expect("client");
    let control = McpExecutionControl::with_timeout(Duration::from_secs(5));
    let (mut transport, status) = LegacySseTransport::connect(
        client,
        url,
        headers,
        control,
        McpLimits::DEFAULT.protocol_message_bytes,
    )
    .await
    .expect("legacy SSE transport");
    let request: TxJsonRpcMessage<RoleClient> = serde_json::from_value(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping",
        "params": {}
    }))
    .expect("request");

    transport.send(request).await.expect("POST message");
    let response = transport.receive().await.expect("SSE message");
    transport.close().await.expect("close");
    let (get, post) = fixture.await.expect("fixture");

    assert!(get.starts_with("GET /sse HTTP/1.1"), "{get}");
    assert!(
        post.starts_with("POST /messages?session=test HTTP/1.1"),
        "{post}"
    );
    assert!(get
        .to_ascii_lowercase()
        .contains("x-test-token: fixture-secret"));
    assert!(post
        .to_ascii_lowercase()
        .contains("x-test-token: fixture-secret"));
    assert_eq!(serde_json::to_value(response).expect("response")["id"], 1);
    assert_eq!(status.failure(), None);
}

#[tokio::test]
async fn transport_refuses_redirects_without_contacting_the_target() {
    let (listener, url) = fixture_listener().await;
    let fixture = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("redirect request");
        let _ = read_request(&mut stream).await;
        stream
            .write_all(
                b"HTTP/1.1 307 Temporary Redirect\r\nLocation: /target\r\nContent-Length: 0\r\n\r\n",
            )
            .await
            .expect("redirect");
        tokio::time::timeout(Duration::from_millis(100), listener.accept())
            .await
            .is_ok()
    });
    let client = network::no_redirect_http_client(Duration::from_secs(2)).expect("client");

    let error = LegacySseTransport::connect(
        client,
        url,
        HeaderMap::new(),
        McpExecutionControl::with_timeout(Duration::from_secs(2)),
        McpLimits::DEFAULT.protocol_message_bytes,
    )
    .await
    .err()
    .expect("redirect rejected");

    assert_eq!(error.code(), McpFailureCode::UpstreamHttp);
    assert!(!fixture.await.expect("fixture"));
}

#[tokio::test]
async fn endpoint_negotiation_uses_the_absolute_deadline() {
    let (listener, url) = fixture_listener().await;
    let fixture = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("SSE request");
        let _ = read_request(&mut stream).await;
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\n",
            )
            .await
            .expect("SSE headers");
        tokio::time::sleep(Duration::from_secs(1)).await;
    });
    let client = network::no_redirect_http_client(Duration::from_secs(2)).expect("client");

    let error = LegacySseTransport::connect(
        client,
        url,
        HeaderMap::new(),
        McpExecutionControl::with_timeout(Duration::from_millis(100)),
        McpLimits::DEFAULT.protocol_message_bytes,
    )
    .await
    .err()
    .expect("deadline");

    assert_eq!(error.code(), McpFailureCode::Timeout);
    fixture.abort();
    let _ = fixture.await;
}

#[tokio::test]
async fn endpoint_negotiation_observes_cancellation() {
    let (listener, url) = fixture_listener().await;
    let fixture = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("SSE request");
        let _ = read_request(&mut stream).await;
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\n",
            )
            .await
            .expect("SSE headers");
        tokio::time::sleep(Duration::from_secs(1)).await;
    });
    let client = network::no_redirect_http_client(Duration::from_secs(2)).expect("client");
    let control = McpExecutionControl::with_timeout(Duration::from_secs(2));
    let cancellation = control.cancellation();
    let cancel = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancellation.cancel();
    });

    let error = LegacySseTransport::connect(
        client,
        url,
        HeaderMap::new(),
        control,
        McpLimits::DEFAULT.protocol_message_bytes,
    )
    .await
    .err()
    .expect("cancelled");

    assert_eq!(error.code(), McpFailureCode::Cancelled);
    cancel.await.expect("cancel task");
    fixture.abort();
    let _ = fixture.await;
}
