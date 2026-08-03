use super::streamable_http::BoundedStreamableHttpTransport;
use crate::contexts::tooling::mcp::application::{McpExecutionControl, McpLimits};
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use crate::platform::network;
use http::HeaderMap;
use rmcp::service::TxJsonRpcMessage;
use rmcp::transport::Transport;
use rmcp::RoleClient;
use std::time::{Duration, Instant};
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

async fn listener_url() -> (TcpListener, Url) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
    let url = Url::parse(&format!(
        "http://{}/mcp",
        listener.local_addr().expect("address")
    ))
    .expect("URL");
    (listener, url)
}

fn message(value: serde_json::Value) -> TxJsonRpcMessage<RoleClient> {
    serde_json::from_value(value).expect("client message")
}

async fn write_json(stream: &mut TcpStream, body: &str, session_id: Option<&str>) {
    let session = session_id
        .map(|id| format!("Mcp-Session-Id: {id}\r\n"))
        .unwrap_or_default();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{session}Content-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .await
        .expect("JSON response");
}

#[tokio::test]
async fn json_sse_notification_session_headers_and_delete_share_one_lifecycle() {
    let (listener, url) = listener_url().await;
    let fixture = tokio::spawn(async move {
        let (mut initialize_stream, _) = listener.accept().await.expect("initialize");
        let initialize = read_request(&mut initialize_stream).await;
        write_json(
            &mut initialize_stream,
            r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{},"serverInfo":{"name":"fixture","version":"1"}}}"#,
            Some("session-123"),
        )
        .await;

        let (mut notification_stream, _) = listener.accept().await.expect("notification");
        let notification = read_request(&mut notification_stream).await;
        notification_stream
            .write_all(b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n")
            .await
            .expect("202");

        let (mut ping_stream, _) = listener.accept().await.expect("ping");
        let ping = read_request(&mut ping_stream).await;
        ping_stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n",
            )
            .await
            .expect("SSE headers");
        let event = b"event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n\n";
        let first = &event[..18];
        let second = &event[18..];
        for part in [first, second] {
            ping_stream
                .write_all(format!("{:X}\r\n", part.len()).as_bytes())
                .await
                .expect("chunk size");
            ping_stream.write_all(part).await.expect("SSE chunk");
            ping_stream.write_all(b"\r\n").await.expect("chunk end");
        }

        let (mut delete_stream, _) = listener.accept().await.expect("delete");
        let delete = read_request(&mut delete_stream).await;
        delete_stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .await
            .expect("DELETE response");
        (initialize, notification, ping, delete)
    });
    let client = network::no_redirect_http_client(Duration::from_secs(5)).expect("client");
    let control = McpExecutionControl::with_timeout(Duration::from_secs(5));
    let (mut transport, status, lease) = BoundedStreamableHttpTransport::new(
        client,
        url,
        HeaderMap::new(),
        control,
        McpLimits::DEFAULT.protocol_message_bytes,
    );

    transport
        .send(message(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}})))
        .await
        .expect("initialize POST");
    assert_eq!(
        serde_json::to_value(transport.receive().await.expect("initialize response"))
            .expect("JSON")["id"],
        1
    );
    transport
        .send(message(
            serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        ))
        .await
        .expect("notification 202");
    transport
        .send(message(
            serde_json::json!({"jsonrpc":"2.0","id":2,"method":"ping","params":{}}),
        ))
        .await
        .expect("ping POST");
    assert_eq!(
        serde_json::to_value(transport.receive().await.expect("SSE response")).expect("JSON")["id"],
        2
    );
    lease
        .shutdown(Instant::now() + Duration::from_secs(1))
        .await
        .expect("DELETE");
    transport.close().await.expect("idempotent close");
    let (initialize, notification, ping, delete) = fixture.await.expect("fixture");

    assert!(!initialize.to_ascii_lowercase().contains("mcp-session-id:"));
    for request in [&notification, &ping, &delete] {
        assert!(request
            .to_ascii_lowercase()
            .contains("mcp-session-id: session-123"));
    }
    for request in [&notification, &ping] {
        assert!(request
            .to_ascii_lowercase()
            .contains("mcp-protocol-version: 2025-06-18"));
    }
    assert!(delete.starts_with("DELETE /mcp HTTP/1.1"), "{delete}");
    assert_eq!(status.failure(), None);
}

#[tokio::test]
async fn streamed_body_limit_plus_one_is_rejected() {
    let (listener, url) = listener_url().await;
    let fixture = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("request");
        let _ = read_request(&mut stream).await;
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n41\r\n")
            .await
            .expect("headers");
        stream.write_all(&vec![b'x'; 65]).await.expect("body");
        stream.write_all(b"\r\n0\r\n\r\n").await.expect("end");
    });
    let client = network::no_redirect_http_client(Duration::from_secs(2)).expect("client");
    let control = McpExecutionControl::with_timeout(Duration::from_secs(2));
    let (mut transport, status, _) =
        BoundedStreamableHttpTransport::new(client, url, HeaderMap::new(), control, 64);

    let error = transport
        .send(message(
            serde_json::json!({"jsonrpc":"2.0","id":1,"method":"ping","params":{}}),
        ))
        .await
        .expect_err("limit plus one");

    assert_eq!(error.code(), McpFailureCode::LimitExceeded);
    assert_eq!(status.failure(), Some(McpFailureCode::LimitExceeded));
    fixture.await.expect("fixture");
}
