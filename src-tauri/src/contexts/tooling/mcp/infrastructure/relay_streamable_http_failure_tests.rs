use super::*;

#[test]
fn http_failures_return_safe_errors_for_the_originating_request() {
    let cases = [
        (
            "302 Found",
            Some("application/json"),
            br#"{"secret":"redirect-body"}"#.as_slice(),
            McpFailureCode::UpstreamHttp,
            -32004,
        ),
        (
            "503 Service Unavailable",
            Some("application/json"),
            br#"{"secret":"status-body"}"#.as_slice(),
            McpFailureCode::UpstreamHttp,
            -32004,
        ),
        (
            "200 OK",
            Some("text/plain"),
            b"credential=invalid-content".as_slice(),
            McpFailureCode::Protocol,
            -32603,
        ),
        (
            "200 OK",
            Some("application/json"),
            b"credential=malformed-json".as_slice(),
            McpFailureCode::Protocol,
            -32603,
        ),
        (
            "200 OK",
            Some("application/json"),
            br#"{"jsonrpc":"2.0","id":999,"result":{}}"#.as_slice(),
            McpFailureCode::Protocol,
            -32603,
        ),
    ];

    for (status, content_type, body, expected_failure, expected_protocol_code) in cases {
        let (listener, url) = listener_url();
        let fixture = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("request");
            let _ = read_request(&mut stream);
            write_response(&mut stream, status, content_type, None, body);
        });
        let mut output = Vec::new();
        let error = run_stream(
            &url,
            &BTreeMap::new(),
            "traceparent",
            Duration::from_secs(2),
            None,
            Cursor::new(
                br#"{"jsonrpc":"2.0","id":"origin-7","method":"tools/call"}
"#,
            ),
            &mut output,
        )
        .expect_err("relay failure");

        assert_eq!(error.code(), expected_failure);
        let response: serde_json::Value = serde_json::from_slice(&output).expect("safe failure");
        assert_eq!(response["id"], "origin-7");
        assert_eq!(response["error"]["code"], expected_protocol_code);
        let serialized = String::from_utf8(output).expect("UTF-8 response");
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("credential"));
        fixture.join().expect("fixture");
    }
}

#[test]
fn disconnect_timeout_and_delete_failure_are_bounded_and_typed() {
    for (delay, timeout, expected_failure, expected_protocol_code) in [
        (
            Duration::ZERO,
            Duration::from_secs(2),
            McpFailureCode::Transport,
            -32000,
        ),
        (
            Duration::from_millis(100),
            Duration::from_millis(20),
            McpFailureCode::Timeout,
            -32001,
        ),
    ] {
        let (listener, url) = listener_url();
        let fixture = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("request");
            let _ = read_request(&mut stream);
            if delay.is_zero() {
                stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 64\r\nConnection: close\r\n\r\n{",
                    )
                    .expect("partial response");
                stream.flush().expect("partial response flush");
            } else {
                thread::sleep(delay);
            }
        });
        let mut output = Vec::new();
        let error = run_stream(
            &url,
            &BTreeMap::new(),
            "traceparent",
            timeout,
            None,
            Cursor::new(
                br#"{"jsonrpc":"2.0","id":8,"method":"ping"}
"#,
            ),
            &mut output,
        )
        .expect_err("transport failure");
        assert_eq!(error.code(), expected_failure);
        let response: serde_json::Value = serde_json::from_slice(&output).expect("safe failure");
        assert_eq!(response["id"], 8);
        assert_eq!(response["error"]["code"], expected_protocol_code);
        fixture.join().expect("fixture");
    }

    let (listener, url) = listener_url();
    let fixture = thread::spawn(move || {
        let (mut request, _) = listener.accept().expect("request");
        let _ = read_request(&mut request);
        write_response(
            &mut request,
            "200 OK",
            Some("application/json"),
            Some("session-cleanup"),
            br#"{"jsonrpc":"2.0","id":9,"result":{}}"#,
        );
        let (mut delete, _) = listener.accept().expect("DELETE");
        assert!(read_request(&mut delete)
            .to_ascii_lowercase()
            .starts_with("delete "));
        write_response(&mut delete, "500 Internal Server Error", None, None, b"");
    });
    let mut output = Vec::new();
    let error = run_stream(
        &url,
        &BTreeMap::new(),
        "traceparent",
        Duration::from_secs(2),
        None,
        Cursor::new(
            br#"{"jsonrpc":"2.0","id":9,"method":"initialize"}
"#,
        ),
        &mut output,
    )
    .expect_err("cleanup failure");
    assert_eq!(error.code(), McpFailureCode::Cleanup);
    assert_eq!(
        String::from_utf8(output).expect("response").lines().count(),
        1
    );
    fixture.join().expect("fixture");
}
