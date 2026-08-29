use crate::{
    contexts::skill_evolution_orchestration::domain::{
        EvolutionCheckpointStatus, EvolutionStageKind,
    },
    platform::database::NativeDatabase,
    test_support::TempDirectory,
};

use super::{OrchestrationPersistenceError, OrchestrationRepository};

#[test]
fn run_without_checkpoint_starts_at_recovery() {
    let (repository, database, _directory) = repository("resume-empty");
    seed_run(&database);
    let position = repository.run_resume_position("run-one").expect("position");
    assert_eq!(position.next_stage, Some(EvolutionStageKind::Recover));
    assert_eq!(position.checkpoint_status, None);
}

#[test]
fn committed_checkpoint_advances_and_continuation_retries_the_same_stage() {
    let (repository, database, _directory) = repository("resume-checkpoint");
    seed_run(&database);
    seed_checkpoint(&database, "checkpoint-one", "assess", "committed", 100);
    let advanced = repository.run_resume_position("run-one").expect("advanced");
    assert_eq!(
        advanced.next_stage,
        Some(EvolutionStageKind::RouteGovernance)
    );
    seed_checkpoint(
        &database,
        "checkpoint-two",
        "route_governance",
        "continuation_required",
        200,
    );
    let continued = repository
        .run_resume_position("run-one")
        .expect("continued");
    assert_eq!(
        continued.next_stage,
        Some(EvolutionStageKind::RouteGovernance)
    );
    assert_eq!(
        continued.checkpoint_status,
        Some(EvolutionCheckpointStatus::ContinuationRequired)
    );
    assert_eq!(continued.cursor_record_id.as_deref(), Some("record:7"));
    assert_eq!(continued.cursor_record_revision, Some(7));
}

#[test]
fn completed_notify_checkpoint_has_no_remaining_stage_and_corruption_fails_closed() {
    let (repository, database, _directory) = repository("resume-complete");
    seed_run(&database);
    seed_checkpoint(&database, "checkpoint-one", "notify", "committed", 100);
    assert_eq!(
        repository
            .run_resume_position("run-one")
            .expect("complete")
            .next_stage,
        None
    );
    database
        .connection()
        .expect("connection")
        .execute("UPDATE evolution_run_checkpoints SET status='pending'", [])
        .expect("corrupt checkpoint");
    assert_eq!(
        repository.run_resume_position("run-one"),
        Err(OrchestrationPersistenceError::Corrupt)
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

fn seed_run(database: &NativeDatabase) {
    let connection = database.connection().expect("connection");
    connection
        .execute("INSERT INTO evolution_run_requests VALUES ('request-one',1,'workspace-one','runtime_trigger','claimed','{}',0,0,'run-one',0,1,1)", [])
        .expect("request");
    connection
        .execute("INSERT INTO evolution_runs VALUES ('run-one',1,'request-one','workspace-one','recovered',NULL,'policy','{}','{}',NULL,'recovery-worker',100,NULL,0,1,1)", [])
        .expect("run");
}

fn seed_checkpoint(
    database: &NativeDatabase,
    checkpoint_id: &str,
    stage: &str,
    status: &str,
    committed_at_ms: i64,
) {
    let continuation_not_before_ms =
        (status == "continuation_required").then_some(committed_at_ms + 30_000);
    database
        .connection()
        .expect("connection")
        .execute(
            "INSERT INTO evolution_run_checkpoints VALUES (?1,'run-one',?2,?3,'record:7',7,'{}',?4,?5)",
            rusqlite::params![checkpoint_id, stage, status, continuation_not_before_ms, committed_at_ms],
        )
        .expect("checkpoint");
}
