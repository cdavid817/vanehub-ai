use crate::{
    contexts::skill_evolution_orchestration::domain::{
        canonical_hash, orchestration_idempotency_key, EvolutionActorProvenance,
        EvolutionTriggerEnvelopeV1, EvolutionTriggerFamily, ORCHESTRATION_SCHEMA_VERSION_V1,
    },
    platform::database::NativeDatabase,
    test_support::TempDirectory,
};

use super::{OrchestrationPersistenceError, OrchestrationRepository, PersistTriggerOutcome};
use std::{sync::Barrier, thread};

#[test]
fn canonical_hash_and_idempotency_key_ignore_object_insertion_order() {
    let left = serde_json::json!({"second": 2, "first": 1});
    let right = serde_json::json!({"first": 1, "second": 2});
    assert_eq!(canonical_hash(&left), canonical_hash(&right));
    assert_eq!(
        orchestration_idempotency_key("assessment", "route", &left),
        orchestration_idempotency_key("assessment", "route", &right)
    );
}

#[test]
fn duplicate_trigger_receipts_return_the_original_identity() {
    let (repository, _, _directory) = repository("orchestration-trigger");
    let first = trigger("receipt-one");
    assert_eq!(
        repository.persist_trigger(&first, 2),
        Ok(PersistTriggerOutcome::Inserted {
            receipt_id: "receipt-one".into()
        })
    );
    let mut replay = first;
    replay.trigger_id = "receipt-replay".into();
    assert_eq!(
        repository.persist_trigger(&replay, 3),
        Ok(PersistTriggerOutcome::Duplicate {
            receipt_id: "receipt-one".into()
        })
    );
}

#[test]
fn stale_and_live_foreign_leases_are_conflict_safe() {
    let (repository, database, _directory) = repository("orchestration-lease");
    seed_run(&database);
    let first = repository
        .acquire_run_lease("run-one", 0, "worker-one", 10, 100)
        .expect("first lease");
    assert_eq!(first.revision, 1);
    assert_eq!(
        repository.acquire_run_lease("run-one", 0, "worker-two", 11, 101),
        Err(OrchestrationPersistenceError::Conflict)
    );
    assert_eq!(
        repository.acquire_run_lease("run-one", 1, "worker-two", 99, 200),
        Err(OrchestrationPersistenceError::Conflict)
    );
    let recovered = repository
        .acquire_run_lease("run-one", 1, "worker-two", 100, 200)
        .expect("expired lease");
    assert_eq!(recovered.revision, 2);
}

#[test]
fn persistence_rejects_unbounded_or_unsafe_safe_metadata() {
    let (repository, _, _directory) = repository("orchestration-invalid");
    let mut input = trigger("receipt-one");
    input.safe_reason_codes = vec!["contains a space".into()];
    assert_eq!(
        repository.persist_trigger(&input, 2),
        Err(OrchestrationPersistenceError::InvalidInput)
    );
}

#[test]
fn corrupt_persisted_safe_metadata_fails_closed() {
    let (repository, database, _directory) = repository("orchestration-corrupt");
    repository
        .persist_trigger(&trigger("receipt-one"), 2)
        .expect("receipt");
    database
        .connection()
        .expect("connection")
        .execute(
            "UPDATE evolution_trigger_receipts SET safe_reason_codes_json='not-json'",
            [],
        )
        .expect("corrupt fixture");
    assert_eq!(
        repository.trigger_safe_reason_codes("receipt-one"),
        Err(OrchestrationPersistenceError::Corrupt)
    );
}

#[test]
fn concurrent_lease_writers_produce_exactly_one_revision_winner() {
    let (repository, database, _directory) = repository("orchestration-concurrent");
    seed_run(&database);
    let barrier = std::sync::Arc::new(Barrier::new(3));
    let handles: Vec<_> = ["worker-one", "worker-two"]
        .into_iter()
        .map(|worker| {
            let repository = repository.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                repository.acquire_run_lease("run-one", 0, worker, 10, 100)
            })
        })
        .collect();
    barrier.wait();
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("lease writer"))
        .collect();
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| { matches!(outcome, Err(OrchestrationPersistenceError::Conflict)) })
            .count(),
        1
    );
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

fn trigger(receipt_id: &str) -> EvolutionTriggerEnvelopeV1 {
    EvolutionTriggerEnvelopeV1 {
        schema_version: ORCHESTRATION_SCHEMA_VERSION_V1,
        trigger_id: receipt_id.into(),
        family: EvolutionTriggerFamily::AgentRunCompletion,
        workspace_id: "workspace-one".into(),
        source_kind: "agent-run".into(),
        source_id: "run-source".into(),
        source_revision: 1,
        occurred_at_ms: 1,
        priority: 10,
        safe_reason_codes: vec!["agent-run-completed".into()],
        actor: EvolutionActorProvenance::RuntimeTrigger,
    }
}

fn seed_run(database: &NativeDatabase) {
    let connection = database.connection().expect("connection");
    connection
        .execute("INSERT INTO evolution_run_requests VALUES ('request-one',1,'workspace-one','runtime_trigger','claimed','{}',0,0,'run-one',0,1,1)", [])
        .expect("request");
    connection
        .execute("INSERT INTO evolution_runs VALUES ('run-one',1,'request-one','workspace-one','requested',NULL,'policy','{}','{}',NULL,NULL,NULL,NULL,0,1,1)", [])
        .expect("run");
}
