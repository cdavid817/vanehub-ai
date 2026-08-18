use super::*;

#[test]
fn generation_start_claims_session_and_persists_correlated_messages_atomically() {
    let fixture = fixture("sessions-generation-start");
    let session = session_record(
        "session-generation-start",
        SessionLifecycle::Idle,
        "Generation",
        "2026-07-01T00:00:00+00:00",
    );
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    let mut assistant_message = correlated_message_record(
        "message-generation-assistant",
        session.id(),
        "run-generation-start",
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "",
    );
    assistant_message.seat_round_id = Some("seat-round-1".to_string());
    assistant_message.parent_execution_run_id = Some("run-previous-seat".to_string());
    let request = GenerationStartRequest {
        session_id: session.id().to_string(),
        execution_run_id: "run-generation-start".to_string(),
        user_message: Some(correlated_message_record(
            "message-generation-user",
            session.id(),
            "run-generation-start",
            MessageRole::User,
            MessageStatus::Completed,
            "request",
        )),
        assistant_message,
        started_at: "2026-07-18T10:00:00+00:00".to_string(),
    };

    let started = SessionTransactionPort::start_generation(&fixture.repository, &request)
        .expect("start generation");

    assert_eq!(
        started.session.aggregate.lifecycle(),
        SessionLifecycle::Starting
    );
    assert_eq!(
        started
            .session
            .aggregate
            .recovery()
            .active_execution_run_id(),
        Some("run-generation-start")
    );
    assert_eq!(started.session.aggregate.recovery().state_revision(), 1);
    assert_eq!(started.session.aggregate.recovery().history_revision(), 2);
    assert_eq!(
        started.session.aggregate.recovery().next_message_sequence(),
        3
    );
    let user = started.user_message.expect("user message");
    assert_eq!(user.message.session_sequence(), 1);
    assert_eq!(
        user.message.execution_run_id(),
        Some("run-generation-start")
    );
    assert_eq!(started.assistant_message.message.session_sequence(), 2);
    assert_eq!(
        started.assistant_message.message.execution_run_id(),
        Some("run-generation-start")
    );
    assert_eq!(
        started.assistant_message.seat_round_id.as_deref(),
        Some("seat-round-1")
    );
    assert_eq!(
        started.assistant_message.parent_execution_run_id.as_deref(),
        Some("run-previous-seat")
    );

    let competing = SessionTransactionPort::start_generation(&fixture.repository, &request);
    assert!(matches!(
        competing,
        Err(SessionsApplicationError::Transaction(_))
    ));
    assert_eq!(
        SessionMessageRepository::list_all(
            &fixture.repository,
            &SessionId::parse(session.id()).expect("session id"),
        )
        .expect("messages")
        .len(),
        2
    );
}

#[test]
fn generation_start_rejects_recovery_gates_before_writing_messages() {
    let fixture = fixture("sessions-generation-start-gates");
    let session = session_record(
        "session-generation-gated",
        SessionLifecycle::Failed,
        "Gated",
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
            "UPDATE sessions SET recovery_status = 'action_required' WHERE id = ?1",
            [session.id()],
        )
        .expect("gate session");
    let request = GenerationStartRequest {
        session_id: session.id().to_string(),
        execution_run_id: "run-gated".to_string(),
        user_message: None,
        assistant_message: correlated_message_record(
            "message-gated-assistant",
            session.id(),
            "run-gated",
            MessageRole::Assistant,
            MessageStatus::Streaming,
            "",
        ),
        started_at: "2026-07-18T10:00:00+00:00".to_string(),
    };

    assert!(matches!(
        SessionTransactionPort::start_generation(&fixture.repository, &request),
        Err(SessionsApplicationError::Transaction(_))
    ));
    let persisted = SessionRepository::find(&fixture.repository, session.aggregate.id())
        .expect("find session")
        .expect("session");
    assert_eq!(persisted.aggregate.lifecycle(), SessionLifecycle::Failed);
    assert_eq!(
        persisted.aggregate.recovery().active_execution_run_id(),
        None
    );
    assert!(
        SessionMessageRepository::list_all(&fixture.repository, session.aggregate.id())
            .expect("messages")
            .is_empty()
    );
}

#[test]
fn generation_terminal_updates_message_usage_and_matching_claim_once() {
    let fixture = fixture("sessions-generation-terminal");
    let session = session_record(
        "session-generation-terminal",
        SessionLifecycle::Idle,
        "Generation terminal",
        "2026-07-01T00:00:00+00:00",
    );
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    let started = SessionTransactionPort::start_generation(
        &fixture.repository,
        &GenerationStartRequest {
            session_id: session.id().to_string(),
            execution_run_id: "run-terminal".to_string(),
            user_message: None,
            assistant_message: correlated_message_record(
                "message-terminal",
                session.id(),
                "run-terminal",
                MessageRole::Assistant,
                MessageStatus::Streaming,
                "partial",
            ),
            started_at: "2026-07-18T10:00:00+00:00".to_string(),
        },
    )
    .expect("start generation");
    let mut terminal_message = started.assistant_message;
    terminal_message
        .message
        .transition_to(MessageStatus::Completed)
        .expect("complete message");
    terminal_message.content = "complete".to_string();
    terminal_message.updated_at = "2026-07-18T10:01:00+00:00".to_string();
    let usage = MessageUsageRecord {
        message_id: terminal_message.message.id().as_str().to_string(),
        session_id: session.id().to_string(),
        agent_id: session.agent_id.clone(),
        provider_id: Some("provider".to_string()),
        model_id: Some("model".to_string()),
        accounting_kind: SessionUsageAccountingKind::Reported,
        unit: SessionUsageUnit::Tokens,
        input_count: 12,
        output_count: 7,
        cache_read_count: 0,
        cache_creation_count: 0,
        source: "provider".to_string(),
        occurred_at: "2026-07-18T10:01:00+00:00".to_string(),
    };
    let request = GenerationTerminalRequest {
        execution_run_id: "run-terminal".to_string(),
        message: terminal_message,
        terminal_status: GenerationTerminalStatus::Completed,
        usage: Some(usage),
        invocation_usage: None,
        finished_at: "2026-07-18T10:01:00+00:00".to_string(),
    };

    let terminal = SessionTransactionPort::terminalize_generation(&fixture.repository, &request)
        .expect("terminalize generation");

    assert_eq!(terminal.message.message.status(), MessageStatus::Completed);
    assert_eq!(terminal.message.content, "complete");
    assert_eq!(
        terminal.session.aggregate.lifecycle(),
        SessionLifecycle::Idle
    );
    assert_eq!(
        terminal
            .session
            .aggregate
            .recovery()
            .active_execution_run_id(),
        None
    );
    assert_eq!(terminal.session.aggregate.recovery().state_revision(), 2);
    assert_eq!(terminal.session.aggregate.recovery().history_revision(), 2);
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
    assert!(matches!(
        SessionTransactionPort::terminalize_generation(&fixture.repository, &request),
        Err(SessionsApplicationError::Transaction(_))
    ));
}

#[test]
fn managed_cli_completion_persists_ledger_without_legacy_usage_row() {
    let fixture = fixture("sessions-managed-cli-ledger");
    let session = session_record(
        "session-managed-cli-ledger",
        SessionLifecycle::Idle,
        "Managed CLI ledger",
        "2026-07-01T00:00:00+00:00",
    );
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    let started = SessionTransactionPort::start_generation(
        &fixture.repository,
        &GenerationStartRequest {
            session_id: session.id().to_string(),
            execution_run_id: "run-managed-cli".to_string(),
            user_message: None,
            assistant_message: correlated_message_record(
                "message-managed-cli",
                session.id(),
                "run-managed-cli",
                MessageRole::Assistant,
                MessageStatus::Streaming,
                "partial",
            ),
            started_at: "2026-07-18T10:00:00+00:00".to_string(),
        },
    )
    .expect("start generation");
    let mut message = started.assistant_message;
    message
        .message
        .transition_to(MessageStatus::Completed)
        .expect("complete message");
    message.content = "complete".to_string();
    message.updated_at = "2026-07-18T10:01:00+00:00".to_string();
    let invocation = NewModelInvocation {
        id: "managed-cli:message-managed-cli:invocation".to_string(),
        generation_id: Some("message-managed-cli".to_string()),
        run_id: Some("run-managed-cli".to_string()),
        operation_id: Some("operation-managed-cli".to_string()),
        session_id: session.id().to_string(),
        message_id: Some("message-managed-cli".to_string()),
        agent_id: session.agent_id.clone(),
        provider_id: Some("provider-managed-cli".to_string()),
        profile_id: None,
        endpoint_id: None,
        model_id: Some("model-managed-cli".to_string()),
        interaction_kind: UsageInteractionKind::ManagedCli,
        purpose: UsagePurpose::AssistantInitial,
        request_sequence: 0,
        attempt: 0,
        started_at: "2026-07-18T10:01:00+00:00".to_string(),
    };
    let invocation_usage = CompletedInvocationAccounting {
        observation: NewUsageObservation {
            id: "managed-cli:message-managed-cli:observation".to_string(),
            invocation_id: invocation.id.clone(),
            quality: MeasurementQuality::Reported,
            unit: AccountingUnit::Tokens,
            measurement_kind: MeasurementKind::Interval,
            dimensions: TokenDimensions {
                input: 12,
                output: 7,
                cached_input: 3,
                provider_total: Some(22),
                ..TokenDimensions::default()
            },
            cache_overlap: TokenOverlap::Exclusive,
            reasoning_overlap: TokenOverlap::Subset,
            normalization_version: "claude-code-result-usage-v1".to_string(),
            source: "cli-reported".to_string(),
            source_key: "managed-cli:message:message-managed-cli".to_string(),
            source_revision: None,
            supersedes_observation_id: None,
            event_at: Some("2026-07-18T10:01:00+00:00".to_string()),
            observed_at: "2026-07-18T10:01:00+00:00".to_string(),
            provenance_hash: None,
        },
        invocation,
        status: UsageStatus::Succeeded,
        completed_at: "2026-07-18T10:01:00+00:00".to_string(),
    };
    SessionTransactionPort::terminalize_generation(
        &fixture.repository,
        &GenerationTerminalRequest {
            execution_run_id: "run-managed-cli".to_string(),
            message,
            terminal_status: GenerationTerminalStatus::Completed,
            usage: None,
            invocation_usage: Some(invocation_usage),
            finished_at: "2026-07-18T10:01:00+00:00".to_string(),
        },
    )
    .expect("terminalize managed CLI generation");

    let connection = fixture.database.connection().expect("connection");
    let legacy_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'usage_records'",
            [],
            |row| row.get(0),
        )
        .expect("legacy count");
    assert_eq!(legacy_count, 0);
    let ledger_facts = connection
        .query_row(
            "SELECT invocation.interaction_kind, invocation.status, observation.quality,
                    observation.input_count, observation.output_count,
                    observation.cached_input_count, observation.provider_total_count,
                    observation.cache_overlap, observation.normalization_version
             FROM model_invocations invocation
             JOIN token_usage_observations observation
               ON observation.invocation_id = invocation.id
             WHERE invocation.message_id = 'message-managed-cli'",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )
        .expect("ledger facts");
    assert_eq!(
        ledger_facts,
        (
            "managed-cli".to_string(),
            "succeeded".to_string(),
            "reported".to_string(),
            12,
            7,
            3,
            Some(22),
            "exclusive".to_string(),
            "claude-code-result-usage-v1".to_string(),
        )
    );
}

#[test]
fn generation_terminal_rolls_back_when_the_active_claim_does_not_match() {
    let fixture = fixture("sessions-generation-terminal-stale");
    let session = session_record(
        "session-generation-terminal-stale",
        SessionLifecycle::Idle,
        "Stale generation terminal",
        "2026-07-01T00:00:00+00:00",
    );
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    let started = SessionTransactionPort::start_generation(
        &fixture.repository,
        &GenerationStartRequest {
            session_id: session.id().to_string(),
            execution_run_id: "run-stale".to_string(),
            user_message: None,
            assistant_message: correlated_message_record(
                "message-stale-terminal",
                session.id(),
                "run-stale",
                MessageRole::Assistant,
                MessageStatus::Streaming,
                "partial",
            ),
            started_at: "2026-07-18T10:00:00+00:00".to_string(),
        },
    )
    .expect("start generation");
    fixture
        .database
        .connection()
        .expect("connection")
        .execute(
            "UPDATE sessions SET active_execution_run_id = 'run-newer' WHERE id = ?1",
            [session.id()],
        )
        .expect("replace claim");
    let mut message = started.assistant_message;
    message
        .message
        .transition_to(MessageStatus::Failed)
        .expect("fail message");
    message.error = Some("failure".to_string());
    let request = GenerationTerminalRequest {
        execution_run_id: "run-stale".to_string(),
        message,
        terminal_status: GenerationTerminalStatus::Failed,
        usage: None,
        invocation_usage: None,
        finished_at: "2026-07-18T10:01:00+00:00".to_string(),
    };

    assert!(matches!(
        SessionTransactionPort::terminalize_generation(&fixture.repository, &request),
        Err(SessionsApplicationError::Transaction(_))
    ));
    let persisted = SessionMessageRepository::find(
        &fixture.repository,
        &MessageId::parse("message-stale-terminal").expect("message id"),
    )
    .expect("find message")
    .expect("message");
    assert_eq!(persisted.message.status(), MessageStatus::Streaming);
}

#[test]
fn concurrent_generation_claims_allow_one_winner_per_session() {
    let fixture = fixture("sessions-generation-concurrent-claim");
    let session = session_record(
        "session-generation-concurrent",
        SessionLifecycle::Idle,
        "Concurrent generation",
        "2026-07-01T00:00:00+00:00",
    );
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    let barrier = Arc::new(Barrier::new(3));
    let results = std::thread::scope(|scope| {
        let handles = (1..=2)
            .map(|attempt| {
                let repository = fixture.repository.clone();
                let barrier = barrier.clone();
                let session_id = session.id().to_string();
                scope.spawn(move || {
                    let run_id = format!("run-concurrent-{attempt}");
                    let request = GenerationStartRequest {
                        session_id: session_id.clone(),
                        execution_run_id: run_id.clone(),
                        user_message: None,
                        assistant_message: correlated_message_record(
                            &format!("message-concurrent-{attempt}"),
                            &session_id,
                            &run_id,
                            MessageRole::Assistant,
                            MessageStatus::Streaming,
                            "",
                        ),
                        started_at: "2026-07-18T10:00:00+00:00".to_string(),
                    };
                    barrier.wait();
                    SessionTransactionPort::start_generation(&repository, &request)
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("claim thread"))
            .collect::<Vec<_>>()
    });

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(SessionsApplicationError::Transaction(_))))
            .count(),
        1
    );
    let persisted = SessionMessageRepository::list_all(
        &fixture.repository,
        &SessionId::parse(session.id()).expect("session id"),
    )
    .expect("messages");
    assert_eq!(persisted.len(), 1);
}

#[test]
fn concurrent_generation_claims_for_unrelated_sessions_are_isolated() {
    let fixture = fixture("sessions-generation-concurrent-isolation");
    for session_id in ["session-isolated-one", "session-isolated-two"] {
        fixture
            .repository
            .create_session(
                &session_record(
                    session_id,
                    SessionLifecycle::Idle,
                    "Isolated generation",
                    "2026-07-01T00:00:00+00:00",
                ),
                SessionActivation::PreserveActive,
            )
            .expect("create session");
    }
    let barrier = Arc::new(Barrier::new(3));
    let results = std::thread::scope(|scope| {
        let handles = ["one", "two"]
            .into_iter()
            .map(|suffix| {
                let repository = fixture.repository.clone();
                let barrier = barrier.clone();
                scope.spawn(move || {
                    let session_id = format!("session-isolated-{suffix}");
                    let run_id = format!("run-isolated-{suffix}");
                    let request = GenerationStartRequest {
                        session_id: session_id.clone(),
                        execution_run_id: run_id.clone(),
                        user_message: None,
                        assistant_message: correlated_message_record(
                            &format!("message-isolated-{suffix}"),
                            &session_id,
                            &run_id,
                            MessageRole::Assistant,
                            MessageStatus::Streaming,
                            "",
                        ),
                        started_at: "2026-07-18T10:00:00+00:00".to_string(),
                    };
                    barrier.wait();
                    SessionTransactionPort::start_generation(&repository, &request)
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("claim thread"))
            .collect::<Vec<_>>()
    });

    assert!(results.into_iter().all(|result| result.is_ok()));
}

#[test]
fn failed_generation_start_leaves_no_partial_writes_after_database_reopen() {
    let fixture = fixture("sessions-generation-start-reopen-rollback");
    let session = session_record(
        "session-start-reopen-rollback",
        SessionLifecycle::Idle,
        "Start rollback",
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
        .execute_batch(
            r#"
            CREATE TRIGGER reject_generation_assistant
            BEFORE INSERT ON messages
            WHEN NEW.role = 'assistant'
            BEGIN
                SELECT RAISE(ABORT, 'injected generation start failure');
            END;
            "#,
        )
        .expect("failure trigger");
    let request = GenerationStartRequest {
        session_id: session.id().to_string(),
        execution_run_id: "run-start-rollback".to_string(),
        user_message: Some(correlated_message_record(
            "message-start-rollback-user",
            session.id(),
            "run-start-rollback",
            MessageRole::User,
            MessageStatus::Completed,
            "request",
        )),
        assistant_message: correlated_message_record(
            "message-start-rollback-assistant",
            session.id(),
            "run-start-rollback",
            MessageRole::Assistant,
            MessageStatus::Streaming,
            "",
        ),
        started_at: "2026-07-18T10:00:00+00:00".to_string(),
    };
    assert!(SessionTransactionPort::start_generation(&fixture.repository, &request).is_err());

    let reopened_database =
        NativeDatabase::new(fixture._directory.path().to_path_buf()).expect("reopen database");
    let reopened = SqliteSessionsRepository::new(reopened_database);
    let persisted = SessionRepository::find(&reopened, session.aggregate.id())
        .expect("find session")
        .expect("session");
    assert_eq!(persisted.aggregate.lifecycle(), SessionLifecycle::Idle);
    assert_eq!(
        persisted.aggregate.recovery().active_execution_run_id(),
        None
    );
    assert_eq!(persisted.aggregate.recovery().state_revision(), 0);
    assert_eq!(persisted.aggregate.recovery().history_revision(), 0);
    assert!(
        SessionMessageRepository::list_all(&reopened, session.aggregate.id())
            .expect("messages")
            .is_empty()
    );
}

#[test]
fn failed_generation_terminal_leaves_no_partial_writes_after_database_reopen() {
    let fixture = fixture("sessions-generation-terminal-reopen-rollback");
    let session = session_record(
        "session-terminal-reopen-rollback",
        SessionLifecycle::Idle,
        "Terminal rollback",
        "2026-07-01T00:00:00+00:00",
    );
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");
    let started = SessionTransactionPort::start_generation(
        &fixture.repository,
        &GenerationStartRequest {
            session_id: session.id().to_string(),
            execution_run_id: "run-terminal-rollback".to_string(),
            user_message: None,
            assistant_message: correlated_message_record(
                "message-terminal-rollback",
                session.id(),
                "run-terminal-rollback",
                MessageRole::Assistant,
                MessageStatus::Streaming,
                "partial",
            ),
            started_at: "2026-07-18T10:00:00+00:00".to_string(),
        },
    )
    .expect("start generation");
    fixture
        .database
        .connection()
        .expect("connection")
        .execute_batch(
            r#"
            CREATE TRIGGER reject_generation_terminal
            BEFORE UPDATE OF active_execution_run_id ON sessions
            WHEN NEW.active_execution_run_id IS NULL
            BEGIN
                SELECT RAISE(ABORT, 'injected generation terminal failure');
            END;
            "#,
        )
        .expect("failure trigger");
    let mut message = started.assistant_message;
    message
        .message
        .transition_to(MessageStatus::Completed)
        .expect("complete message");
    message.content = "complete".to_string();
    let usage = MessageUsageRecord {
        message_id: message.message.id().as_str().to_string(),
        session_id: session.id().to_string(),
        agent_id: session.agent_id.clone(),
        provider_id: None,
        model_id: None,
        accounting_kind: SessionUsageAccountingKind::Reported,
        unit: SessionUsageUnit::Tokens,
        input_count: 1,
        output_count: 1,
        cache_read_count: 0,
        cache_creation_count: 0,
        source: "provider".to_string(),
        occurred_at: "2026-07-18T10:01:00+00:00".to_string(),
    };
    assert!(SessionTransactionPort::terminalize_generation(
        &fixture.repository,
        &GenerationTerminalRequest {
            execution_run_id: "run-terminal-rollback".to_string(),
            message,
            terminal_status: GenerationTerminalStatus::Completed,
            usage: Some(usage),
            invocation_usage: None,
            finished_at: "2026-07-18T10:01:00+00:00".to_string(),
        },
    )
    .is_err());

    let reopened_database =
        NativeDatabase::new(fixture._directory.path().to_path_buf()).expect("reopen database");
    let reopened = SqliteSessionsRepository::new(reopened_database);
    let persisted = SessionRepository::find(&reopened, session.aggregate.id())
        .expect("find session")
        .expect("session");
    assert_eq!(persisted.aggregate.lifecycle(), SessionLifecycle::Starting);
    assert_eq!(
        persisted.aggregate.recovery().active_execution_run_id(),
        Some("run-terminal-rollback")
    );
    let message = SessionMessageRepository::find(
        &reopened,
        &MessageId::parse("message-terminal-rollback").expect("message id"),
    )
    .expect("find message")
    .expect("message");
    assert_eq!(message.message.status(), MessageStatus::Streaming);
    let usage_count: i64 = reopened
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
