use chrono::{TimeZone, Utc};
use serde_json::{json, Value};

use super::*;
use crate::contexts::skill_evolution_evidence::domain::{
    extract_registered_signals, EvidenceSanitizer, EvidenceSourceEnvelope, SignalDraft,
    TaskFingerprintBuilder,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

const KEY: &[u8; 32] = &[13; 32];

fn fixture(
    name: &str,
) -> (
    TempDirectory,
    NativeDatabase,
    SqliteEvolutionEvidenceRepository,
) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteEvolutionEvidenceRepository::new(database.clone());
    (directory, database, repository)
}

fn common(event: &str, time: &str, workspace: &str, run: &str) -> Value {
    json!({
        "sourceEventId": event,
        "occurredAt": time,
        "stableAgentId": "onepiece",
        "sessionId": "conversation-1",
        "messageId": format!("message-{event}"),
        "runId": run,
        "attemptId": format!("attempt-{event}"),
        "workspace": workspace,
        "fidelity": "native",
        "observedSkillRevisions": [{
            "skillId": "review",
            "revision": "rev-a",
            "associationKind": "injected",
            "observedAt": time
        }]
    })
}

fn extract(value: Value) -> SignalDraft {
    let envelope: EvidenceSourceEnvelope = serde_json::from_value(value).expect("envelope");
    envelope.validate().expect("valid envelope");
    let sanitizer = EvidenceSanitizer::new(KEY).expect("sanitizer");
    let sanitized = envelope
        .sanitized_registered_text(&sanitizer)
        .expect("sanitize");
    extract_registered_signals(&envelope, sanitized.as_ref()).remove(0)
}

fn failure(event: &str, time: &str, workspace: &str, run: &str) -> SignalDraft {
    extract(json!({
        "sourceKind": "native_execution",
        "schemaVersion": 1,
        "common": common(event, time, workspace, run),
        "operationClass": "tool",
        "outcome": "failed",
        "failureClass": "sandbox",
        "safeCounts": { "attempts": 1, "failures": 1 }
    }))
}

fn feedback(event: &str, time: &str, workspace: &str, corrected: bool) -> SignalDraft {
    extract(json!({
        "sourceKind": "explicit_feedback",
        "schemaVersion": 1,
        "common": common(event, time, workspace, &format!("run-{event}")),
        "feedback": if corrected { "corrected" } else { "helpful" },
        "feedbackRevision": 1,
        "correctionNote": corrected.then_some("Use the bounded retry path.")
    }))
}

fn persist(repository: &SqliteEvolutionEvidenceRepository, signal: &SignalDraft) -> String {
    repository
        .persist_signal(
            signal,
            &TaskFingerprintBuilder::new(KEY).expect("fingerprints"),
        )
        .expect("persist")
        .signal_id()
        .to_string()
}

fn signal_events(database: &NativeDatabase) -> Vec<String> {
    let connection = database.connection().expect("connection");
    let mut statement = connection
        .prepare("SELECT source_event_id FROM evolution_signals ORDER BY source_event_id")
        .expect("query");
    statement
        .query_map([], |row| row.get(0))
        .expect("rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("events")
}

#[test]
fn retention_and_workspace_quota_expire_old_then_preserve_high_value_evidence() {
    let (_directory, database, repository) = fixture("evidence-retention-quota");
    persist(
        &repository,
        &failure("expired", "2026-01-01T00:00:00Z", "a", "run-old"),
    );
    persist(
        &repository,
        &failure("failure", "2026-08-01T00:00:00Z", "a", "run-1"),
    );
    persist(
        &repository,
        &feedback("helpful", "2026-08-02T00:00:00Z", "a", false),
    );
    persist(
        &repository,
        &feedback("corrected", "2026-08-03T00:00:00Z", "a", true),
    );
    let outcome = repository
        .maintain(
            Utc.with_ymd_and_hms(2026, 8, 13, 0, 0, 0)
                .single()
                .expect("time"),
            EvidenceGovernancePolicy {
                signals_per_workspace: 2,
                ..EvidenceGovernancePolicy::default()
            },
        )
        .expect("maintenance");

    assert_eq!(outcome.expired_signals, 1);
    assert_eq!(outcome.quota_evicted_signals, 1);
    assert_eq!(signal_events(&database), vec!["corrected", "failure"]);
}

#[test]
fn scoped_and_full_purge_remove_only_evidence_rows() {
    let (_directory, database, repository) = fixture("evidence-scoped-purge");
    persist(
        &repository,
        &failure("a", "2026-08-01T00:00:00Z", "a", "run-a"),
    );
    persist(
        &repository,
        &failure("b", "2026-08-01T00:00:00Z", "b", "run-b"),
    );
    let outcome = repository
        .purge(&EvidencePurgeRequest {
            operation_id: "purge-workspace-a".to_string(),
            scope: EvidencePurgeScope::Workspace("a".to_string()),
        })
        .expect("workspace purge");
    assert_eq!(outcome.deleted_signals, 1);
    assert_eq!(signal_events(&database), vec!["b"]);

    let all = repository
        .purge(&EvidencePurgeRequest {
            operation_id: "purge-all".to_string(),
            scope: EvidencePurgeScope::All,
        })
        .expect("full purge");
    assert_eq!(all.deleted_signals, 1);
    assert!(signal_events(&database).is_empty());
}

#[test]
fn interrupted_purge_rolls_back_the_entire_batch() {
    let (_directory, database, repository) = fixture("evidence-purge-rollback");
    persist(
        &repository,
        &failure("a", "2026-08-01T00:00:00Z", "a", "run-a"),
    );
    persist(
        &repository,
        &failure("b", "2026-08-01T00:00:00Z", "b", "run-b"),
    );
    database.connection().expect("connection").execute_batch(
        "CREATE TRIGGER interrupt_evidence_purge BEFORE DELETE ON evolution_signals WHEN OLD.source_event_id = 'b' BEGIN SELECT RAISE(ABORT, 'interrupted'); END;",
    ).expect("trigger");

    assert!(repository
        .purge(&EvidencePurgeRequest {
            operation_id: "purge-interrupted".to_string(),
            scope: EvidencePurgeScope::All,
        })
        .is_err());
    assert_eq!(signal_events(&database), vec!["a", "b"]);
}

#[test]
fn seed_lineage_is_bounded_and_discloses_truncation() {
    let (_directory, database, repository) = fixture("evidence-lineage-cap");
    for index in 0..101 {
        persist(
            &repository,
            &failure(
                &format!("event-{index:03}"),
                "2026-08-01T00:00:00Z",
                "a",
                &format!("run-{index:03}"),
            ),
        );
    }
    repository.rebuild_dirty_seeds().expect("seed rebuild");
    let connection = database.connection().expect("connection");
    let values: (i64, i64, i64) = connection
        .query_row(
            "SELECT lineage_signal_count, lineage_truncated_count, (SELECT COUNT(*) FROM evolution_candidate_seed_signals) FROM evolution_candidate_seeds",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("lineage metadata");
    assert_eq!(values, (101, 1, 100));
}
