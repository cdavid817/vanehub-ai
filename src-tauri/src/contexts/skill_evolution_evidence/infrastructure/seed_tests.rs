use rusqlite::OptionalExtension;
use serde_json::{json, Value};

use super::*;
use crate::contexts::skill_evolution_evidence::domain::*;
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

const SANITIZER_KEY: &[u8; 32] = &[7; 32];
const FINGERPRINT_KEY: &[u8; 32] = &[9; 32];

fn common(event: &str, occurred_at: &str, run: &str, attempt: &str, workspace: &str) -> Value {
    json!({
        "sourceEventId": event,
        "occurredAt": occurred_at,
        "stableAgentId": "onepiece",
        "sessionId": "session-1",
        "messageId": "message-1",
        "runId": run,
        "attemptId": attempt,
        "workspace": workspace,
        "fidelity": "native",
        "observedSkillRevisions": [{
            "skillId": "review",
            "revision": "rev-a",
            "associationKind": "injected",
            "observedAt": occurred_at
        }]
    })
}

fn extract(value: Value) -> Vec<SignalDraft> {
    let envelope: EvidenceSourceEnvelope = serde_json::from_value(value).expect("envelope");
    envelope.validate().expect("valid envelope");
    let sanitizer = EvidenceSanitizer::new(SANITIZER_KEY).expect("sanitizer");
    let sanitized = envelope
        .sanitized_registered_text(&sanitizer)
        .expect("sanitize");
    extract_registered_signals(&envelope, sanitized.as_ref())
}

fn failure(event: &str, time: &str, run: &str, attempt: &str, workspace: &str) -> SignalDraft {
    extract(json!({
        "sourceKind": "native_execution",
        "schemaVersion": 1,
        "common": common(event, time, run, attempt, workspace),
        "operationClass": "tool",
        "outcome": "failed",
        "failureClass": "sandbox",
        "safeCounts": { "attempts": 1, "failures": 1 }
    }))
    .remove(0)
}

fn failure_with_revision(
    event: &str,
    time: &str,
    run: &str,
    attempt: &str,
    revision: &str,
) -> SignalDraft {
    let mut common = common(event, time, run, attempt, "workspace-a");
    common["observedSkillRevisions"][0]["revision"] = json!(revision);
    extract(json!({
        "sourceKind": "native_execution",
        "schemaVersion": 1,
        "common": common,
        "operationClass": "tool",
        "outcome": "failed",
        "failureClass": "sandbox",
        "safeCounts": { "attempts": 1, "failures": 1 }
    }))
    .remove(0)
}

fn correlated_failure(event: &str, time: &str, run: &str) -> SignalDraft {
    let mut common = common(event, time, run, "cli-attempt", "workspace-a");
    common["fidelity"] = json!("proxied");
    common["observedSkillRevisions"] = json!([]);
    extract(json!({
        "sourceKind": "managed_cli",
        "schemaVersion": 1,
        "common": common,
        "outcome": "failed",
        "failureClass": "process",
        "mountSnapshot": {
            "manifestHash": "manifest-a",
            "skills": [{ "skillId": "review", "revision": "rev-a" }]
        },
        "configuredBindingIds": ["binding-a"]
    }))
    .remove(0)
}

fn corrected(event: &str, time: &str, workspace: &str) -> SignalDraft {
    extract(json!({
        "sourceKind": "explicit_feedback",
        "schemaVersion": 1,
        "common": common(event, time, "run-feedback", "attempt-feedback", workspace),
        "feedback": "corrected",
        "feedbackRevision": 1,
        "correctionNote": "Use alice@example.com instead."
    }))
    .remove(0)
}

fn recovery(event: &str, time: &str, predecessor: &str) -> SignalDraft {
    extract(json!({
        "sourceKind": "run_verification",
        "schemaVersion": 1,
        "common": common(event, time, "run-recovery", "attempt-recovery", "workspace-a"),
        "runId": "run-1",
        "verifier": "test",
        "outcome": "passed",
        "passedCount": 3,
        "failedCount": 0,
        "predecessorAttemptId": predecessor
    }))
    .into_iter()
    .find(|signal| signal.category() == SignalCategory::RetryRecovery)
    .expect("recovery signal")
}

fn fixture(name: &str) -> (NativeDatabase, SqliteEvolutionEvidenceRepository) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    (
        database.clone(),
        SqliteEvolutionEvidenceRepository::new(database),
    )
}

fn fingerprints() -> TaskFingerprintBuilder {
    TaskFingerprintBuilder::new(FINGERPRINT_KEY).expect("fingerprints")
}

fn persist(repository: &SqliteEvolutionEvidenceRepository, signal: &SignalDraft) -> String {
    repository
        .persist_signal(signal, &fingerprints())
        .expect("persist")
        .signal_id()
        .to_string()
}

#[test]
fn fingerprints_are_versioned_installation_and_workspace_scoped() {
    let first = failure(
        "event-1",
        "2026-08-01T00:00:00Z",
        "run-1",
        "attempt-1",
        "workspace-a",
    );
    let same_shape = failure(
        "event-2",
        "2026-08-02T00:00:00Z",
        "run-2",
        "attempt-2",
        "workspace-a",
    );
    let other_workspace = failure(
        "event-3",
        "2026-08-02T00:00:00Z",
        "run-3",
        "attempt-3",
        "workspace-b",
    );
    let builder = fingerprints();
    let a = builder.build(&first).expect("fingerprint");
    assert_eq!(a, builder.build(&same_shape).expect("same fingerprint"));
    assert_ne!(a, builder.build(&other_workspace).expect("other workspace"));
    assert_ne!(
        a,
        TaskFingerprintBuilder::new(&[3; 32])
            .expect("other key")
            .build(&first)
            .expect("other installation")
    );
    assert_eq!(a.version(), TASK_FINGERPRINT_V1);
}

#[test]
fn two_independent_failures_build_one_reproducible_ready_seed() {
    let (database, repository) = fixture("seed-independent-failures");
    let first_id = persist(
        &repository,
        &failure(
            "event-1",
            "2026-08-01T00:00:00Z",
            "run-1",
            "attempt-1",
            "workspace-a",
        ),
    );
    let second_id = persist(
        &repository,
        &failure(
            "event-2",
            "2026-08-05T00:00:00Z",
            "run-2",
            "attempt-2",
            "workspace-a",
        ),
    );

    let first = repository.rebuild_dirty_seeds().expect("rebuild");
    assert_eq!(first.seed_count, 1);
    let connection = database.connection().expect("connection");
    let snapshot: (String, String, String, i64, String) = connection
        .query_row(
            "SELECT seed_id, readiness, readiness_reason, independent_run_count, lineage_summary_json FROM evolution_candidate_seeds",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .expect("seed");
    assert_eq!(snapshot.1, "ready");
    assert_eq!(snapshot.2, "independent_negative_runs");
    assert_eq!(snapshot.3, 2);
    assert!(snapshot.4.contains("signalIds"));
    assert!(snapshot.4.contains("extractorVersions"));
    let lineage: Vec<String> = connection
        .prepare("SELECT signal_id FROM evolution_candidate_seed_signals ORDER BY lineage_order")
        .expect("lineage query")
        .query_map([], |row| row.get(0))
        .expect("lineage rows")
        .collect::<Result<_, _>>()
        .expect("lineage values");
    assert_eq!(lineage, vec![first_id, second_id]);
    drop(connection);

    database
        .connection()
        .expect("connection")
        .execute("UPDATE evolution_pipeline_state SET state_value = 'dirty' WHERE state_key = 'signal_set'", [])
        .expect("force reproducibility rebuild");
    let second = repository.rebuild_dirty_seeds().expect("stable rerun");
    assert!(second.rebuilt);
    let connection = database.connection().expect("connection");
    let same_id: String = connection
        .query_row("SELECT seed_id FROM evolution_candidate_seeds", [], |row| {
            row.get(0)
        })
        .expect("same seed");
    assert_eq!(snapshot.0, same_id);
}

#[test]
fn workspace_revision_cohort_and_fourteen_day_window_prevent_false_grouping() {
    let (database, repository) = fixture("seed-boundaries");
    persist(
        &repository,
        &failure(
            "event-1",
            "2026-08-01T00:00:00Z",
            "run-1",
            "attempt-1",
            "workspace-a",
        ),
    );
    persist(
        &repository,
        &failure_with_revision(
            "event-4",
            "2026-08-03T00:00:00Z",
            "run-4",
            "attempt-4",
            "rev-b",
        ),
    );
    persist(
        &repository,
        &failure(
            "event-2",
            "2026-08-16T00:00:01Z",
            "run-2",
            "attempt-2",
            "workspace-a",
        ),
    );
    persist(
        &repository,
        &failure(
            "event-3",
            "2026-08-02T00:00:00Z",
            "run-3",
            "attempt-3",
            "workspace-b",
        ),
    );

    let result = repository.rebuild_dirty_seeds().expect("rebuild");
    assert_eq!(result.seed_count, 0);
    assert_eq!(
        database
            .connection()
            .expect("connection")
            .query_row(
                "SELECT COUNT(*) FROM evolution_candidate_seeds",
                [],
                |row| row.get::<_, i64>(0)
            )
            .expect("count"),
        0
    );
}

#[test]
fn repeated_correlated_cli_failures_are_human_review_only() {
    let (database, repository) = fixture("seed-correlated-cli");
    persist(
        &repository,
        &correlated_failure("cli-1", "2026-08-01T00:00:00Z", "cli-run-1"),
    );
    persist(
        &repository,
        &correlated_failure("cli-2", "2026-08-02T00:00:00Z", "cli-run-2"),
    );
    repository.rebuild_dirty_seeds().expect("rebuild");
    let connection = database.connection().expect("connection");
    let (readiness, reason): (String, String) = connection
        .query_row(
            "SELECT readiness, readiness_reason FROM evolution_candidate_seeds",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("correlated seed");
    assert_eq!(readiness, "human_review_only");
    assert_eq!(reason, "correlated_evidence_only");
}

#[test]
fn verified_correction_builds_a_single_source_seed_without_raw_note() {
    let (database, repository) = fixture("seed-correction");
    persist(
        &repository,
        &corrected("feedback-1", "2026-08-03T00:00:00Z", "workspace-a"),
    );
    repository.rebuild_dirty_seeds().expect("rebuild");
    let connection = database.connection().expect("connection");
    let (readiness, reason, lineage): (String, String, String) = connection
        .query_row("SELECT readiness, readiness_reason, lineage_summary_json FROM evolution_candidate_seeds", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("seed");
    assert_eq!(readiness, "ready");
    assert_eq!(reason, "verified_corrected_feedback");
    assert!(!lineage.contains("alice@example.com"));
}

#[test]
fn recovery_attaches_to_predecessor_and_supersession_rebuilds_lineage() {
    let (database, repository) = fixture("seed-recovery-rebuild");
    let negative_id = persist(
        &repository,
        &failure(
            "event-1",
            "2026-08-01T00:00:00Z",
            "run-1",
            "attempt-1",
            "workspace-a",
        ),
    );
    let recovery_id = persist(
        &repository,
        &recovery("event-2", "2026-08-02T00:00:00Z", "attempt-1"),
    );
    repository.rebuild_dirty_seeds().expect("recovery rebuild");
    let connection = database.connection().expect("connection");
    let (has_recovery, lineage_count): (i64, i64) = connection
        .query_row("SELECT has_recovery, (SELECT COUNT(*) FROM evolution_candidate_seed_signals) FROM evolution_candidate_seeds", [], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("recovery seed");
    assert_eq!((has_recovery, lineage_count), (1, 2));
    drop(connection);

    assert!(repository
        .supersede_signal(&recovery_id, None)
        .expect("supersede"));
    let rebuilt = repository.rebuild_dirty_seeds().expect("dirty rebuild");
    assert_eq!(rebuilt.seed_count, 0);
    let connection = database.connection().expect("connection");
    assert!(connection
        .query_row("SELECT seed_id FROM evolution_candidate_seeds", [], |row| {
            row.get::<_, String>(0)
        })
        .optional()
        .expect("query")
        .is_none());
    assert_eq!(
        connection
            .query_row(
                "SELECT lineage_status FROM evolution_signals WHERE signal_id = ?1",
                [&recovery_id],
                |row| row.get::<_, String>(0)
            )
            .expect("status"),
        "superseded"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT lineage_status FROM evolution_signals WHERE signal_id = ?1",
                [&negative_id],
                |row| row.get::<_, String>(0)
            )
            .expect("negative"),
        "active"
    );
}
