use super::SqliteContextManifestRepository;
use crate::contexts::agent_runtime::application::ContextManifestRepository;
use crate::contexts::agent_runtime::domain::{
    ContextEvidenceManifest, ContextEvidenceSummary, ContextRange, ContextReasonCode,
    ContextSourceKind, ContextSourceOutcome,
};
use crate::platform::database::NativeDatabase;
use std::collections::BTreeMap;

fn repository() -> (tempfile::TempDir, SqliteContextManifestRepository) {
    let directory = tempfile::tempdir().expect("tempdir");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteContextManifestRepository::new(database);
    (directory, repository)
}

fn manifest(generation_id: &str, recorded_at: &str) -> ContextEvidenceManifest {
    ContextEvidenceManifest {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        generation_id: generation_id.to_string(),
        recorded_at: recorded_at.to_string(),
        policy_version: "context-engine-v1".to_string(),
        evidence_budget: 1_000,
        occupied_tokens: 100,
        selected: vec![ContextEvidenceSummary {
            id: "candidate-1".to_string(),
            source_kind: ContextSourceKind::Retrieval,
            source_ref: "src/lib.rs".to_string(),
            range: ContextRange::new(2, 8),
            symbol: Some("run".to_string()),
            token_estimate: 100,
            safe_fingerprint: "sha256:safe".to_string(),
            reasons: vec![ContextReasonCode::SemanticMatch],
        }],
        rejected: vec![("candidate-2".to_string(), ContextReasonCode::BudgetRejected)],
        source_outcomes: BTreeMap::from([(
            ContextSourceKind::Retrieval,
            ContextSourceOutcome::Ready,
        )]),
        duplicate_tokens_saved: 20,
        collection_latency_bucket: "sub-10ms".to_string(),
        ranking_latency_bucket: "sub-10ms".to_string(),
        compaction_triggered: false,
    }
}

#[test]
fn persists_content_free_manifest_and_supports_lookup() {
    let (_directory, repository) = repository();
    repository
        .save(&manifest("generation-1", "100"))
        .expect("save");
    let loaded = repository
        .get("generation-1")
        .expect("get")
        .expect("manifest");
    assert_eq!(loaded.selected[0].source_ref, "src/lib.rs");
    let database_text = std::fs::read(repository.database.db_path.clone()).expect("database bytes");
    assert!(!String::from_utf8_lossy(&database_text).contains("secret source body"));
}

#[test]
fn lists_newest_first_with_cursor_and_empty_unknown_detail() {
    let (_directory, repository) = repository();
    repository
        .save(&manifest("generation-1", "100"))
        .expect("save");
    repository
        .save(&manifest("generation-2", "200"))
        .expect("save");
    let first = repository.list(Some("session-1"), None, 1).expect("list");
    assert_eq!(first.items[0].generation_id, "generation-2");
    let second = repository
        .list(Some("session-1"), first.next_cursor.as_deref(), 1)
        .expect("next");
    assert_eq!(second.items[0].generation_id, "generation-1");
    assert_eq!(repository.get("missing").expect("get"), None);
}

#[test]
fn rejects_unsafe_or_unbounded_manifest_metadata() {
    let (_directory, repository) = repository();
    let mut unsafe_manifest = manifest("generation-1", "100");
    unsafe_manifest.occupied_tokens = 1_001;
    assert!(repository.save(&unsafe_manifest).is_err());
    assert!(repository.list(None, None, 101).is_err());
    assert!(repository.list(None, Some("missing"), 10).is_err());
}
