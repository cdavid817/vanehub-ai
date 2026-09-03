use super::*;

#[test]
fn post_status_failure_returns_safe_error_for_originating_request() {
    let (listener, url) = listener_url();
    let fixture = thread::spawn(move || {
        let (mut events, _) = listener.accept().expect("event stream");
        let _ = read_request(&mut events);
        open_event_stream(&mut events, "/messages");
        let (mut post, _) = listener.accept().expect("POST");
        let _ = read_request(&mut post);
        status_response(
            &mut post,
            "503 Service Unavailable",
            b"credential=upstream-secret",
        );
    });
    let mut output = Vec::new();

    let error = run_stream(
        &url,
        &BTreeMap::new(),
        "traceparent",
        Duration::from_secs(2),
        McpCancellation::default(),
        None,
        Cursor::new(
            br#"{"jsonrpc":"2.0","id":21,"method":"tools/call"}
"#,
        ),
        &mut output,
    )
    .expect_err("POST failure");

    assert_eq!(error.code(), McpFailureCode::UpstreamHttp);
    let response: serde_json::Value = serde_json::from_slice(&output).expect("safe failure");
    assert_eq!(response["id"], 21);
    assert_eq!(response["error"]["code"], -32004);
    assert!(!String::from_utf8(output)
        .expect("UTF-8")
        .contains("upstream-secret"));
    fixture.join().expect("fixture");
}

#[test]
fn malformed_and_oversized_sse_data_fail_the_pending_request() {
    for (body, expected_failure, expected_protocol_code) in [
        (
            b"event: message\ndata: credential=malformed\n\n".to_vec(),
            McpFailureCode::Protocol,
            -32603,
        ),
        (
            format!(
                "event: message\ndata: {}\n\n",
                "x".repeat(McpLimits::DEFAULT.protocol_message_bytes + 1)
            )
            .into_bytes(),
            McpFailureCode::LimitExceeded,
            -32003,
        ),
    ] {
        let (listener, url) = listener_url();
        let fixture = thread::spawn(move || {
            let (mut events, _) = listener.accept().expect("event stream");
            let _ = read_request(&mut events);
            open_event_stream(&mut events, "/messages");
            let (mut post, _) = listener.accept().expect("POST");
            let _ = read_request(&mut post);
            accepted(&mut post);
            events.write_all(&body).expect("failure event");
            events.flush().expect("event flush");
        });
        let mut output = Vec::new();
        let error = run_stream(
            &url,
            &BTreeMap::new(),
            "traceparent",
            Duration::from_secs(5),
            McpCancellation::default(),
            None,
            Cursor::new(
                br#"{"jsonrpc":"2.0","id":"legacy-origin","method":"ping"}
"#,
            ),
            &mut output,
        )
        .expect_err("SSE failure");

        assert_eq!(error.code(), expected_failure);
        let response: serde_json::Value = serde_json::from_slice(&output).expect("safe failure");
        assert_eq!(response["id"], "legacy-origin");
        assert_eq!(response["error"]["code"], expected_protocol_code);
        assert!(!String::from_utf8(output)
            .expect("UTF-8")
            .contains("credential"));
        fixture.join().expect("fixture");
    }
}
