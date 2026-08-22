//! Publication against a real filesystem and a real database.

use super::{
    read_snapshot, ExtensionRoots, FilesystemSnapshotContentStore, SqliteSnapshotPointerRepository,
};
use crate::contexts::tooling::extension_platform::application::{
    SnapshotContentStore, SnapshotPointerRepository,
};
use crate::contexts::tooling::extension_platform::domain::{
    ContentPublication, ExtensionId, InstallationId, ManifestDigest, PackageHash, SnapshotId,
    SnapshotPublicationError, SnapshotRecord,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use semver::Version;
use std::sync::Arc;

const PACKAGE_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const OTHER_DIGEST: &str = "3333333333333333333333333333333333333333333333333333333333333333";
const MANIFEST_DIGEST: &str = "2222222222222222222222222222222222222222222222222222222222222222";

struct Fixture {
    _directory: TempDirectory,
    database: Arc<NativeDatabase>,
    roots: ExtensionRoots,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDirectory::new(label);
    let database = Arc::new(
        NativeDatabase::new(directory.path().to_path_buf()).expect("database should open"),
    );
    let roots = ExtensionRoots::new(directory.path().join("extensions"));
    roots.prepare().expect("roots");
    Fixture {
        _directory: directory,
        database,
        roots,
    }
}

fn record(snapshot: &str, hash: &str) -> SnapshotRecord {
    SnapshotRecord {
        snapshot: SnapshotId::parse(snapshot).expect("snapshot"),
        extension: ExtensionId::parse("acme.git-guardian").expect("extension"),
        version: Version::parse("1.2.0").expect("version"),
        package_hash: PackageHash::parse(hash).expect("hash"),
        manifest_digest: ManifestDigest::parse(MANIFEST_DIGEST).expect("digest"),
        created_at: "2026-08-22T00:00:00Z".to_string(),
    }
}

fn staged(fixture: &Fixture, name: &str) -> std::path::PathBuf {
    let path = fixture
        .roots
        .root(crate::contexts::tooling::extension_platform::domain::ExtensionRootScope::Quarantine)
        .join(name);
    fixture.roots.create(&path).expect("staged directory");
    std::fs::write(path.join("vanehub-extension.yaml"), b"schema_version: 1\n")
        .expect("staged content");
    path
}

fn pointers(fixture: &Fixture) -> SqliteSnapshotPointerRepository {
    SqliteSnapshotPointerRepository::new(
        fixture.database.clone(),
        InstallationId::parse("install-1").expect("installation"),
    )
}

#[test]
fn content_is_published_by_moving_the_staged_directory_into_place() {
    let fixture = fixture("snapshot-publish");
    let store = FilesystemSnapshotContentStore::new(fixture.roots.clone());
    let staged = staged(&fixture, "operation-1");
    let hash = PackageHash::parse(PACKAGE_DIGEST).expect("hash");

    assert_eq!(
        store.publish(&staged, &hash).expect("publish"),
        ContentPublication::Published
    );

    let destination = fixture.roots.package(&hash).expect("package path");
    assert!(destination.join("vanehub-extension.yaml").is_file());
    assert!(!staged.exists(), "the staged copy moved rather than copied");
}

#[test]
fn publishing_content_that_is_already_there_discards_the_staged_copy_and_succeeds() {
    // Content is addressed by its own digest, so what is there is what would have been written.
    let fixture = fixture("snapshot-already-present");
    let store = FilesystemSnapshotContentStore::new(fixture.roots.clone());
    let hash = PackageHash::parse(PACKAGE_DIGEST).expect("hash");
    store
        .publish(&staged(&fixture, "operation-1"), &hash)
        .expect("first publish");

    let second = staged(&fixture, "operation-2");
    assert_eq!(
        store.publish(&second, &hash).expect("second publish"),
        ContentPublication::AlreadyPresent
    );
    assert!(!second.exists());
    assert!(fixture
        .roots
        .package(&hash)
        .expect("package path")
        .join("vanehub-extension.yaml")
        .is_file());
}

#[test]
fn a_pointer_moves_once_and_keeps_what_it_replaced() {
    let fixture = fixture("snapshot-pointer");
    let pointers = pointers(&fixture);
    let extension = ExtensionId::parse("acme.git-guardian").expect("extension");

    assert_eq!(pointers.active(&extension).expect("active"), None);

    let first = pointers
        .point_at(&record("snapshot-1", PACKAGE_DIGEST), 0)
        .expect("first pointer");
    assert_eq!(first.active.as_str(), "snapshot-1");
    assert_eq!(first.previous, None);
    assert_eq!(first.revision, 1);

    let second = pointers
        .point_at(&record("snapshot-2", OTHER_DIGEST), 1)
        .expect("second pointer");
    assert_eq!(second.active.as_str(), "snapshot-2");
    assert_eq!(
        second.previous.as_ref().map(SnapshotId::as_str),
        Some("snapshot-1")
    );
    assert_eq!(second.revision, 2);

    assert_eq!(pointers.active(&extension).expect("active"), Some(second));
}

#[test]
fn a_stale_writer_is_refused_and_changes_nothing() {
    let fixture = fixture("snapshot-stale");
    let pointers = pointers(&fixture);
    pointers
        .point_at(&record("snapshot-1", PACKAGE_DIGEST), 0)
        .expect("first pointer");

    assert_eq!(
        pointers.point_at(&record("snapshot-2", OTHER_DIGEST), 0),
        Err(SnapshotPublicationError::StaleRevision {
            expected: 0,
            actual: 1
        })
    );

    let extension = ExtensionId::parse("acme.git-guardian").expect("extension");
    let current = pointers
        .active(&extension)
        .expect("active")
        .expect("pointer");
    assert_eq!(current.active.as_str(), "snapshot-1");
    assert_eq!(current.revision, 1);
    assert_eq!(
        read_snapshot(
            &fixture.database,
            &SnapshotId::parse("snapshot-2").expect("snapshot")
        )
        .expect("read"),
        None,
        "the refused snapshot row is not written either"
    );
}

#[test]
fn a_snapshot_row_is_written_alongside_the_pointer_and_survives_being_replaced() {
    // Which bytes an installation ran, and when, is evidence. Publication never deletes a row.
    let fixture = fixture("snapshot-rows");
    let pointers = pointers(&fixture);
    pointers
        .point_at(&record("snapshot-1", PACKAGE_DIGEST), 0)
        .expect("first pointer");
    pointers
        .point_at(&record("snapshot-2", OTHER_DIGEST), 1)
        .expect("second pointer");

    for id in ["snapshot-1", "snapshot-2"] {
        let stored = read_snapshot(&fixture.database, &SnapshotId::parse(id).expect("snapshot"))
            .expect("read")
            .expect("row");
        assert_eq!(stored.extension.as_str(), "acme.git-guardian");
        assert_eq!(stored.manifest_digest.as_str(), MANIFEST_DIGEST);
    }
}

#[test]
fn publishing_the_same_snapshot_twice_writes_one_row() {
    let fixture = fixture("snapshot-idempotent");
    let pointers = pointers(&fixture);
    pointers
        .point_at(&record("snapshot-1", PACKAGE_DIGEST), 0)
        .expect("first pointer");
    pointers
        .point_at(&record("snapshot-1", PACKAGE_DIGEST), 1)
        .expect("same snapshot again");

    let connection = fixture.database.connection().expect("connection");
    let rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM extension_platform_snapshots WHERE snapshot_id = 'snapshot-1'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(rows, 1);
}
