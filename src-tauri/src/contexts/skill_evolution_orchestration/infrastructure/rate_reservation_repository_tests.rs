use crate::{
    contexts::skill_evolution_orchestration::domain::{
        AutoRateReservationV1, RateReservationHistoryObservationV1, RateReservationStatus,
        ROLLING_DAY_MS, ROLLING_WEEK_MS,
    },
    platform::database::NativeDatabase,
    test_support::TempDirectory,
};

use super::{AutomaticRateLimit, RateReservationError, SqliteRateReservationRepository};

#[test]
fn transactional_reservations_enforce_run_workspace_and_skill_windows() {
    let (database, repository, _directory) = fixture("rate-limits");
    add_run(&database, "run-one", "workspace-one");
    let first = reservation("one", "run-one", "workspace-one", "skill-one", 1);
    assert_eq!(repository.reserve(&first), Ok(true));
    assert_eq!(repository.reserve(&first), Ok(false));
    assert_eq!(
        repository.reserve(&reservation(
            "same-run",
            "run-one",
            "workspace-one",
            "skill-two",
            2,
        )),
        Err(RateReservationError::Limited(AutomaticRateLimit::Run))
    );

    for index in 2..=3 {
        let run = format!("run-{index}");
        add_run(&database, &run, "workspace-one");
        repository
            .reserve(&reservation(
                &index.to_string(),
                &run,
                "workspace-one",
                &format!("skill-{index}"),
                ROLLING_DAY_MS,
            ))
            .expect("workspace reservation");
    }
    add_run(&database, "run-four", "workspace-one");
    assert_eq!(
        repository.reserve(&reservation(
            "four",
            "run-four",
            "workspace-one",
            "skill-four",
            ROLLING_DAY_MS,
        )),
        Err(RateReservationError::Limited(
            AutomaticRateLimit::Workspace24Hours
        ))
    );

    add_run(&database, "run-skill-window", "workspace-two");
    assert_eq!(
        repository.reserve(&reservation(
            "skill-window",
            "run-skill-window",
            "workspace-two",
            "skill-one",
            ROLLING_WEEK_MS,
        )),
        Err(RateReservationError::Limited(
            AutomaticRateLimit::Skill7Days
        ))
    );
    add_run(&database, "run-skill-expired", "workspace-two");
    assert!(repository
        .reserve(&reservation(
            "skill-expired",
            "run-skill-expired",
            "workspace-two",
            "skill-one",
            ROLLING_WEEK_MS + 2,
        ))
        .is_ok());
}

#[test]
fn reconciliation_never_releases_partial_or_inconsistent_history() {
    let (database, repository, _directory) = fixture("rate-reconcile");
    add_run(&database, "run-one", "workspace-one");
    repository
        .reserve(&reservation(
            "one",
            "run-one",
            "workspace-one",
            "skill-one",
            1,
        ))
        .expect("reserve");
    let partial = RateReservationHistoryObservationV1 {
        automatic_application_id: None,
        curator_application_id: Some("curator-one".into()),
        overlay_application_id: None,
    };
    let recovery = repository
        .reconcile("reservation-one", 0, &partial, 2)
        .expect("recovery required");
    assert_eq!(recovery.status, RateReservationStatus::RecoveryRequired);

    let absent = RateReservationHistoryObservationV1 {
        automatic_application_id: None,
        curator_application_id: None,
        overlay_application_id: None,
    };
    let released = repository
        .reconcile("reservation-one", 1, &absent, 3)
        .expect("release after both histories are empty");
    assert_eq!(released.status, RateReservationStatus::Released);

    add_run(&database, "run-two", "workspace-one");
    assert!(repository
        .reserve(&reservation(
            "two",
            "run-two",
            "workspace-one",
            "skill-one",
            4,
        ))
        .is_ok());
}

#[test]
fn complete_three_way_history_commits_reservation_and_stale_reconcile_loses_cas() {
    let (database, repository, _directory) = fixture("rate-commit");
    add_run(&database, "run-one", "workspace-one");
    repository
        .reserve(&reservation(
            "one",
            "run-one",
            "workspace-one",
            "skill-one",
            1,
        ))
        .expect("reserve");
    let committed = RateReservationHistoryObservationV1 {
        automatic_application_id: Some("application-one".into()),
        curator_application_id: Some("curator-one".into()),
        overlay_application_id: Some("overlay-one".into()),
    };
    let result = repository
        .reconcile("reservation-one", 0, &committed, 2)
        .expect("commit");
    assert_eq!(result.status, RateReservationStatus::Committed);
    assert_eq!(result.application_id.as_deref(), Some("application-one"));
    assert_eq!(
        repository.reconcile("reservation-one", 0, &committed, 3),
        Err(RateReservationError::Conflict)
    );
}

fn fixture(
    name: &str,
) -> (
    NativeDatabase,
    SqliteRateReservationRepository,
    TempDirectory,
) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    (
        database.clone(),
        SqliteRateReservationRepository::new(database),
        directory,
    )
}

fn add_run(database: &NativeDatabase, run_id: &str, workspace_id: &str) {
    let connection = database.connection().expect("connection");
    let request_id = format!("request-{run_id}");
    connection.execute("INSERT INTO evolution_run_requests VALUES (?1,1,?2,'runtime_trigger','completed','{}',0,0,?3,0,1,1)", rusqlite::params![request_id, workspace_id, run_id]).expect("request");
    connection.execute("INSERT INTO evolution_runs VALUES (?1,1,?2,?3,'completed',NULL,'policy','{}','{}',NULL,NULL,NULL,NULL,0,1,1)", rusqlite::params![run_id, request_id, workspace_id]).expect("run");
}

fn reservation(
    suffix: &str,
    run_id: &str,
    workspace_id: &str,
    skill_id: &str,
    now_ms: i64,
) -> AutoRateReservationV1 {
    AutoRateReservationV1 {
        reservation_id: format!("reservation-{suffix}"),
        run_id: run_id.into(),
        workspace_id: workspace_id.into(),
        skill_id: skill_id.into(),
        status: RateReservationStatus::Reserved,
        application_id: None,
        reserved_at_ms: now_ms,
        updated_at_ms: now_ms,
        revision: 0,
    }
}
