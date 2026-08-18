use super::*;

#[test]
fn malformed_candidate_revisions_are_normalized_and_do_not_block_later_sessions() {
    let fixture = fixture("sessions-recovery-malformed-candidate");
    for (session_id, run_id) in [
        ("session-candidate-a-malformed", "run-candidate-a-malformed"),
        ("session-candidate-b-healthy", "run-candidate-b-healthy"),
    ] {
        let mut session = session_record(
            session_id,
            SessionLifecycle::Idle,
            "Recovery candidate",
            "2026-07-01T00:00:00+00:00",
        );
        session.interaction_mode = "api".to_string();
        fixture
            .repository
            .create_session(&session, SessionActivation::PreserveActive)
            .expect("create session");
        let mut assistant = correlated_message_record(
            &format!("message-{session_id}"),
            session_id,
            run_id,
            MessageRole::Assistant,
            MessageStatus::Streaming,
            "partial response",
        );
        assistant.tool_use = None;
        SessionTransactionPort::start_generation(
            &fixture.repository,
            &GenerationStartRequest {
                session_id: session_id.to_string(),
                execution_run_id: run_id.to_string(),
                user_message: None,
                assistant_message: assistant,
                started_at: "2026-07-18T10:00:00+00:00".to_string(),
            },
        )
        .expect("start generation");
    }
    let connection = fixture.database.connection().expect("database connection");
    connection
        .execute_batch("PRAGMA ignore_check_constraints = ON")
        .expect("allow malformed fixture row");
    connection
        .execute(
            r#"UPDATE sessions
               SET recovery_revision = -1, state_revision = -2, history_revision = -3
               WHERE id = ?1"#,
            ["session-candidate-a-malformed"],
        )
        .expect("corrupt candidate revisions");
    let repository = Arc::new(fixture.repository.clone());
    let coordinator = SessionRecoveryCoordinator::new(
        repository.clone(),
        repository,
        Arc::new(AbsentHandleEvidence {
            repository: fixture.repository.clone(),
        }),
        Arc::new(SystemSessionClock),
        Arc::new(NoopSessionLogging),
    );

    let result = coordinator
        .run_until_drained(100, RecoveryTrigger::Startup)
        .expect("recover all candidates");
    let malformed = SessionRepository::find(
        &fixture.repository,
        &SessionId::parse("session-candidate-a-malformed").expect("session id"),
    )
    .expect("find malformed session")
    .expect("malformed session");
    let healthy = SessionRepository::find(
        &fixture.repository,
        &SessionId::parse("session-candidate-b-healthy").expect("session id"),
    )
    .expect("find healthy session")
    .expect("healthy session");

    assert_eq!(result.published, 2);
    assert_eq!(
        malformed.aggregate.recovery().status().as_str(),
        "quarantined"
    );
    assert_eq!(malformed.aggregate.recovery().recovery_revision(), 1);
    assert_eq!(healthy.aggregate.recovery().status().as_str(), "clean");
    assert_eq!(healthy.aggregate.lifecycle(), SessionLifecycle::Failed);
}

#[test]
fn recovery_candidate_scan_is_bounded_and_claim_is_revision_guarded() {
    let fixture = fixture("sessions-recovery-candidate-claim");
    for session_id in [
        "session-recovery-candidate-a",
        "session-recovery-candidate-b",
    ] {
        fixture
            .repository
            .create_session(
                &session_record(
                    session_id,
                    SessionLifecycle::Running,
                    "Recovery candidate",
                    "2026-07-01T00:00:00+00:00",
                ),
                SessionActivation::PreserveActive,
            )
            .expect("create candidate");
    }
    let candidates =
        SessionRepository::recovery_candidates(&fixture.repository, 1).expect("scan candidates");
    assert_eq!(candidates.len(), 1);
    let request = ClaimRecoveryCandidateRequest {
        candidate: candidates[0].clone(),
        claimed_at: "2026-07-18T10:00:00+00:00".to_string(),
    };

    let claimed = SessionTransactionPort::claim_recovery_candidate(&fixture.repository, &request)
        .expect("claim candidate")
        .expect("claim won");

    assert_eq!(claimed.state_revision, 1);
    assert_eq!(claimed.history_revision, 0);
    let session = SessionRepository::find(
        &fixture.repository,
        &SessionId::parse(&claimed.session_id).expect("session id"),
    )
    .expect("find session")
    .expect("session");
    assert_eq!(
        session.aggregate.recovery().status(),
        crate::contexts::sessions::domain::SessionRecoveryStatus::Reconciling
    );
    assert!(
        SessionTransactionPort::claim_recovery_candidate(&fixture.repository, &request)
            .expect("stale claim")
            .is_none()
    );
}

#[test]
fn recovery_publication_applies_projection_and_report_once_under_claim_revisions() {
    let fixture = fixture("sessions-recovery-publication");
    let session = session_record(
        "session-recovery-publication",
        SessionLifecycle::Idle,
        "Recovery publication",
        "2026-07-01T00:00:00+00:00",
    );
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    SessionTransactionPort::start_generation(
        &fixture.repository,
        &GenerationStartRequest {
            session_id: session.id().to_string(),
            execution_run_id: "run-recovery-publication".to_string(),
            user_message: None,
            assistant_message: correlated_message_record(
                "message-recovery-publication",
                session.id(),
                "run-recovery-publication",
                MessageRole::Assistant,
                MessageStatus::Streaming,
                "partial response",
            ),
            started_at: "2026-07-18T10:00:00+00:00".to_string(),
        },
    )
    .expect("start generation");
    let candidate = SessionRepository::recovery_candidates(&fixture.repository, 10)
        .expect("candidates")
        .into_iter()
        .find(|candidate| candidate.session_id == session.id())
        .expect("candidate");
    let claim = SessionTransactionPort::claim_recovery_candidate(
        &fixture.repository,
        &ClaimRecoveryCandidateRequest {
            candidate,
            claimed_at: "2026-07-18T10:01:00+00:00".to_string(),
        },
    )
    .expect("claim")
    .expect("claim won");
    let report = SessionRecoveryReport::new(
        "report-recovery-publication".to_string(),
        session.id().to_string(),
        claim.recovery_revision + 1,
        RecoveryTrigger::Startup,
        "starting".to_string(),
        Some("run-recovery-publication".to_string()),
        RecoveryDecision::InterruptedWithoutToolAmbiguity,
        vec![RecoveryReasonCode::InterruptedToolFreeResponse],
        vec![RecoveryEvidenceReference::Message {
            message_id: "message-recovery-publication".to_string(),
            execution_run_id: Some("run-recovery-publication".to_string()),
            status: "streaming".to_string(),
        }],
        "2026-07-18T10:02:00+00:00".to_string(),
    );
    let request = PublishRecoveryRequest {
        claim,
        assistant_message_id: Some("message-recovery-publication".to_string()),
        report,
        published_at: "2026-07-18T10:02:00+00:00".to_string(),
    };

    assert!(
        SessionTransactionPort::publish_recovery(&fixture.repository, &request)
            .expect("publish recovery")
    );
    assert!(
        !SessionTransactionPort::publish_recovery(&fixture.repository, &request)
            .expect("duplicate publication")
    );
    let persisted = SessionRepository::find(&fixture.repository, session.aggregate.id())
        .expect("find session")
        .expect("session");
    assert_eq!(persisted.aggregate.lifecycle(), SessionLifecycle::Failed);
    assert_eq!(
        persisted.aggregate.recovery().status(),
        crate::contexts::sessions::domain::SessionRecoveryStatus::Clean
    );
    assert_eq!(persisted.aggregate.recovery().recovery_revision(), 1);
    assert_eq!(
        persisted.aggregate.recovery().active_execution_run_id(),
        None
    );
    let message = SessionMessageRepository::find(
        &fixture.repository,
        &MessageId::parse("message-recovery-publication").expect("message id"),
    )
    .expect("find message")
    .expect("message");
    assert_eq!(message.message.status(), MessageStatus::Failed);
    assert_eq!(message.content, "partial response");
    let reports = SessionRecoveryReportRepository::list_reports(
        &fixture.repository,
        session.aggregate.id(),
        10,
    )
    .expect("reports");
    assert_eq!(reports.len(), 1);
}

#[test]
fn recovery_coordinator_repeated_pass_is_idempotent() {
    let fixture = fixture("sessions-recovery-coordinator-idempotent");
    let mut session = session_record(
        "session-recovery-coordinator",
        SessionLifecycle::Idle,
        "Recovery coordinator",
        "2026-07-01T00:00:00+00:00",
    );
    session.interaction_mode = "api".to_string();
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    let mut assistant_message = correlated_message_record(
        "message-recovery-coordinator",
        session.id(),
        "run-recovery-coordinator",
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "partial response",
    );
    assistant_message.tool_use = None;
    SessionTransactionPort::start_generation(
        &fixture.repository,
        &GenerationStartRequest {
            session_id: session.id().to_string(),
            execution_run_id: "run-recovery-coordinator".to_string(),
            user_message: None,
            assistant_message,
            started_at: "2026-07-18T10:00:00+00:00".to_string(),
        },
    )
    .expect("start generation");
    let repository = Arc::new(fixture.repository.clone());
    let logging = Arc::new(CapturingSessionLogging::default());
    let events = Arc::new(CapturingRecoveryEvents::default());
    let coordinator = SessionRecoveryCoordinator::new(
        repository.clone(),
        repository,
        Arc::new(AbsentHandleEvidence {
            repository: fixture.repository.clone(),
        }),
        Arc::new(SystemSessionClock),
        logging.clone(),
    )
    .with_events(events.clone());

    let first = coordinator
        .run_batch(10, RecoveryTrigger::Startup)
        .expect("first pass");
    let second = coordinator
        .run_batch(10, RecoveryTrigger::Startup)
        .expect("second pass");

    assert_eq!(first.published, 1);
    assert_eq!(second.published, 0);
    assert_eq!(second.scanned, 0);
    assert_eq!(
        *events.events.lock().expect("recovery events"),
        vec![
            SessionRecoveryEvent {
                kind: SessionRecoveryEventKind::Started,
                session_id: session.id().to_string(),
                recovery_revision: 0,
            },
            SessionRecoveryEvent {
                kind: SessionRecoveryEventKind::Completed,
                session_id: session.id().to_string(),
                recovery_revision: 1,
            },
        ]
    );
    let entries = logging.entries.lock().expect("recovery logs");
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].session_id.as_deref(),
        Some("session-recovery-coordinator")
    );
    assert_eq!(
        entries[0].execution_run_id.as_deref(),
        Some("run-recovery-coordinator")
    );
    assert!(entries[0].recovery_report_id.is_some());
    assert_eq!(
        SessionRecoveryReportRepository::list_reports(
            &fixture.repository,
            session.aggregate.id(),
            10,
        )
        .expect("reports")
        .len(),
        1
    );
}

#[test]
fn startup_recovery_drains_more_than_one_bounded_batch() {
    let fixture = fixture("sessions-recovery-multiple-batches");
    for index in 0..105 {
        let session_id = format!("session-recovery-batch-{index:03}");
        let run_id = format!("run-recovery-batch-{index:03}");
        let mut session = session_record(
            &session_id,
            SessionLifecycle::Idle,
            "Recovery batch",
            "2026-07-01T00:00:00+00:00",
        );
        session.interaction_mode = "api".to_string();
        fixture
            .repository
            .create_session(&session, SessionActivation::PreserveActive)
            .expect("create session");
        let mut assistant_message = correlated_message_record(
            &format!("message-recovery-batch-{index:03}"),
            &session_id,
            &run_id,
            MessageRole::Assistant,
            MessageStatus::Streaming,
            "partial response",
        );
        assistant_message.tool_use = None;
        SessionTransactionPort::start_generation(
            &fixture.repository,
            &GenerationStartRequest {
                session_id,
                execution_run_id: run_id,
                user_message: None,
                assistant_message,
                started_at: "2026-07-18T10:00:00+00:00".to_string(),
            },
        )
        .expect("start generation");
    }
    let repository = Arc::new(fixture.repository.clone());
    let coordinator = SessionRecoveryCoordinator::new(
        repository.clone(),
        repository,
        Arc::new(AbsentHandleEvidence {
            repository: fixture.repository.clone(),
        }),
        Arc::new(SystemSessionClock),
        Arc::new(NoopSessionLogging),
    );

    let result = coordinator
        .run_until_drained(100, RecoveryTrigger::Startup)
        .expect("drain startup candidates");

    assert_eq!(result.published, 105);
    assert_eq!(
        SessionRepository::recovery_candidate_count(&fixture.repository)
            .expect("remaining candidates"),
        0
    );
}

#[test]
fn startup_recovery_defers_each_retry_later_candidate_once_per_pass() {
    let fixture = fixture("sessions-recovery-retry-later-batches");
    for index in 0..105 {
        let session_id = format!("session-recovery-deferred-{index:03}");
        let run_id = format!("run-recovery-deferred-{index:03}");
        let mut session = session_record(
            &session_id,
            SessionLifecycle::Idle,
            "Deferred recovery batch",
            "2026-07-01T00:00:00+00:00",
        );
        session.interaction_mode = "api".to_string();
        fixture
            .repository
            .create_session(&session, SessionActivation::PreserveActive)
            .expect("create session");
        let mut assistant_message = correlated_message_record(
            &format!("message-recovery-deferred-{index:03}"),
            &session_id,
            &run_id,
            MessageRole::Assistant,
            MessageStatus::Streaming,
            "partial response",
        );
        assistant_message.tool_use = None;
        SessionTransactionPort::start_generation(
            &fixture.repository,
            &GenerationStartRequest {
                session_id,
                execution_run_id: run_id,
                user_message: None,
                assistant_message,
                started_at: "2026-07-18T10:00:00+00:00".to_string(),
            },
        )
        .expect("start generation");
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
        .run_until_drained(100, RecoveryTrigger::Startup)
        .expect("defer startup candidates");

    assert_eq!(result.scanned, 105);
    assert_eq!(result.deferred, 105);
    assert_eq!(result.published, 0);
    assert_eq!(
        SessionRepository::recovery_candidate_count(&fixture.repository)
            .expect("remaining candidates"),
        105
    );
}

#[test]
fn startup_recovery_runs_explicit_retry_before_returning_to_dependents() {
    let fixture = fixture("sessions-recovery-explicit-retry");
    let mut session = session_record(
        "session-recovery-explicit-retry",
        SessionLifecycle::Idle,
        "Explicit recovery retry",
        "2026-07-01T00:00:00+00:00",
    );
    session.interaction_mode = "api".to_string();
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    let mut assistant = correlated_message_record(
        "message-recovery-explicit-retry",
        session.id(),
        "run-recovery-explicit-retry",
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "partial response",
    );
    assistant.tool_use = None;
    SessionTransactionPort::start_generation(
        &fixture.repository,
        &GenerationStartRequest {
            session_id: session.id().to_string(),
            execution_run_id: "run-recovery-explicit-retry".to_string(),
            user_message: None,
            assistant_message: assistant,
            started_at: "2026-07-18T10:00:00+00:00".to_string(),
        },
    )
    .expect("start generation");
    let repository = Arc::new(fixture.repository.clone());
    let coordinator = SessionRecoveryCoordinator::new(
        repository.clone(),
        repository,
        Arc::new(SequencedHandleEvidence {
            repository: fixture.repository.clone(),
            reads: AtomicUsize::new(0),
        }),
        Arc::new(SystemSessionClock),
        Arc::new(NoopSessionLogging),
    );

    let final_pass = coordinator
        .run_startup_with_retry(100)
        .expect("run startup recovery and explicit retry");
    let persisted = SessionRepository::find(&fixture.repository, session.aggregate.id())
        .expect("find session")
        .expect("session");
    let reports = SessionRecoveryReportRepository::list_reports(
        &fixture.repository,
        session.aggregate.id(),
        10,
    )
    .expect("reports");

    assert_eq!(final_pass.published, 1);
    assert_eq!(final_pass.deferred, 0);
    assert_eq!(persisted.aggregate.recovery().status().as_str(), "clean");
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].trigger(), RecoveryTrigger::ExplicitRetry);
}

#[test]
fn startup_recovery_defers_database_contention_without_failing_the_pass() {
    let fixture = fixture("sessions-recovery-database-contention");
    let mut session = session_record(
        "session-recovery-contention",
        SessionLifecycle::Idle,
        "Recovery contention",
        "2026-07-01T00:00:00+00:00",
    );
    session.interaction_mode = "api".to_string();
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    let mut assistant = correlated_message_record(
        "message-recovery-contention",
        session.id(),
        "run-recovery-contention",
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "partial response",
    );
    assistant.tool_use = None;
    SessionTransactionPort::start_generation(
        &fixture.repository,
        &GenerationStartRequest {
            session_id: session.id().to_string(),
            execution_run_id: "run-recovery-contention".to_string(),
            user_message: None,
            assistant_message: assistant,
            started_at: "2026-07-18T10:00:00+00:00".to_string(),
        },
    )
    .expect("start generation");
    let blocker =
        rusqlite::Connection::open(&fixture.database.db_path).expect("blocking connection");
    blocker
        .execute_batch("BEGIN IMMEDIATE")
        .expect("hold writer lock");
    let repository = Arc::new(fixture.repository.clone());
    let coordinator = SessionRecoveryCoordinator::new(
        repository.clone(),
        repository,
        Arc::new(AbsentHandleEvidence {
            repository: fixture.repository.clone(),
        }),
        Arc::new(SystemSessionClock),
        Arc::new(NoopSessionLogging),
    );

    let deferred = coordinator
        .run_until_drained(100, RecoveryTrigger::Startup)
        .expect("defer contended recovery");
    let unchanged = SessionRepository::find(&fixture.repository, session.aggregate.id())
        .expect("find session")
        .expect("session");

    assert_eq!(deferred.deferred, 1);
    assert_eq!(deferred.published, 0);
    assert_eq!(unchanged.aggregate.recovery().status().as_str(), "clean");
    assert_eq!(
        unchanged.aggregate.recovery().active_execution_run_id(),
        Some("run-recovery-contention")
    );

    blocker
        .execute_batch("ROLLBACK")
        .expect("release writer lock");
    let recovered = coordinator
        .run_until_drained(100, RecoveryTrigger::Startup)
        .expect("retry recovery after contention");
    assert_eq!(recovered.published, 1);
}

#[test]
fn crash_reopen_recovery_uses_persisted_operation_terminal_evidence() {
    let directory = TempDirectory::new("sessions-operation-evidence-crash-reopen");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteSessionsRepository::new(database.clone());
    let mut session = session_record(
        "session-operation-evidence-reopen",
        SessionLifecycle::Idle,
        "Operation evidence reopen",
        "2026-07-01T00:00:00+00:00",
    );
    session.interaction_mode = "api".to_string();
    repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    let mut assistant_message = correlated_message_record(
        "message-operation-evidence-reopen",
        session.id(),
        "run-operation-evidence-reopen",
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "partial response",
    );
    assistant_message.tool_use = None;
    SessionTransactionPort::start_generation(
        &repository,
        &GenerationStartRequest {
            session_id: session.id().to_string(),
            execution_run_id: "run-operation-evidence-reopen".to_string(),
            user_message: None,
            assistant_message,
            started_at: "2026-07-18T10:00:00+00:00".to_string(),
        },
    )
    .expect("start generation");
    let operations = OperationsApi::new(persistent_operation_service(database.clone()));
    let operation = operations
        .start(OperationKind::Agent, Some("codex-cli".to_string()), None)
        .expect("start operation");
    operations
        .correlate_execution(
            &operation.id,
            "run-operation-evidence-reopen".to_string(),
            "trace-operation-evidence-reopen".to_string(),
        )
        .expect("correlate operation");
    operations
        .complete(&operation.id, None)
        .expect("complete operation");
    drop(operations);
    drop(repository);
    drop(database);

    let reopened_database =
        NativeDatabase::new(directory.path().to_path_buf()).expect("reopen database");
    let reopened_repository = SqliteSessionsRepository::new(reopened_database.clone());
    let reopened_operations =
        OperationsApi::new(persistent_operation_service(reopened_database.clone()));
    let repository = Arc::new(reopened_repository.clone());
    let coordinator = SessionRecoveryCoordinator::new(
        repository.clone(),
        repository,
        Arc::new(ReopenedOperationEvidence {
            repository: reopened_repository.clone(),
            operations: reopened_operations,
        }),
        Arc::new(SystemSessionClock),
        Arc::new(NoopSessionLogging),
    );

    let result = coordinator
        .run_until_drained(100, RecoveryTrigger::Startup)
        .expect("recover reopened session");

    assert_eq!(result.published, 1);
    let recovered = SessionRepository::find(
        &reopened_repository,
        &SessionId::parse("session-operation-evidence-reopen").expect("session id"),
    )
    .expect("find recovered session")
    .expect("recovered session");
    assert_eq!(recovered.aggregate.lifecycle(), SessionLifecycle::Idle);
    assert_eq!(
        recovered.aggregate.recovery().active_execution_run_id(),
        None
    );
    let reports = SessionRecoveryReportRepository::list_reports(
        &reopened_repository,
        recovered.aggregate.id(),
        10,
    )
    .expect("recovery reports");
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].decision(), RecoveryDecision::Completed);
}

#[test]
fn file_backed_recovery_preserves_partial_stream_and_duplicate_pass_is_noop() {
    let directory = TempDirectory::new("sessions-recovery-reopen-partial");
    let session_id = SessionId::parse("session-recovery-reopen-partial").expect("session id");
    {
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteSessionsRepository::new(database);
        let mut session = session_record(
            session_id.as_str(),
            SessionLifecycle::Idle,
            "Reopen partial recovery",
            "2026-07-01T00:00:00+00:00",
        );
        session.interaction_mode = "api".to_string();
        repository
            .create_session(&session, SessionActivation::PreserveActive)
            .expect("create session");
        let mut assistant_message = correlated_message_record(
            "message-recovery-reopen-partial",
            session.id(),
            "run-recovery-reopen-partial",
            MessageRole::Assistant,
            MessageStatus::Streaming,
            "",
        );
        assistant_message.tool_use = None;
        let started = SessionTransactionPort::start_generation(
            &repository,
            &GenerationStartRequest {
                session_id: session.id().to_string(),
                execution_run_id: "run-recovery-reopen-partial".to_string(),
                user_message: None,
                assistant_message,
                started_at: "2026-07-18T10:00:00+00:00".to_string(),
            },
        )
        .expect("claim generation");
        let mut partial = started.assistant_message;
        partial.content = "durable partial stream".to_string();
        partial.updated_at = "2026-07-18T10:00:01+00:00".to_string();
        SessionMessageRepository::save_stream_fields(&repository, &partial)
            .expect("persist partial stream");
    }

    let reopened_database =
        NativeDatabase::new(directory.path().to_path_buf()).expect("reopen database");
    let reopened = Arc::new(SqliteSessionsRepository::new(reopened_database));
    let coordinator = SessionRecoveryCoordinator::new(
        reopened.clone(),
        reopened.clone(),
        Arc::new(AbsentHandleEvidence {
            repository: reopened.as_ref().clone(),
        }),
        Arc::new(SystemSessionClock),
        Arc::new(NoopSessionLogging),
    );

    let first = coordinator
        .run_batch(10, RecoveryTrigger::Startup)
        .expect("first recovery pass");
    let second = coordinator
        .run_batch(10, RecoveryTrigger::Startup)
        .expect("duplicate recovery pass");
    let message = SessionMessageRepository::find(
        reopened.as_ref(),
        &MessageId::parse("message-recovery-reopen-partial").expect("message id"),
    )
    .expect("find message")
    .expect("message");
    let reports = SessionRecoveryReportRepository::list_reports(reopened.as_ref(), &session_id, 10)
        .expect("reports");

    assert_eq!(first.published, 1);
    assert_eq!(second.scanned, 0);
    assert_eq!(
        reports[0].reason_codes(),
        &[RecoveryReasonCode::InterruptedToolFreeResponse]
    );
    assert_eq!(
        reports[0].decision(),
        RecoveryDecision::InterruptedWithoutToolAmbiguity
    );
    assert_eq!(message.content, "durable partial stream");
    assert_eq!(message.message.status(), MessageStatus::Failed);
    assert_eq!(reports.len(), 1);
}

#[test]
fn file_backed_recovery_interrupts_only_the_active_seat_run() {
    let directory = TempDirectory::new("sessions-recovery-active-seat");
    let session_id = SessionId::parse("session-recovery-active-seat").expect("session id");
    {
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteSessionsRepository::new(database);
        let mut session = session_record(
            session_id.as_str(),
            SessionLifecycle::Idle,
            "Active seat recovery",
            "2026-07-01T00:00:00+00:00",
        );
        session.interaction_mode = "api".to_string();
        repository
            .create_session(&session, SessionActivation::PreserveActive)
            .expect("create session");

        let mut first_message = correlated_message_record(
            "message-seat-first",
            session.id(),
            "run-seat-first",
            MessageRole::Assistant,
            MessageStatus::Streaming,
            "first reply",
        );
        first_message.tool_use = None;
        first_message.seat_round_id = Some("seat-round-1".to_string());
        let first = SessionTransactionPort::start_generation(
            &repository,
            &GenerationStartRequest {
                session_id: session.id().to_string(),
                execution_run_id: "run-seat-first".to_string(),
                user_message: None,
                assistant_message: first_message,
                started_at: "2026-07-18T10:00:00+00:00".to_string(),
            },
        )
        .expect("start first seat");
        let mut completed_first = first.assistant_message;
        completed_first
            .message
            .transition_to(MessageStatus::Completed)
            .expect("complete first message");
        SessionTransactionPort::terminalize_generation(
            &repository,
            &GenerationTerminalRequest {
                execution_run_id: "run-seat-first".to_string(),
                message: completed_first,
                terminal_status: GenerationTerminalStatus::Completed,
                usage: None,
                invocation_usage: None,
                finished_at: "2026-07-18T10:00:01+00:00".to_string(),
            },
        )
        .expect("terminalize first seat");

        let mut second_message = correlated_message_record(
            "message-seat-second",
            session.id(),
            "run-seat-second",
            MessageRole::Assistant,
            MessageStatus::Streaming,
            "partial second reply",
        );
        second_message.tool_use = None;
        second_message.seat_round_id = Some("seat-round-1".to_string());
        second_message.parent_execution_run_id = Some("run-seat-first".to_string());
        SessionTransactionPort::start_generation(
            &repository,
            &GenerationStartRequest {
                session_id: session.id().to_string(),
                execution_run_id: "run-seat-second".to_string(),
                user_message: None,
                assistant_message: second_message,
                started_at: "2026-07-18T10:00:02+00:00".to_string(),
            },
        )
        .expect("start second seat");
    }

    let reopened_database =
        NativeDatabase::new(directory.path().to_path_buf()).expect("reopen database");
    let reopened = Arc::new(SqliteSessionsRepository::new(reopened_database));
    let coordinator = SessionRecoveryCoordinator::new(
        reopened.clone(),
        reopened.clone(),
        Arc::new(AbsentHandleEvidence {
            repository: reopened.as_ref().clone(),
        }),
        Arc::new(SystemSessionClock),
        Arc::new(NoopSessionLogging),
    );
    let outcome = coordinator
        .run_batch(10, RecoveryTrigger::Startup)
        .expect("recover active seat");
    let repeated = coordinator
        .run_batch(10, RecoveryTrigger::Startup)
        .expect("repeat active seat recovery");
    let first = SessionMessageRepository::find(
        reopened.as_ref(),
        &MessageId::parse("message-seat-first").expect("first id"),
    )
    .expect("find first")
    .expect("first message");
    let second = SessionMessageRepository::find(
        reopened.as_ref(),
        &MessageId::parse("message-seat-second").expect("second id"),
    )
    .expect("find second")
    .expect("second message");
    let reports = SessionRecoveryReportRepository::list_reports(reopened.as_ref(), &session_id, 10)
        .expect("reports");

    assert_eq!(outcome.published, 1);
    assert_eq!(repeated.scanned, 0);
    assert_eq!(first.message.status(), MessageStatus::Completed);
    assert_eq!(first.content, "first reply");
    assert_eq!(second.message.status(), MessageStatus::Failed);
    assert_eq!(second.content, "partial second reply");
    assert_eq!(second.seat_round_id.as_deref(), Some("seat-round-1"));
    assert_eq!(
        second.parent_execution_run_id.as_deref(),
        Some("run-seat-first")
    );
    assert_eq!(
        reports[0].observed_execution_run_id(),
        Some("run-seat-second")
    );
    assert_eq!(reports.len(), 1);
}

#[test]
fn file_backed_recovery_honors_terminal_message_and_rejects_stale_publication() {
    let directory = TempDirectory::new("sessions-recovery-reopen-terminal-stale");
    let session_id = SessionId::parse("session-recovery-reopen-terminal").expect("session id");
    let stale_claim = {
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteSessionsRepository::new(database.clone());
        let mut session = session_record(
            session_id.as_str(),
            SessionLifecycle::Idle,
            "Reopen terminal recovery",
            "2026-07-01T00:00:00+00:00",
        );
        session.interaction_mode = "api".to_string();
        repository
            .create_session(&session, SessionActivation::PreserveActive)
            .expect("create session");
        let mut assistant_message = correlated_message_record(
            "message-recovery-reopen-terminal",
            session.id(),
            "run-recovery-reopen-terminal",
            MessageRole::Assistant,
            MessageStatus::Streaming,
            "final response",
        );
        assistant_message.tool_use = None;
        SessionTransactionPort::start_generation(
            &repository,
            &GenerationStartRequest {
                session_id: session.id().to_string(),
                execution_run_id: "run-recovery-reopen-terminal".to_string(),
                user_message: None,
                assistant_message,
                started_at: "2026-07-18T10:00:00+00:00".to_string(),
            },
        )
        .expect("start generation");
        database
            .connection()
            .expect("connection")
            .execute(
                "UPDATE messages SET status = 'completed' WHERE id = ?1",
                ["message-recovery-reopen-terminal"],
            )
            .expect("simulate terminal message crash point");
        let candidate = SessionRepository::recovery_candidates(&repository, 10)
            .expect("candidates")
            .into_iter()
            .find(|candidate| candidate.session_id == session.id())
            .expect("candidate");
        let claim = SessionTransactionPort::claim_recovery_candidate(
            &repository,
            &ClaimRecoveryCandidateRequest {
                candidate,
                claimed_at: "2026-07-18T10:01:00+00:00".to_string(),
            },
        )
        .expect("claim")
        .expect("claim won");
        database
            .connection()
            .expect("connection")
            .execute(
                "UPDATE sessions SET state_revision = state_revision + 1 WHERE id = ?1",
                [session.id()],
            )
            .expect("advance revision after claim");
        claim
    };

    let reopened_database =
        NativeDatabase::new(directory.path().to_path_buf()).expect("reopen database");
    let reopened = Arc::new(SqliteSessionsRepository::new(reopened_database));
    let stale_report = SessionRecoveryReport::new(
        "report-recovery-reopen-stale".to_string(),
        session_id.as_str().to_string(),
        stale_claim.recovery_revision + 1,
        RecoveryTrigger::Startup,
        "starting".to_string(),
        Some("run-recovery-reopen-terminal".to_string()),
        RecoveryDecision::Completed,
        vec![RecoveryReasonCode::ConfirmedCompletedMessage],
        Vec::new(),
        "2026-07-18T10:02:00+00:00".to_string(),
    );
    assert!(!SessionTransactionPort::publish_recovery(
        reopened.as_ref(),
        &PublishRecoveryRequest {
            claim: stale_claim,
            assistant_message_id: Some("message-recovery-reopen-terminal".to_string()),
            report: stale_report,
            published_at: "2026-07-18T10:02:00+00:00".to_string(),
        },
    )
    .expect("stale publication"));
    let coordinator = SessionRecoveryCoordinator::new(
        reopened.clone(),
        reopened.clone(),
        Arc::new(AbsentHandleEvidence {
            repository: reopened.as_ref().clone(),
        }),
        Arc::new(SystemSessionClock),
        Arc::new(NoopSessionLogging),
    );
    let result = coordinator
        .run_batch(10, RecoveryTrigger::Startup)
        .expect("recovery after stale publication");
    let session = SessionRepository::find(reopened.as_ref(), &session_id)
        .expect("find session")
        .expect("session");
    let reports = SessionRecoveryReportRepository::list_reports(reopened.as_ref(), &session_id, 10)
        .expect("reports");

    assert_eq!(result.published, 1);
    assert_eq!(
        reports[0].reason_codes(),
        &[RecoveryReasonCode::ConfirmedCompletedMessage]
    );
    assert_eq!(reports[0].decision(), RecoveryDecision::Completed);
    assert_eq!(session.aggregate.lifecycle(), SessionLifecycle::Idle);
    assert_eq!(reports.len(), 1);
}

#[test]
fn recovery_diagnostics_keep_correlation_and_exclude_raw_evidence_errors() {
    let fixture = fixture("sessions-recovery-safe-diagnostics");
    let mut session = session_record(
        "session-recovery-safe-log",
        SessionLifecycle::Idle,
        "Recovery diagnostics",
        "2026-07-01T00:00:00+00:00",
    );
    session.interaction_mode = "api".to_string();
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    SessionTransactionPort::start_generation(
        &fixture.repository,
        &GenerationStartRequest {
            session_id: session.id().to_string(),
            execution_run_id: "run-recovery-safe-log".to_string(),
            user_message: None,
            assistant_message: correlated_message_record(
                "message-recovery-safe-log",
                session.id(),
                "run-recovery-safe-log",
                MessageRole::Assistant,
                MessageStatus::Streaming,
                "private prompt and tool payload",
            ),
            started_at: "2026-07-18T10:00:00+00:00".to_string(),
        },
    )
    .expect("start generation");
    let repository = Arc::new(fixture.repository.clone());
    let logging = Arc::new(CapturingSessionLogging::default());
    let coordinator = SessionRecoveryCoordinator::new(
        repository.clone(),
        repository,
        Arc::new(SensitiveUnavailableEvidence),
        Arc::new(SystemSessionClock),
        logging.clone(),
    );

    let result = coordinator
        .run_batch(10, RecoveryTrigger::Startup)
        .expect("deferred recovery");
    let entries = logging.entries.lock().expect("recovery logs");

    assert_eq!(result.deferred, 1);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].category, "session.recovery");
    assert_eq!(
        entries[0].message,
        "Recovery evidence was temporarily unavailable; candidate deferred."
    );
    assert_eq!(
        entries[0].session_id.as_deref(),
        Some("session-recovery-safe-log")
    );
    assert_eq!(
        entries[0].execution_run_id.as_deref(),
        Some("run-recovery-safe-log")
    );
    assert_eq!(entries[0].recovery_report_id, None);
    let serialized = format!("{entries:?}");
    for forbidden in [
        "private prompt",
        "tool_payload",
        "rm",
        "credential",
        "D:\\private",
        "provider=raw",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }
}

#[test]
fn recovery_acknowledgement_is_revision_checked_and_preserves_ambiguous_evidence() {
    let fixture = fixture("sessions-recovery-acknowledgement");
    let mut session = session_record(
        "session-recovery-acknowledgement",
        SessionLifecycle::Idle,
        "Recovery acknowledgement",
        "2026-07-01T00:00:00+00:00",
    );
    session.interaction_mode = "api".to_string();
    session.runtime_session_id = Some("provider-resume-id".to_string());
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    SessionTransactionPort::start_generation(
        &fixture.repository,
        &GenerationStartRequest {
            session_id: session.id().to_string(),
            execution_run_id: "run-recovery-acknowledgement".to_string(),
            user_message: None,
            assistant_message: correlated_message_record(
                "message-recovery-acknowledgement",
                session.id(),
                "run-recovery-acknowledgement",
                MessageRole::Assistant,
                MessageStatus::Streaming,
                "ambiguous partial response",
            ),
            started_at: "2026-07-18T10:00:00+00:00".to_string(),
        },
    )
    .expect("start generation");
    let repository = Arc::new(fixture.repository.clone());
    let coordinator = SessionRecoveryCoordinator::new(
        repository.clone(),
        repository,
        Arc::new(AbsentHandleEvidence {
            repository: fixture.repository.clone(),
        }),
        Arc::new(SystemSessionClock),
        Arc::new(NoopSessionLogging),
    );
    coordinator
        .run_batch(10, RecoveryTrigger::Startup)
        .expect("publish action required");

    let acknowledged = SessionTransactionPort::acknowledge_recovery(
        &fixture.repository,
        &AcknowledgeRecoveryRequest {
            session_id: session.id().to_string(),
            expected_recovery_revision: 1,
            acknowledged_at: "2026-07-18T10:02:00+00:00".to_string(),
        },
    )
    .expect("acknowledge recovery");
    let message = SessionMessageRepository::find(
        &fixture.repository,
        &MessageId::parse("message-recovery-acknowledgement").expect("message id"),
    )
    .expect("find message")
    .expect("message");

    assert_eq!(
        acknowledged.session.aggregate.lifecycle(),
        SessionLifecycle::Starting
    );
    assert_eq!(
        acknowledged.session.aggregate.recovery().status(),
        crate::contexts::sessions::domain::SessionRecoveryStatus::Clean
    );
    assert_eq!(
        acknowledged
            .session
            .aggregate
            .recovery()
            .active_execution_run_id(),
        None
    );
    assert_eq!(
        acknowledged.session.runtime_session_id.as_deref(),
        Some("provider-resume-id")
    );
    assert_eq!(
        acknowledged.report.decision(),
        RecoveryDecision::Acknowledged
    );
    assert_eq!(acknowledged.report.recovery_revision(), 2);
    assert_eq!(message.message.status(), MessageStatus::Streaming);
    assert_eq!(message.content, "ambiguous partial response");
    assert!(message.tool_use.is_some());
    assert!(matches!(
        SessionTransactionPort::acknowledge_recovery(
            &fixture.repository,
            &AcknowledgeRecoveryRequest {
                session_id: session.id().to_string(),
                expected_recovery_revision: 1,
                acknowledged_at: "2026-07-18T10:03:00+00:00".to_string(),
            }
        ),
        Err(SessionsApplicationError::RecoveryRevisionConflict {
            current_revision: 2,
            ref current_status,
            ..
        }) if current_status == "clean"
    ));
    assert_eq!(
        SessionRecoveryReportRepository::list_reports(
            &fixture.repository,
            session.aggregate.id(),
            10,
        )
        .expect("reports")
        .len(),
        2
    );
}

#[test]
fn quarantined_recovery_cannot_be_acknowledged() {
    let fixture = fixture("sessions-recovery-quarantined-acknowledgement");
    let session = session_record(
        "session-recovery-quarantined",
        SessionLifecycle::Failed,
        "Quarantined recovery",
        "2026-07-01T00:00:00+00:00",
    );
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    fixture
        .database
        .connection()
        .expect("connection")
        .execute(
            "UPDATE sessions SET recovery_status = 'quarantined', recovery_revision = 4 WHERE id = ?1",
            [session.id()],
        )
        .expect("quarantine session");

    assert!(matches!(
        SessionTransactionPort::acknowledge_recovery(
            &fixture.repository,
            &AcknowledgeRecoveryRequest {
                session_id: session.id().to_string(),
                expected_recovery_revision: 4,
                acknowledged_at: "2026-07-18T10:03:00+00:00".to_string(),
            }
        ),
        Err(SessionsApplicationError::RecoveryActionNotAllowed {
            current_revision: 4,
            ref current_status,
            ..
        }) if current_status == "quarantined"
    ));
}
