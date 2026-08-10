use super::*;
use std::io::{Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn listener_url() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let url = format!("http://{}", listener.local_addr().expect("address"));
    (listener, url)
}

fn read_request(stream: &mut TcpStream) -> String {
    try_read_request(stream).expect("request ended before headers")
}

fn try_read_request(stream: &mut TcpStream) -> Option<String> {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let count = stream.read(&mut chunk).expect("request bytes");
        if count == 0 {
            return None;
        }
        bytes.extend_from_slice(&chunk[..count]);
        let Some(header_end) = bytes.windows(4).position(|value| value == b"\r\n\r\n") else {
            continue;
        };
        let body_start = header_end + 4;
        let headers = String::from_utf8_lossy(&bytes[..body_start]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|value| value.trim().parse::<usize>().ok())
            })
            .unwrap_or(0);
        if bytes.len() >= body_start + content_length {
            return Some(String::from_utf8(bytes).expect("UTF-8 request"));
        }
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: Option<&str>,
    session_id: Option<&str>,
    body: &[u8],
) {
    let mut headers = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(content_type) = content_type {
        headers.push_str(&format!("Content-Type: {content_type}\r\n"));
    }
    if let Some(session_id) = session_id {
        headers.push_str(&format!("Mcp-Session-Id: {session_id}\r\n"));
    }
    headers.push_str("\r\n");
    stream.write_all(headers.as_bytes()).expect("headers");
    stream.write_all(body).expect("body");
    stream.flush().expect("response");
}

#[test]
fn json_notification_session_headers_and_delete_share_one_relay_lifecycle() {
    let (listener, url) = listener_url();
    let response = br#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18"}}"#;
    let fixture = thread::spawn(move || {
        let (mut initialize, _) = listener.accept().expect("initialize");
        let initialize_request = read_request(&mut initialize);
        assert!(!initialize_request
            .to_ascii_lowercase()
            .contains("mcp-session-id:"));
        write_response(
            &mut initialize,
            "200 OK",
            Some("application/json; charset=utf-8"),
            Some("session-7"),
            response,
        );

        let (mut notification, _) = listener.accept().expect("notification");
        let notification_request = read_request(&mut notification).to_ascii_lowercase();
        assert!(notification_request.contains("mcp-session-id: session-7"));
        assert!(notification_request.contains("mcp-protocol-version: 2025-06-18"));
        write_response(&mut notification, "202 Accepted", None, None, b"");

        let (mut delete, _) = listener.accept().expect("delete");
        let delete_request = read_request(&mut delete).to_ascii_lowercase();
        assert!(delete_request.starts_with("delete "));
        assert!(delete_request.contains("mcp-session-id: session-7"));
        write_response(&mut delete, "200 OK", None, None, b"");
    });
    let input = br#"{"jsonrpc":"2.0","id":1,"method":"initialize"}
{"jsonrpc":"2.0","method":"notifications/initialized"}
"#;
    let mut output = Vec::new();

    run_stream(
        &url,
        &BTreeMap::new(),
        "traceparent",
        Duration::from_secs(2),
        None,
        Cursor::new(input),
        &mut output,
    )
    .expect("relay");

    assert_eq!(
        output,
        br#"{"id":1,"jsonrpc":"2.0","result":{"protocolVersion":"2025-06-18"}}
"#
    );
    fixture.join().expect("fixture");
}

#[test]
fn incremental_sse_emits_only_json_rpc_data_lines_until_matching_response() {
    let (listener, url) = listener_url();
    let fixture = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("SSE request");
        let _ = read_request(&mut stream);
        let body = format!(
            ":{}\nevent: message\ndata: {{\"jsonrpc\":\"2.0\",\"id\":\"server-9\",\"method\":\"roots/list\"}}\n\nevent: ignored\ndata: not-json\n\ndata: {{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{{\"ok\":true}}}}\n\n",
            "x".repeat(9_000)
        );
        write_response(
            &mut stream,
            "200 OK",
            Some("text/event-stream"),
            None,
            body.as_bytes(),
        );
    });
    let mut output = Vec::new();

    run_stream(
        &url,
        &BTreeMap::new(),
        "traceparent",
        Duration::from_secs(2),
        None,
        Cursor::new(
            br#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}
"#,
        ),
        &mut output,
    )
    .expect("SSE relay");

    let output = String::from_utf8(output).expect("output");
    assert_eq!(output.lines().count(), 2);
    assert!(output.contains("\"method\":\"roots/list\""));
    assert!(output.contains("\"id\":2"));
    assert!(!output.contains("data:"));
    assert!(!output.contains("event:"));
    assert!(!output.contains("not-json"));
    fixture.join().expect("fixture");
}

#[test]
fn oversized_content_length_is_rejected_before_body_buffering() {
    let (listener, url) = listener_url();
    let fixture = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("request");
        let _ = read_request(&mut stream);
        let length = McpLimits::DEFAULT.protocol_message_bytes + 1;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
        )
        .expect("response");
    });

    let mut output = Vec::new();
    let error = run_stream(
        &url,
        &BTreeMap::new(),
        "traceparent",
        Duration::from_secs(2),
        None,
        Cursor::new(
            br#"{"jsonrpc":"2.0","id":3,"method":"ping"}
"#,
        ),
        &mut output,
    )
    .expect_err("oversized response");

    assert!(error.to_string().contains("exceeded"));
    assert_eq!(error.code(), McpFailureCode::LimitExceeded);
    let response: serde_json::Value = serde_json::from_slice(&output).expect("limit response");
    assert_eq!(response["id"], 3);
    assert_eq!(response["error"]["code"], -32003);
    fixture.join().expect("fixture");
}

#[path = "relay_streamable_http_failure_tests.rs"]
mod failure_tests;
