use super::*;

fn initialized(adapter: &mut ClaudeDelegationAdapter) {
    assert_eq!(
        adapter
            .decode_stdout_line(br#"{"type":"system","session_id":"fresh"}"#)
            .expect("initialization"),
        vec![DelegationProviderEvent::Initialized]
    );
}

#[test]
fn successful_result_requires_initialization_schema_and_zero_exit() {
    let mut adapter = ClaudeDelegationAdapter::new();
    initialized(&mut adapter);
    let events = adapter
        .decode_stdout_line(
            br#"{"type":"result","subtype":"success","is_error":false,"structured_output":{"schema_version":1},"usage":{"input_tokens":2}}"#,
        )
        .expect("terminal");
    assert_eq!(
        events,
        vec![
            DelegationProviderEvent::UsageUpdated,
            DelegationProviderEvent::FinalCandidate
        ]
    );
    assert_eq!(
        adapter.finalize(Some(0)).expect("success")["schema_version"],
        1
    );
}

#[test]
fn malformed_missing_duplicate_and_exit_mismatch_fail_explicitly() {
    assert_eq!(
        ClaudeDelegationAdapter::new().decode_stdout_line(b"not-json"),
        Err(ClaudeProtocolError::MalformedJson)
    );
    let mut missing = ClaudeDelegationAdapter::new();
    initialized(&mut missing);
    assert_eq!(
        missing.finalize(Some(0)),
        Err(ClaudeProtocolError::MissingTerminal)
    );

    let mut duplicate = ClaudeDelegationAdapter::new();
    initialized(&mut duplicate);
    let terminal = br#"{"type":"result","subtype":"success","structured_output":{}}"#;
    duplicate.decode_stdout_line(terminal).expect("first");
    assert_eq!(
        duplicate.decode_stdout_line(terminal),
        Err(ClaudeProtocolError::DuplicateTerminal)
    );

    let mut exit = ClaudeDelegationAdapter::new();
    initialized(&mut exit);
    exit.decode_stdout_line(terminal).expect("terminal");
    assert_eq!(
        exit.finalize(Some(1)),
        Err(ClaudeProtocolError::ExitMismatch)
    );
}

#[test]
fn action_lifecycle_is_stateful_and_unknown_data_is_reduced() {
    let mut adapter = ClaudeDelegationAdapter::new();
    initialized(&mut adapter);
    assert_eq!(
        adapter
            .decode_stdout_line(br#"{"type":"tool_use","id":"tool-1","input":{"secret":"drop"}}"#)
            .expect("start"),
        vec![DelegationProviderEvent::ActionStarted {
            id: "tool-1".into()
        }]
    );
    assert_eq!(
        adapter.decode_stdout_line(br#"{"type":"tool_result","tool_use_id":"other"}"#),
        Err(ClaudeProtocolError::InvalidActionOrder)
    );
    let unknown = adapter
        .decode_stdout_line(br#"{"type":"future_event","payload":"private"}"#)
        .expect("forward-compatible");
    assert!(matches!(
        &unknown[0],
        DelegationProviderEvent::UnknownEvent { event_type, hash, size }
            if event_type == "future_event" && hash.starts_with("sha256:") && *size > 0
    ));
}

#[test]
fn real_message_envelopes_drive_tool_lifecycle_and_partial_events_stay_progress() {
    let mut adapter = ClaudeDelegationAdapter::new();
    initialized(&mut adapter);
    assert_eq!(
        adapter
            .decode_stdout_line(
                br#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"call-1","name":"Read"}]}}"#,
            )
            .expect("assistant tool call"),
        vec![DelegationProviderEvent::ActionStarted { id: "call-1".into() }]
    );
    assert_eq!(
        adapter
            .decode_stdout_line(
                br#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"private"}}}"#,
            )
            .expect("partial progress"),
        vec![DelegationProviderEvent::Progress]
    );
    assert_eq!(
        adapter
            .decode_stdout_line(
                br#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"call-1","content":"private"}]}}"#,
            )
            .expect("tool result"),
        vec![DelegationProviderEvent::ActionCompleted { id: "call-1".into() }]
    );
}
