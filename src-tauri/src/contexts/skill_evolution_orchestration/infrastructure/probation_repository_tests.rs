use super::*;
use crate::{
    contexts::skill_evolution_orchestration::domain::*, platform::database::NativeDatabase,
    test_support::TempDirectory,
};

fn fixture(name: &str) -> (NativeDatabase, SqliteProbationRepository, TempDirectory) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let connection = database.connection().expect("connection");
    connection
        .execute_batch(
            "PRAGMA foreign_keys=OFF;
             INSERT INTO evolution_curator_candidates
             (candidate_id,schema_version,workspace_id,seed_id,seed_revision,
              assessment_attempt_id,assessment_revision,target_skill_id,target_revision,
              overlay_scope,route,risk,confidence,policy_witness_hash,witness_hash,snapshot_json,
              state,staleness_json,revision,created_at_ms,updated_at_ms)
             VALUES ('candidate-1',1,'workspace-1','seed-1','seed-revision-1','assessment-1',
                     'assessment-revision-1','skill-1','target-revision-1','project','advance',
                     'low','high','policy','candidate-witness','{}','applied','[]',1,1,1);
             INSERT INTO evolution_curator_applications
             (application_id,candidate_id,decision_id,status,approved_witness_hash,
              overlay_revision,overlay_history_id,revision,created_at_ms,updated_at_ms)
             VALUES ('application-1','candidate-1','decision-1','applied','preview',
                     '2','history-1',2,1,1);
             INSERT INTO evolution_auto_applications VALUES
             ('application-1','run-1','eligibility-1','preflight','policy','reservation-1',
              'application-1','application-1','skill-1','prior','current','system_policy',1);
             INSERT INTO evolution_auto_probations VALUES
             ('probation-1','application-1','workspace-1','skill-1','active','prior','current',
              'fingerprint','[\"verification\",\"security\"]','baseline',1,604800001,0);
             PRAGMA foreign_keys=ON;",
        )
        .expect("seed probation");
    drop(connection);
    (
        database.clone(),
        SqliteProbationRepository::new(database),
        directory,
    )
}

fn observation(id: &str, source_id: &str, category: &str) -> ProbationObservationV1 {
    ProbationObservationV1 {
        observation_id: id.into(),
        probation_id: "probation-1".into(),
        source_kind: "verification".into(),
        source_id: source_id.into(),
        source_revision: 1,
        verified: true,
        negative: true,
        baseline_exceeded: true,
        harmful_correction: false,
        safe_category: category.into(),
        witness_hash: format!("witness-{id}"),
        observed_at_ms: 10,
    }
}

#[test]
fn independent_regression_suspends_skill_and_routes_one_curator_review_without_revert() {
    let (database, repository, _directory) = fixture("probation-regression");
    let first = observation("one", "run-one", "verification");
    assert_eq!(
        repository
            .record_observation(&first, 10)
            .expect("first")
            .evaluation,
        ProbationEvaluation::Active
    );
    assert!(
        repository
            .record_observation(&first, 10)
            .expect("duplicate")
            .duplicate
    );
    let outcome = repository
        .record_observation(&observation("two", "run-two", "verification"), 10)
        .expect("regression");
    assert_eq!(outcome.evaluation, ProbationEvaluation::Regressed);
    assert!(outcome.rollback_candidate_id.is_some());
    assert!(!outcome.security_escalated);

    let connection = database.connection().expect("connection");
    let state: (String, String, i64, String, i64) = connection
        .query_row(
            "SELECT p.status,b.status,
                (SELECT COUNT(*) FROM evolution_curator_rollback_candidates),
                (SELECT overlay_revision FROM evolution_curator_applications
                 WHERE application_id='application-1'),
                (SELECT COUNT(*) FROM evolution_curator_notification_receipts
                 WHERE event_kind='probation_regression' AND delivery_status='pending')
         FROM evolution_auto_probations p JOIN evolution_auto_breakers b
           ON b.workspace_id=p.workspace_id AND b.skill_id=p.skill_id",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("regression state");
    assert_eq!(state, ("regressed".into(), "open".into(), 1, "2".into(), 1));
}

#[test]
fn healthy_expiry_is_auditable_and_security_regression_escalates_workspace() {
    let (database, repository, _directory) = fixture("probation-security");
    let healthy = repository
        .evaluate_expired("probation-1", 604800001)
        .expect("healthy expiry");
    assert_eq!(healthy.evaluation, ProbationEvaluation::Healthy);

    let connection = database.connection().expect("connection");
    connection
        .execute(
            "UPDATE evolution_auto_probations SET status='active',revision=0",
            [],
        )
        .expect("reactivate fixture");
    drop(connection);
    repository
        .record_observation(&observation("security-one", "run-one", "security"), 10)
        .expect("security one");
    let outcome = repository
        .record_observation(&observation("security-two", "run-two", "security"), 10)
        .expect("security regression");
    assert!(outcome.security_escalated);
    let connection = database.connection().expect("connection");
    let workspace_open: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM evolution_auto_breakers WHERE workspace_id='workspace-1'
         AND skill_id IS NULL AND status='open' AND safe_cause_code='security_regression')",
            [],
            |row| row.get(0),
        )
        .expect("workspace breaker");
    assert!(workspace_open);
}
