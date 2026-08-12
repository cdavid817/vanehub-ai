use super::overlay_layout::OverlayStorageLayout;
use super::overlay_manifest::serialize_overlay_manifest;
use super::overlay_payload::{OverlayPayloadRecoveryState, OverlayPayloadStore};
use crate::contexts::tooling::skills::application::{
    OverlayApplicationError, OverlayIntegrityCode, OverlayKey, OverlayPayloadRepository,
    OverlayPayloadWrite, SkillApplicationError,
};
use crate::contexts::tooling::skills::domain::{
    OverlayBaseWitness, OverlayDocument, OverlayFile, OverlayScope, OverlayTrust, SkillId,
};
use crate::test_support::TempDirectory;
use sha2::{Digest, Sha256};
use std::fs;

fn key(scope: OverlayScope) -> OverlayKey {
    OverlayKey {
        canonical_skill_id: SkillId::parse("code-review").expect("valid Skill id"),
        scope,
        workspace_identity: None,
    }
}

fn hash(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn document(scope: OverlayScope, content_hash: &str) -> OverlayDocument {
    let mut document = OverlayDocument::new(
        SkillId::parse("code-review").expect("valid Skill id"),
        scope,
        None,
        OverlayBaseWitness::new("system:code-review:v1", "instruction-hash", "package-hash")
            .expect("base witness"),
        OverlayTrust::trusted_local(1),
        "2026-08-11T00:00:00Z",
    )
    .expect("Overlay document");
    document.files.push(
        OverlayFile::new(
            "file-1",
            "references/team.md",
            "text/markdown",
            12,
            content_hash,
            &format!("sha256/{content_hash}"),
            "2026-08-11T00:00:00Z",
        )
        .expect("Overlay file"),
    );
    document
}

fn write_manifest(home: &TempDirectory, key: &OverlayKey, document: &OverlayDocument) {
    let layout = OverlayStorageLayout::resolve(home.path(), key).expect("layout");
    fs::create_dir_all(layout.manifest_path.parent().expect("manifest parent"))
        .expect("create manifest parent");
    fs::write(
        layout.manifest_path,
        serialize_overlay_manifest(document).expect("serialize manifest"),
    )
    .expect("write manifest");
}

fn publish(
    store: &OverlayPayloadStore,
    key: &OverlayKey,
    content: &[u8],
    transaction: &str,
) -> String {
    let content_hash = hash(content);
    let stage = store
        .stage(
            key,
            &OverlayPayloadWrite {
                content_hash: content_hash.clone(),
                content: content.to_vec(),
            },
            transaction,
        )
        .expect("stage payload");
    store.publish(stage).expect("publish payload");
    content_hash
}

#[test]
fn payload_is_verified_before_staging_and_published_by_content_hash() {
    let home = TempDirectory::new("overlay-payload-stage");
    let store = OverlayPayloadStore::with_home_root(home.path().to_path_buf());
    let key = key(OverlayScope::System);
    let content = b"bounded guidance";
    let content_hash = hash(content);

    let mismatch = store.stage(
        &key,
        &OverlayPayloadWrite {
            content_hash: "0".repeat(64),
            content: content.to_vec(),
        },
        "transaction-1",
    );
    assert!(matches!(
        mismatch,
        Err(SkillApplicationError::Overlay(
            OverlayApplicationError::Integrity {
                code: OverlayIntegrityCode::PayloadHashMismatch
            }
        ))
    ));

    let stage = store
        .stage(
            &key,
            &OverlayPayloadWrite {
                content_hash: content_hash.clone(),
                content: content.to_vec(),
            },
            "transaction-2",
        )
        .expect("stage payload");
    assert!(stage.staged_path().is_some_and(|path| path.is_file()));
    assert!(!stage.final_path().exists());
    assert_eq!(stage.payload_ref(), format!("sha256/{content_hash}"));

    store.publish(stage).expect("publish payload");
    assert_eq!(
        store
            .read_verified(&key, &content_hash)
            .expect("read payload"),
        content
    );

    let discarded = store
        .stage(
            &key,
            &OverlayPayloadWrite {
                content_hash: hash(b"discard me"),
                content: b"discard me".to_vec(),
            },
            "transaction-3",
        )
        .expect("stage disposable payload");
    let discarded_path = discarded
        .staged_path()
        .expect("new payload has a staging path")
        .to_path_buf();
    store.discard_stage(discarded).expect("discard stage");
    assert!(!discarded_path.exists());
}

#[test]
fn read_refuses_a_tampered_payload() {
    let home = TempDirectory::new("overlay-payload-tamper");
    let store = OverlayPayloadStore::with_home_root(home.path().to_path_buf());
    let key = key(OverlayScope::System);
    let content_hash = publish(&store, &key, b"original", "transaction-tamper");
    let layout = OverlayStorageLayout::resolve(home.path(), &key).expect("layout");
    fs::write(
        layout.payload_root.join("sha256").join(&content_hash),
        b"tampered",
    )
    .expect("tamper payload");

    assert!(matches!(
        store.read_verified(&key, &content_hash),
        Err(SkillApplicationError::Overlay(
            OverlayApplicationError::Integrity {
                code: OverlayIntegrityCode::PayloadHashMismatch
            }
        ))
    ));
}

#[test]
fn reference_tracking_includes_all_manifest_files_regardless_of_mutation_state() {
    let home = TempDirectory::new("overlay-payload-references");
    let store = OverlayPayloadStore::with_home_root(home.path().to_path_buf());
    let key = key(OverlayScope::System);
    let first = hash(b"first");
    let second = hash(b"second");
    let mut manifest = document(OverlayScope::System, &first);
    let mut reverted = OverlayFile::new(
        "file-2",
        "references/old.md",
        "text/markdown",
        6,
        &second,
        &format!("sha256/{second}"),
        "2026-08-11T00:00:00Z",
    )
    .expect("Overlay file");
    reverted
        .revert("2026-08-11T00:01:00Z")
        .expect("revert file");
    manifest.files.push(reverted);
    write_manifest(&home, &key, &manifest);

    let mut expected = vec![first, second];
    expected.sort();
    assert_eq!(
        store
            .referenced_content_hashes(&key)
            .expect("manifest references"),
        expected
    );
}

#[test]
fn orphan_cleanup_waits_for_recovery_and_preserves_shared_and_backup_references() {
    let home = TempDirectory::new("overlay-payload-cleanup");
    let store = OverlayPayloadStore::with_home_root(home.path().to_path_buf());
    let system_key = key(OverlayScope::System);
    let user_key = key(OverlayScope::User);
    let system_hash = publish(&store, &system_key, b"system", "transaction-system");
    let user_hash = publish(&store, &user_key, b"user", "transaction-user");
    let backup_hash = publish(&store, &system_key, b"backup", "transaction-backup");
    let orphan_hash = publish(&store, &system_key, b"orphan", "transaction-orphan");
    write_manifest(
        &home,
        &system_key,
        &document(OverlayScope::System, &system_hash),
    );
    write_manifest(&home, &user_key, &document(OverlayScope::User, &user_hash));
    let backup = document(OverlayScope::System, &backup_hash);

    assert!(store
        .cleanup_orphans(
            &system_key,
            std::slice::from_ref(&backup),
            OverlayPayloadRecoveryState::Pending,
        )
        .is_err());
    assert!(store.read_verified(&system_key, &orphan_hash).is_ok());

    assert_eq!(
        store
            .cleanup_orphans(
                &system_key,
                &[backup],
                OverlayPayloadRecoveryState::Complete,
            )
            .expect("cleanup after recovery"),
        vec![orphan_hash.clone()]
    );
    for retained in [system_hash, user_hash, backup_hash] {
        assert!(store.read_verified(&system_key, &retained).is_ok());
    }
    assert!(store.read_verified(&system_key, &orphan_hash).is_err());
}
