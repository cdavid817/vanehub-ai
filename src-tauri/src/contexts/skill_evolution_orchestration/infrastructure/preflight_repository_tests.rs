use crate::{
    contexts::skill_evolution_orchestration::domain::{
        evaluate_automatic_preflight, AutomaticPreflightInputV1, AutomaticPreflightWitnessV1,
    },
    platform::database::NativeDatabase,
    test_support::TempDirectory,
};

use super::{PreflightRepositoryError, SqlitePreflightRepository};

#[test]
fn witness_is_issued_idempotently_and_consumed_exactly_once() {
    let (_database, repository, _directory) = fixture("preflight-consume");
    let witness = witness(100);
    assert_eq!(repository.issue(&witness), Ok(true));
    assert_eq!(repository.issue(&witness), Ok(false));
    assert_eq!(
        repository.consume(
            &witness.witness_id,
            &witness.proof_hash,
            &witness.overlay_preview_hash,
            101,
        ),
        Ok(AutomaticPreflightWitnessV1 {
            revision: 1,
            ..witness.clone()
        })
    );
    assert_eq!(
        repository.consume(
            &witness.witness_id,
            &witness.proof_hash,
            &witness.overlay_preview_hash,
            102,
        ),
        Err(PreflightRepositoryError::AlreadyConsumed)
    );
    assert_eq!(
        repository.recover_consumed(
            &witness.witness_id,
            &witness.proof_hash,
            &witness.overlay_preview_hash,
        ),
        Ok(AutomaticPreflightWitnessV1 {
            revision: 1,
            ..witness
        })
    );
}

#[test]
fn changed_preview_and_five_second_boundary_fail_closed() {
    let (_database, repository, _directory) = fixture("preflight-stale");
    let witness = witness(100);
    repository.issue(&witness).expect("issue");
    assert_eq!(
        repository.consume(
            &witness.witness_id,
            &witness.proof_hash,
            &format!("sha256:{}", "c".repeat(64)),
            101,
        ),
        Err(PreflightRepositoryError::Conflict)
    );
    assert_eq!(
        repository.consume(
            &witness.witness_id,
            &witness.proof_hash,
            &witness.overlay_preview_hash,
            witness.expires_at_ms,
        ),
        Err(PreflightRepositoryError::Expired)
    );
}

#[test]
fn issue_rechecks_eligible_proof_and_reserved_rate_state_in_one_transaction() {
    let (database, repository, _directory) = fixture("preflight-source-race");
    let witness = witness(100);
    let connection = database.connection().expect("connection");
    connection
        .execute(
            "UPDATE evolution_auto_rate_reservations SET status='released'",
            [],
        )
        .expect("release reservation");
    assert_eq!(
        repository.issue(&witness),
        Err(PreflightRepositoryError::Conflict)
    );
}

fn fixture(name: &str) -> (NativeDatabase, SqlitePreflightRepository, TempDirectory) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let connection = database.connection().expect("connection");
    connection.execute("INSERT INTO evolution_run_requests VALUES ('request-one',1,'workspace:one','runtime_trigger','completed','{}',0,0,'run-one',0,1,1)", []).expect("request");
    connection.execute("INSERT INTO evolution_runs VALUES ('run-one',1,'request-one','workspace:one','completed',NULL,'policy','{}','{}',NULL,NULL,NULL,NULL,0,1,1)", []).expect("run");
    connection.execute("INSERT INTO evolution_correction_authorizations VALUES ('authorization-one','feedback-one',1,'disclosure-v1',1,'interactive_user','sha256:authorization',1,NULL)", []).expect("authorization");
    connection.execute("INSERT INTO evolution_deterministic_drafts VALUES ('draft-one','workspace:one','skill-one','authorization-one','assessment-one','producer-v1','sha256:content',7,'deterministic_authorized_correction','sha256:source',1)", []).expect("draft");
    connection.execute("INSERT INTO evolution_auto_eligibility VALUES ('eligibility-one','run-one','draft-one','skill-one','eligible','[]',?1,NULL,1,0)", [format!("sha256:{}", "a".repeat(64))]).expect("eligibility");
    connection.execute("INSERT INTO evolution_auto_rate_reservations VALUES ('reservation-one','run-one','workspace:one','skill-one','reserved',NULL,1,1,0)", []).expect("reservation");
    drop(connection);
    (
        database.clone(),
        SqlitePreflightRepository::new(database),
        directory,
    )
}

fn witness(issued_at_ms: i64) -> AutomaticPreflightWitnessV1 {
    evaluate_automatic_preflight(&AutomaticPreflightInputV1 {
        run_id: "run-one".into(),
        eligibility_id: "eligibility-one".into(),
        eligibility_proof_hash: format!("sha256:{}", "a".repeat(64)),
        reservation_id: "reservation-one".into(),
        overlay_preview_hash: format!("sha256:{}", "b".repeat(64)),
        automatic_mode_enabled: true,
        policy_current: true,
        consent_current: true,
        authorization_current: true,
        allowlist_current: true,
        assessment_current: true,
        draft_current: true,
        target_current: true,
        skill_mutable: true,
        overlay_revision_current: true,
        overlay_trusted: true,
        overlay_unpinned: true,
        quality_current: true,
        rate_reserved: true,
        idle_snapshot_fresh: true,
        probation_clear: true,
        circuit_breakers_closed: true,
        issued_at_ms,
    })
    .expect("preflight")
}
