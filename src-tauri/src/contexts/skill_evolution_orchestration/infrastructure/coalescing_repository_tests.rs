use crate::{
    contexts::skill_evolution_orchestration::domain::{
        EvolutionActorProvenance, EvolutionTriggerCountersV1, EvolutionTriggerEnvelopeV1,
        EvolutionTriggerFamily, ORCHESTRATION_SCHEMA_VERSION_V1,
    },
    platform::database::NativeDatabase,
    test_support::TempDirectory,
};

use super::{OrchestrationPersistenceError, OrchestrationRepository, ReceiveTriggerOutcome};

#[test]
fn duplicate_delivery_does_not_increment_or_link_twice() {
    let (repository, database, _directory) = repository("trigger-deduplication");
    let input = trigger("receipt-one", EvolutionTriggerFamily::AgentRunCompletion, 1);
    let first = repository.receive_trigger(&input, 100).expect("first");
    let duplicate = repository.receive_trigger(&input, 101).expect("duplicate");
    assert!(matches!(first, ReceiveTriggerOutcome::Queued { .. }));
    assert_eq!(
        duplicate,
        ReceiveTriggerOutcome::Duplicate {
            receipt_id: "receipt-one".into()
        }
    );
    assert_eq!(count(&database, "evolution_trigger_receipts"), 1);
    assert_eq!(count(&database, "evolution_run_request_trigger_links"), 1);
    assert_eq!(counters(&database).agent_run_completion, 1);
}

#[test]
fn automatic_burst_and_out_of_order_events_share_the_first_debounce_window() {
    let (repository, database, _directory) = repository("trigger-burst");
    let first = repository
        .receive_trigger(
            &trigger(
                "receipt-one",
                EvolutionTriggerFamily::ConversationCompletion,
                20,
            ),
            1_000,
        )
        .expect("first");
    let second = repository
        .receive_trigger(
            &trigger(
                "receipt-two",
                EvolutionTriggerFamily::VerificationCompletion,
                10,
            ),
            1_500,
        )
        .expect("second");
    let (first_request, first_not_before) = queued_identity(first);
    let (second_request, second_not_before) = queued_identity(second);
    assert_eq!(first_request, second_request);
    assert_eq!(first_not_before, 31_000);
    assert_eq!(second_not_before, 31_000);
    let counters = counters(&database);
    assert_eq!(counters.conversation_completion, 1);
    assert_eq!(counters.verification_completion, 1);
}

#[test]
fn workspaces_have_independent_pending_requests() {
    let (repository, _, _directory) = repository("trigger-workspaces");
    let first = repository
        .receive_trigger(
            &trigger(
                "receipt-one",
                EvolutionTriggerFamily::PeriodicMaintenance,
                1,
            ),
            100,
        )
        .expect("first");
    let mut other = trigger(
        "receipt-two",
        EvolutionTriggerFamily::PeriodicMaintenance,
        1,
    );
    other.workspace_id = "workspace-two".into();
    let second = repository.receive_trigger(&other, 100).expect("second");
    assert_ne!(queued_identity(first).0, queued_identity(second).0);
}

#[test]
fn active_workspace_folds_triggers_and_manual_request_removes_debounce_wait() {
    let (repository, database, _directory) = repository("trigger-follow-up");
    seed_active_run(&database);
    let automatic = repository
        .receive_trigger(
            &trigger(
                "receipt-one",
                EvolutionTriggerFamily::ExplicitFeedbackCommit,
                1,
            ),
            1_000,
        )
        .expect("automatic");
    let manual = repository
        .receive_trigger(
            &trigger("receipt-two", EvolutionTriggerFamily::ManualRunRequest, 1),
            1_500,
        )
        .expect("manual");
    let (automatic_request, automatic_not_before) = queued_identity(automatic);
    let (manual_request, manual_not_before) = queued_identity(manual);
    assert_eq!(automatic_request, manual_request);
    assert_eq!(automatic_not_before, 31_000);
    assert_eq!(manual_not_before, 1_500);
    let follow_up: i64 = database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT follow_up FROM evolution_run_requests WHERE request_id=?1",
            [automatic_request],
            |row| row.get(0),
        )
        .expect("follow-up");
    assert_eq!(follow_up, 1);
}

#[test]
fn unsupported_trigger_version_is_rejected_without_a_receipt() {
    let (repository, database, _directory) = repository("trigger-version");
    let mut input = trigger("receipt-one", EvolutionTriggerFamily::StartupRecovery, 1);
    input.schema_version = 2;
    assert_eq!(
        repository.receive_trigger(&input, 100),
        Err(OrchestrationPersistenceError::InvalidInput)
    );
    assert_eq!(count(&database, "evolution_trigger_receipts"), 0);
}

#[test]
fn startup_recovery_replay_creates_one_request_instead_of_missed_interval_runs() {
    let (repository, database, _directory) = repository("startup-recovery-singleton");
    let first = trigger(
        "startup-receipt-one",
        EvolutionTriggerFamily::StartupRecovery,
        100,
    );
    repository
        .receive_trigger(&first, 100)
        .expect("startup recovery");
    let mut replay = first;
    replay.trigger_id = "startup-receipt-replay".into();
    assert_eq!(
        repository.receive_trigger(&replay, 101),
        Ok(ReceiveTriggerOutcome::Duplicate {
            receipt_id: "startup-receipt-one".into()
        })
    );
    assert_eq!(count(&database, "evolution_run_requests"), 1);
}

fn repository(name: &str) -> (OrchestrationRepository, NativeDatabase, TempDirectory) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    (
        OrchestrationRepository::new(database.clone()),
        database,
        directory,
    )
}

fn trigger(
    receipt_id: &str,
    family: EvolutionTriggerFamily,
    occurred_at_ms: i64,
) -> EvolutionTriggerEnvelopeV1 {
    EvolutionTriggerEnvelopeV1 {
        schema_version: ORCHESTRATION_SCHEMA_VERSION_V1,
        trigger_id: receipt_id.into(),
        family,
        workspace_id: "workspace-one".into(),
        source_kind: format!("source-{}", family.as_str()),
        source_id: receipt_id.into(),
        source_revision: 1,
        occurred_at_ms,
        priority: 50,
        safe_reason_codes: vec!["source-completed".into()],
        actor: EvolutionActorProvenance::RuntimeTrigger,
    }
}

fn queued_identity(outcome: ReceiveTriggerOutcome) -> (String, i64) {
    match outcome {
        ReceiveTriggerOutcome::Queued {
            request_id,
            not_before_ms,
            ..
        } => (request_id, not_before_ms),
        ReceiveTriggerOutcome::Duplicate { .. } => panic!("expected queued trigger"),
    }
}

fn count(database: &NativeDatabase, table: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    database
        .connection()
        .expect("connection")
        .query_row(&sql, [], |row| row.get(0))
        .expect("count")
}

fn counters(database: &NativeDatabase) -> EvolutionTriggerCountersV1 {
    let json: String = database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT trigger_counters_json FROM evolution_run_requests
             WHERE status='pending' ORDER BY created_at_ms LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("counters");
    serde_json::from_str(&json).expect("valid counters")
}

fn seed_active_run(database: &NativeDatabase) {
    let connection = database.connection().expect("connection");
    connection
        .execute("INSERT INTO evolution_run_requests VALUES ('active-request',1,'workspace-one','runtime_trigger','claimed','{}',0,0,'active-run',0,1,1)", [])
        .expect("request");
    connection
        .execute("INSERT INTO evolution_runs VALUES ('active-run',1,'active-request','workspace-one','running',NULL,'policy','{}','{}',NULL,NULL,NULL,NULL,0,1,1)", [])
        .expect("run");
}
