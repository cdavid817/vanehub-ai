use crate::{
    contexts::skill_evolution_orchestration::domain::EvolutionRunStatus,
    platform::database::NativeDatabase, test_support::TempDirectory,
};

use super::{OrchestrationPersistenceError, OrchestrationRepository};

#[test]
fn heartbeat_requires_the_live_owner_and_current_revision() {
    let (repository, database, _directory) = repository("run-heartbeat");
    seed_run(&database, "running", Some(("worker-one", 100)));
    let heartbeat = repository
        .heartbeat_run_lease("run-one", 0, "worker-one", 50, 150)
        .expect("heartbeat");
    assert_eq!(heartbeat.revision, 1);
    assert_eq!(heartbeat.status, EvolutionRunStatus::Running);
    assert_eq!(
        repository.heartbeat_run_lease("run-one", 0, "worker-one", 60, 160),
        Err(OrchestrationPersistenceError::Conflict)
    );
    assert_eq!(
        repository.heartbeat_run_lease("run-one", 1, "worker-two", 60, 160),
        Err(OrchestrationPersistenceError::Conflict)
    );
}

#[test]
fn cooperative_cancellation_reaches_terminal_state_and_clears_the_lease() {
    let (repository, database, _directory) = repository("run-cancellation");
    seed_run(&database, "running", Some(("worker-one", 100)));
    let requested = repository
        .request_run_cancellation("run-one", 0, 20)
        .expect("request cancellation");
    assert_eq!(requested.status, EvolutionRunStatus::CancelRequested);
    let cancelled = repository
        .transition_run_status(
            "run-one",
            1,
            "worker-one",
            EvolutionRunStatus::Cancelled,
            30,
        )
        .expect("cancelled");
    assert_eq!(cancelled.revision, 2);
    let state: (String, Option<String>, Option<i64>) = database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT status,lease_owner,lease_expires_at_ms FROM evolution_runs",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("state");
    assert_eq!(state, ("cancelled".into(), None, None));
    assert_eq!(
        repository.transition_run_status(
            "run-one",
            2,
            "worker-one",
            EvolutionRunStatus::Running,
            40,
        ),
        Err(OrchestrationPersistenceError::Conflict)
    );
}

#[test]
fn expired_leases_are_bounded_and_recovered_with_cas() {
    let (repository, database, _directory) = repository("run-recovery");
    seed_run(&database, "running", Some(("dead-worker", 100)));
    let expired = repository.expired_run_leases(100, 10).expect("expired");
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].lease_owner, "dead-worker");
    let recovered = repository
        .recover_expired_run("run-one", 0, "recovery-worker", 100, 200)
        .expect("recovered");
    assert_eq!(recovered.status, EvolutionRunStatus::Recovered);
    assert_eq!(recovered.revision, 1);
    assert!(repository
        .expired_run_leases(100, 10)
        .expect("after recovery")
        .is_empty());
    assert_eq!(
        repository.recover_expired_run("run-one", 0, "other-worker", 100, 200),
        Err(OrchestrationPersistenceError::Conflict)
    );
}

#[test]
fn invalid_status_transition_is_rejected_without_revision_change() {
    let (repository, database, _directory) = repository("run-transition");
    seed_run(&database, "running", Some(("worker-one", 100)));
    assert_eq!(
        repository.transition_run_status(
            "run-one",
            0,
            "worker-one",
            EvolutionRunStatus::WaitingIdle,
            20,
        ),
        Err(OrchestrationPersistenceError::Conflict)
    );
    let revision: i64 = database
        .connection()
        .expect("connection")
        .query_row("SELECT revision FROM evolution_runs", [], |row| row.get(0))
        .expect("revision");
    assert_eq!(revision, 0);
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

fn seed_run(database: &NativeDatabase, status: &str, lease: Option<(&str, i64)>) {
    let connection = database.connection().expect("connection");
    connection
        .execute("INSERT INTO evolution_run_requests VALUES ('request-one',1,'workspace-one','runtime_trigger','claimed','{}',0,0,'run-one',0,1,1)", [])
        .expect("request");
    let (owner, expires) = lease.map_or((None, None), |(owner, expires)| {
        (Some(owner), Some(expires))
    });
    connection
        .execute(
            "INSERT INTO evolution_runs
             (run_id,schema_version,request_id,workspace_id,status,current_stage,
              policy_witness_hash,budget_json,usage_json,cancel_requested_at_ms,
              lease_owner,lease_expires_at_ms,safe_failure_code,revision,created_at_ms,updated_at_ms)
             VALUES ('run-one',1,'request-one','workspace-one',?1,NULL,'policy','{}','{}',NULL,?2,?3,NULL,0,1,1)",
            rusqlite::params![status, owner, expires],
        )
        .expect("run");
}
