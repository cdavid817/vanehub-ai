use crate::{
    contexts::skill_evolution_orchestration::domain::{
        EvolutionPolicyMode, EvolutionPolicyMutationV1,
    },
    platform::database::NativeDatabase,
    test_support::TempDirectory,
};
use std::{sync::Barrier, thread};

use super::{OrchestrationPersistenceError, OrchestrationRepository};

#[test]
fn absent_policy_is_default_off_and_enabled_writes_are_durable() {
    let (repository, _, _directory) = repository("policy-default");
    assert_eq!(
        repository
            .policy("workspace-one", 10)
            .expect("default")
            .mode,
        EvolutionPolicyMode::Off
    );
    let written = repository
        .update_policy(
            "workspace-one",
            mutation(0, EvolutionPolicyMode::Enabled, &["skill-one"], true, 20),
        )
        .expect("enabled policy");
    assert_eq!(written.policy.revision, 1);
    assert_eq!(
        repository.policy("workspace-one", 30).expect("persisted"),
        written.policy
    );
}

#[test]
fn concurrent_policy_writers_have_one_cas_winner() {
    let (repository, _, _directory) = repository("policy-cas");
    repository
        .update_policy(
            "workspace-one",
            mutation(0, EvolutionPolicyMode::Observe, &[], false, 10),
        )
        .expect("initial policy");
    let barrier = std::sync::Arc::new(Barrier::new(3));
    let handles: Vec<_> = [20, 21]
        .into_iter()
        .map(|updated_at_ms| {
            let repository = repository.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                repository.update_policy(
                    "workspace-one",
                    mutation(1, EvolutionPolicyMode::Observe, &[], false, updated_at_ms),
                )
            })
        })
        .collect();
    barrier.wait();
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("writer"))
        .collect();
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Err(OrchestrationPersistenceError::Conflict)))
            .count(),
        1
    );
}

#[test]
fn allowlist_removal_invalidates_only_affected_workspace_eligibility() {
    let (repository, database, _directory) = repository("policy-invalidation");
    repository
        .update_policy(
            "workspace-one",
            mutation(
                0,
                EvolutionPolicyMode::Enabled,
                &["skill-one", "skill-two"],
                true,
                10,
            ),
        )
        .expect("initial policy");
    seed_eligibility(&database, "one", "workspace-one", "skill-one");
    seed_eligibility(&database, "two", "workspace-one", "skill-two");
    seed_eligibility(&database, "other", "workspace-two", "skill-one");
    let result = repository
        .update_policy(
            "workspace-one",
            mutation(1, EvolutionPolicyMode::Enabled, &["skill-two"], false, 20),
        )
        .expect("remove allowlist item");
    assert_eq!(result.invalidated_eligibility, 1);
    let connection = database.connection().expect("connection");
    assert_eq!(
        eligibility_result(&connection, "eligibility-one"),
        "ineligible"
    );
    assert_eq!(
        eligibility_result(&connection, "eligibility-two"),
        "eligible"
    );
    assert_eq!(
        eligibility_result(&connection, "eligibility-other"),
        "eligible"
    );
}

#[test]
fn revocation_is_persisted_and_import_strips_local_consent() {
    let (repository, _, _directory) = repository("policy-consent");
    let source = repository
        .update_policy(
            "workspace-one",
            mutation(0, EvolutionPolicyMode::Enabled, &["skill-one"], true, 10),
        )
        .expect("source")
        .policy;
    let imported = repository
        .import_policy(&source, "workspace-two", 20)
        .expect("imported")
        .policy;
    assert_eq!(imported.mode, EvolutionPolicyMode::Observe);
    assert!(imported.consent.is_none());
    let revoked = repository
        .revoke_policy_consent("workspace-one", 1, 30)
        .expect("revoked")
        .policy;
    assert_eq!(revoked.mode, EvolutionPolicyMode::Off);
    assert_eq!(
        revoked.consent.and_then(|consent| consent.revoked_at_ms),
        Some(30)
    );
}

#[test]
fn unsupported_or_tampered_consent_fields_fail_closed() {
    let (repository, database, _directory) = repository("policy-corrupt");
    repository
        .update_policy(
            "workspace-one",
            mutation(0, EvolutionPolicyMode::Enabled, &["skill-one"], true, 10),
        )
        .expect("policy");
    let connection = database.connection().expect("connection");
    let consent: String = connection
        .query_row(
            "SELECT consent_json FROM evolution_orchestration_policy",
            [],
            |row| row.get(0),
        )
        .expect("consent");
    let mut value: serde_json::Value = serde_json::from_str(&consent).expect("json");
    value["unsupportedField"] = serde_json::json!(true);
    connection
        .execute(
            "UPDATE evolution_orchestration_policy SET consent_json=?1",
            [serde_json::to_string(&value).expect("serialize")],
        )
        .expect("corrupt fixture");
    assert_eq!(
        repository.policy("workspace-one", 20),
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

fn mutation(
    expected_revision: u64,
    mode: EvolutionPolicyMode,
    allowed: &[&str],
    acknowledge: bool,
    updated_at_ms: i64,
) -> EvolutionPolicyMutationV1 {
    EvolutionPolicyMutationV1 {
        expected_revision,
        mode,
        allowed_skill_ids: allowed.iter().map(|value| (*value).into()).collect(),
        acknowledge_current_disclosure: acknowledge,
        notify_routine_completion: false,
        updated_at_ms,
    }
}

fn seed_eligibility(database: &NativeDatabase, suffix: &str, workspace: &str, skill: &str) {
    let connection = database.connection().expect("connection");
    connection.execute("INSERT INTO evolution_run_requests VALUES (?1,1,?2,'runtime_trigger','completed','{}',0,0,?3,0,1,1)", rusqlite::params![format!("request-{suffix}"), workspace, format!("run-{suffix}")]).expect("request");
    connection.execute("INSERT INTO evolution_runs VALUES (?1,1,?2,?3,'completed',NULL,'policy','{}','{}',NULL,NULL,NULL,NULL,0,1,1)", rusqlite::params![format!("run-{suffix}"), format!("request-{suffix}"), workspace]).expect("run");
    connection.execute("INSERT INTO evolution_correction_authorizations VALUES (?1,?2,0,'disclosure',1,'interactive_user','hash',1,NULL)", rusqlite::params![format!("authorization-{suffix}"), format!("feedback-{suffix}")]).expect("authorization");
    connection.execute("INSERT INTO evolution_deterministic_drafts VALUES (?1,?2,?3,?4,?5,'v1','content',7,'deterministic_authorized_correction','source',1)", rusqlite::params![format!("draft-{suffix}"), workspace, skill, format!("authorization-{suffix}"), format!("assessment-{suffix}")]).expect("draft");
    connection.execute("INSERT INTO evolution_auto_eligibility VALUES (?1,?2,?3,?4,'eligible','[]','proof','preview',1,0)", rusqlite::params![format!("eligibility-{suffix}"), format!("run-{suffix}"), format!("draft-{suffix}"), skill]).expect("eligibility");
}

fn eligibility_result(connection: &rusqlite::Connection, eligibility_id: &str) -> String {
    connection
        .query_row(
            "SELECT result FROM evolution_auto_eligibility WHERE eligibility_id=?1",
            [eligibility_id],
            |row| row.get(0),
        )
        .expect("eligibility result")
}
