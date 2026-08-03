use super::*;
use crate::contexts::tooling::mcp::application::McpLimits;
use std::io::{BufReader, Cursor};

fn parse(bytes: &[u8]) -> JsonRpcFrame {
    parse_json_rpc_frame(bytes).expect("valid JSON-RPC")
}

#[test]
fn bounded_reader_accepts_exact_limit_and_rejects_limit_plus_one() {
    let maximum = McpLimits::DEFAULT.protocol_message_bytes;
    let mut exact = vec![b' '; maximum];
    exact.push(b'\n');
    let mut exact_source = BufReader::new(Cursor::new(exact.clone()));
    let mut frame = Vec::new();

    assert_eq!(
        read_bounded_frame(&mut exact_source, &mut frame, maximum).expect("exact frame"),
        exact.len()
    );
    assert_eq!(frame, exact);

    let oversized = vec![b'x'; maximum + 1];
    let mut oversized_source = BufReader::new(Cursor::new(oversized));
    assert_eq!(
        read_bounded_frame(&mut oversized_source, &mut frame, maximum)
            .expect_err("limit plus one")
            .kind(),
        std::io::ErrorKind::InvalidData
    );
}

#[test]
fn parser_distinguishes_requests_notifications_and_responses() {
    assert_eq!(
        parse(br#"{"jsonrpc":"2.0","id":"a","method":"tools/call"}"#),
        JsonRpcFrame::Request {
            id: JsonRpcId::String("a".to_string()),
            method: "tools/call".to_string(),
        }
    );
    assert_eq!(
        parse(br#"{"jsonrpc":"2.0","method":"notifications/progress"}"#),
        JsonRpcFrame::Notification {
            method: "notifications/progress".to_string(),
        }
    );
    assert_eq!(
        parse(br#"{"jsonrpc":"2.0","id":7,"error":{"code":-1}}"#),
        JsonRpcFrame::Response {
            id: JsonRpcId::Number("7".to_string()),
            success: false,
        }
    );
}

#[test]
fn same_id_is_correlated_independently_in_both_directions() {
    let mut correlation = JsonRpcCorrelation::default();
    let id = JsonRpcId::Number("1".to_string());
    correlation
        .insert_request(
            RelayDirection::ParentToUpstream,
            id.clone(),
            ("tools/call", "client-span"),
        )
        .expect("client request");
    correlation
        .insert_request(
            RelayDirection::UpstreamToParent,
            id.clone(),
            ("roots/list", "server-span"),
        )
        .expect("server request");

    let client = correlation
        .complete_response(RelayDirection::UpstreamToParent, &id)
        .expect("client response");
    assert_eq!(client.token, ("tools/call", "client-span"));
    assert_eq!(correlation.pending_count(), 1);

    let server = correlation
        .complete_response(RelayDirection::ParentToUpstream, &id)
        .expect("server response");
    assert_eq!(server.token, ("roots/list", "server-span"));
    assert_eq!(correlation.pending_count(), 0);
}

#[test]
fn notifications_create_no_correlation_state() {
    let frame = parse(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    let correlation = JsonRpcCorrelation::<()>::default();

    assert!(matches!(frame, JsonRpcFrame::Notification { .. }));
    assert_eq!(correlation.pending_count(), 0);
}

#[test]
fn oldest_request_deadline_is_removed_and_remaining_requests_are_drained() {
    let mut correlation = JsonRpcCorrelation::default();
    correlation
        .insert_request(
            RelayDirection::ParentToUpstream,
            JsonRpcId::Number("1".to_string()),
            "first",
        )
        .expect("first");
    correlation
        .insert_request(
            RelayDirection::UpstreamToParent,
            JsonRpcId::Number("2".to_string()),
            "second",
        )
        .expect("second");

    let deadline = correlation
        .oldest_deadline(Duration::ZERO)
        .expect("deadline");
    let expired = correlation
        .take_expired(deadline, Duration::ZERO)
        .expect("expired request");

    assert_eq!(expired.direction, RelayDirection::ParentToUpstream);
    assert_eq!(expired.id, JsonRpcId::Number("1".to_string()));
    assert_eq!(expired.pending.token, "first");
    assert_eq!(correlation.close_and_drain()[0].token, "second");
}

#[test]
fn invalid_json_rpc_shapes_are_rejected() {
    for frame in [
        br#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#.as_slice(),
        br#"{"jsonrpc":"2.0","id":true,"method":"ping"}"#.as_slice(),
        br#"{"jsonrpc":"2.0","id":1}"#.as_slice(),
        br#"[]"#.as_slice(),
    ] {
        assert!(parse_json_rpc_frame(frame).is_err(), "{frame:?}");
    }
}
