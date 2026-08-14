use super::*;

#[test]
fn completed_turn_requires_private_schema_valid_final_file_and_zero_exit() {
    let mut adapter = CodexDelegationAdapter::new();
    assert_eq!(
        adapter
            .decode_stdout_line(br#"{"type":"thread.started","thread_id":"one"}"#)
            .expect("start"),
        vec![DelegationProviderEvent::Initialized]
    );
    adapter
        .decode_stdout_line(br#"{"type":"turn.completed","usage":{}}"#)
        .expect("terminal");
    assert_eq!(
        adapter
            .finalize(Some(0), br#"{"schema_version":1}"#)
            .expect("final")["schema_version"],
        1
    );
}

#[test]
fn invalid_order_duplicate_terminal_and_outside_final_fail_closed() {
    let mut adapter = CodexDelegationAdapter::new();
    assert_eq!(
        adapter.decode_stdout_line(br#"{"type":"turn.started"}"#),
        Err(CodexProtocolError::EventBeforeInitialization)
    );
    adapter
        .decode_stdout_line(br#"{"type":"thread.started"}"#)
        .expect("start");
    let terminal = br#"{"type":"turn.completed"}"#;
    adapter.decode_stdout_line(terminal).expect("terminal");
    assert_eq!(
        adapter.decode_stdout_line(terminal),
        Err(CodexProtocolError::DuplicateTerminal)
    );
    assert_eq!(
        adapter.finalize(Some(0), b"not-json"),
        Err(CodexProtocolError::InvalidFinalOutput)
    );
}
