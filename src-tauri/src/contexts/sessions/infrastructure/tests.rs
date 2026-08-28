use super::sqlite_repository::{
    allocate_message_sequences, compatibility_search_statement, indexed_search_statement,
};
use super::*;
use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::domain::{OperationKind, OperationStatus};
use crate::contexts::operations::infrastructure::persistent_operation_service;
use crate::contexts::sessions::application::{
    AcknowledgeRecoveryRequest, CategoryRecord, ChatConfigurationValues,
    ClaimRecoveryCandidateRequest, CompletedInvocationAccounting, FileReferenceInput,
    GenerationStartRequest, GenerationTerminalRequest, GenerationTerminalStatus,
    InvocationDetailQuery, LoopSessionOwnership, MessagePageQuery, MessageRecord,
    MessageTokenUsage, MessageUsageRecord, NewModelInvocation, NewUsageObservation,
    PublishRecoveryRequest, SessionApplicationLog, SessionCategoryRepository,
    SessionConfigurationRepository, SessionListScope, SessionLoggingPort, SessionMessageRepository,
    SessionRecord, SessionRecoveryCoordinator, SessionRecoveryEvent, SessionRecoveryEventKind,
    SessionRecoveryEventPort, SessionRecoveryReportRepository, SessionRemoteWorkspace,
    SessionRepository, SessionSearchMatchKind, SessionSearchQuery, SessionSearchResult,
    SessionSshBinding, SessionTerminalEvidencePort, SessionTransactionPort,
    SessionUsageAccountingKind, SessionUsageRepository, SessionUsageUnit, SessionWorkspace,
    SessionsApplicationError, TokenAccountingQueryPort, TokenAccountingRepository,
    UsageBreakdownDimension, UsageCursor, UsageCursorAdvance, UsageStatisticsRange,
    UsageSummaryQuery,
};
use crate::contexts::sessions::domain::evidence::{
    ExecutionEvidenceFidelity, LiveHandleEvidence, OperationTerminalEvidence,
    OperationTerminalStatus, ToolActivityEvidence, MAX_RECOVERY_EVIDENCE_OPERATIONS,
};
use crate::contexts::sessions::domain::recovery::{
    RecoveryDecision, RecoveryEvidenceReference, RecoveryReasonCode, RecoveryTrigger,
    SessionRecoveryReport,
};
use crate::contexts::sessions::domain::{
    normalize_chat_preferences, AccountingUnit, CategoryId, CategoryName, ChatConfigurationRequest,
    FileReference, FileReferenceSet, LoopSessionRole, MeasurementKind, MeasurementQuality,
    MessageId, MessageRole, MessageStatus, SessionActivation, SessionAggregate, SessionCategory,
    SessionId, SessionLifecycle, SessionMessage, SessionOwner, SessionPersonalizationMode,
    SessionSeat, SessionTitle, TokenDimensions, TokenOverlap, UsageInteractionKind, UsagePurpose,
    UsageStatus,
};
use crate::platform::database::{migrate, NativeDatabase};
use crate::test_support::TempDirectory;
use rusqlite::params;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};

mod configuration_and_seats;
mod generation_lifecycle;
mod legacy_usage_retirement;
mod personalization_mode;
mod recovery;
mod search;
mod terminal_evidence;
mod usage_accounting;

struct Fixture {
    _directory: TempDirectory,
    database: NativeDatabase,
    repository: SqliteSessionsRepository,
}

#[derive(Clone)]
struct AbsentHandleEvidence {
    repository: SqliteSessionsRepository,
}

struct SequencedHandleEvidence {
    repository: SqliteSessionsRepository,
    reads: AtomicUsize,
}

struct NoopSessionLogging;

#[derive(Default)]
struct CapturingRecoveryEvents {
    events: Mutex<Vec<SessionRecoveryEvent>>,
}

impl SessionRecoveryEventPort for CapturingRecoveryEvents {
    fn publish_recovery_event(
        &self,
        event: SessionRecoveryEvent,
    ) -> Result<(), SessionsApplicationError> {
        self.events.lock().expect("recovery events").push(event);
        Ok(())
    }
}

impl SessionLoggingPort for NoopSessionLogging {
    fn write(&self, _log: SessionApplicationLog) -> Result<(), SessionsApplicationError> {
        Ok(())
    }
}

#[derive(Default)]
struct CapturingSessionLogging {
    entries: Mutex<Vec<SessionApplicationLog>>,
}

impl SessionLoggingPort for CapturingSessionLogging {
    fn write(&self, log: SessionApplicationLog) -> Result<(), SessionsApplicationError> {
        self.entries.lock().expect("recovery logs").push(log);
        Ok(())
    }
}

struct SensitiveUnavailableEvidence;

#[derive(Clone)]
struct ReopenedOperationEvidence {
    repository: SqliteSessionsRepository,
    operations: OperationsApi,
}

impl SessionTerminalEvidencePort for ReopenedOperationEvidence {
    fn read_terminal_evidence(
        &self,
        session_id: &SessionId,
        execution_run_id: Option<&str>,
    ) -> Result<
        crate::contexts::sessions::domain::evidence::SessionTerminalEvidence,
        SessionsApplicationError,
    > {
        let mut evidence = self
            .repository
            .read_terminal_evidence(session_id, execution_run_id)?;
        if let Some(run_id) = execution_run_id {
            let operations = self
                .operations
                .list_recovery_evidence(run_id, MAX_RECOVERY_EVIDENCE_OPERATIONS + 1)
                .map_err(|error| SessionsApplicationError::Runtime(error.to_string()))?
                .into_iter()
                .map(|operation| OperationTerminalEvidence {
                    operation_id: operation.operation_id,
                    execution_run_id: Some(operation.execution_run_id),
                    status: match operation.status {
                        OperationStatus::Queued | OperationStatus::Running => {
                            OperationTerminalStatus::Running
                        }
                        OperationStatus::Succeeded => OperationTerminalStatus::Succeeded,
                        OperationStatus::Failed => OperationTerminalStatus::Failed,
                        OperationStatus::Cancelled => OperationTerminalStatus::Cancelled,
                    },
                })
                .collect();
            evidence.replace_operations(operations).map_err(|error| {
                SessionsApplicationError::Runtime(format!(
                    "operation evidence exceeded its bounded read: {error:?}"
                ))
            })?;
        }
        evidence.live_handle = LiveHandleEvidence::Absent;
        Ok(evidence)
    }
}

impl SessionTerminalEvidencePort for SensitiveUnavailableEvidence {
    fn read_terminal_evidence(
        &self,
        _session_id: &SessionId,
        _execution_run_id: Option<&str>,
    ) -> Result<
        crate::contexts::sessions::domain::evidence::SessionTerminalEvidence,
        SessionsApplicationError,
    > {
        Err(SessionsApplicationError::Repository(
            "prompt=secret command=rm credential=token path=D:\\private tool_payload=secret provider=raw"
                .to_string(),
        ))
    }
}

impl SessionTerminalEvidencePort for AbsentHandleEvidence {
    fn read_terminal_evidence(
        &self,
        session_id: &SessionId,
        execution_run_id: Option<&str>,
    ) -> Result<
        crate::contexts::sessions::domain::evidence::SessionTerminalEvidence,
        SessionsApplicationError,
    > {
        let mut evidence = self
            .repository
            .read_terminal_evidence(session_id, execution_run_id)?;
        evidence.live_handle = LiveHandleEvidence::Absent;
        Ok(evidence)
    }
}

impl SessionTerminalEvidencePort for SequencedHandleEvidence {
    fn read_terminal_evidence(
        &self,
        session_id: &SessionId,
        execution_run_id: Option<&str>,
    ) -> Result<
        crate::contexts::sessions::domain::evidence::SessionTerminalEvidence,
        SessionsApplicationError,
    > {
        let mut evidence = self
            .repository
            .read_terminal_evidence(session_id, execution_run_id)?;
        if self.reads.fetch_add(1, Ordering::SeqCst) > 0 {
            evidence.live_handle = LiveHandleEvidence::Absent;
        }
        Ok(evidence)
    }
}

fn fixture(name: &str) -> Fixture {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    database.connection().expect("migrated connection");
    let repository = SqliteSessionsRepository::new(database.clone());
    Fixture {
        _directory: directory,
        database,
        repository,
    }
}

fn session_record(
    id: &str,
    lifecycle: SessionLifecycle,
    title: &str,
    updated_at: &str,
) -> SessionRecord {
    SessionRecord {
        personalization_mode: SessionPersonalizationMode::Standard,
        aggregate: SessionAggregate::rehydrate(
            SessionId::parse(id).expect("session id"),
            SessionTitle::for_creation(Some(title)),
            lifecycle,
            SessionOwner::desktop(),
            None,
            false,
            false,
        ),
        agent_id: "codex-cli".to_string(),
        seats: vec![SessionSeat {
            seat_id: "seat-1".to_string(),
            agent_id: "codex-cli".to_string(),
            role_id: None,
            role_snapshot: None,
            joined_at: updated_at.to_string(),
            left_at: None,
            provider_thread_id: None,
        }],
        interaction_mode: "interactive".to_string(),
        workspace: SessionWorkspace {
            folder: Some("D:\\code\\fixture".to_string()),
            project_path: Some("D:\\code\\fixture".to_string()),
            ..Default::default()
        },
        runtime_session_id: None,
        execution_origin_kind: "user".to_string(),
        execution_origin_id: None,
        created_at: "2026-07-01T00:00:00+00:00".to_string(),
        updated_at: updated_at.to_string(),
    }
}

#[test]
fn message_sequence_allocator_reserves_consecutive_ranges_atomically() {
    let fixture = fixture("sessions-message-sequence-allocation");
    let session = session_record(
        "session-sequence",
        SessionLifecycle::Idle,
        "Sequence",
        "2026-07-01T00:00:00+00:00",
    );
    fixture
        .repository
        .create_session(&session, SessionActivation::PreserveActive)
        .expect("create session");

    let mut connection = fixture.database.connection().expect("connection");
    let transaction = connection.transaction().expect("transaction");
    let first = allocate_message_sequences(
        &transaction,
        &SessionId::parse("session-sequence").expect("session id"),
        2,
    )
    .expect("first range");
    let second = allocate_message_sequences(
        &transaction,
        &SessionId::parse("session-sequence").expect("session id"),
        3,
    )
    .expect("second range");
    transaction.commit().expect("commit ranges");

    let persisted: (i64, i64) = connection
        .query_row(
            "SELECT next_message_sequence, history_revision FROM sessions WHERE id = ?1",
            ["session-sequence"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("persisted allocator");
    assert_eq!((first, second), (1, 3));
    assert_eq!(persisted, (6, 5));
    assert!(allocate_message_sequences(
        &connection,
        &SessionId::parse("session-sequence").expect("session id"),
        0,
    )
    .is_err());
}

#[test]
fn recovery_reports_are_session_owned_and_legacy_run_ids_remain_null() {
    let fixture = fixture("sessions-recovery-report-ownership");
    let session = session_record(
        "session-recovery-report",
        SessionLifecycle::Running,
        "Recovery report",
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
            r#"
            INSERT INTO messages (
                id, session_id, role, status, content, created_at, updated_at, session_sequence
            ) VALUES (?1, ?2, 'assistant', 'completed', 'legacy', ?3, ?3, 1)
            "#,
            params![
                "legacy-message",
                "session-recovery-report",
                "2026-07-01T00:00:00+00:00"
            ],
        )
        .expect("insert legacy message");
    let legacy = SessionMessageRepository::find(
        &fixture.repository,
        &MessageId::parse("legacy-message").expect("message id"),
    )
    .expect("find legacy message")
    .expect("legacy message");
    assert_eq!(legacy.message.execution_run_id(), None);

    let report = SessionRecoveryReport::new(
        "report-1".to_string(),
        "session-recovery-report".to_string(),
        1,
        RecoveryTrigger::Startup,
        "running".to_string(),
        None,
        RecoveryDecision::ActionRequired,
        vec![RecoveryReasonCode::MissingExecutionRun],
        vec![RecoveryEvidenceReference::Message {
            message_id: "legacy-message".to_string(),
            execution_run_id: None,
            status: "completed".to_string(),
        }],
        "2026-07-01T00:00:01+00:00".to_string(),
    );
    fixture
        .repository
        .insert_report(&report)
        .expect("insert report");
    assert_eq!(
        fixture
            .repository
            .list_reports(
                &SessionId::parse("session-recovery-report").expect("session id"),
                10,
            )
            .expect("list reports"),
        vec![report]
    );

    fixture
        .repository
        .delete_session(&SessionId::parse("session-recovery-report").expect("session id"))
        .expect("delete session");
    assert!(fixture
        .repository
        .list_reports(
            &SessionId::parse("session-recovery-report").expect("session id"),
            10,
        )
        .expect("list reports after delete")
        .is_empty());
}

fn message_record(
    id: &str,
    session_id: &str,
    role: MessageRole,
    status: MessageStatus,
    content: &str,
) -> MessageRecord {
    MessageRecord {
        message: SessionMessage::rehydrate(
            MessageId::parse(id).expect("message id"),
            SessionId::parse(session_id).expect("session id"),
            role,
            status,
            FileReferenceSet::new(vec![FileReference::new(
                "reference-1",
                "src/main.rs",
                "main.rs",
                Some(12),
                Some("hash".to_string()),
                None,
            )
            .expect("reference")])
            .expect("references"),
        ),
        speaker_seat_id: None,
        seat_index: None,
        seat_round_id: None,
        parent_execution_run_id: None,
        content: content.to_string(),
        thinking_content: Some("thinking".to_string()),
        tool_use: Some(vec![json!({"id": "tool-1", "name": "read"})]),
        rich_blocks: Some(vec![json!({"id": "block-1", "kind": "card", "v": 1})]),
        token_usage: None,
        error: None,
        created_at: "2026-07-18T10:00:00+00:00".to_string(),
        updated_at: "2026-07-18T10:00:00+00:00".to_string(),
    }
}

fn correlated_message_record(
    id: &str,
    session_id: &str,
    execution_run_id: &str,
    role: MessageRole,
    status: MessageStatus,
    content: &str,
) -> MessageRecord {
    let mut record = message_record(id, session_id, role, status, content);
    record.message = SessionMessage::rehydrate_with_correlation(
        MessageId::parse(id).expect("message id"),
        SessionId::parse(session_id).expect("session id"),
        role,
        status,
        FileReferenceSet::default(),
        0,
        Some(execution_run_id.to_string()),
    );
    record
}

#[test]
fn remote_ssh_binding_round_trips_and_updates_without_mutating_snapshot() {
    let fixture = fixture("session-remote-ssh-binding");
    let mut session = session_record(
        "session-remote-binding",
        SessionLifecycle::Idle,
        "Remote",
        "2026-07-24T00:00:00Z",
    );
    session.workspace.remote_workspace = Some(SessionRemoteWorkspace {
        host: "remote.example.test".to_string(),
        port: Some(22),
        user: Some("dev".to_string()),
        path: "/work/app".to_string(),
        display_name: "Remote App".to_string(),
        uri: "ssh://dev@remote.example.test/work/app".to_string(),
    });
    session.workspace.remote_ssh_binding = Some(SessionSshBinding {
        connection_id: "ssh-first".to_string(),
        revision: 3,
    });
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create remote session");

    let mut loaded = SessionRepository::find(&fixture.repository, session.aggregate.id())
        .expect("find remote session")
        .expect("remote session");
    assert_eq!(
        loaded.workspace.remote_ssh_binding,
        session.workspace.remote_ssh_binding
    );
    let snapshot = loaded.workspace.remote_workspace.clone();
    loaded.workspace.remote_ssh_binding = Some(SessionSshBinding {
        connection_id: "ssh-second".to_string(),
        revision: 7,
    });
    let rebound =
        SessionRepository::save(&fixture.repository, &loaded).expect("save rebound session");

    assert_eq!(rebound.workspace.remote_workspace, snapshot);
    assert_eq!(
        rebound.workspace.remote_ssh_binding,
        Some(SessionSshBinding {
            connection_id: "ssh-second".to_string(),
            revision: 7,
        })
    );
}

fn usage_record(message_id: &str, session_id: &str, agent_id: &str) -> MessageUsageRecord {
    MessageUsageRecord {
        message_id: message_id.to_string(),
        session_id: session_id.to_string(),
        agent_id: agent_id.to_string(),
        provider_id: Some("openai".to_string()),
        model_id: Some("gpt-5-5".to_string()),
        accounting_kind: SessionUsageAccountingKind::Reported,
        unit: SessionUsageUnit::Tokens,
        input_count: 7,
        output_count: 11,
        cache_read_count: 2,
        cache_creation_count: 3,
        source: "provider".to_string(),
        occurred_at: "2026-07-18T10:00:00+00:00".to_string(),
    }
}

#[test]
fn loop_owned_sessions_round_trip_but_stay_out_of_default_navigation() {
    let fixture = fixture("sessions-loop-ownership");
    let repository = &fixture.repository;
    let normal = session_record(
        "session-normal",
        SessionLifecycle::Idle,
        "Normal session",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &normal, SessionActivation::PreserveActive)
        .expect("normal session");
    let mut role = session_record(
        "session-loop-verifier",
        SessionLifecycle::Idle,
        "Loop verifier",
        "2026-07-18T11:00:00+00:00",
    );
    role.workspace.loop_ownership = Some(LoopSessionOwnership {
        run_id: "run-1".to_string(),
        iteration_id: "iteration-1".to_string(),
        role: LoopSessionRole::Verifier,
    });
    SessionTransactionPort::create_session(repository, &role, SessionActivation::PreserveActive)
        .expect("role session");

    let default_list =
        SessionRepository::list(repository, SessionListScope::Current).expect("default sessions");
    assert_eq!(default_list.len(), 1);
    assert_eq!(default_list[0].id(), "session-normal");
    let all = SessionRepository::list_including_loop_owned(repository, SessionListScope::Current)
        .expect("all sessions");
    assert_eq!(all.len(), 2);
    let loaded = SessionRepository::find(repository, role.aggregate.id())
        .expect("find role")
        .expect("role");
    assert_eq!(
        loaded.workspace.loop_ownership.expect("ownership").role,
        LoopSessionRole::Verifier
    );
    let search = SessionRepository::search(
        repository,
        &SessionSearchQuery {
            text: "Loop verifier".to_string(),
            limit: 10,
        },
    )
    .expect("search");
    assert!(search.is_empty());
}

#[test]
fn repositories_round_trip_rows_and_preserve_bounded_query_contracts() {
    let fixture = fixture("sessions-sqlite-round-trip");
    let repository = &fixture.repository;
    let mut session = session_record(
        "session-round-trip",
        SessionLifecycle::Idle,
        "Needle Session",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::Activate)
        .expect("create session");

    let loaded = SessionRepository::find(repository, session.aggregate.id())
        .expect("find session")
        .expect("session");
    assert_eq!(
        loaded.workspace.project_path.as_deref(),
        Some("D:\\code\\fixture")
    );
    assert_eq!(
        SessionRepository::active_session(repository)
            .expect("active session")
            .expect("active")
            .id(),
        session.id()
    );
    assert_eq!(
        SessionRepository::list(repository, SessionListScope::Current)
            .expect("sessions")
            .len(),
        1
    );

    let category = CategoryRecord {
        category: SessionCategory::new(
            CategoryId::parse("category-1").expect("category id"),
            CategoryName::parse("Work").expect("category name"),
            0,
        ),
        created_at: "2026-07-18T10:00:00+00:00".to_string(),
        updated_at: "2026-07-18T10:00:00+00:00".to_string(),
    };
    SessionCategoryRepository::insert(repository, &category).expect("insert category");
    assert!(SessionCategoryRepository::name_exists(repository, "work", None).expect("name exists"));
    session
        .aggregate
        .assign_category(Some(category.category.id().clone()));
    session.aggregate.set_pinned(true);
    SessionRepository::save(repository, &session).expect("save session");

    let preferences = normalize_chat_preferences(
        "codex-cli",
        "openai",
        "gpt-5-5",
        ChatConfigurationRequest {
            execution_mode: "execute",
            provider_id: Some("openai"),
            model_id: Some("gpt-5-5"),
            reasoning_depth: Some("high"),
            streaming: true,
            thinking: true,
            long_context: true,
        },
    )
    .expect("preferences");
    SessionConfigurationRepository::save(
        repository,
        session.aggregate.id(),
        &preferences,
        "2026-07-18T11:00:00+00:00",
    )
    .expect("save configuration");
    let configuration = SessionConfigurationRepository::load(repository, session.aggregate.id())
        .expect("configuration")
        .expect("stored configuration");
    assert_eq!(configuration.model_id.as_deref(), Some("gpt-5-5"));

    let message = message_record(
        "message-1",
        session.id(),
        MessageRole::User,
        MessageStatus::Completed,
        "message needle",
    );
    SessionMessageRepository::insert(repository, &message).expect("insert message");
    let listed = SessionMessageRepository::list(
        repository,
        &MessagePageQuery {
            session_id: session.id().to_string(),
            limit: 50,
            before_id: None,
        },
    )
    .expect("messages");
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].message.file_references().as_slice()[0].path(),
        "src/main.rs"
    );
    assert_eq!(
        listed[0].rich_blocks.as_ref().expect("rich blocks")[0]["id"],
        "block-1"
    );

    let results = SessionRepository::search(
        repository,
        &SessionSearchQuery {
            text: "needle".to_string(),
            limit: 100,
        },
    )
    .expect("search");
    assert_eq!(results.len(), 1);
    assert!(results[0]
        .matches
        .iter()
        .any(|matched| matched.kind == SessionSearchMatchKind::Title));
    assert!(results[0]
        .matches
        .iter()
        .any(|matched| matched.kind == SessionSearchMatchKind::Message));
}

#[test]
fn missing_active_session_is_cleared_when_read() {
    let fixture = fixture("sessions-stale-active-pointer");
    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "UPDATE workflow_state SET active_session_id = 'missing-session' WHERE id = 1",
            [],
        )
        .expect("seed stale active session");

    assert!(SessionRepository::active_session(&fixture.repository)
        .expect("active session")
        .is_none());
    let active = connection
        .query_row(
            "SELECT active_session_id FROM workflow_state WHERE id = 1",
            [],
            |row| row.get::<_, Option<String>>(0),
        )
        .expect("read active session");
    assert!(active.is_none());
}

#[test]
fn invalid_persisted_domain_values_fail_explicit_row_mapping() {
    let fixture = fixture("sessions-invalid-row");
    let session = session_record(
        "session-invalid",
        SessionLifecycle::Idle,
        "Invalid",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create session");
    fixture
        .database
        .connection()
        .expect("connection")
        .execute(
            "UPDATE sessions SET source_kind = 'im', source_connector = NULL WHERE id = ?1",
            [session.id()],
        )
        .expect("corrupt owner");

    let result = SessionRepository::find(&fixture.repository, session.aggregate.id());
    assert!(matches!(
        result,
        Err(crate::contexts::sessions::application::SessionsApplicationError::Domain(_))
    ));
}

#[test]
fn create_and_activate_roll_back_when_workflow_update_cannot_commit() {
    let fixture = fixture("sessions-create-rollback");
    fixture
        .database
        .connection()
        .expect("connection")
        .execute("DELETE FROM workflow_state WHERE id = 1", [])
        .expect("remove workflow row");
    let session = session_record(
        "session-rollback",
        SessionLifecycle::Idle,
        "Rollback",
        "2026-07-18T10:00:00+00:00",
    );

    assert!(SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::Activate,
    )
    .is_err());
    let count: i64 = fixture
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM sessions WHERE id = ?1",
            [session.id()],
            |row| row.get(0),
        )
        .expect("session count");
    assert_eq!(count, 0);
}

#[test]
fn category_delete_rolls_back_session_unassignment_on_delete_failure() {
    let fixture = fixture("sessions-category-rollback");
    let repository = &fixture.repository;
    let mut session = session_record(
        "session-category",
        SessionLifecycle::Idle,
        "Category",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create session");
    let category = CategoryRecord {
        category: SessionCategory::new(
            CategoryId::parse("category-rollback").expect("category id"),
            CategoryName::parse("Rollback").expect("category name"),
            0,
        ),
        created_at: "100".to_string(),
        updated_at: "100".to_string(),
    };
    SessionCategoryRepository::insert(repository, &category).expect("insert category");
    session
        .aggregate
        .assign_category(Some(category.category.id().clone()));
    SessionRepository::save(repository, &session).expect("assign category");
    fixture
        .database
        .connection()
        .expect("connection")
        .execute_batch(
            "CREATE TRIGGER reject_category_delete BEFORE DELETE ON session_categories BEGIN SELECT RAISE(ABORT, 'rejected'); END;",
        )
        .expect("failure trigger");

    assert!(
        SessionTransactionPort::delete_category(repository, category.category.id(), "200",)
            .is_err()
    );
    let connection = fixture.database.connection().expect("connection");
    let assigned: Option<String> = connection
        .query_row(
            "SELECT category_id FROM sessions WHERE id = ?1",
            [session.id()],
            |row| row.get(0),
        )
        .expect("category assignment");
    let category_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM session_categories WHERE id = ?1",
            [category.category.id().as_str()],
            |row| row.get(0),
        )
        .expect("category count");
    assert_eq!(assigned.as_deref(), Some("category-rollback"));
    assert_eq!(category_count, 1);
}
