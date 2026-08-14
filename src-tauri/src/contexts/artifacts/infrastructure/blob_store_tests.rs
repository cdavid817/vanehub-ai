use super::*;
use crate::test_support::TempDirectory;

fn policy() -> ArtifactBlobStorePolicy {
    ArtifactBlobStorePolicy {
        max_blob_bytes: 1024,
        max_operation_items: 2,
        max_operation_bytes: 1024,
        max_total_bytes: 2048,
    }
}

#[test]
fn sealing_is_content_addressed_atomic_and_deduplicated() {
    let directory = TempDirectory::new("artifact-blob-store");
    let store = ArtifactBlobStore::new(directory.path(), policy()).expect("store");
    let first = store
        .seal_bytes(
            "operation-1",
            "result.json",
            "application/json",
            br#"{"ok":true}"#,
        )
        .expect("first");
    let second = store
        .seal_bytes(
            "operation-2",
            "copy.json",
            "application/json",
            br#"{"ok":true}"#,
        )
        .expect("second");

    assert_eq!(first.content_hash, second.content_hash);
    assert!(!first.deduplicated);
    assert!(second.deduplicated);
    assert_eq!(
        store.read_verified(&first.content_hash).expect("read"),
        br#"{"ok":true}"#
    );
    assert!(!first
        .storage_key
        .contains(directory.path().to_string_lossy().as_ref()));
    let staged = directory.path().join("artifacts/staging/operation-1");
    assert_eq!(fs::read_dir(staged).expect("staging").count(), 0);
}

#[test]
fn admission_rejects_unsafe_names_media_and_operation_quotas() {
    let directory = TempDirectory::new("artifact-blob-admission");
    let mut limits = policy();
    limits.max_operation_items = 1;
    let store = ArtifactBlobStore::new(directory.path(), limits).expect("store");
    assert_eq!(
        store.seal_bytes("operation-1", "../secret", "text/plain", b"secret"),
        Err(ArtifactBlobStoreError::UnsafeDisplayName)
    );
    assert_eq!(
        store.seal_bytes("operation-1", "page.html", "text/html", b"<script/>"),
        Err(ArtifactBlobStoreError::UnsupportedMediaType)
    );
    store
        .seal_bytes("operation-1", "one.txt", "text/plain", b"one")
        .expect("one");
    assert_eq!(
        store.seal_bytes("operation-1", "two.txt", "text/plain", b"two"),
        Err(ArtifactBlobStoreError::ItemQuotaExceeded)
    );
}

#[test]
fn verified_reads_detect_tampering() {
    let directory = TempDirectory::new("artifact-blob-integrity");
    let store = ArtifactBlobStore::new(directory.path(), policy()).expect("store");
    let metadata = store
        .seal_bytes("operation-1", "note.txt", "text/plain", b"original")
        .expect("seal");
    let digest = metadata.content_hash.strip_prefix("sha256:").expect("hash");
    let path = directory
        .path()
        .join("artifacts/blobs/sha256")
        .join(&digest[..2])
        .join(&digest[2..]);
    fs::write(path, b"tampered").expect("tamper fixture");

    assert_eq!(
        store.read_verified(&metadata.content_hash),
        Err(ArtifactBlobStoreError::IntegrityFailure)
    );
}

#[test]
fn byte_quotas_media_signatures_and_special_storage_entries_fail_closed() {
    let directory = TempDirectory::new("artifact-blob-hardening");
    let mut limits = policy();
    limits.max_blob_bytes = 4;
    let store = ArtifactBlobStore::new(directory.path(), limits).expect("store");
    assert_eq!(
        store.seal_bytes("operation-1", "large.txt", "text/plain", b"12345"),
        Err(ArtifactBlobStoreError::BlobByteQuotaExceeded)
    );
    assert_eq!(
        store.seal_bytes("operation-1", "fake.png", "image/png", b"not-png"),
        Err(ArtifactBlobStoreError::InvalidMediaContent)
    );

    let blocked = TempDirectory::new("artifact-blob-special-entry");
    fs::create_dir_all(blocked.path().join("artifacts/blobs")).expect("blobs");
    fs::write(
        blocked.path().join("artifacts/blobs/sha256"),
        b"not a directory",
    )
    .expect("special entry");
    assert_eq!(
        ArtifactBlobStore::new(blocked.path(), policy()).expect_err("must reject special entry"),
        ArtifactBlobStoreError::IntegrityFailure
    );
}

#[test]
fn symlinked_storage_root_is_rejected_when_the_platform_allows_the_fixture() {
    let directory = TempDirectory::new("artifact-blob-symlink");
    let target = directory.path().join("target");
    fs::create_dir_all(&target).expect("target");
    fs::create_dir_all(directory.path().join("artifacts/blobs")).expect("blobs");
    let link = directory.path().join("artifacts/blobs/sha256");
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_dir(&target, &link).is_ok();
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(&target, &link).is_ok();
    if linked {
        assert_eq!(
            ArtifactBlobStore::new(directory.path(), policy()).expect_err("reject symlink"),
            ArtifactBlobStoreError::IntegrityFailure
        );
    }
}
