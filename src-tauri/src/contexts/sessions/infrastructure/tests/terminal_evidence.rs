use super::*;

#[test]
fn terminal_evidence_read_is_ordered_bounded_and_run_keyed() {
    let fixture = fixture("sessions-terminal-evidence-read");
    let mut session = session_record(
        "session-terminal-evidence",
        SessionLifecycle::Running,
        "Terminal evidence",
        "2026-07-01T00:00:00+00:00",
    );
    session.runtime_session_id = Some("provider-resume".to_string());
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    let mut first = correlated_message_record(
        "message-evidence-first",
        session.id(),
        "run-evidence",
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "partial",
    );
    first.message = SessionMessage::rehydrate_with_correlation(
        first.message.id().clone(),
        session.aggregate.id().clone(),
        MessageRole::Assistant,
        MessageStatus::Streaming,
        FileReferenceSet::default(),
        1,
        Some("run-evidence".to_string()),
    );
    first.tool_use = Some(vec![json!({
        "id": "tool-1",
        "status": "running"
    })]);
    let mut second = correlated_message_record(
        "message-evidence-second",
        session.id(),
        "run-evidence",
        MessageRole::Assistant,
        MessageStatus::Completed,
        "done",
    );
    second.message = SessionMessage::rehydrate_with_correlation(
        second.message.id().clone(),
        session.aggregate.id().clone(),
        MessageRole::Assistant,
        MessageStatus::Completed,
        FileReferenceSet::default(),
        2,
        Some("run-evidence".to_string()),
    );
    SessionMessageRepository::insert(&fixture.repository, &first).expect("insert first");
    SessionMessageRepository::insert(&fixture.repository, &second).expect("insert second");

    let evidence = SessionTerminalEvidencePort::read_terminal_evidence(
        &fixture.repository,
        session.aggregate.id(),
        Some("run-evidence"),
    )
    .expect("read evidence");

    assert_eq!(
        evidence.observed_execution_run_id.as_deref(),
        Some("run-evidence")
    );
    assert_eq!(
        evidence.session.execution_fidelity,
        ExecutionEvidenceFidelity::InteractiveOpaque
    );
    assert_eq!(evidence.live_handle, LiveHandleEvidence::Unavailable);
    assert!(evidence.provider_resume.metadata_present);
    assert_eq!(evidence.messages().len(), 2);
    assert_eq!(evidence.messages()[0].session_sequence, 1);
    assert!(matches!(
        evidence.messages()[0].tool_activity,
        ToolActivityEvidence::Incomplete { count: 1, .. }
    ));
    assert_eq!(evidence.messages()[1].session_sequence, 2);
    assert!(evidence.operations().is_empty());
}

#[test]
fn terminal_evidence_ignores_long_history_and_reports_unfinished_cross_run_work() {
    let fixture = fixture("sessions-terminal-evidence-long-history");
    let session = session_record(
        "session-terminal-evidence-long",
        SessionLifecycle::Running,
        "Long terminal evidence",
        "2026-07-01T00:00:00+00:00",
    );
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    for index in 0..300 {
        let historical = correlated_message_record(
            &format!("message-history-{index:03}"),
            session.id(),
            &format!("run-history-{index:03}"),
            MessageRole::Assistant,
            MessageStatus::Completed,
            "historical",
        );
        SessionMessageRepository::insert(&fixture.repository, &historical)
            .expect("insert historical message");
    }
    let active = correlated_message_record(
        "message-active-run",
        session.id(),
        "run-active",
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "partial",
    );
    SessionMessageRepository::insert(&fixture.repository, &active).expect("insert active message");

    let evidence = SessionTerminalEvidencePort::read_terminal_evidence(
        &fixture.repository,
        session.aggregate.id(),
        Some("run-active"),
    )
    .expect("read active-run evidence");
    assert_eq!(evidence.messages().len(), 1);
    assert_eq!(evidence.messages()[0].message_id, "message-active-run");
    assert!(evidence.conflicting_message().is_none());

    let conflicting = correlated_message_record(
        "message-conflicting-run",
        session.id(),
        "run-conflicting",
        MessageRole::Assistant,
        MessageStatus::Pending,
        "",
    );
    SessionMessageRepository::insert(&fixture.repository, &conflicting)
        .expect("insert conflicting message");
    let evidence = SessionTerminalEvidencePort::read_terminal_evidence(
        &fixture.repository,
        session.aggregate.id(),
        Some("run-active"),
    )
    .expect("read conflicting evidence");
    assert_eq!(
        evidence
            .conflicting_message()
            .map(|message| message.message_id.as_str()),
        Some("message-conflicting-run")
    );
}

#[test]
fn structurally_oversized_active_run_evidence_is_quarantined() {
    let fixture = fixture("sessions-terminal-evidence-structural-bound");
    let mut session = session_record(
        "session-structural-bound",
        SessionLifecycle::Idle,
        "Structural recovery evidence",
        "2026-07-01T00:00:00+00:00",
    );
    session.interaction_mode = "api".to_string();
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    let assistant = correlated_message_record(
        "message-structural-active",
        session.id(),
        "run-structural-bound",
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "partial",
    );
    SessionTransactionPort::start_generation(
        &fixture.repository,
        &GenerationStartRequest {
            session_id: session.id().to_string(),
            execution_run_id: "run-structural-bound".to_string(),
            user_message: None,
            assistant_message: assistant,
            started_at: "2026-07-18T10:00:00+00:00".to_string(),
        },
    )
    .expect("start generation");
    for index in 0..256 {
        let duplicate = correlated_message_record(
            &format!("message-structural-{index:03}"),
            session.id(),
            "run-structural-bound",
            MessageRole::Assistant,
            MessageStatus::Completed,
            "duplicate",
        );
        SessionMessageRepository::insert(&fixture.repository, &duplicate)
            .expect("insert structural evidence");
    }
    let repository = Arc::new(fixture.repository.clone());
    let coordinator = SessionRecoveryCoordinator::new(
        repository.clone(),
        repository.clone(),
        repository,
        Arc::new(SystemSessionClock),
        Arc::new(NoopSessionLogging),
    );

    let result = coordinator
        .run_batch(10, RecoveryTrigger::Startup)
        .expect("publish structural quarantine");
    let persisted = SessionRepository::find(&fixture.repository, session.aggregate.id())
        .expect("find session")
        .expect("session");
    let reports = SessionRecoveryReportRepository::list_reports(
        &fixture.repository,
        session.aggregate.id(),
        10,
    )
    .expect("reports");

    assert_eq!(result.published, 1);
    assert_eq!(
        persisted.aggregate.recovery().status().as_str(),
        "quarantined"
    );
    assert_eq!(
        persisted.aggregate.recovery().active_execution_run_id(),
        Some("run-structural-bound")
    );
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].decision(), RecoveryDecision::Quarantined);
    assert_eq!(
        reports[0].reason_codes(),
        &[RecoveryReasonCode::InvalidExecutionCorrelation]
    );
}

#[test]
fn malformed_persisted_message_evidence_is_quarantined_without_exposing_payload() {
    let fixture = fixture("sessions-terminal-evidence-malformed-row");
    let mut session = session_record(
        "session-malformed-evidence",
        SessionLifecycle::Idle,
        "Malformed recovery evidence",
        "2026-07-01T00:00:00+00:00",
    );
    session.interaction_mode = "api".to_string();
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    let assistant = correlated_message_record(
        "message-malformed-evidence",
        session.id(),
        "run-malformed-evidence",
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "private malformed payload",
    );
    SessionTransactionPort::start_generation(
        &fixture.repository,
        &GenerationStartRequest {
            session_id: session.id().to_string(),
            execution_run_id: "run-malformed-evidence".to_string(),
            user_message: None,
            assistant_message: assistant,
            started_at: "2026-07-18T10:00:00+00:00".to_string(),
        },
    )
    .expect("start generation");
    let connection = fixture.database.connection().expect("database connection");
    connection
        .execute_batch("PRAGMA ignore_check_constraints = ON")
        .expect("allow malformed fixture row");
    connection
        .execute(
            "UPDATE messages SET status = 'malformed-status' WHERE id = ?1",
            ["message-malformed-evidence"],
        )
        .expect("corrupt message status");
    let logging = Arc::new(CapturingSessionLogging::default());
    let repository = Arc::new(fixture.repository.clone());
    let coordinator = SessionRecoveryCoordinator::new(
        repository.clone(),
        repository,
        Arc::new(AbsentHandleEvidence {
            repository: fixture.repository.clone(),
        }),
        Arc::new(SystemSessionClock),
        logging.clone(),
    );

    let result = coordinator
        .run_batch(10, RecoveryTrigger::Startup)
        .expect("quarantine malformed evidence");
    let persisted = SessionRepository::find(&fixture.repository, session.aggregate.id())
        .expect("find session")
        .expect("session");
    let reports = SessionRecoveryReportRepository::list_reports(
        &fixture.repository,
        session.aggregate.id(),
        10,
    )
    .expect("reports");
    let logs = logging.entries.lock().expect("recovery logs");

    assert_eq!(result.published, 1);
    assert_eq!(
        persisted.aggregate.recovery().status().as_str(),
        "quarantined"
    );
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].decision(), RecoveryDecision::Quarantined);
    assert!(logs
        .iter()
        .all(|entry| !entry.message.contains("malformed-status")
            && !entry.message.contains("private malformed payload")));
}

/// A reader holding a snapshot sees one whole generation across a concurrent commit.
///
/// This is the property `read_terminal_evidence` rests on. It reads a session's lifecycle and
/// revisions, then that session's messages, then whether another run has an unfinished message,
/// and it compares them against each other. A session row from before a recovery transition beside
/// messages from after it would describe a state that never existed -- each half individually
/// consistent, which is what makes the bug survive review.
///
/// Two independent pooled connections and a channel rendezvous. No sleep, and no assertion about
/// elapsed time.
#[test]
fn a_reader_sees_one_whole_generation_while_a_writer_commits_the_next() {
    use crate::platform::database::{begin_read_transaction, begin_write_transaction};
    use std::sync::mpsc;

    let fixture = fixture("sessions-evidence-snapshot-generations");
    let session = session_record(
        "session-snapshot",
        SessionLifecycle::Running,
        "Snapshot",
        "2026-08-24T00:00:00+00:00",
    );
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");

    let connection = fixture.database.connection().expect("reader connection");
    let transaction = begin_read_transaction(&connection).expect("read transaction");
    let (lifecycle, revision): (String, i64) = transaction
        .query_row(
            "SELECT lifecycle_state, state_revision FROM sessions WHERE id = ?1",
            [session.id()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read the session");

    // A second connection advances the session while the reader's snapshot is open.
    let writing_database = fixture.database.clone();
    let session_id = session.id().to_string();
    let (committed, done) = mpsc::channel();
    let writing = std::thread::spawn(move || {
        let writer = writing_database.connection().expect("writer connection");
        let transaction = begin_write_transaction(&writer).expect("write transaction");
        transaction
            .execute(
                "UPDATE sessions SET lifecycle_state = 'idle', \
                 state_revision = state_revision + 1 WHERE id = ?1",
                [session_id.as_str()],
            )
            .expect("advance the session");
        transaction.commit().expect("commit");
        committed.send(()).expect("signal");
    });
    done.recv().expect("the writer committed");

    // The reader's second statement must still belong to the generation it started in.
    let (still_lifecycle, still_revision): (String, i64) = transaction
        .query_row(
            "SELECT lifecycle_state, state_revision FROM sessions WHERE id = ?1",
            [session.id()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read again inside the same snapshot");

    assert_eq!(
        (still_lifecycle.as_str(), still_revision),
        (lifecycle.as_str(), revision),
        "both statements must see one generation; a mixture is a state that never existed"
    );

    drop(transaction);
    writing.join().expect("writer thread");

    // A reader that starts after the commit sees the new generation whole -- so this proved
    // isolation rather than that the write never landed.
    let after = begin_read_transaction(&connection).expect("read transaction");
    let (moved_lifecycle, moved_revision): (String, i64) = after
        .query_row(
            "SELECT lifecycle_state, state_revision FROM sessions WHERE id = ?1",
            [session.id()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read after the transaction closed");

    assert_eq!(moved_lifecycle, "idle");
    assert_eq!(moved_revision, revision + 1);
}
