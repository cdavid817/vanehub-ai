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
    SessionId, SessionLifecycle, SessionMessage, SessionOwner, SessionSeat, SessionTitle,
    TokenDimensions, TokenOverlap, UsageInteractionKind, UsagePurpose, UsageStatus,
};
use crate::platform::database::{migrate, NativeDatabase};
use crate::test_support::TempDirectory;
use rusqlite::params;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};

struct Fixture {
    _directory: TempDirectory,
    database: NativeDatabase,
    repository: SqliteSessionsRepository,
}

fn seed_accounting_scope(fixture: &Fixture) {
    fixture
        .database
        .connection()
        .expect("accounting connection")
        .execute_batch(
            r#"
            INSERT INTO agents (id, display_name, provider, launch_kind)
            VALUES ('accounting-agent', 'Accounting Agent', 'test', 'api');
            INSERT INTO sessions
                (id, title, agent_id, interaction_mode, lifecycle_state, created_at, updated_at)
            VALUES
                ('accounting-session', 'Accounting Session', 'accounting-agent', 'api', 'idle',
                 '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z');
            "#,
        )
        .expect("seed accounting scope");
}

fn accounting_invocation(id: &str) -> NewModelInvocation {
    NewModelInvocation {
        id: id.to_string(),
        generation_id: Some("generation-1".to_string()),
        run_id: Some("run-1".to_string()),
        operation_id: Some("operation-1".to_string()),
        session_id: "accounting-session".to_string(),
        message_id: None,
        agent_id: "accounting-agent".to_string(),
        provider_id: Some("provider-1".to_string()),
        profile_id: Some("profile-1".to_string()),
        endpoint_id: Some("endpoint-1".to_string()),
        model_id: Some("model-1".to_string()),
        interaction_kind: UsageInteractionKind::NativeApi,
        purpose: UsagePurpose::AssistantInitial,
        request_sequence: 0,
        attempt: 0,
        started_at: "2026-08-12T00:00:01Z".to_string(),
    }
}

fn accounting_observation(
    id: &str,
    source_key: &str,
    quality: MeasurementQuality,
    output: i64,
) -> NewUsageObservation {
    NewUsageObservation {
        id: id.to_string(),
        invocation_id: "invocation-1".to_string(),
        quality,
        unit: if quality == MeasurementQuality::Estimated {
            AccountingUnit::Characters
        } else {
            AccountingUnit::Tokens
        },
        measurement_kind: MeasurementKind::Interval,
        dimensions: TokenDimensions {
            input: 10,
            output,
            provider_total: (quality != MeasurementQuality::Estimated).then_some(10 + output),
            ..TokenDimensions::default()
        },
        cache_overlap: TokenOverlap::Subset,
        reasoning_overlap: TokenOverlap::Subset,
        normalization_version: "test-v1".to_string(),
        source: "test-provider".to_string(),
        source_key: source_key.to_string(),
        source_revision: Some("1".to_string()),
        supersedes_observation_id: None,
        event_at: Some("2026-08-12T00:00:02Z".to_string()),
        observed_at: "2026-08-12T00:00:03Z".to_string(),
        provenance_hash: Some("safe-hash".to_string()),
    }
}

#[test]
fn accounting_ledger_is_idempotent_and_supersedes_estimates() {
    let fixture = fixture("accounting-ledger-idempotency");
    seed_accounting_scope(&fixture);
    let invocation = fixture
        .repository
        .start_invocation(&accounting_invocation("invocation-1"))
        .expect("start invocation");
    assert_eq!(invocation.status, UsageStatus::Running);
    let mut conflicting_invocation = accounting_invocation("invocation-1");
    conflicting_invocation.model_id = Some("different-model".to_string());
    assert!(matches!(
        fixture.repository.start_invocation(&conflicting_invocation),
        Err(SessionsApplicationError::Validation(_))
    ));

    let estimated = accounting_observation(
        "observation-estimated",
        "test:request-1:estimated",
        MeasurementQuality::Estimated,
        20,
    );
    let first = fixture
        .repository
        .record_observation(&estimated)
        .expect("record estimate");
    let mut replay = estimated.clone();
    replay.id = "ignored-replay-id".to_string();
    replay.observed_at = "2026-08-12T00:01:00Z".to_string();
    let replayed = fixture
        .repository
        .record_observation(&replay)
        .expect("replay observation");
    assert_eq!(replayed.observation.id, first.observation.id);

    let mut mismatch = replay.clone();
    mismatch.dimensions.output = 21;
    assert!(matches!(
        fixture.repository.record_observation(&mismatch),
        Err(SessionsApplicationError::Validation(_))
    ));

    let mut reported = accounting_observation(
        "observation-reported",
        "test:request-1:reported",
        MeasurementQuality::Reported,
        7,
    );
    reported.supersedes_observation_id = Some(first.observation.id.clone());
    fixture
        .repository
        .record_observation(&reported)
        .expect("upgrade estimate");
    fixture
        .repository
        .finalize_invocation(
            "invocation-1",
            UsageStatus::Succeeded,
            "2026-08-12T00:00:04Z",
        )
        .expect("finalize invocation");
    assert!(matches!(
        fixture.repository.finalize_invocation(
            "invocation-1",
            UsageStatus::Failed,
            "2026-08-12T00:00:05Z",
        ),
        Err(SessionsApplicationError::Validation(_))
    ));

    let details = fixture
        .repository
        .invocation_details(&InvocationDetailQuery {
            session_id: Some("accounting-session".to_string()),
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: Some(MeasurementQuality::Reported),
            status: Some(UsageStatus::Succeeded),
            after_id: None,
            limit: 10,
        })
        .expect("query details");
    assert_eq!(details.invocations.len(), 1);
    assert_eq!(details.observations.len(), 1);
    assert_eq!(details.observations[0].observation.dimensions.output, 7);

    let connection = fixture.database.connection().expect("ledger connection");
    let active: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM token_usage_observations WHERE superseded_by_observation_id IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("active observations");
    assert_eq!(active, 1);
}

#[test]
fn cumulative_cursor_rejects_stale_advances_and_requires_reset_epochs() {
    let fixture = fixture("accounting-cursor-cas");
    let first = UsageCursor {
        source_id: "codex:session-1".to_string(),
        provider_session_id: "provider-session-1".to_string(),
        epoch: 0,
        dimensions: TokenDimensions {
            input: 10,
            output: 5,
            provider_total: Some(15),
            ..TokenDimensions::default()
        },
        ordering_key: "001".to_string(),
        source_revision: Some("1".to_string()),
        revision: 0,
        updated_at: "2026-08-12T00:00:01Z".to_string(),
    };
    fixture
        .repository
        .advance_cursor(&UsageCursorAdvance {
            previous: None,
            current: first.clone(),
            observation: None,
        })
        .expect("open cursor");

    let mut second = first.clone();
    second.dimensions.input = 14;
    second.dimensions.provider_total = Some(19);
    second.ordering_key = "002".to_string();
    second.revision = 1;
    fixture
        .repository
        .advance_cursor(&UsageCursorAdvance {
            previous: Some(first.clone()),
            current: second.clone(),
            observation: None,
        })
        .expect("advance cursor");

    assert!(matches!(
        fixture.repository.advance_cursor(&UsageCursorAdvance {
            previous: Some(first),
            current: second.clone(),
            observation: None,
        }),
        Err(SessionsApplicationError::Transaction(_))
    ));

    let mut invalid_reset = second.clone();
    invalid_reset.dimensions = TokenDimensions::default();
    invalid_reset.ordering_key = "003".to_string();
    invalid_reset.revision = 2;
    assert!(matches!(
        fixture.repository.advance_cursor(&UsageCursorAdvance {
            previous: Some(second.clone()),
            current: invalid_reset.clone(),
            observation: None,
        }),
        Err(SessionsApplicationError::Validation(_))
    ));

    invalid_reset.epoch = 1;
    let reset = fixture
        .repository
        .advance_cursor(&UsageCursorAdvance {
            previous: Some(second),
            current: invalid_reset,
            observation: None,
        })
        .expect("open reset epoch");
    assert_eq!(reset.epoch, 1);
}

#[test]
fn ledger_projection_separates_quality_purpose_and_failed_usage() {
    let fixture = fixture("accounting-ledger-projection");
    seed_accounting_scope(&fixture);
    let cases = [
        (
            "invocation-1",
            UsagePurpose::AssistantInitial,
            UsageStatus::Succeeded,
            MeasurementQuality::Reported,
            AccountingUnit::Tokens,
            TokenDimensions {
                input: 10,
                output: 7,
                provider_total: Some(17),
                ..TokenDimensions::default()
            },
            TokenOverlap::Unknown,
        ),
        (
            "invocation-2",
            UsagePurpose::ContextCompaction,
            UsageStatus::Succeeded,
            MeasurementQuality::ReportedDerived,
            AccountingUnit::Tokens,
            TokenDimensions {
                input: 5,
                output: 2,
                ..TokenDimensions::default()
            },
            TokenOverlap::Subset,
        ),
        (
            "invocation-3",
            UsagePurpose::ToolContinuation,
            UsageStatus::Failed,
            MeasurementQuality::Estimated,
            AccountingUnit::Characters,
            TokenDimensions {
                input: 100,
                output: 20,
                ..TokenDimensions::default()
            },
            TokenOverlap::Subset,
        ),
        (
            "invocation-4",
            UsagePurpose::TerminalInterval,
            UsageStatus::Cancelled,
            MeasurementQuality::Reported,
            AccountingUnit::Tokens,
            TokenDimensions {
                cached_input: 8,
                ..TokenDimensions::default()
            },
            TokenOverlap::Exclusive,
        ),
    ];
    for (index, (id, purpose, status, quality, unit, dimensions, cache_overlap)) in
        cases.into_iter().enumerate()
    {
        let mut invocation = accounting_invocation(id);
        invocation.purpose = purpose;
        invocation.request_sequence = u32::try_from(index).expect("bounded fixture index");
        invocation.provider_id = (id != "invocation-4").then(|| "provider-1".to_string());
        fixture
            .repository
            .start_invocation(&invocation)
            .expect("start projected invocation");
        let mut observation = accounting_observation(
            &format!("observation-{id}"),
            &format!("test:{id}"),
            quality,
            dimensions.output,
        );
        observation.invocation_id = id.to_string();
        observation.unit = unit;
        observation.dimensions = dimensions;
        observation.cache_overlap = cache_overlap;
        observation.reasoning_overlap = TokenOverlap::Subset;
        fixture
            .repository
            .record_observation(&observation)
            .expect("record projected observation");
        fixture
            .repository
            .finalize_invocation(id, status, "2026-08-12T00:00:04Z")
            .expect("finalize projected invocation");
    }

    let summary = fixture
        .repository
        .usage_summary(&UsageSummaryQuery {
            session_id: Some("accounting-session".to_string()),
            message_id: None,
            generation_id: None,
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            range_start: Some("2026-08-12T00:00:00Z".to_string()),
            range_end: Some("2026-08-13T00:00:00Z".to_string()),
            breakdown_limit: 10,
            generated_at: "2026-08-12T01:00:00Z".to_string(),
        })
        .expect("project usage summary");
    assert_eq!(summary.counts.calls, 4);
    assert_eq!(summary.counts.generations, 1);
    assert_eq!(summary.counts.sessions, 1);
    assert_eq!(summary.totals.reported.headline_total, Some(25));
    assert_eq!(summary.totals.reported.dimensions.cached_input, 8);
    assert_eq!(summary.totals.reported_derived.headline_total, Some(7));
    assert_eq!(summary.totals.estimated.headline_total, Some(120));
    assert_eq!(summary.internal.reported_derived.headline_total, Some(7));
    assert_eq!(summary.user_response.reported.headline_total, Some(25));
    assert_eq!(summary.daily.len(), 1);
    let provider_breakdown = summary
        .breakdowns
        .iter()
        .find(|breakdown| breakdown.dimension == UsageBreakdownDimension::Provider)
        .expect("provider breakdown");
    assert_eq!(provider_breakdown.entries[0].key, "provider-1");
    assert_eq!(provider_breakdown.entries[0].counts.calls, 3);
    assert_eq!(provider_breakdown.entries[1].key, "unknown");

    let failed = fixture
        .repository
        .usage_summary(&UsageSummaryQuery {
            session_id: None,
            message_id: None,
            generation_id: None,
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: Some(UsageStatus::Failed),
            range_start: None,
            range_end: None,
            breakdown_limit: 1,
            generated_at: "2026-08-12T01:00:00Z".to_string(),
        })
        .expect("filter failed usage");
    assert_eq!(failed.counts.calls, 1);
    assert_eq!(failed.totals.estimated.headline_total, Some(120));
    assert_eq!(failed.totals.reported.headline_total, Some(0));

    let page = fixture
        .repository
        .invocation_details(&InvocationDetailQuery {
            session_id: Some("accounting-session".to_string()),
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            after_id: None,
            limit: 1,
        })
        .expect("first invocation page");
    assert_eq!(page.invocations.len(), 1);
    assert_eq!(page.next_cursor.as_deref(), Some("invocation-1"));
}

#[test]
fn ledger_projection_preserves_authoritative_cache_reasoning_and_unknown_semantics() {
    let fixture = fixture("accounting-ledger-semantics");
    seed_accounting_scope(&fixture);
    let cases = [
        (
            "invocation-authoritative",
            "model-authoritative",
            TokenDimensions {
                input: 10,
                output: 5,
                cached_input: 100,
                reasoning_output: 50,
                provider_total: Some(20),
                ..TokenDimensions::default()
            },
            TokenOverlap::Unknown,
            TokenOverlap::Unknown,
            Some(20),
        ),
        (
            "invocation-cache-only",
            "model-cache-only",
            TokenDimensions {
                cached_input: 7,
                ..TokenDimensions::default()
            },
            TokenOverlap::Exclusive,
            TokenOverlap::Subset,
            Some(7),
        ),
        (
            "invocation-reasoning",
            "model-reasoning",
            TokenDimensions {
                reasoning_output: 9,
                ..TokenDimensions::default()
            },
            TokenOverlap::Subset,
            TokenOverlap::Exclusive,
            Some(9),
        ),
        (
            "invocation-unknown",
            "model-unknown",
            TokenDimensions {
                input: 3,
                cached_input: 2,
                ..TokenDimensions::default()
            },
            TokenOverlap::Unknown,
            TokenOverlap::Subset,
            None,
        ),
    ];
    for (sequence, (id, model, dimensions, cache_overlap, reasoning_overlap, expected)) in
        cases.into_iter().enumerate()
    {
        let mut invocation = accounting_invocation(id);
        invocation.model_id = Some(model.to_string());
        invocation.request_sequence = u32::try_from(sequence).expect("bounded sequence");
        fixture
            .repository
            .start_invocation(&invocation)
            .expect("start semantic invocation");
        let mut observation = accounting_observation(
            &format!("observation-{id}"),
            &format!("semantic:{id}"),
            MeasurementQuality::Reported,
            dimensions.output,
        );
        observation.invocation_id = id.to_string();
        observation.dimensions = dimensions;
        observation.cache_overlap = cache_overlap;
        observation.reasoning_overlap = reasoning_overlap;
        fixture
            .repository
            .record_observation(&observation)
            .expect("record semantic observation");
        fixture
            .repository
            .finalize_invocation(id, UsageStatus::Succeeded, "2026-08-12T00:00:04Z")
            .expect("finalize semantic invocation");

        let summary = fixture
            .repository
            .usage_summary(&UsageSummaryQuery {
                session_id: Some("accounting-session".to_string()),
                message_id: None,
                generation_id: None,
                agent_id: None,
                provider_id: None,
                model_id: Some(model.to_string()),
                purpose: Some(UsagePurpose::AssistantInitial),
                quality: Some(MeasurementQuality::Reported),
                status: Some(UsageStatus::Succeeded),
                range_start: None,
                range_end: None,
                breakdown_limit: 10,
                generated_at: "2026-08-12T01:00:00Z".to_string(),
            })
            .expect("query semantic summary");
        assert_eq!(summary.counts.calls, 1);
        assert_eq!(summary.totals.reported.headline_total, expected);
        assert_eq!(summary.totals.reported.dimensions, dimensions);
    }

    let first = fixture
        .repository
        .invocation_details(&InvocationDetailQuery {
            session_id: Some("accounting-session".to_string()),
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            after_id: None,
            limit: 2,
        })
        .expect("first bounded details page");
    assert_eq!(first.invocations.len(), 2);
    let cursor = first.next_cursor.expect("first page cursor");
    let second = fixture
        .repository
        .invocation_details(&InvocationDetailQuery {
            session_id: Some("accounting-session".to_string()),
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            after_id: Some(cursor),
            limit: 2,
        })
        .expect("second bounded details page");
    assert_eq!(second.invocations.len(), 2);
    assert!(second.next_cursor.is_none());

    let empty = fixture
        .repository
        .usage_summary(&UsageSummaryQuery {
            session_id: Some("accounting-session".to_string()),
            message_id: None,
            generation_id: None,
            agent_id: None,
            provider_id: Some("missing-provider".to_string()),
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            range_start: None,
            range_end: None,
            breakdown_limit: 10,
            generated_at: "2026-08-12T01:00:00Z".to_string(),
        })
        .expect("empty filtered summary");
    assert_eq!(empty.counts.calls, 0);
    assert!(empty.daily.is_empty());
    assert_eq!(empty.totals.reported.headline_total, Some(0));
}

#[test]
fn accounting_schema_cascades_and_never_projects_message_content() {
    let fixture = fixture("accounting-ledger-integrity");
    seed_accounting_scope(&fixture);
    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO messages (id, session_id, role, status, content, created_at, updated_at)
             VALUES ('secret-message', 'accounting-session', 'user', 'completed', ?1, ?2, ?2)",
            params!["prompt-secret-never-project", "2026-08-12T00:00:00Z"],
        )
        .expect("seed private message content");
    drop(connection);

    let mut invocation = accounting_invocation("invocation-1");
    invocation.message_id = Some("secret-message".to_string());
    fixture
        .repository
        .start_invocation(&invocation)
        .expect("start invocation");
    fixture
        .repository
        .record_observation(&accounting_observation(
            "observation-1",
            "safe:source:1",
            MeasurementQuality::Reported,
            4,
        ))
        .expect("record observation");
    fixture
        .repository
        .finalize_invocation("invocation-1", UsageStatus::Failed, "2026-08-12T00:00:04Z")
        .expect("finalize failed invocation");

    let details = fixture
        .repository
        .invocation_details(&InvocationDetailQuery {
            session_id: Some("accounting-session".to_string()),
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            after_id: None,
            limit: 10,
        })
        .expect("safe details");
    let rendered = format!("{details:?}");
    assert!(!rendered.contains("prompt-secret-never-project"));
    for forbidden in [
        "prompt",
        "response",
        "credential",
        "header",
        "tool_payload",
        "raw_frame",
    ] {
        let count: i64 = fixture
            .database
            .connection()
            .expect("schema connection")
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('token_usage_observations')
                 WHERE lower(name) LIKE '%' || ?1 || '%'",
                [forbidden],
                |row| row.get(0),
            )
            .expect("inspect accounting columns");
        assert_eq!(count, 0, "forbidden accounting column: {forbidden}");
    }

    SessionTransactionPort::delete_session(
        &fixture.repository,
        &SessionId::parse("accounting-session").expect("session id"),
    )
    .expect("delete accounting session");
    let connection = fixture.database.connection().expect("cascade connection");
    for table in ["model_invocations", "token_usage_observations"] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("cascade count");
        assert_eq!(count, 0, "{table} must cascade with its session");
    }
    let mut unknown = accounting_invocation("unknown-invocation");
    unknown.session_id = "missing-session".to_string();
    assert!(fixture.repository.start_invocation(&unknown).is_err());
}

#[test]
fn accounting_queries_are_bounded_under_cardinality_and_concurrent_ingestion() {
    let fixture = fixture("accounting-ledger-bounded-concurrency");
    seed_accounting_scope(&fixture);
    for index in 0..50_u32 {
        let id = format!("bulk-invocation-{index:03}");
        let mut invocation = accounting_invocation(&id);
        invocation.generation_id = Some(format!("bulk-generation-{index:03}"));
        invocation.provider_id = Some(format!("provider-{index:03}"));
        invocation.model_id = Some(format!("model-{index:03}"));
        invocation.request_sequence = index;
        fixture
            .repository
            .start_invocation(&invocation)
            .expect("start bulk invocation");
        let mut observation = accounting_observation(
            &format!("bulk-observation-{index:03}"),
            &format!("bulk:source:{index:03}"),
            MeasurementQuality::Reported,
            1,
        );
        observation.invocation_id = id.clone();
        observation.event_at = Some("2026-08-10T12:00:00Z".to_string());
        fixture
            .repository
            .record_observation(&observation)
            .expect("record bulk observation");
        fixture
            .repository
            .finalize_invocation(&id, UsageStatus::Succeeded, "2026-08-12T00:00:04Z")
            .expect("finalize bulk invocation");
    }
    let query = UsageSummaryQuery {
        session_id: Some("accounting-session".to_string()),
        message_id: None,
        generation_id: None,
        agent_id: None,
        provider_id: None,
        model_id: None,
        purpose: None,
        quality: None,
        status: None,
        range_start: None,
        range_end: None,
        breakdown_limit: 5,
        generated_at: "2026-08-12T01:00:00Z".to_string(),
    };
    let summary = fixture
        .repository
        .usage_summary(&query)
        .expect("bounded summary");
    assert_eq!(summary.counts.calls, 50);
    assert_eq!(summary.daily[0].local_date, "2026-08-10");
    assert!(summary
        .breakdowns
        .iter()
        .all(|breakdown| breakdown.entries.len() <= 5));
    let details = fixture
        .repository
        .invocation_details(&InvocationDetailQuery {
            session_id: Some("accounting-session".to_string()),
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            after_id: None,
            limit: 7,
        })
        .expect("bounded details");
    assert_eq!(details.invocations.len(), 7);
    assert!(details.next_cursor.is_some());

    let repository = fixture.repository.clone();
    let query_repository = fixture.repository.clone();
    let query = query.clone();
    let barrier = Arc::new(Barrier::new(2));
    let writer_barrier = barrier.clone();
    let writer = std::thread::spawn(move || {
        writer_barrier.wait();
        let mut invocation = accounting_invocation("concurrent-invocation");
        invocation.request_sequence = 51;
        repository
            .start_invocation(&invocation)
            .expect("concurrent start");
        let mut observation = accounting_observation(
            "concurrent-observation",
            "concurrent:source:1",
            MeasurementQuality::Reported,
            2,
        );
        observation.invocation_id = invocation.id.clone();
        repository
            .record_observation(&observation)
            .expect("concurrent observation");
        repository
            .finalize_invocation(
                &invocation.id,
                UsageStatus::Succeeded,
                "2026-08-12T00:00:05Z",
            )
            .expect("concurrent finalize");
    });
    barrier.wait();
    let during = query_repository
        .usage_summary(&query)
        .expect("concurrent query");
    writer.join().expect("concurrent writer");
    let after = query_repository
        .usage_summary(&query)
        .expect("post-ingestion query");
    assert!((50..=51).contains(&during.counts.calls));
    assert_eq!(after.counts.calls, 51);
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
fn sqlite_search_and_message_queries_honor_limits_and_cursors() {
    let fixture = fixture("sessions-bounded-adapter-queries");
    let repository = &fixture.repository;
    for index in 1..=3 {
        let session = session_record(
            &format!("session-search-{index}"),
            SessionLifecycle::Idle,
            &format!("Needle {index}"),
            &format!("2026-07-18T1{index}:00:00+00:00"),
        );
        SessionTransactionPort::create_session(
            repository,
            &session,
            SessionActivation::PreserveActive,
        )
        .expect("create search session");
    }

    let search_results = SessionRepository::search(
        repository,
        &SessionSearchQuery {
            text: "Needle".to_string(),
            limit: 2,
        },
    )
    .expect("bounded search");
    assert_eq!(search_results.len(), 2);

    for (id, created_at) in [
        ("message-page-1", "2026-07-18T10:00:00+00:00"),
        ("message-page-2", "2026-07-18T11:00:00+00:00"),
        ("message-page-3", "2026-07-18T12:00:00+00:00"),
    ] {
        let mut message = message_record(
            id,
            "session-search-1",
            MessageRole::User,
            MessageStatus::Completed,
            id,
        );
        message.created_at = created_at.to_string();
        message.updated_at = created_at.to_string();
        SessionMessageRepository::insert(repository, &message).expect("insert paged message");
    }

    let latest = SessionMessageRepository::list(
        repository,
        &MessagePageQuery {
            session_id: "session-search-1".to_string(),
            limit: 2,
            before_id: None,
        },
    )
    .expect("latest page");
    assert_eq!(
        latest
            .iter()
            .map(|message| message.message.id().as_str())
            .collect::<Vec<_>>(),
        ["message-page-2", "message-page-3"]
    );

    let previous = SessionMessageRepository::list(
        repository,
        &MessagePageQuery {
            session_id: "session-search-1".to_string(),
            limit: 2,
            before_id: Some("message-page-2".to_string()),
        },
    )
    .expect("previous page");
    assert_eq!(
        previous
            .iter()
            .map(|message| message.message.id().as_str())
            .collect::<Vec<_>>(),
        ["message-page-1"]
    );
}

#[test]
fn message_search_index_tracks_content_mutations_and_uses_fts() {
    let fixture = fixture("sessions-message-search-index");
    let repository = &fixture.repository;
    let session = session_record(
        "session-indexed-search",
        SessionLifecycle::Idle,
        "Indexed search",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create indexed session");
    let mut message = message_record(
        "message-indexed-search",
        session.id(),
        MessageRole::User,
        MessageStatus::Completed,
        "alpha indexed needle omega",
    );
    SessionMessageRepository::insert(repository, &message).expect("insert indexed message");

    assert_eq!(fts_match_count(&fixture, "\"indexed needle\""), 1);
    let short_query_results = SessionRepository::search(
        repository,
        &SessionSearchQuery {
            text: "ph".to_string(),
            limit: 10,
        },
    )
    .expect("two-character compatibility search");
    assert_eq!(short_query_results.len(), 1);
    assert!(short_query_results[0]
        .matches
        .iter()
        .any(|matched| matched.kind == SessionSearchMatchKind::Message));
    let plan = fts_query_plan(&fixture, "\"indexed needle\"");
    assert!(
        plan.iter()
            .any(|detail| detail.contains("VIRTUAL TABLE INDEX")),
        "expected FTS virtual-table plan, got {plan:?}"
    );

    message.content = "replacement searchable phrase".to_string();
    SessionMessageRepository::save(repository, &message).expect("update indexed message");
    assert_eq!(fts_match_count(&fixture, "\"indexed needle\""), 0);
    assert_eq!(fts_match_count(&fixture, "\"searchable phrase\""), 1);

    fixture
        .database
        .connection()
        .expect("delete connection")
        .execute(
            "DELETE FROM messages WHERE id = ?1",
            [message.message.id().as_str()],
        )
        .expect("delete indexed message");
    assert_eq!(fts_match_count(&fixture, "\"searchable phrase\""), 0);
}

#[test]
fn fts_migration_keeps_archived_sessions_with_existing_messages_searchable() {
    let fixture = fixture("sessions-archived-search-migration");
    let repository = &fixture.repository;
    {
        let connection = fixture
            .database
            .connection()
            .expect("pre-migration connection");
        connection
            .execute_batch(
                r#"
                DROP TRIGGER messages_fts_insert;
                DROP TRIGGER messages_fts_delete;
                DROP TRIGGER messages_fts_update;
                DROP TABLE session_message_fts;
                DELETE FROM schema_migrations WHERE version = 33;
                "#,
            )
            .expect("simulate schema before message search migration");
    }

    let session = session_record(
        "session-archived-before-fts",
        SessionLifecycle::Idle,
        "Archived migration fixture",
        "2026-07-18T11:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create pre-migration session");
    fixture
        .database
        .connection()
        .expect("archive connection")
        .execute(
            "UPDATE sessions SET archived = 1 WHERE id = ?1",
            [session.id()],
        )
        .expect("archive pre-migration session");
    let message = message_record(
        "message-archived-before-fts",
        session.id(),
        MessageRole::User,
        MessageStatus::Completed,
        "quartz migration searchable payload",
    );
    SessionMessageRepository::insert(repository, &message).expect("insert pre-migration message");

    {
        let connection = fixture.database.connection().expect("migration connection");
        migrate(&connection).expect("apply message search migration");
    }

    let results = SessionRepository::search(
        repository,
        &SessionSearchQuery {
            text: "quartz migration".to_string(),
            limit: 10,
        },
    )
    .expect("search migrated archived session");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].session.id(), session.id());
    assert!(results[0].session.aggregate.is_archived());
    assert!(results[0].matches.iter().any(|matched| {
        matched.kind == SessionSearchMatchKind::Message
            && matched.message_id.as_deref() == Some(message.message.id().as_str())
    }));
}

/// Seeds three sessions whose ordering and per-session newest-match are unambiguous, so
/// the same expectations hold for both the indexed and the compatibility search branch.
///
/// The seeded content carries "目标词" so a three-character query reaches the trigram
/// index while a two-character "目标" substring falls to the compatibility path.
fn seed_search_ranking_fixture(fixture: &Fixture) {
    let repository = &fixture.repository;
    for (id, title, updated_at) in [
        ("session-alpha", "Alpha", "2026-07-18T12:00:00+00:00"),
        ("session-bravo", "Bravo", "2026-07-18T13:00:00+00:00"),
        ("session-charlie", "Charlie", "2026-07-18T11:00:00+00:00"),
    ] {
        let session = session_record(id, SessionLifecycle::Idle, title, updated_at);
        SessionTransactionPort::create_session(
            repository,
            &session,
            SessionActivation::PreserveActive,
        )
        .expect("create ranking session");
    }

    // Distinct timestamps: the later message must win.
    insert_message_at(
        fixture,
        "message-alpha-old",
        "session-alpha",
        "定位 目标词 旧",
        "2026-07-18T10:00:00+00:00",
    );
    insert_message_at(
        fixture,
        "message-alpha-new",
        "session-alpha",
        "定位 目标词 新",
        "2026-07-18T11:00:00+00:00",
    );
    // Identical timestamps: only the rowid tiebreak separates them, and the later insert
    // holds the higher rowid.
    insert_message_at(
        fixture,
        "message-bravo-tie-early",
        "session-bravo",
        "定位 目标词 平局一",
        "2026-07-18T10:00:00+00:00",
    );
    insert_message_at(
        fixture,
        "message-bravo-tie-late",
        "session-bravo",
        "定位 目标词 平局二",
        "2026-07-18T10:00:00+00:00",
    );
    // Charlie must never appear: neither its title nor its message matches.
    insert_message_at(
        fixture,
        "message-charlie",
        "session-charlie",
        "无关内容",
        "2026-07-18T10:00:00+00:00",
    );
}

fn insert_message_at(fixture: &Fixture, id: &str, session_id: &str, content: &str, at: &str) {
    let mut record = message_record(
        id,
        session_id,
        MessageRole::User,
        MessageStatus::Completed,
        content,
    );
    record.created_at = at.to_string();
    record.updated_at = at.to_string();
    SessionMessageRepository::insert(&fixture.repository, &record).expect("insert ranking message");
}

fn searched_session_ids(results: &[SessionSearchResult]) -> Vec<String> {
    results
        .iter()
        .map(|result| result.session.aggregate.id().as_str().to_string())
        .collect()
}

fn matched_message_id(result: &SessionSearchResult) -> Option<String> {
    result
        .matches
        .iter()
        .find(|matched| matched.kind == SessionSearchMatchKind::Message)
        .and_then(|matched| matched.message_id.clone())
}

fn assert_ranking_expectations(results: &[SessionSearchResult], label: &str) {
    assert_eq!(
        searched_session_ids(results),
        vec!["session-bravo".to_string(), "session-alpha".to_string()],
        "{label}: sessions come back newest-updated first, without the non-matching session"
    );
    assert_eq!(
        matched_message_id(&results[0]).as_deref(),
        Some("message-bravo-tie-late"),
        "{label}: the rowid tiebreak decides when created_at ties"
    );
    assert_eq!(
        matched_message_id(&results[1]).as_deref(),
        Some("message-alpha-new"),
        "{label}: the newest matching message is the match context"
    );
}

#[test]
fn short_query_search_orders_sessions_and_picks_the_newest_matching_message() {
    let fixture = fixture("sessions-short-query-ranking");
    seed_search_ranking_fixture(&fixture);

    let results = SessionRepository::search(
        &fixture.repository,
        &SessionSearchQuery {
            // Two characters, below the trigram floor, so this takes the compatibility path.
            text: "目标".to_string(),
            limit: 10,
        },
    )
    .expect("short query search");

    assert_ranking_expectations(&results, "short query");
}

#[test]
fn indexed_query_search_orders_sessions_and_picks_the_newest_matching_message() {
    let fixture = fixture("sessions-indexed-query-ranking");
    seed_search_ranking_fixture(&fixture);

    let results = SessionRepository::search(
        &fixture.repository,
        &SessionSearchQuery {
            // Three characters, so this one reaches the trigram index.
            text: "目标词".to_string(),
            limit: 10,
        },
    )
    .expect("indexed query search");

    assert_ranking_expectations(&results, "indexed query");
}

#[test]
fn search_that_matches_nothing_returns_no_sessions() {
    let fixture = fixture("sessions-search-no-match");
    seed_search_ranking_fixture(&fixture);

    for text in ["零", "零零", "零零零"] {
        let results = SessionRepository::search(
            &fixture.repository,
            &SessionSearchQuery {
                text: text.to_string(),
                limit: 10,
            },
        )
        .expect("no-match search");
        assert!(
            results.is_empty(),
            "{text} matched unexpectedly: {results:?}"
        );
    }
}

fn search_query_plan(fixture: &Fixture, sql: &str, message_query: &str) -> Vec<String> {
    let connection = fixture.database.connection().expect("plan connection");
    let mut statement = connection
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .expect("prepare search plan");
    statement
        .query_map(params![message_query, "%目标%", 10_i64], |row| row.get(3))
        .expect("query search plan")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect search plan")
}

#[test]
fn short_query_plan_seeks_the_session_index_instead_of_ranking_every_match() {
    let fixture = fixture("sessions-short-query-plan");
    seed_search_ranking_fixture(&fixture);

    let plan = search_query_plan(&fixture, &compatibility_search_statement(), "%目标%");

    assert!(
        plan.iter()
            .any(|detail| detail.contains("idx_messages_session_sequence")),
        "expected a per-session index seek, got {plan:?}"
    );
    assert!(
        !plan.iter().any(|detail| detail.contains("MATERIALIZE")),
        "the compatibility path must not materialize a ranked match set, got {plan:?}"
    );
}

#[test]
fn indexed_query_plan_still_drives_the_full_text_index() {
    let fixture = fixture("sessions-indexed-query-plan");
    seed_search_ranking_fixture(&fixture);

    let plan = search_query_plan(&fixture, &indexed_search_statement(), "\"目标词\"");

    assert!(
        plan.iter()
            .any(|detail| detail.contains("session_message_fts")),
        "expected the FTS branch to keep using the index, got {plan:?}"
    );
}

fn fts_match_count(fixture: &Fixture, query: &str) -> i64 {
    fixture
        .database
        .connection()
        .expect("FTS count connection")
        .query_row(
            "SELECT COUNT(*) FROM session_message_fts WHERE session_message_fts MATCH ?1",
            [query],
            |row| row.get(0),
        )
        .expect("FTS match count")
}

fn fts_query_plan(fixture: &Fixture, query: &str) -> Vec<String> {
    let connection = fixture.database.connection().expect("FTS plan connection");
    let mut statement = connection
        .prepare(
            "EXPLAIN QUERY PLAN SELECT rowid FROM session_message_fts \
             WHERE session_message_fts MATCH ?1",
        )
        .expect("prepare FTS query plan");
    statement
        .query_map([query], |row| row.get(3))
        .expect("query FTS plan")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect FTS plan")
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

#[test]
fn invalid_configuration_json_maps_to_no_persisted_snapshot() {
    let fixture = fixture("sessions-invalid-configuration");
    let session = session_record(
        "session-config-invalid",
        SessionLifecycle::Idle,
        "Configuration",
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
            "UPDATE sessions SET chat_preferences = '{not-json}' WHERE id = ?1",
            [session.id()],
        )
        .expect("invalid snapshot");

    assert_eq!(
        SessionConfigurationRepository::load(&fixture.repository, session.aggregate.id())
            .expect("load configuration"),
        None
    );
}

#[test]
fn persisted_configuration_shape_is_separate_from_domain_preferences() {
    let values = ChatConfigurationValues {
        execution_mode: "execute".to_string(),
        provider_id: Some("openai".to_string()),
        model_id: Some("gpt-5-5".to_string()),
        reasoning_depth: Some("high".to_string()),
        streaming: true,
        thinking: true,
        long_context: true,
    };
    let raw = serde_json::to_value(&values).expect("serialize values");
    assert_eq!(raw["executionMode"], "execute");
    let reference = FileReferenceInput {
        id: "reference".to_string(),
        path: "src/main.rs".to_string(),
        name: "main.rs".to_string(),
        size_bytes: Some(12),
        content_hash: None,
        start_line: Some(10),
        end_line: Some(50),
    };
    let serialized = serde_json::to_value(reference).expect("serialize reference");
    assert_eq!(serialized["sizeBytes"], 12);
    assert_eq!(serialized["startLine"], 10);
    assert_eq!(serialized["endLine"], 50);
    // A row written before line ranges existed must still deserialize, as a whole-file
    // reference — this is what makes the added fields need no schema migration.
    let legacy: FileReferenceInput = serde_json::from_str(
        r#"{"id":"legacy","path":"src/main.rs","name":"main.rs","sizeBytes":12,"contentHash":null}"#,
    )
    .expect("deserialize legacy reference");
    assert_eq!(legacy.start_line, None);
    assert_eq!(legacy.end_line, None);
}

#[test]
fn seats_survive_a_create_and_are_updated_on_save() {
    let fixture = fixture("sessions-seats");
    let mut session = session_record(
        "session-seats",
        SessionLifecycle::Idle,
        "多 Agent 会话",
        "2026-08-07T00:00:00+00:00",
    );
    session.seats = vec![
        SessionSeat {
            seat_id: "seat-1".to_string(),
            agent_id: "claude-code".to_string(),
            role_id: Some("role-architect".to_string()),
            role_snapshot: None,
            joined_at: "2026-08-07T00:00:00+00:00".to_string(),
            left_at: None,
        },
        SessionSeat {
            seat_id: "seat-2".to_string(),
            agent_id: "codex-cli".to_string(),
            role_id: Some("role-reviewer".to_string()),
            role_snapshot: None,
            joined_at: "2026-08-07T00:00:00+00:00".to_string(),
            left_at: None,
        },
    ];
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create seated session");

    let mut loaded = SessionRepository::find(&fixture.repository, session.aggregate.id())
        .expect("find seated session")
        .expect("seated session");
    assert_eq!(loaded.seats, session.seats);

    // A seat added mid-session must be routable from the next turn, so `save` has to carry seats.
    loaded.seats.push(SessionSeat {
        seat_id: "seat-3".to_string(),
        agent_id: "gemini-cli".to_string(),
        role_id: None,
        role_snapshot: None,
        joined_at: "2026-08-07T00:00:01+00:00".to_string(),
        left_at: None,
    });
    let saved = SessionRepository::save(&fixture.repository, &loaded).expect("save seats");
    assert_eq!(saved.seats, loaded.seats);
}

/// Sessions created before seats existed store `[]`, and each must still open as its own Agent.
#[test]
fn a_session_without_seats_reads_as_one_seat() {
    let fixture = fixture("sessions-no-seats");
    let session = session_record(
        "session-single",
        SessionLifecycle::Idle,
        "单 Agent 会话",
        "2026-08-07T00:00:00+00:00",
    );
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create single session");
    fixture
        .database
        .connection()
        .expect("connection")
        .execute(
            "UPDATE sessions SET seats = '[]' WHERE id = ?1",
            [session.id()],
        )
        .expect("clear legacy seats");

    let loaded = SessionRepository::find(&fixture.repository, session.aggregate.id())
        .expect("find single session")
        .expect("single session");
    assert_eq!(
        loaded.seats,
        vec![SessionSeat {
            seat_id: "session-single:seat:0".to_string(),
            agent_id: "codex-cli".to_string(),
            role_id: None,
            role_snapshot: None,
            joined_at: "2026-07-01T00:00:00+00:00".to_string(),
            left_at: None,
        }]
    );
}

/// Rendering a thread means naming who spoke, so the seat has to survive persistence.
#[test]
fn a_message_records_the_seat_that_spoke_it() {
    let fixture = fixture("messages-seat-index");
    let session = session_record(
        "session-speakers",
        SessionLifecycle::Idle,
        "多 Agent 会话",
        "2026-08-07T00:00:00+00:00",
    );
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create session");

    let mut seated = message_record(
        "message-seated",
        "session-speakers",
        MessageRole::Assistant,
        MessageStatus::Completed,
        "方案如下",
    );
    seated.speaker_seat_id = Some("seat-reviewer".to_string());
    seated.seat_index = Some(1);
    SessionMessageRepository::insert(&fixture.repository, &seated).expect("insert seated");

    // A user message has no seat, and a default of 0 would attribute it to the first one.
    let spoken_by_user = message_record(
        "message-user",
        "session-speakers",
        MessageRole::User,
        MessageStatus::Completed,
        "改下登录",
    );
    SessionMessageRepository::insert(&fixture.repository, &spoken_by_user).expect("insert user");

    let loaded = SessionMessageRepository::find(&fixture.repository, seated.message.id())
        .expect("find seated")
        .expect("seated message");
    assert_eq!(loaded.seat_index, Some(1));
    assert_eq!(loaded.speaker_seat_id.as_deref(), Some("seat-reviewer"));

    let loaded_user =
        SessionMessageRepository::find(&fixture.repository, spoken_by_user.message.id())
            .expect("find user")
            .expect("user message");
    assert_eq!(loaded_user.seat_index, None);
}

#[test]
fn message_inserts_allocate_unique_sequences_in_the_current_shared_schema() {
    let fixture = fixture("messages-additive-session-sequence");
    let session = session_record(
        "session-sequences",
        SessionLifecycle::Idle,
        "共享数据库序号",
        "2026-08-08T00:00:00+00:00",
    );
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create session");

    for id in ["message-sequence-1", "message-sequence-2"] {
        let message = message_record(
            id,
            session.id(),
            MessageRole::User,
            MessageStatus::Completed,
            id,
        );
        SessionMessageRepository::insert(&fixture.repository, &message).expect("insert message");
    }

    let connection = fixture.database.connection().expect("connection");
    let sequences = connection
        .prepare(
            "SELECT session_sequence FROM messages WHERE session_id = ?1 ORDER BY session_sequence",
        )
        .expect("prepare sequences")
        .query_map([session.id()], |row| row.get::<_, i64>(0))
        .expect("query sequences")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect sequences");
    assert_eq!(sequences, vec![1, 2]);
    assert_eq!(
        connection
            .query_row(
                "SELECT next_message_sequence FROM sessions WHERE id = ?1",
                [session.id()],
                |row| row.get::<_, i64>(0),
            )
            .expect("next sequence"),
        3
    );
}

#[test]
fn stable_participant_schema_normalizes_legacy_seats_and_backfills_only_valid_speakers() {
    let connection = rusqlite::Connection::open_in_memory().expect("database");
    connection
        .execute_batch(
            r#"
            CREATE TABLE sessions (id TEXT PRIMARY KEY, agent_id TEXT NOT NULL, created_at TEXT NOT NULL, seats TEXT NOT NULL);
            CREATE TABLE messages (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, seat_index INTEGER, created_at TEXT NOT NULL);
            "#,
        )
        .expect("legacy schema");
    connection
        .execute(
            "INSERT INTO sessions(id, agent_id, created_at, seats) VALUES (?1, ?2, ?3, ?4)",
            params![
                "shared",
                "codex-cli",
                "2026-08-01T00:00:00Z",
                r#"[{"agentId":"codex-cli","roleId":"reviewer"},{"agentId":"gemini-cli","roleId":"architect","leftAt":"2026-08-02T00:00:00Z"}]"#
            ],
        )
        .expect("shared session");
    connection
        .execute(
            "INSERT INTO sessions(id, agent_id, created_at, seats) VALUES (?1, ?2, ?3, ?4)",
            params!["single", "claude-code", "2026-08-01T00:00:00Z", "malformed"],
        )
        .expect("single session");
    for (id, session_id, seat_index) in [
        ("valid", "shared", 1_i64),
        ("invalid", "shared", 8_i64),
        ("single-valid", "single", 0_i64),
    ] {
        connection
            .execute(
                "INSERT INTO messages(id, session_id, seat_index, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![id, session_id, seat_index, "2026-08-03T00:00:00Z"],
            )
            .expect("legacy message");
    }

    apply_stable_participant_schema(&connection).expect("first migration");
    apply_stable_participant_schema(&connection).expect("idempotent migration");

    let shared: String = connection
        .query_row(
            "SELECT seats FROM sessions WHERE id = 'shared'",
            [],
            |row| row.get(0),
        )
        .expect("shared seats");
    let shared = crate::contexts::sessions::domain::decode_seats(
        &shared,
        "shared",
        "codex-cli",
        "2026-08-01T00:00:00Z",
    );
    assert_eq!(shared[0].seat_id, "shared:seat:0");
    assert_eq!(shared[1].seat_id, "shared:seat:1");
    assert_eq!(shared[1].left_at.as_deref(), Some("2026-08-02T00:00:00Z"));
    let speaker = |message_id: &str| -> Option<String> {
        connection
            .query_row(
                "SELECT speaker_seat_id FROM messages WHERE id = ?1",
                [message_id],
                |row| row.get(0),
            )
            .expect("speaker")
    };
    assert_eq!(speaker("valid").as_deref(), Some("shared:seat:1"));
    assert_eq!(speaker("invalid"), None);
    assert_eq!(speaker("single-valid").as_deref(), Some("single:seat:0"));
}
