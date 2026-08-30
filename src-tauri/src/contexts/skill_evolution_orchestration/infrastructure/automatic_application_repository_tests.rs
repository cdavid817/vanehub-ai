use super::*;
use crate::contexts::skill_evolution_orchestration::domain::*;
use crate::platform::database::NativeDatabase;

fn seeded_database(directory: &tempfile::TempDir) -> NativeDatabase {
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let connection = database.connection().expect("connection");
    connection
        .execute_batch(
            "PRAGMA foreign_keys=OFF;
             INSERT INTO evolution_runs
             (run_id,schema_version,request_id,workspace_id,status,current_stage,
              policy_witness_hash,budget_json,usage_json,revision,created_at_ms,updated_at_ms)
             VALUES ('run-1',1,'request-1','workspace-1','running','evaluate_auto_apply',
                     'policy-proof','{}','{}',0,1,1);
             INSERT INTO evolution_auto_eligibility
             VALUES ('eligibility-1','run-1','draft-1','skill-1','eligible','[]',
                     'eligibility-proof','preview-proof',2,0);
             INSERT INTO evolution_auto_rate_reservations
             VALUES ('reservation-1','run-1','workspace-1','skill-1','reserved',NULL,2,2,0);
             INSERT INTO evolution_auto_preflight_witnesses
             VALUES ('preflight-1','run-1','eligibility-1','eligibility-proof','reservation-1',
                     'preview-proof','preflight-proof',3,5003,4,'consumed',1);
             INSERT INTO evolution_run_items
             VALUES ('item-1','run-1','evaluate_auto_apply','item-key','eligibility-1',0,NULL,NULL,2);
             INSERT INTO evolution_curator_applications
             (application_id,candidate_id,decision_id,status,approved_witness_hash,
              overlay_revision,overlay_history_id,revision,created_at_ms,updated_at_ms)
             VALUES ('auto-app-1','candidate-1','decision-1','applied','preview-proof',
                     '2','overlay-history-1',2,4,5);
             INSERT INTO evolution_curator_system_policy_authorizations
             VALUES ('auto-app-1','run-1','eligibility-1','eligibility-proof','preflight-proof',
                     'policy-proof','reservation-1','system_policy',4);
             PRAGMA foreign_keys=ON;",
        )
        .expect("seed provenance chain");
    drop(connection);
    database
}

fn application() -> AutomaticEvolutionApplicationV1 {
    AutomaticEvolutionApplicationV1 {
        application_id: "auto-app-1".into(),
        run_id: "run-1".into(),
        eligibility_id: "eligibility-1".into(),
        preflight_witness_hash: "preflight-proof".into(),
        policy_witness_hash: "policy-proof".into(),
        rate_reservation_id: "reservation-1".into(),
        curator_application_id: "auto-app-1".into(),
        overlay_application_id: "auto-app-1".into(),
        target_skill_id: "skill-1".into(),
        prior_effective_hash: "prior-hash".into(),
        resulting_effective_hash: "result-hash".into(),
        actor: EvolutionActorProvenance::SystemPolicy,
        committed_at_ms: 5,
    }
}

fn probation() -> AutoApplyProbationV1 {
    AutoApplyProbationV1 {
        probation_id: "probation-auto-app-1".into(),
        application_id: "auto-app-1".into(),
        workspace_id: "workspace-1".into(),
        skill_id: "skill-1".into(),
        status: ProbationStatus::Active,
        prior_effective_hash: "prior-hash".into(),
        current_effective_hash: "result-hash".into(),
        evidence_fingerprint: "fingerprint-1".into(),
        evidence_categories: vec!["verified_correction".into()],
        baseline_witness_hash: "baseline-proof".into(),
        starts_at_ms: 5,
        ends_at_ms: 5 + ROLLING_WEEK_MS,
        revision: 0,
    }
}

#[test]
fn finalization_commits_application_rate_run_result_and_probation_once() {
    let directory = tempfile::tempdir().expect("tempdir");
    let database = seeded_database(&directory);
    let repository = SqliteAutomaticApplicationRepository::new(database.clone());
    assert_eq!(
        repository.finalize(&application(), &probation(), "item-1", 0),
        Ok(true)
    );
    assert_eq!(
        repository.finalize(&application(), &probation(), "item-1", 0),
        Ok(false)
    );
    let mut changed_probation = probation();
    changed_probation.evidence_categories = vec!["different_category".into()];
    assert_eq!(
        repository.finalize(&application(), &changed_probation, "item-1", 0),
        Err(AutomaticApplicationStoreError::Conflict)
    );
    let connection = database.connection().expect("connection");
    let state: (String, String, String, i64) = connection
        .query_row(
            "SELECT r.status,r.application_id,i.committed_receipt_id,
                    (SELECT COUNT(*) FROM evolution_auto_probations)
             FROM evolution_auto_rate_reservations r JOIN evolution_run_items i
             ON i.item_id='item-1' WHERE r.reservation_id='reservation-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("final state");
    assert_eq!(
        state,
        (
            "committed".into(),
            "auto-app-1".into(),
            "auto-app-1".into(),
            1
        )
    );
}

#[test]
fn finalization_failure_rolls_back_every_local_result() {
    let directory = tempfile::tempdir().expect("tempdir");
    let database = seeded_database(&directory);
    let connection = database.connection().expect("connection");
    connection
        .execute_batch(
            "CREATE TRIGGER fail_probation BEFORE INSERT ON evolution_auto_probations
             BEGIN SELECT RAISE(ABORT, 'probation failed'); END;",
        )
        .expect("failure trigger");
    drop(connection);
    let repository = SqliteAutomaticApplicationRepository::new(database.clone());
    assert_eq!(
        repository.finalize(&application(), &probation(), "item-1", 0),
        Err(AutomaticApplicationStoreError::Storage)
    );
    let connection = database.connection().expect("connection");
    let state: (i64, String, Option<String>) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM evolution_auto_applications),r.status,
                    i.committed_receipt_id FROM evolution_auto_rate_reservations r
             JOIN evolution_run_items i ON i.item_id='item-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("rolled back state");
    assert_eq!(state, (0, "reserved".into(), None));
}
