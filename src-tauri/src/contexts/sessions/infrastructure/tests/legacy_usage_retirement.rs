use super::*;

#[test]
fn legacy_message_usage_is_ignored_after_ledger_cutover() {
    let fixture = fixture("sessions-message-usage");
    let repository = &fixture.repository;
    let session = session_record(
        "session-usage",
        SessionLifecycle::Idle,
        "Usage",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create session");
    let streaming = message_record(
        "message-usage",
        session.id(),
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "",
    );
    SessionMessageRepository::insert(repository, &streaming).expect("insert message");
    let mut completed = streaming.clone();
    completed
        .message
        .transition_to(MessageStatus::Completed)
        .expect("complete transition");
    completed.content = "done".to_string();
    completed.token_usage = Some(MessageTokenUsage {
        input: 7,
        output: 11,
    });
    SessionTransactionPort::complete_message(
        repository,
        &completed,
        Some(&usage_record("message-usage", session.id(), "codex-cli")),
        None,
    )
    .expect("complete with usage");

    let statistics = SessionUsageRepository::statistics(
        repository,
        UsageStatisticsRange::All,
        None,
        "2026-07-18T11:00:00+00:00",
    )
    .expect("statistics");
    assert_eq!(statistics.reported.total_tokens, 0);
    assert_eq!(statistics.coverage.reported_responses, 0);
    assert_eq!(statistics.counted_sessions, 0);

    SessionTransactionPort::delete_session(repository, session.aggregate.id())
        .expect("delete session");
    let usage_count: i64 = fixture
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'usage_records'",
            [],
            |row| row.get(0),
        )
        .expect("usage count");
    assert_eq!(usage_count, 0);
}

#[test]
fn terminal_usage_message_lookup_is_retired() {
    let fixture = fixture("sessions-terminal-usage-lookup");
    let repository = &fixture.repository;
    let session = session_record(
        "session-terminal-usage",
        SessionLifecycle::Idle,
        "Terminal Usage",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create session");

    for (message_id, updated_at, source) in [
        (
            "message-terminal-older",
            "2026-07-18T10:01:00+00:00",
            "cli-session-log",
        ),
        (
            "message-terminal-newer",
            "2026-07-18T10:02:00+00:00",
            "cli-session-log",
        ),
        (
            "message-provider-newest",
            "2026-07-18T10:03:00+00:00",
            "provider",
        ),
    ] {
        let streaming = message_record(
            message_id,
            session.id(),
            MessageRole::Assistant,
            MessageStatus::Streaming,
            "",
        );
        SessionMessageRepository::insert(repository, &streaming).expect("insert message");
        let mut completed = streaming;
        completed
            .message
            .transition_to(MessageStatus::Completed)
            .expect("complete transition");
        completed.updated_at = updated_at.to_string();
        let mut usage = usage_record(message_id, session.id(), "codex-cli");
        usage.source = source.to_string();
        SessionTransactionPort::complete_message(repository, &completed, Some(&usage), None)
            .expect("complete with usage");
    }

    let retired_table_count: i64 = fixture
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'usage_records'",
            [],
            |row| row.get(0),
        )
        .expect("retired table count");
    assert_eq!(retired_table_count, 0);
}

#[test]
fn deleting_an_assistant_message_never_recreates_the_retired_usage_table() {
    let fixture = fixture("sessions-message-owned-usage");
    let repository = &fixture.repository;
    let session = session_record(
        "session-message-owner",
        SessionLifecycle::Idle,
        "Message Owner",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create session");
    let streaming = message_record(
        "message-owned-usage",
        session.id(),
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "",
    );
    SessionMessageRepository::insert(repository, &streaming).expect("insert message");
    let mut completed = streaming.clone();
    completed
        .message
        .transition_to(MessageStatus::Completed)
        .expect("complete transition");
    SessionTransactionPort::complete_message(
        repository,
        &completed,
        Some(&usage_record(
            "message-owned-usage",
            session.id(),
            "codex-cli",
        )),
        None,
    )
    .expect("complete with usage");

    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "DELETE FROM messages WHERE id = ?1",
            ["message-owned-usage"],
        )
        .expect("delete message");
    let usage_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'usage_records'",
            [],
            |row| row.get(0),
        )
        .expect("usage count");
    let session_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
        .expect("session count");

    assert_eq!(usage_count, 0);
    assert_eq!(session_count, 1);
}

#[test]
fn legacy_usage_payload_does_not_affect_message_completion() {
    let fixture = fixture("sessions-usage-rollback");
    let repository = &fixture.repository;
    let session = session_record(
        "session-usage-rollback",
        SessionLifecycle::Idle,
        "Usage Rollback",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create session");
    let streaming = message_record(
        "message-usage-rollback",
        session.id(),
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "",
    );
    SessionMessageRepository::insert(repository, &streaming).expect("insert message");
    let mut completed = streaming.clone();
    completed
        .message
        .transition_to(MessageStatus::Completed)
        .expect("complete transition");

    SessionTransactionPort::complete_message(
        repository,
        &completed,
        Some(&usage_record(
            "message-usage-rollback",
            session.id(),
            "missing-agent",
        )),
        None,
    )
    .expect("legacy usage is ignored");
    let status: String = fixture
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT status FROM messages WHERE id = ?1",
            ["message-usage-rollback"],
            |row| row.get(0),
        )
        .expect("message status");
    assert_eq!(status, "completed");
}

#[test]
fn runtime_stream_updates_cannot_resurrect_cancelled_messages_and_sync_active_lifecycle() {
    let fixture = fixture("sessions-runtime-stream-cancel");
    let repository = &fixture.repository;
    let session = session_record(
        "session-runtime-cancel",
        SessionLifecycle::Idle,
        "Runtime Cancel",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::Activate)
        .expect("create active session");
    let streaming = message_record(
        "message-runtime-cancel",
        session.id(),
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "",
    );
    SessionMessageRepository::insert(repository, &streaming).expect("insert message");

    let mut cancelled = streaming.clone();
    cancelled
        .message
        .transition_to(MessageStatus::Cancelled)
        .expect("cancel transition");
    cancelled.updated_at = "2026-07-18T10:01:00+00:00".to_string();
    assert_eq!(
        SessionTransactionPort::cancel_messages(repository, &[cancelled]).expect("cancel message"),
        vec!["message-runtime-cancel".to_string()]
    );

    let mut stale_stream = streaming;
    stale_stream.content = "late token".to_string();
    stale_stream.updated_at = "2026-07-18T10:02:00+00:00".to_string();
    SessionMessageRepository::save_stream_fields(repository, &stale_stream)
        .expect("save late stream fields");

    let mut running = session;
    running
        .aggregate
        .transition_to(SessionLifecycle::Running)
        .expect("running transition");
    running.updated_at = "2026-07-18T10:03:00+00:00".to_string();
    SessionTransactionPort::save_runtime_session(repository, &running)
        .expect("save runtime session");

    let connection = fixture.database.connection().expect("connection");
    let (status, content): (String, String) = connection
        .query_row(
            "SELECT status, content FROM messages WHERE id = ?1",
            ["message-runtime-cancel"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("message state");
    let workflow_lifecycle: String = connection
        .query_row(
            "SELECT lifecycle_state FROM workflow_state WHERE active_session_id = ?1",
            ["session-runtime-cancel"],
            |row| row.get(0),
        )
        .expect("workflow lifecycle");

    assert_eq!(status, "cancelled");
    assert_eq!(content, "late token");
    assert_eq!(workflow_lifecycle, "running");
}

#[test]
fn retired_usage_schema_is_a_repeatable_no_op() {
    let fixture = fixture("sessions-usage-backfill");
    let repository = &fixture.repository;
    let session = session_record(
        "session-backfill",
        SessionLifecycle::Idle,
        "Backfill",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create session");
    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            r#"
            INSERT INTO messages (
                id, session_id, role, status, content, token_input, token_output,
                created_at, updated_at
            ) VALUES (?1, ?2, 'assistant', 'completed', '', 12, 7, ?3, ?3)
            "#,
            params![
                "message-backfill",
                session.id(),
                "2026-07-18T10:00:00+00:00"
            ],
        )
        .expect("legacy message");
    apply_usage_schema(&connection).expect("first schema apply");
    apply_usage_schema(&connection).expect("second schema apply");

    let table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'usage_records'",
            [],
            |row| row.get(0),
        )
        .expect("retired table count");
    assert_eq!(table_count, 0);
}
