use crate::{
    contexts::skill_evolution_orchestration::domain::{
        canonical_hash, AutoApplyEligibilityV1, AutoEligibilityResult, EligibilityPredicateV1,
    },
    platform::database::NativeDatabase,
    test_support::TempDirectory,
};

use super::{EligibilityRepositoryError, PersistEligibilityOutcome, SqliteEligibilityRepository};

#[test]
fn eligibility_proof_is_inserted_idempotently_and_updated_with_cas() {
    let (repository, _directory) = fixture("eligibility-cas");
    let initial = proof(0, "sha256:initial", AutoEligibilityResult::Eligible);
    assert_eq!(
        repository.persist(&initial, None),
        Ok(PersistEligibilityOutcome::Inserted)
    );
    assert_eq!(
        repository.persist(&initial, None),
        Ok(PersistEligibilityOutcome::Duplicate)
    );

    let updated = proof(1, "sha256:updated", AutoEligibilityResult::Waiting);
    assert_eq!(
        repository.persist(&updated, Some(0)),
        Ok(PersistEligibilityOutcome::Updated)
    );
    let stale = proof(2, "sha256:stale", AutoEligibilityResult::Eligible);
    assert_eq!(
        repository.persist(&stale, Some(0)),
        Err(EligibilityRepositoryError::Conflict)
    );
}

#[test]
fn incomplete_predicate_proofs_fail_before_storage() {
    let (repository, _directory) = fixture("eligibility-invalid");
    let mut invalid = proof(0, "sha256:invalid", AutoEligibilityResult::Eligible);
    invalid.predicates.pop();
    assert_eq!(
        repository.persist(&invalid, None),
        Err(EligibilityRepositoryError::InvalidInput)
    );
}

fn fixture(name: &str) -> (SqliteEligibilityRepository, TempDirectory) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let connection = database.connection().expect("connection");
    connection.execute("INSERT INTO evolution_run_requests VALUES ('request-one',1,'workspace:one','runtime_trigger','completed','{}',0,0,'run-one',0,1,1)", []).expect("request");
    connection.execute("INSERT INTO evolution_runs VALUES ('run-one',1,'request-one','workspace:one','completed',NULL,'policy','{}','{}',NULL,NULL,NULL,NULL,0,1,1)", []).expect("run");
    connection.execute("INSERT INTO evolution_correction_authorizations VALUES ('authorization-one','feedback-one',1,'disclosure-v1',1,'interactive_user','sha256:authorization',1,NULL)", []).expect("authorization");
    connection.execute("INSERT INTO evolution_deterministic_drafts VALUES ('draft-one','workspace:one','skill-one','authorization-one','assessment-one','producer-v1','sha256:content',7,'deterministic_authorized_correction','sha256:source',1)", []).expect("draft");
    drop(connection);
    (SqliteEligibilityRepository::new(database), directory)
}

fn proof(revision: u64, proof_hash: &str, result: AutoEligibilityResult) -> AutoApplyEligibilityV1 {
    let predicates = (0..26)
        .map(|index| EligibilityPredicateV1 {
            condition: format!("condition-{index}"),
            passed: true,
            safe_reason_code: None,
            witness_hash: Some(canonical_hash(&(index, true)).expect("hash")),
        })
        .collect();
    AutoApplyEligibilityV1 {
        eligibility_id: "eligibility-one".into(),
        run_id: "run-one".into(),
        draft_id: "draft-one".into(),
        target_skill_id: "skill-one".into(),
        result,
        predicates,
        proof_hash: proof_hash.into(),
        overlay_preview_hash: None,
        evaluated_at_ms: 10 + i64::try_from(revision).expect("revision"),
        revision,
    }
}
