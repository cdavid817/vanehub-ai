use super::streamable_http::BoundedStreamableHttpTransport;
use crate::contexts::tooling::mcp::application::McpExecutionControl;
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

async fn read_request(stream: &mut TcpStream) {
    let mut request = Vec::new();
    let mut byte = [0_u8; 1];
    while !request.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).await.expect("request header");
        request.push(byte[0]);
    }
    let header = String::from_utf8_lossy(&request);
    let length = header
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    let mut body = vec![0_u8; length];
    stream.read_exact(&mut body).await.expect("request body");
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

fn ping() -> TxJsonRpcMessage<RoleClient> {
    serde_json::from_value(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"ping","params":{}}))
        .expect("ping")
}

fn transport(url: Url, control: McpExecutionControl) -> BoundedStreamableHttpTransport {
    let client = network::no_redirect_http_client(Duration::from_secs(2)).expect("client");
    BoundedStreamableHttpTransport::new(client, url, HeaderMap::new(), control, 1024).0
}

#[tokio::test]
async fn post_redirect_is_refused_without_contacting_the_target() {
    let (listener, url) = listener_url().await;
    let fixture = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("request");
        read_request(&mut stream).await;
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
    let mut transport = transport(
        url,
        McpExecutionControl::with_timeout(Duration::from_secs(1)),
    );

    let error = transport.send(ping()).await.expect_err("redirect");

    assert_eq!(error.code(), McpFailureCode::UpstreamHttp);
    assert!(!fixture.await.expect("fixture"));
}

#[tokio::test]
async fn response_wait_uses_the_absolute_deadline() {
    let (listener, url) = listener_url().await;
    let fixture = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("request");
        read_request(&mut stream).await;
        tokio::time::sleep(Duration::from_secs(1)).await;
    });
    let mut transport = transport(
        url,
        McpExecutionControl::with_timeout(Duration::from_millis(100)),
    );

    let error = transport.send(ping()).await.expect_err("deadline");

    assert_eq!(error.code(), McpFailureCode::Timeout);
    fixture.abort();
    let _ = fixture.await;
}

#[tokio::test]
async fn cancellation_before_send_opens_no_connection() {
    let (listener, url) = listener_url().await;
    let control = McpExecutionControl::with_timeout(Duration::from_secs(1));
    control.cancellation().cancel();
    let mut transport = transport(url, control);

    let error = transport.send(ping()).await.expect_err("cancelled");
    let accepted = tokio::time::timeout(Duration::from_millis(100), listener.accept()).await;

    assert_eq!(error.code(), McpFailureCode::Cancelled);
    assert!(accepted.is_err());
}

#[tokio::test]
async fn established_session_delete_is_bounded_and_reports_cleanup() {
    let (listener, url) = listener_url().await;
    let fixture = tokio::spawn(async move {
        let (mut initialize, _) = listener.accept().await.expect("initialize");
        read_request(&mut initialize).await;
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nMcp-Session-Id: session-timeout\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        initialize
            .write_all(response.as_bytes())
            .await
            .expect("response");
        let (mut delete, _) = listener.accept().await.expect("delete");
        read_request(&mut delete).await;
        tokio::time::sleep(Duration::from_secs(1)).await;
    });
    let client = network::no_redirect_http_client(Duration::from_secs(2)).expect("client");
    let control = McpExecutionControl::with_timeout(Duration::from_secs(2));
    let (mut transport, status, lease) =
        BoundedStreamableHttpTransport::new(client, url, HeaderMap::new(), control, 1024);
    transport.send(ping()).await.expect("initialize response");
    let _ = transport.receive().await.expect("response");

    let error = lease
        .shutdown(Instant::now() + Duration::from_millis(100))
        .await
        .expect_err("DELETE timeout");

    assert_eq!(error.code(), McpFailureCode::Cleanup);
    assert_eq!(status.failure(), Some(McpFailureCode::Cleanup));
    fixture.abort();
    let _ = fixture.await;
}
