use super::overlay_history::FilesystemOverlayHistoryRepository;
use super::overlay_layout::OverlayStorageLayout;
use super::overlay_manifest::{parse_overlay_manifest, serialize_overlay_manifest};
use super::overlay_transaction::{
    FilesystemOverlayTransactionExecutor, OverlayTransactionInterruption,
};
use crate::contexts::tooling::skills::application::{
    OverlayActor, OverlayHistoryAction, OverlayHistoryEntry, OverlayHistoryQuery,
    OverlayHistoryRepository, OverlayKey, OverlayPageIntegrity, OverlayPayloadWrite,
    OverlayTransactionExecutor, OverlayTransactionPlan, OverlayUsageDelta,
};
use crate::contexts::tooling::skills::domain::{
    OverlayBaseWitness, OverlayDocument, OverlayFile, OverlayScope, OverlayTrust, SkillId,
};
use crate::test_support::TempDirectory;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const TIMESTAMP: &str = "2026-08-11T10:00:00Z";
const NEXT_TIMESTAMP: &str = "2026-08-11T10:01:00Z";

fn key() -> OverlayKey {
    OverlayKey {
        canonical_skill_id: SkillId::parse("transaction-skill").expect("skill id"),
        scope: OverlayScope::User,
        workspace_identity: None,
    }
}

fn initial_plan() -> OverlayTransactionPlan {
    let key = key();
    let content = b"transaction payload".to_vec();
    let content_hash = sha256(&content);
    let mut document = OverlayDocument::new(
        key.canonical_skill_id.clone(),
        key.scope,
        None,
        OverlayBaseWitness::new(
            "system:transaction-skill",
            "instruction-hash",
            "package-hash",
        )
        .expect("base witness"),
        OverlayTrust::trusted_local(1),
        TIMESTAMP,
    )
    .expect("overlay document");
    document.files.push(
        OverlayFile::new(
            "file-1",
            "references/transaction.md",
            "text/markdown",
            content.len() as u64,
            &content_hash,
            &format!("sha256/{content_hash}"),
            TIMESTAMP,
        )
        .expect("overlay file"),
    );
    let manifest = serialize_overlay_manifest(&document).expect("manifest");
    let document_hash = sha256(&manifest);
    OverlayTransactionPlan {
        key: key.clone(),
        expected_revision: None,
        expected_document_hash: None,
        next_manifest: crate::contexts::tooling::skills::application::OverlayManifestSnapshot {
            document,
            document_hash: document_hash.clone(),
        },
        payload_additions: vec![OverlayPayloadWrite {
            content_hash,
            content,
        }],
        history_event: OverlayHistoryEntry {
            event_id: "event-1".to_string(),
            canonical_skill_id: key.canonical_skill_id,
            scope: key.scope,
            prior_revision: None,
            next_revision: 1,
            actor: OverlayActor::User,
            action: OverlayHistoryAction::File,
            timestamp: TIMESTAMP.to_string(),
            prior_document_hash: None,
            next_document_hash: document_hash,
            scanner_version: "overlay-scanner-v1".to_string(),
            safe_outcome: "supporting-file-added".to_string(),
            prior_event_hash: None,
            event_hash: String::new(),
        },
        usage_delta: OverlayUsageDelta {
            patch_count_delta: 0,
            overlay_mutation_count_delta: 1,
            timestamp: TIMESTAMP.to_string(),
            expected_revision_witness: "usage-0".to_string(),
        },
    }
}

fn next_plan(home: &TempDirectory) -> OverlayTransactionPlan {
    let key = key();
    let layout = OverlayStorageLayout::resolve(home.path(), &key).expect("layout");
    let prior_bytes = fs::read(&layout.manifest_path).expect("prior manifest");
    let prior_hash = sha256(&prior_bytes);
    let mut document = parse_overlay_manifest(&prior_bytes).expect("prior document");
    document
        .advance_revision(&prior_hash, NEXT_TIMESTAMP)
        .expect("next revision");
    let content = b"transaction payload revision two".to_vec();
    let content_hash = sha256(&content);
    document.files.push(
        OverlayFile::new(
            "file-2",
            "references/transaction-two.md",
            "text/markdown",
            content.len() as u64,
            &content_hash,
            &format!("sha256/{content_hash}"),
            NEXT_TIMESTAMP,
        )
        .expect("second overlay file"),
    );
    let manifest = serialize_overlay_manifest(&document).expect("next manifest");
    let document_hash = sha256(&manifest);
    OverlayTransactionPlan {
        key: key.clone(),
        expected_revision: Some(1),
        expected_document_hash: Some(prior_hash.clone()),
        next_manifest: crate::contexts::tooling::skills::application::OverlayManifestSnapshot {
            document,
            document_hash: document_hash.clone(),
        },
        payload_additions: vec![OverlayPayloadWrite {
            content_hash,
            content,
        }],
        history_event: OverlayHistoryEntry {
            event_id: "event-2".to_string(),
            canonical_skill_id: key.canonical_skill_id,
            scope: key.scope,
            prior_revision: Some(1),
            next_revision: 2,
            actor: OverlayActor::User,
            action: OverlayHistoryAction::File,
            timestamp: NEXT_TIMESTAMP.to_string(),
            prior_document_hash: Some(prior_hash),
            next_document_hash: document_hash,
            scanner_version: "overlay-scanner-v1".to_string(),
            safe_outcome: "second-supporting-file-added".to_string(),
            prior_event_hash: None,
            event_hash: String::new(),
        },
        usage_delta: OverlayUsageDelta {
            patch_count_delta: 0,
            overlay_mutation_count_delta: 1,
            timestamp: NEXT_TIMESTAMP.to_string(),
            expected_revision_witness: "usage-1".to_string(),
        },
    }
}

fn imported_plan() -> OverlayTransactionPlan {
    let mut plan = initial_plan();
    plan.next_manifest
        .document
        .quarantine_import("transaction-import.zip".to_string());
    let manifest =
        serialize_overlay_manifest(&plan.next_manifest.document).expect("imported manifest");
    plan.next_manifest.document_hash = sha256(&manifest);
    plan.history_event.action = OverlayHistoryAction::Import;
    plan.history_event.next_document_hash = plan.next_manifest.document_hash.clone();
    plan.history_event.safe_outcome = "overlay-imported-untrusted".to_string();
    plan
}

fn promotion_plan(home: &TempDirectory) -> OverlayTransactionPlan {
    let key = key();
    let layout = OverlayStorageLayout::resolve(home.path(), &key).expect("layout");
    let prior_bytes = fs::read(&layout.manifest_path).expect("prior manifest");
    let prior_hash = sha256(&prior_bytes);
    let mut document = parse_overlay_manifest(&prior_bytes).expect("prior document");
    document
        .promote_import(1, &prior_hash, NEXT_TIMESTAMP)
        .expect("promote imported revision");
    let manifest = serialize_overlay_manifest(&document).expect("promoted manifest");
    let document_hash = sha256(&manifest);
    let history = FilesystemOverlayHistoryRepository::with_home_root(home.path().to_path_buf());
    let prior_event_hash = history.verified_tail_hash(&key).expect("history tail");
    OverlayTransactionPlan {
        key: key.clone(),
        expected_revision: Some(1),
        expected_document_hash: Some(prior_hash.clone()),
        next_manifest: crate::contexts::tooling::skills::application::OverlayManifestSnapshot {
            document,
            document_hash: document_hash.clone(),
        },
        payload_additions: Vec::new(),
        history_event: OverlayHistoryEntry {
            event_id: "event-promote-1".to_string(),
            canonical_skill_id: key.canonical_skill_id,
            scope: key.scope,
            prior_revision: Some(1),
            next_revision: 1,
            actor: OverlayActor::User,
            action: OverlayHistoryAction::Promote,
            timestamp: NEXT_TIMESTAMP.to_string(),
            prior_document_hash: Some(prior_hash),
            next_document_hash: document_hash,
            scanner_version: "overlay-scanner-v1".to_string(),
            safe_outcome: "import-trust-promoted".to_string(),
            prior_event_hash,
            event_hash: String::new(),
        },
        usage_delta: OverlayUsageDelta {
            patch_count_delta: 0,
            overlay_mutation_count_delta: 1,
            timestamp: NEXT_TIMESTAMP.to_string(),
            expected_revision_witness: "usage-1".to_string(),
        },
    }
}

fn assert_previous_state(home: &TempDirectory, next: &OverlayTransactionPlan) {
    let layout = OverlayStorageLayout::resolve(home.path(), &key()).expect("layout");
    let document = parse_overlay_manifest(&fs::read(&layout.manifest_path).expect("manifest"))
        .expect("valid manifest");
    assert_eq!(document.revision(), 1);
    assert!(layout
        .payload_root
        .join("sha256")
        .join(&initial_plan().payload_additions[0].content_hash)
        .is_file());
    assert!(!layout
        .payload_root
        .join("sha256")
        .join(&next.payload_additions[0].content_hash)
        .exists());
    let history = FilesystemOverlayHistoryRepository::with_home_root(home.path().to_path_buf());
    let page = history
        .read_verified_page(&key(), &OverlayHistoryQuery::bounded(None, 10, 10))
        .expect("history");
    assert_eq!(page.integrity, OverlayPageIntegrity::Verified);
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].next_revision, 1);
    assert_eq!(usage_counts(home), (0, 1));
    assert!(!transaction_root(home).exists());
}

fn assert_next_state(home: &TempDirectory, next: &OverlayTransactionPlan) {
    let layout = OverlayStorageLayout::resolve(home.path(), &key()).expect("layout");
    let document = parse_overlay_manifest(&fs::read(&layout.manifest_path).expect("manifest"))
        .expect("valid manifest");
    assert_eq!(document.revision(), 2);
    assert!(layout
        .payload_root
        .join("sha256")
        .join(&initial_plan().payload_additions[0].content_hash)
        .is_file());
    assert!(layout
        .payload_root
        .join("sha256")
        .join(&next.payload_additions[0].content_hash)
        .is_file());
    let history = FilesystemOverlayHistoryRepository::with_home_root(home.path().to_path_buf());
    let page = history
        .read_verified_page(&key(), &OverlayHistoryQuery::bounded(None, 10, 10))
        .expect("history");
    assert_eq!(page.integrity, OverlayPageIntegrity::Verified);
    assert_eq!(page.entries.len(), 2);
    assert_eq!(page.entries[0].next_revision, 2);
    assert_eq!(usage_counts(home), (0, 2));
    assert!(!transaction_root(home).exists());
}

fn usage_counts(home: &TempDirectory) -> (u64, u64) {
    let record = usage_record(home);
    (
        record["patchCount"].as_u64().unwrap_or(0),
        record["overlayMutationCount"].as_u64().unwrap_or(0),
    )
}

fn usage_record(home: &TempDirectory) -> Value {
    let path = home.path().join(".vanehub/skills/.usage.json");
    let Ok(bytes) = fs::read(path) else {
        return Value::Null;
    };
    let value: Value = serde_json::from_slice(&bytes).expect("usage json");
    value["records"]["overlay:transaction-skill"].clone()
}

fn transaction_root(home: &TempDirectory) -> std::path::PathBuf {
    home.path()
        .join(".vanehub/skill_overlays/.transactions/user-transaction-skill")
}

fn sha256(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn interrupt_and_recover(point: OverlayTransactionInterruption, committed: bool) {
    let home = TempDirectory::new("overlay-transaction-recovery");
    let executor = FilesystemOverlayTransactionExecutor::with_home_root(home.path().to_path_buf());
    executor.execute(initial_plan()).expect("initial commit");
    let next = next_plan(&home);
    executor.inject_interruption_once(point);
    executor
        .execute(next.clone())
        .expect_err("injected interruption");

    let restarted = FilesystemOverlayTransactionExecutor::with_home_root(home.path().to_path_buf());
    restarted.recover(&key()).expect("transaction recovery");
    if committed {
        assert_next_state(&home, &next);
    } else {
        assert_previous_state(&home, &next);
    }
}

#[test]
fn recovery_rolls_back_an_interruption_before_payload_staging() {
    interrupt_and_recover(OverlayTransactionInterruption::PayloadStaging, false);
}

#[test]
fn recovery_rolls_back_an_interruption_before_manifest_swap() {
    interrupt_and_recover(OverlayTransactionInterruption::ManifestSwap, false);
}

#[test]
fn recovery_rolls_back_an_interruption_before_history_append() {
    interrupt_and_recover(OverlayTransactionInterruption::HistoryAppend, false);
}

#[test]
fn recovery_rolls_back_an_interruption_before_usage_update() {
    interrupt_and_recover(OverlayTransactionInterruption::UsageUpdate, false);
}

#[test]
fn recovery_rolls_back_an_interruption_before_commit_marker() {
    interrupt_and_recover(OverlayTransactionInterruption::CommitMarker, false);
}

#[test]
fn recovery_finishes_an_interruption_before_cleanup() {
    interrupt_and_recover(OverlayTransactionInterruption::Cleanup, true);
}

#[test]
fn successful_patch_increments_patch_and_overlay_usage_once_with_timestamps() {
    let home = TempDirectory::new("overlay-transaction-patch-usage");
    let executor = FilesystemOverlayTransactionExecutor::with_home_root(home.path().to_path_buf());
    let mut plan = initial_plan();
    plan.history_event.action = OverlayHistoryAction::Patch;
    plan.history_event.safe_outcome = "exact-patch-created".to_string();
    plan.usage_delta.patch_count_delta = 1;

    executor.execute(plan).expect("patch transaction");

    let record = usage_record(&home);
    assert_eq!(usage_counts(&home), (1, 1));
    assert_eq!(record["lastPatchedAt"].as_str(), Some(TIMESTAMP));
    assert_eq!(record["lastOverlayMutationAt"].as_str(), Some(TIMESTAMP));
    assert_eq!(record["revisionWitness"].as_str(), Some("usage-1"));
}

#[test]
fn successful_non_patch_increments_only_overlay_usage_once() {
    let home = TempDirectory::new("overlay-transaction-non-patch-usage");
    let executor = FilesystemOverlayTransactionExecutor::with_home_root(home.path().to_path_buf());

    executor
        .execute(initial_plan())
        .expect("supporting file transaction");

    let record = usage_record(&home);
    assert_eq!(usage_counts(&home), (0, 1));
    assert!(record["lastPatchedAt"].is_null());
    assert_eq!(record["lastOverlayMutationAt"].as_str(), Some(TIMESTAMP));
}

#[test]
fn imported_overlay_preserves_a_reviewable_non_initial_revision() {
    let home = TempDirectory::new("overlay-transaction-import-revision");
    let executor = FilesystemOverlayTransactionExecutor::with_home_root(home.path().to_path_buf());
    let mut plan = imported_plan();
    plan.next_manifest
        .document
        .advance_revision("import-prior-revision-hash", NEXT_TIMESTAMP)
        .expect("imported revision");
    plan.next_manifest
        .document
        .quarantine_import("transaction-import.zip".to_string());
    let manifest = serialize_overlay_manifest(&plan.next_manifest.document).expect("manifest");
    plan.next_manifest.document_hash = sha256(&manifest);
    plan.history_event.next_revision = 2;
    plan.history_event.next_document_hash = plan.next_manifest.document_hash.clone();

    let outcome = executor.execute(plan).expect("import transaction");

    assert_eq!(outcome.committed_revision, 2);
    let history = FilesystemOverlayHistoryRepository::with_home_root(home.path().to_path_buf());
    let page = history
        .read_verified_page(&key(), &OverlayHistoryQuery::bounded(None, 10, 10))
        .expect("history");
    assert_eq!(page.entries[0].action, OverlayHistoryAction::Import);
    assert_eq!(page.entries[0].next_revision, 2);
}

#[test]
fn exact_revision_trust_promotion_commits_without_advancing_the_reviewed_revision() {
    let home = TempDirectory::new("overlay-transaction-trust-promotion");
    let executor = FilesystemOverlayTransactionExecutor::with_home_root(home.path().to_path_buf());
    executor.execute(imported_plan()).expect("import commit");
    let promotion = promotion_plan(&home);

    let outcome = executor.execute(promotion).expect("promotion commit");

    assert_eq!(outcome.committed_revision, 1);
    let layout = OverlayStorageLayout::resolve(home.path(), &key()).expect("layout");
    let document = parse_overlay_manifest(&fs::read(layout.manifest_path).expect("manifest"))
        .expect("promoted document");
    assert_eq!(document.revision(), 1);
    assert!(document.trust().is_trusted_for_revision(1));
    assert_eq!(usage_counts(&home), (0, 2));
    let history = FilesystemOverlayHistoryRepository::with_home_root(home.path().to_path_buf());
    let page = history
        .read_verified_page(&key(), &OverlayHistoryQuery::bounded(None, 10, 10))
        .expect("history");
    assert_eq!(page.entries.len(), 2);
    assert_eq!(page.entries[0].action, OverlayHistoryAction::Promote);
    assert_eq!(page.entries[0].prior_revision, Some(1));
    assert_eq!(page.entries[0].next_revision, 1);
}

#[test]
fn recovery_rolls_back_patch_usage_written_before_the_commit_marker() {
    let home = TempDirectory::new("overlay-transaction-patch-usage-rollback");
    let executor = FilesystemOverlayTransactionExecutor::with_home_root(home.path().to_path_buf());
    executor.execute(initial_plan()).expect("initial commit");
    let mut patch = next_plan(&home);
    patch.history_event.action = OverlayHistoryAction::Patch;
    patch.history_event.safe_outcome = "exact-patch-created".to_string();
    patch.usage_delta.patch_count_delta = 1;
    executor.inject_interruption_once(OverlayTransactionInterruption::CommitMarker);

    executor
        .execute(patch.clone())
        .expect_err("interruption after usage write");
    assert_eq!(usage_counts(&home), (1, 2));

    let restarted = FilesystemOverlayTransactionExecutor::with_home_root(home.path().to_path_buf());
    restarted.recover(&key()).expect("transaction recovery");

    assert_previous_state(&home, &patch);
    let record = usage_record(&home);
    assert!(record["lastPatchedAt"].is_null());
    assert_eq!(record["lastOverlayMutationAt"].as_str(), Some(TIMESTAMP));
    assert_eq!(record["revisionWitness"].as_str(), Some("usage-1"));
}

#[test]
fn concurrent_callers_recheck_revision_after_the_per_overlay_lock() {
    let home = TempDirectory::new("overlay-transaction-concurrency");
    let first = FilesystemOverlayTransactionExecutor::with_home_root(home.path().to_path_buf());
    first.execute(initial_plan()).expect("initial commit");
    let next = next_plan(&home);
    let second = FilesystemOverlayTransactionExecutor::with_home_root(home.path().to_path_buf());
    let (entered_sender, entered_receiver) = mpsc::channel();
    let (release_sender, release_receiver) = mpsc::channel();
    first.inject_pause_after_lock_once(entered_sender, release_receiver);

    let first_plan = next.clone();
    let first_call = thread::spawn(move || first.execute(first_plan));
    entered_receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("first caller acquired the Overlay lock");
    let second_plan = next.clone();
    let second_call = thread::spawn(move || second.execute(second_plan));
    release_sender.send(()).expect("release first caller");

    let first_result = first_call.join().expect("first caller");
    let second_error = second_call
        .join()
        .expect("second caller")
        .expect_err("stale concurrent caller");
    assert_eq!(first_result.expect("winning caller").committed_revision, 2);
    assert!(matches!(
        second_error,
        crate::contexts::tooling::skills::application::SkillApplicationError::ConcurrentModification(ref id)
            if id == "transaction-skill"
    ));
    assert_next_state(&home, &next);
}
