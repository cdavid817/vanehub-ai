//! Reconciliation against a real filesystem.

use super::{
    reconcile, referenced_package_hashes, ExtensionRoots, SqliteSnapshotPointerRepository,
};
use crate::contexts::tooling::extension_platform::application::SnapshotPointerRepository;
use crate::contexts::tooling::extension_platform::domain::{
    ExtensionId, ExtensionRootScope, InstallationId, ManifestDigest, PackageHash, SnapshotId,
    SnapshotRecord,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use semver::Version;
use std::collections::BTreeSet;
use std::sync::Arc;

const REFERENCED: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const ORPHANED: &str = "2222222222222222222222222222222222222222222222222222222222222222";

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

/// Creates a directory under a root and puts a file in it, so a collection has to actually remove
/// a tree rather than an empty directory.
fn populate(roots: &ExtensionRoots, scope: ExtensionRootScope, segments: &[&str]) {
    let mut path = roots.root(scope);
    for segment in segments {
        path.push(segment);
    }
    roots.create(&path).expect("create");
    std::fs::write(path.join("content.bin"), b"bytes").expect("write");
}

fn record_snapshot(fixture: &Fixture, hash: &str) {
    SqliteSnapshotPointerRepository::new(
        fixture.database.clone(),
        InstallationId::parse("install-1").expect("installation"),
    )
    .point_at(
        &SnapshotRecord {
            snapshot: SnapshotId::parse("snapshot-1").expect("snapshot"),
            extension: ExtensionId::parse("acme.git-guardian").expect("extension"),
            version: Version::parse("1.2.0").expect("version"),
            package_hash: PackageHash::parse(hash).expect("hash"),
            manifest_digest: ManifestDigest::parse(REFERENCED).expect("digest"),
            created_at: "2026-08-22T00:00:00Z".to_string(),
        },
        0,
    )
    .expect("snapshot row");
}

#[test]
fn a_restart_empties_quarantine_scratch_and_sidecars() {
    let fixture = fixture("reconcile-collect");
    populate(
        &fixture.roots,
        ExtensionRootScope::Quarantine,
        &["operation-1"],
    );
    populate(
        &fixture.roots,
        ExtensionRootScope::Scratch,
        &["install-1", "generation-1"],
    );
    populate(
        &fixture.roots,
        ExtensionRootScope::Sidecars,
        &["install-1", "generation-1"],
    );

    let summary = reconcile(&fixture.roots, &BTreeSet::new());

    assert_eq!(
        summary.collected,
        vec![
            "install-1/generation-1".to_string(),
            "install-1/generation-1".to_string(),
            "operation-1".to_string(),
        ]
    );
    assert!(summary.is_clean());
    assert!(!fixture
        .roots
        .root(ExtensionRootScope::Quarantine)
        .join("operation-1")
        .exists());
}

#[test]
fn package_content_a_snapshot_row_names_survives_and_the_rest_does_not() {
    let fixture = fixture("reconcile-packages");
    record_snapshot(&fixture, REFERENCED);
    populate(
        &fixture.roots,
        ExtensionRootScope::Packages,
        &["sha256", REFERENCED],
    );
    populate(
        &fixture.roots,
        ExtensionRootScope::Packages,
        &["sha256", ORPHANED],
    );

    let referenced = referenced_package_hashes(&fixture.database).expect("referenced");
    let summary = reconcile(&fixture.roots, &referenced);

    assert_eq!(summary.retained, vec![format!("sha256/{REFERENCED}")]);
    assert_eq!(summary.collected, vec![format!("sha256/{ORPHANED}")]);
    assert!(fixture
        .roots
        .root(ExtensionRootScope::Packages)
        .join("sha256")
        .join(REFERENCED)
        .join("content.bin")
        .is_file());
}

#[test]
fn a_snapshot_that_is_no_longer_active_still_protects_its_bytes() {
    // The rollback target. Collecting its content because nothing points at it right now would
    // delete exactly what a rollback needs.
    let fixture = fixture("reconcile-rollback-target");
    let pointers = SqliteSnapshotPointerRepository::new(
        fixture.database.clone(),
        InstallationId::parse("install-1").expect("installation"),
    );
    for (index, (snapshot, hash)) in [("snapshot-1", REFERENCED), ("snapshot-2", ORPHANED)]
        .into_iter()
        .enumerate()
    {
        pointers
            .point_at(
                &SnapshotRecord {
                    snapshot: SnapshotId::parse(snapshot).expect("snapshot"),
                    extension: ExtensionId::parse("acme.git-guardian").expect("extension"),
                    version: Version::parse("1.2.0").expect("version"),
                    package_hash: PackageHash::parse(hash).expect("hash"),
                    manifest_digest: ManifestDigest::parse(REFERENCED).expect("digest"),
                    created_at: "2026-08-22T00:00:00Z".to_string(),
                },
                index as i64,
            )
            .expect("snapshot row");
    }
    populate(
        &fixture.roots,
        ExtensionRootScope::Packages,
        &["sha256", REFERENCED],
    );

    let referenced = referenced_package_hashes(&fixture.database).expect("referenced");
    let summary = reconcile(&fixture.roots, &referenced);

    assert_eq!(summary.retained, vec![format!("sha256/{REFERENCED}")]);
    assert!(summary.collected.is_empty());
}

#[test]
fn anything_unrecognised_is_reported_and_left_exactly_where_it_is() {
    let fixture = fixture("reconcile-unrecognised");
    let quarantine = fixture.roots.root(ExtensionRootScope::Quarantine);
    std::fs::write(quarantine.join("notes.txt"), b"an operator put this here").expect("stray file");
    let packages = fixture.roots.root(ExtensionRootScope::Packages);
    std::fs::create_dir_all(packages.join("sha512")).expect("wrong algorithm");
    std::fs::write(packages.join("sha512").join("thing"), b"bytes").expect("write");

    let summary = reconcile(&fixture.roots, &BTreeSet::new());

    assert_eq!(
        summary.unrecognised,
        vec!["notes.txt".to_string(), "sha512/thing".to_string()],
        "a stray file is reported by name, not silently skipped"
    );
    assert!(
        quarantine.join("notes.txt").is_file(),
        "a file where a directory belongs is reported, not deleted"
    );
    assert!(!summary.is_clean());
}

#[test]
fn a_clean_installation_reconciles_to_nothing() {
    let fixture = fixture("reconcile-clean");

    let summary = reconcile(&fixture.roots, &BTreeSet::new());

    assert_eq!(summary, Default::default());
    assert!(summary.is_clean());
}
