use std::thread;

use rusqlite::{params, Connection};
use serde_json::json;

use super::*;
use crate::contexts::skill_evolution_evidence::domain::{
    extract_registered_signals, EvidenceSanitizer, EvidenceSourceEnvelope, SignalDraft,
    TaskFingerprintBuilder,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

const KEY: &[u8; 32] = b"repository-test-installation-key";
const TABLES: [&str; 9] = [
    "evolution_signal_receipts",
    "evolution_signals",
    "evolution_signal_skill_associations",
    "evolution_signal_source_links",
    "evolution_candidate_seeds",
    "evolution_candidate_seed_signals",
    "evolution_feedback_current",
    "evolution_feedback_events",
    "evolution_pipeline_state",
];

fn fixture_signal() -> SignalDraft {
    let envelope: EvidenceSourceEnvelope = serde_json::from_value(json!({
        "sourceKind": "explicit_feedback",
        "schemaVersion": 1,
        "common": {
            "sourceEventId": "event:repository:1",
            "occurredAt": "2026-08-13T03:00:00Z",
            "stableAgentId": "onepiece",
            "sessionId": "session-1",
            "messageId": "message-1",
            "runId": "run-1",
            "attemptId": "attempt-1",
            "workspace": "workspace:7f3a",
            "fidelity": "native",
            "observedSkillRevisions": [{
                "skillId": "review",
                "revision": "rev-a",
                "associationKind": "injected",
                "observedAt": "2026-08-13T02:59:59Z"
            }]
        },
        "feedback": "corrected",
        "feedbackRevision": 1,
        "correctionNote": "Use alice@example.com instead."
    }))
    .expect("fixture envelope");
    envelope.validate().expect("valid fixture");
    let sanitizer = EvidenceSanitizer::new(KEY).expect("sanitizer");
    let sanitized = envelope
        .sanitized_registered_text(&sanitizer)
        .expect("sanitization");
    extract_registered_signals(&envelope, sanitized.as_ref())
        .into_iter()
        .next()
        .expect("signal")
}

fn repository_fixture(name: &str) -> (NativeDatabase, SqliteEvolutionEvidenceRepository) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    (
        database.clone(),
        SqliteEvolutionEvidenceRepository::new(database),
    )
}

fn fingerprints() -> TaskFingerprintBuilder {
    TaskFingerprintBuilder::new(KEY).expect("fingerprint builder")
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("row count")
}

#[test]
fn schema_creates_all_tables_and_registered_indexes_without_touching_legacy_data() {
    let connection = Connection::open_in_memory().expect("connection");
    connection
        .execute_batch(
            "CREATE TABLE legacy_data (value TEXT); INSERT INTO legacy_data VALUES ('keep');",
        )
        .expect("legacy fixture");

    apply_schema(&connection).expect("first migration");
    apply_schema(&connection).expect("idempotent migration");

    for table in TABLES {
        let expected = i64::from(table == "evolution_pipeline_state");
        assert_eq!(count(&connection, table), expected, "{table}");
    }
    let legacy: String = connection
        .query_row("SELECT value FROM legacy_data", [], |row| row.get(0))
        .expect("legacy row");
    let indexes: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name LIKE 'idx_evolution_%'",
            [],
            |row| row.get(0),
        )
        .expect("index count");
    assert_eq!(legacy, "keep");
    assert!(indexes >= 10, "all query paths are indexed");
}

#[test]
fn failed_migration_transaction_preserves_the_preexisting_schema() {
    let mut connection = Connection::open_in_memory().expect("connection");
    connection
        .execute_batch(
            "CREATE TABLE evolution_signals (signal_id TEXT PRIMARY KEY); \
             INSERT INTO evolution_signals VALUES ('legacy-signal');",
        )
        .expect("legacy schema");

    let transaction = connection.transaction().expect("transaction");
    assert!(apply_schema(&transaction).is_err());
    transaction.rollback().expect("rollback");

    assert_eq!(count(&connection, "evolution_signals"), 1);
    let receipt_table: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'evolution_signal_receipts'",
            [],
            |row| row.get(0),
        )
        .expect("receipt table lookup");
    assert_eq!(receipt_table, 0, "partial additive schema was rolled back");
}

#[test]
fn signal_associations_links_receipt_and_revision_commit_atomically() {
    let (database, repository) = repository_fixture("evidence-atomic-commit");
    let outcome = repository
        .persist_signal(&fixture_signal(), &fingerprints())
        .expect("persist");
    assert!(matches!(outcome, PersistSignalOutcome::Inserted { .. }));

    let connection = database.connection().expect("connection");
    assert_eq!(count(&connection, "evolution_signals"), 1);
    assert_eq!(count(&connection, "evolution_signal_receipts"), 1);
    assert_eq!(count(&connection, "evolution_signal_skill_associations"), 1);
    assert_eq!(count(&connection, "evolution_signal_source_links"), 4);
    let summary: String = connection
        .query_row("SELECT safe_summary FROM evolution_signals", [], |row| {
            row.get(0)
        })
        .expect("safe summary");
    assert!(!summary.contains("alice@example.com"));
    assert!(summary.contains("<redacted:email:"));
    let revision: i64 = connection
        .query_row(
            "SELECT revision FROM evolution_pipeline_state WHERE state_key = 'signal_set'",
            [],
            |row| row.get(0),
        )
        .expect("signal set revision");
    assert_eq!(revision, 1);
}

#[test]
fn duplicate_delivery_and_restart_return_the_existing_signal() {
    let (database, repository) = repository_fixture("evidence-replay");
    let signal = fixture_signal();
    let first = repository
        .persist_signal(&signal, &fingerprints())
        .expect("first");
    let reopened = SqliteEvolutionEvidenceRepository::new(database.clone());
    let second = reopened
        .persist_signal(&signal, &fingerprints())
        .expect("replay");

    let first_id = first.signal_id();
    assert_eq!(
        second,
        PersistSignalOutcome::Replayed {
            signal_id: first_id.into()
        }
    );
    let connection = database.connection().expect("connection");
    assert_eq!(count(&connection, "evolution_signals"), 1);
    assert_eq!(count(&connection, "evolution_signal_receipts"), 1);
}

#[test]
fn transaction_failure_rolls_back_every_evidence_row() {
    let (database, repository) = repository_fixture("evidence-rollback");
    let connection = database.connection().expect("connection");
    connection
        .execute_batch(
            "CREATE TRIGGER reject_evidence_association BEFORE INSERT ON evolution_signal_skill_associations BEGIN SELECT RAISE(ABORT, 'fixture rejection'); END;",
        )
        .expect("failure trigger");
    drop(connection);

    assert!(repository
        .persist_signal(&fixture_signal(), &fingerprints())
        .is_err());
    let connection = database.connection().expect("connection");
    for table in [
        "evolution_signals",
        "evolution_signal_receipts",
        "evolution_signal_skill_associations",
        "evolution_signal_source_links",
    ] {
        assert_eq!(count(&connection, table), 0, "{table} rolled back");
    }
    let revision: i64 = connection
        .query_row(
            "SELECT revision FROM evolution_pipeline_state WHERE state_key = 'signal_set'",
            [],
            |row| row.get(0),
        )
        .expect("revision");
    assert_eq!(revision, 0);
}

#[test]
fn orphan_receipt_is_reported_as_corruption_instead_of_false_replay() {
    let (database, repository) = repository_fixture("evidence-corruption");
    let connection = database.connection().expect("connection");
    connection
        .pragma_update(None, "foreign_keys", "OFF")
        .expect("disable fk");
    connection
        .execute(
            "INSERT INTO evolution_signal_receipts (source_event_id, extractor_id, extractor_version, discriminator, signal_id, created_at) VALUES (?1, ?2, 1, ?3, 'missing-signal', ?4)",
            params!["event:repository:1", "explicit_feedback", "feedback:1", "2026-08-13T03:00:01Z"],
        )
        .expect("orphan receipt");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable fk");
    drop(connection);

    let error = repository
        .persist_signal(&fixture_signal(), &fingerprints())
        .expect_err("corruption");
    assert!(matches!(error, EvidenceRepositoryError::CorruptReceipt));
}

#[test]
fn concurrent_duplicate_ingestion_commits_one_signal() {
    let (database, repository) = repository_fixture("evidence-concurrent-replay");
    let signal = fixture_signal();
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let repository = repository.clone();
            let signal = signal.clone();
            thread::spawn(move || {
                repository
                    .persist_signal(&signal, &fingerprints())
                    .expect("concurrent persist")
            })
        })
        .collect();
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("worker"))
        .collect();

    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, PersistSignalOutcome::Inserted { .. }))
            .count(),
        1
    );
    let expected = outcomes[0].signal_id();
    assert!(outcomes
        .iter()
        .all(|outcome| outcome.signal_id() == expected));
    let connection = database.connection().expect("connection");
    assert_eq!(count(&connection, "evolution_signals"), 1);
    assert_eq!(count(&connection, "evolution_signal_receipts"), 1);
}
