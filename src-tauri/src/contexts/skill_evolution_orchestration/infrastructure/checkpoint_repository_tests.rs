use crate::{
    contexts::skill_evolution_orchestration::domain::{
        EvolutionCheckpointStatus, EvolutionRunUsageV1, EvolutionStageKind,
    },
    platform::database::NativeDatabase,
    test_support::TempDirectory,
};

use super::{OrchestrationPersistenceError, OrchestrationRepository, StageCheckpointCommitV1};

#[test]
fn stable_record_cursor_and_run_usage_commit_in_one_transaction() {
    let (repository, database, _directory) = repository();
    seed_run(&database);
    let usage = EvolutionRunUsageV1 {
        evidence_items: 11,
        ..EvolutionRunUsageV1::default()
    };
    let request = StageCheckpointCommitV1 {
        run_id: "run-one".into(),
        expected_run_revision: 0,
        stage: EvolutionStageKind::MaintainEvidence,
        status: EvolutionCheckpointStatus::Committed,
        cursor_record_id: Some("evidence:000011".into()),
        cursor_record_revision: Some(7),
        usage,
        continuation_not_before_ms: None,
        committed_at_ms: 100,
    };
    let outcome = repository
        .commit_stage_checkpoint(&request)
        .expect("checkpoint");
    assert_eq!(outcome.run_revision, 1);
    let connection = database.connection().expect("connection");
    let cursor: (String, i64) = connection
        .query_row(
            "SELECT cursor_record_id,cursor_record_revision
             FROM evolution_run_checkpoints WHERE checkpoint_id=?1",
            [outcome.checkpoint_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("cursor");
    assert_eq!(cursor, ("evidence:000011".into(), 7));
    let usage_json: String = connection
        .query_row("SELECT usage_json FROM evolution_runs", [], |row| {
            row.get(0)
        })
        .expect("usage");
    assert_eq!(
        serde_json::from_str::<EvolutionRunUsageV1>(&usage_json)
            .expect("usage model")
            .evidence_items,
        11
    );
    assert_eq!(
        repository.commit_stage_checkpoint(&request),
        Err(OrchestrationPersistenceError::Conflict)
    );
}

#[test]
fn idle_timeout_atomically_checkpoints_a_bounded_continuation() {
    let (repository, database, _directory) = repository();
    seed_run(&database);
    let checkpoint = repository
        .defer_run_for_idle(
            "run-one",
            0,
            EvolutionStageKind::MaintainEvidence,
            &EvolutionRunUsageV1::default(),
            900_000,
            930_000,
        )
        .expect("checkpoint");
    assert_eq!(checkpoint.revision, 1);
    assert_eq!(checkpoint.continuation_not_before_ms, 930_000);
    let connection = database.connection().expect("connection");
    let run: (String, String, i64) = connection
        .query_row(
            "SELECT status,current_stage,revision FROM evolution_runs WHERE run_id='run-one'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("run");
    assert_eq!(run, ("partial".into(), "maintain_evidence".into(), 1));
    let status: String = connection
        .query_row(
            "SELECT status FROM evolution_run_checkpoints WHERE checkpoint_id=?1",
            [checkpoint.checkpoint_id],
            |row| row.get(0),
        )
        .expect("checkpoint status");
    assert_eq!(status, "continuation_required");
}

#[test]
fn stale_idle_deferral_revision_cannot_create_a_second_checkpoint() {
    let (repository, database, _directory) = repository();
    seed_run(&database);
    repository
        .defer_run_for_idle(
            "run-one",
            0,
            EvolutionStageKind::Recover,
            &EvolutionRunUsageV1::default(),
            900_000,
            930_000,
        )
        .expect("first");
    assert_eq!(
        repository.defer_run_for_idle(
            "run-one",
            0,
            EvolutionStageKind::Recover,
            &EvolutionRunUsageV1::default(),
            900_001,
            930_001,
        ),
        Err(OrchestrationPersistenceError::Conflict)
    );
    let count: i64 = database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM evolution_run_checkpoints",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(count, 1);
}

fn repository() -> (OrchestrationRepository, NativeDatabase, TempDirectory) {
    let directory = TempDirectory::new("idle-checkpoint");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    (
        OrchestrationRepository::new(database.clone()),
        database,
        directory,
    )
}

fn seed_run(database: &NativeDatabase) {
    let connection = database.connection().expect("connection");
    connection
        .execute("INSERT INTO evolution_run_requests VALUES ('request-one',1,'workspace-one','runtime_trigger','claimed','{}',0,0,'run-one',0,1,1)", [])
        .expect("request");
    connection
        .execute("INSERT INTO evolution_runs VALUES ('run-one',1,'request-one','workspace-one','waiting_idle',NULL,'policy','{}','{}',NULL,NULL,NULL,NULL,0,1,1)", [])
        .expect("run");
}
