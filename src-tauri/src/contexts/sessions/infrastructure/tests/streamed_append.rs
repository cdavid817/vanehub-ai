use super::*;

/// Inserts one streaming assistant message and returns the fixture holding it. `message_record`
/// populates the structured columns, which is what makes them useful as untouched witnesses.
fn streaming_fixture(name: &str) -> Fixture {
    let fixture = fixture(name);
    let session = session_record(
        "session-stream-append",
        SessionLifecycle::Idle,
        "Streaming",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create session");
    let message = message_record(
        "message-stream-append",
        session.id(),
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "",
    );
    SessionMessageRepository::insert(&fixture.repository, &message).expect("insert message");
    fixture
}

fn stored(fixture: &Fixture) -> MessageRecord {
    SessionMessageRepository::find(
        &fixture.repository,
        &MessageId::parse("message-stream-append").expect("message id"),
    )
    .expect("find message")
    .expect("message")
}

fn append(fixture: &Fixture, field: StreamTextField, delta: &str, at: &str) {
    SessionMessageRepository::append_stream_text(
        &fixture.repository,
        &MessageId::parse("message-stream-append").expect("message id"),
        field,
        delta,
        at,
    )
    .expect("append streamed text");
}

#[test]
fn successive_content_appends_concatenate_in_order() {
    let fixture = streaming_fixture("sessions-stream-append-order");

    append(
        &fixture,
        StreamTextField::Content,
        "one ",
        "2026-07-18T10:00:01+00:00",
    );
    append(
        &fixture,
        StreamTextField::Content,
        "two ",
        "2026-07-18T10:00:02+00:00",
    );
    append(
        &fixture,
        StreamTextField::Content,
        "three",
        "2026-07-18T10:00:03+00:00",
    );

    assert_eq!(stored(&fixture).content, "one two three");
}

#[test]
fn appending_content_leaves_the_structured_columns_untouched() {
    // The token path never carries tool calls or rich blocks, so rewriting -- and re-serializing --
    // those columns on every flush is pure cost. This is the assertion that keeps them out of it.
    let fixture = streaming_fixture("sessions-stream-append-structured");
    let before = stored(&fixture);

    append(
        &fixture,
        StreamTextField::Content,
        "delta",
        "2026-07-18T10:00:01+00:00",
    );

    let after = stored(&fixture);
    assert_eq!(after.tool_use, before.tool_use);
    assert_eq!(after.rich_blocks, before.rich_blocks);
    assert_eq!(after.thinking_content, before.thinking_content);
}

#[test]
fn appending_content_advances_the_update_timestamp() {
    let fixture = streaming_fixture("sessions-stream-append-timestamp");

    append(
        &fixture,
        StreamTextField::Content,
        "delta",
        "2026-07-18T10:00:09+00:00",
    );

    assert_eq!(stored(&fixture).updated_at, "2026-07-18T10:00:09+00:00");
}

#[test]
fn thinking_appends_onto_an_absent_column() {
    let fixture = streaming_fixture("sessions-stream-append-thinking-null");
    let mut cleared = stored(&fixture);
    cleared.thinking_content = None;
    SessionMessageRepository::save(&fixture.repository, &cleared).expect("clear thinking");

    append(
        &fixture,
        StreamTextField::Thinking,
        "first",
        "2026-07-18T10:00:01+00:00",
    );
    append(
        &fixture,
        StreamTextField::Thinking,
        "-second",
        "2026-07-18T10:00:02+00:00",
    );

    assert_eq!(
        stored(&fixture).thinking_content,
        Some("first-second".to_string())
    );
}

#[test]
fn appending_content_leaves_thinking_untouched() {
    let fixture = streaming_fixture("sessions-stream-append-isolation");

    append(
        &fixture,
        StreamTextField::Thinking,
        " more",
        "2026-07-18T10:00:01+00:00",
    );
    append(
        &fixture,
        StreamTextField::Content,
        "body",
        "2026-07-18T10:00:02+00:00",
    );

    let after = stored(&fixture);
    assert_eq!(after.content, "body");
    assert_eq!(after.thinking_content, Some("thinking more".to_string()));
}

#[test]
fn appending_to_a_missing_message_reports_it_as_missing() {
    let fixture = streaming_fixture("sessions-stream-append-missing");

    let error = SessionMessageRepository::append_stream_text(
        &fixture.repository,
        &MessageId::parse("message-absent").expect("message id"),
        StreamTextField::Content,
        "delta",
        "2026-07-18T10:00:01+00:00",
    )
    .expect_err("missing message");

    assert!(matches!(
        error,
        SessionsApplicationError::MessageNotFound(id) if id == "message-absent"
    ));
}
