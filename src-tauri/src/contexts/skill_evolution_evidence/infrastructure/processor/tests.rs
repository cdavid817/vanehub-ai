use rusqlite::Connection;
use serde_json::json;

use super::*;
use crate::contexts::skill_evolution_evidence::domain::EvidenceSourceEnvelope;
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

const KEY: &[u8; 32] = b"processor-test-installation-key!";

#[test]
fn database_writer_lock_becomes_a_safe_storage_failure() {
    let directory = TempDirectory::new("evidence-processor-lock");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let processor = EvidenceIngestionProcessor::new(
        SqliteEvolutionEvidenceRepository::new(database.clone()),
        KEY,
    )
    .expect("processor");
    let envelope: EvidenceSourceEnvelope = serde_json::from_value(json!({
        "sourceKind": "explicit_feedback",
        "schemaVersion": 1,
        "common": {
            "sourceEventId": "lock-event",
            "occurredAt": "2026-08-13T10:00:00Z",
            "stableAgentId": null,
            "sessionId": null,
            "messageId": null,
            "runId": null,
            "attemptId": null,
            "workspace": null,
            "fidelity": "native",
            "observedSkillRevisions": []
        },
        "feedback": "helpful",
        "feedbackRevision": 1,
        "correctionNote": null
    }))
    .expect("envelope");
    let locker = Connection::open(&database.db_path).expect("lock connection");
    locker
        .execute_batch("BEGIN IMMEDIATE")
        .expect("acquire writer lock");

    assert_eq!(
        processor.process(&envelope),
        Err(PipelineFailureCategory::Storage)
    );
    locker.execute_batch("ROLLBACK").expect("release lock");
}
