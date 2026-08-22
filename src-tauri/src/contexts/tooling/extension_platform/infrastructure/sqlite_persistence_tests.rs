//! Migration 86 against a real database: references, idempotency, rollback, and two-connection CAS.
//!
//! Every concurrency test here opens **two independent connections** from the pool. A single
//! connection serialises by construction, so a CAS test that shares one proves nothing about the
//! thing it claims to prove.

use super::{
    claim_for, record_operation_witness, record_package, record_snapshot_detail,
    SqliteRuntimeGenerationRepository, SqliteSnapshotPointerRepository,
    SqliteVersionClaimRepository,
};
use crate::contexts::tooling::extension_platform::application::{
    RuntimeGenerationRepository, SnapshotPointerRepository, VersionClaimRepository,
};
use crate::contexts::tooling::extension_platform::domain::{
    CapabilityDiff, ClaimOutcome, ClaimProvenance, CompatibilityOutcome, ExtensionId,
    ExtensionInstallWitness, InstallWitnessSubject, InstallationId, ManifestDigest, PackageHash,
    PublisherId, RuntimeGenerationError, RuntimeGenerationId, RuntimeGenerationRecord,
    SignatureSummary, SnapshotId, SnapshotRecord, TrustProfile,
};
use crate::platform::database::{migrate, NativeDatabase};
use crate::test_support::TempDirectory;
use rusqlite::Connection;
use semver::Version;
use std::sync::Arc;

const FIRST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";

struct Fixture {
    _directory: TempDirectory,
    database: Arc<NativeDatabase>,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDirectory::new(label);
    let database = Arc::new(
        NativeDatabase::new(directory.path().to_path_buf()).expect("database should open"),
    );
    Fixture {
        _directory: directory,
        database,
    }
}

fn publisher() -> PublisherId {
    PublisherId::parse("acme").expect("publisher")
}

fn extension() -> ExtensionId {
    ExtensionId::parse("acme.git-guardian").expect("extension")
}

fn version() -> Version {
    Version::parse("1.2.0").expect("version")
}

fn hash(value: &str) -> PackageHash {
    PackageHash::parse(value).expect("hash")
}

/// A witness over an unchanged world, so two operations recording one produce the same digest --
/// which is the case the identity has to survive.
fn witness() -> ExtensionInstallWitness {
    ExtensionInstallWitness::issue(InstallWitnessSubject {
        extension: extension(),
        version: version(),
        package_hash: hash(FIRST),
        manifest_digest: ManifestDigest::parse(SECOND).expect("digest"),
        signature: SignatureSummary {
            state: "verified",
            key_fingerprint: None,
        },
        installed: None,
        compatibility: CompatibilityOutcome::Compatible,
        trust_profile: TrustProfile::Strict,
        dependencies: Vec::new(),
        capabilities: CapabilityDiff::default(),
        contributions: Vec::new(),
    })
}

fn installation(value: &str) -> InstallationId {
    InstallationId::parse(value).expect("installation")
}

fn generation(value: &str) -> RuntimeGenerationId {
    RuntimeGenerationId::parse(value).expect("generation")
}

/// Publishes a snapshot and its installation, so tables with references have something to point at.
fn install(
    fixture: &Fixture,
    installation_id: &str,
    extension_id: &str,
    snapshot: &str,
    digest: &str,
) {
    SqliteSnapshotPointerRepository::new(fixture.database.clone(), installation(installation_id))
        .point_at(
            &SnapshotRecord {
                snapshot: SnapshotId::parse(snapshot).expect("snapshot"),
                extension: ExtensionId::parse(extension_id).expect("extension"),
                version: version(),
                package_hash: hash(digest),
                manifest_digest: ManifestDigest::parse(FIRST).expect("digest"),
                created_at: "2026-08-22T00:00:00Z".to_string(),
            },
            0,
        )
        .expect("install");
}

// ---------------------------------------------------------------------------
// Version claims
// ---------------------------------------------------------------------------

#[test]
fn a_version_binds_to_one_hash_and_the_same_hash_is_idempotent() {
    let fixture = fixture("claims-idempotent");
    let claims = SqliteVersionClaimRepository::new(fixture.database.clone());
    let offered = claim_for(
        &publisher(),
        &extension(),
        &version(),
        &hash(FIRST),
        ClaimProvenance::Signed,
        "2026-08-01T00:00:00Z",
    );

    assert_eq!(
        claims
            .claim(&offered, "2026-08-01T00:00:00Z")
            .expect("claim"),
        ClaimOutcome::Bound
    );
    assert_eq!(
        claims
            .claim(&offered, "2026-08-20T00:00:00Z")
            .expect("claim"),
        ClaimOutcome::AlreadyBound
    );
    assert_eq!(
        claims
            .held(&offered)
            .expect("held")
            .map(|held| held.first_claimed_at),
        Some("2026-08-01T00:00:00Z".to_string()),
        "the first binding's moment is what is kept"
    );
}

#[test]
fn the_same_version_with_different_bytes_is_refused_and_the_offered_hash_is_kept() {
    let fixture = fixture("claims-conflict");
    let claims = SqliteVersionClaimRepository::new(fixture.database.clone());
    let bound = claim_for(
        &publisher(),
        &extension(),
        &version(),
        &hash(FIRST),
        ClaimProvenance::Signed,
        "2026-08-01T00:00:00Z",
    );
    claims.claim(&bound, "2026-08-01T00:00:00Z").expect("claim");

    let other = claim_for(
        &publisher(),
        &extension(),
        &version(),
        &hash(SECOND),
        ClaimProvenance::Unsigned,
        "2026-08-20T00:00:00Z",
    );
    let outcome = claims.claim(&other, "2026-08-20T00:00:00Z").expect("claim");

    assert!(
        !outcome.admits_snapshot(),
        "no activatable snapshot may follow a conflicting claim"
    );
    assert_eq!(
        claims
            .held(&bound)
            .expect("held")
            .map(|held| held.package_hash),
        Some(hash(FIRST)),
        "the binding does not move"
    );
    assert_eq!(
        claims.conflicts(&extension()).expect("conflicts"),
        vec![SECOND.to_string()],
        "the refused hash is evidence, not something to throw away"
    );
}

#[test]
fn two_connections_claiming_the_same_version_produce_exactly_one_binding() {
    // Two independent connections. The read and the write are in one transaction, so the loser
    // sees the winner's row rather than an unheld version.
    let fixture = fixture("claims-cas");
    let first = Arc::new(SqliteVersionClaimRepository::new(fixture.database.clone()));
    let second = Arc::new(SqliteVersionClaimRepository::new(fixture.database.clone()));

    let one = Arc::clone(&first);
    let two = Arc::clone(&second);
    let left = std::thread::spawn(move || {
        one.claim(
            &claim_for(
                &publisher(),
                &extension(),
                &version(),
                &hash(FIRST),
                ClaimProvenance::Signed,
                "2026-08-01T00:00:00Z",
            ),
            "2026-08-01T00:00:00Z",
        )
    });
    let right = std::thread::spawn(move || {
        two.claim(
            &claim_for(
                &publisher(),
                &extension(),
                &version(),
                &hash(SECOND),
                ClaimProvenance::Signed,
                "2026-08-01T00:00:00Z",
            ),
            "2026-08-01T00:00:00Z",
        )
    });

    let outcomes = [
        left.join().expect("thread").expect("claim"),
        right.join().expect("thread").expect("claim"),
    ];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == ClaimOutcome::Bound)
            .count(),
        1,
        "exactly one may bind: {outcomes:?}"
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| !outcome.admits_snapshot())
            .count(),
        1,
        "and the other is a conflict: {outcomes:?}"
    );
}

// ---------------------------------------------------------------------------
// Runtime generations and the active pointer
// ---------------------------------------------------------------------------

#[test]
fn a_generation_must_belong_to_an_installation_that_exists() {
    let fixture = fixture("generations-fk");
    let generations = SqliteRuntimeGenerationRepository::new(fixture.database.clone());

    assert_eq!(
        generations.record(&RuntimeGenerationRecord {
            generation: generation("generation-1"),
            installation: installation("install-missing"),
            snapshot: SnapshotId::parse("snapshot-1").expect("snapshot"),
            started_at: "2026-08-22T00:00:00Z".to_string(),
        }),
        Err(RuntimeGenerationError::UnknownInstallation)
    );
}

#[test]
fn one_installation_cannot_be_pointed_at_another_installations_generation() {
    // The composite reference is the whole reason this cannot happen. A single-column reference
    // would find `generation-b` in the table and be satisfied.
    let fixture = fixture("generations-composite-fk");
    install(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snapshot-a",
        FIRST,
    );
    install(&fixture, "install-b", "acme.other", "snapshot-b", SECOND);
    let generations = SqliteRuntimeGenerationRepository::new(fixture.database.clone());
    generations
        .record(&RuntimeGenerationRecord {
            generation: generation("generation-b"),
            installation: installation("install-b"),
            snapshot: SnapshotId::parse("snapshot-b").expect("snapshot"),
            started_at: "2026-08-22T00:00:00Z".to_string(),
        })
        .expect("record");

    assert_eq!(
        generations.activate(
            &installation("install-a"),
            &generation("generation-b"),
            0,
            "2026-08-22T00:00:00Z"
        ),
        Err(RuntimeGenerationError::UnknownGeneration)
    );
    assert_eq!(
        generations
            .active(&installation("install-a"))
            .expect("active"),
        None
    );
}

#[test]
fn the_active_pointer_moves_once_and_is_one_row() {
    let fixture = fixture("generations-pointer");
    install(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snapshot-a",
        FIRST,
    );
    let generations = SqliteRuntimeGenerationRepository::new(fixture.database.clone());
    for id in ["generation-1", "generation-2"] {
        generations
            .record(&RuntimeGenerationRecord {
                generation: generation(id),
                installation: installation("install-a"),
                snapshot: SnapshotId::parse("snapshot-a").expect("snapshot"),
                started_at: "2026-08-22T00:00:00Z".to_string(),
            })
            .expect("record");
    }

    let first = generations
        .activate(
            &installation("install-a"),
            &generation("generation-1"),
            0,
            "2026-08-22T00:00:00Z",
        )
        .expect("activate");
    assert_eq!(first.revision, 1);

    let second = generations
        .activate(
            &installation("install-a"),
            &generation("generation-2"),
            1,
            "2026-08-23T00:00:00Z",
        )
        .expect("activate");
    assert_eq!(second.revision, 2);
    assert_eq!(second.generation.as_str(), "generation-2");

    let connection = fixture.database.connection().expect("connection");
    let rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM extension_platform_active_runtime_generations \
             WHERE installation_id = 'install-a'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(
        rows, 1,
        "two active rows is not a state this design has a meaning for"
    );
}

#[test]
fn two_connections_activating_from_the_same_revision_leave_one_winner() {
    let fixture = fixture("generations-cas");
    install(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snapshot-a",
        FIRST,
    );
    let writer = SqliteRuntimeGenerationRepository::new(fixture.database.clone());
    for id in ["generation-1", "generation-2"] {
        writer
            .record(&RuntimeGenerationRecord {
                generation: generation(id),
                installation: installation("install-a"),
                snapshot: SnapshotId::parse("snapshot-a").expect("snapshot"),
                started_at: "2026-08-22T00:00:00Z".to_string(),
            })
            .expect("record");
    }

    let first = Arc::new(SqliteRuntimeGenerationRepository::new(
        fixture.database.clone(),
    ));
    let second = Arc::new(SqliteRuntimeGenerationRepository::new(
        fixture.database.clone(),
    ));
    let one = Arc::clone(&first);
    let two = Arc::clone(&second);
    let left = std::thread::spawn(move || {
        one.activate(
            &installation("install-a"),
            &generation("generation-1"),
            0,
            "2026-08-22T00:00:00Z",
        )
    });
    let right = std::thread::spawn(move || {
        two.activate(
            &installation("install-a"),
            &generation("generation-2"),
            0,
            "2026-08-22T00:00:00Z",
        )
    });

    let outcomes = [left.join().expect("thread"), right.join().expect("thread")];
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_ok()).count(),
        1,
        "exactly one write may land: {outcomes:?}"
    );
    assert_eq!(
        first
            .active(&installation("install-a"))
            .expect("active")
            .map(|active| active.revision),
        Some(1)
    );
}

// ---------------------------------------------------------------------------
// Evidence tables
// ---------------------------------------------------------------------------

#[test]
fn snapshot_detail_packages_and_witnesses_are_idempotent() {
    let fixture = fixture("evidence-idempotent");
    install(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snapshot-a",
        FIRST,
    );
    let snapshot = SnapshotId::parse("snapshot-a").expect("snapshot");

    for _ in 0..2 {
        record_snapshot_detail(
            &fixture.database,
            &snapshot,
            &[(
                "skill".to_string(),
                "code-reviewer".to_string(),
                ">=2.0.0".to_string(),
                false,
            )],
            &[(
                "ext::acme.git-guardian::tool::git_status".to_string(),
                "tool".to_string(),
                "git_status".to_string(),
            )],
        )
        .expect("detail");
        record_package(
            &fixture.database,
            &hash(FIRST),
            1_024,
            "verified",
            Some(SECOND),
            "2026-08-22T00:00:00Z",
        )
        .expect("package");
        record_operation_witness(
            &fixture.database,
            "witness-1",
            "operation-1",
            &witness(),
            "2026-08-22T00:00:00Z",
        )
        .expect("witness");
    }

    let connection = fixture.database.connection().expect("connection");
    for (table, expected) in [
        ("extension_platform_snapshot_dependencies", 1),
        ("extension_platform_snapshot_contributions", 1),
        ("extension_platform_packages", 1),
        ("extension_platform_operation_witnesses", 1),
    ] {
        let rows: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("count");
        assert_eq!(rows, expected, "{table}");
    }
}

#[test]
fn two_operations_previewing_the_same_world_both_record_a_witness() {
    // The digest covers the state a confirmation is bound to and deliberately not the operation,
    // so two previews of an unchanged world produce the same digest. A digest primary key would
    // have made the second collide with the first and silently vanish.
    let fixture = fixture("witness-identity");

    for (witness_id, operation_id) in [("witness-1", "operation-1"), ("witness-2", "operation-2")] {
        record_operation_witness(
            &fixture.database,
            witness_id,
            operation_id,
            &witness(),
            "2026-08-22T00:00:00Z",
        )
        .expect("witness");
    }

    let connection = fixture.database.connection().expect("connection");
    let rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM extension_platform_operation_witnesses",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(rows, 2);
}

#[test]
fn evidence_is_not_removed_by_deleting_what_points_at_it() {
    // ON DELETE RESTRICT everywhere, CASCADE nowhere. Deleting an installation that still has a
    // generation fails and forces whoever is doing it to say what should happen to the generation.
    let fixture = fixture("evidence-restrict");
    install(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snapshot-a",
        FIRST,
    );
    SqliteRuntimeGenerationRepository::new(fixture.database.clone())
        .record(&RuntimeGenerationRecord {
            generation: generation("generation-1"),
            installation: installation("install-a"),
            snapshot: SnapshotId::parse("snapshot-a").expect("snapshot"),
            started_at: "2026-08-22T00:00:00Z".to_string(),
        })
        .expect("record");

    let connection = fixture.database.connection().expect("connection");
    assert!(
        connection
            .execute(
                "DELETE FROM extension_platform_installations WHERE installation_id = 'install-a'",
                [],
            )
            .is_err(),
        "an installation with a live generation cannot be deleted out from under it"
    );
    assert!(
        connection
            .execute(
                "DELETE FROM extension_platform_snapshots WHERE snapshot_id = 'snapshot-a'",
                [],
            )
            .is_err(),
        "and neither can the snapshot it runs"
    );
}

// ---------------------------------------------------------------------------
// The migration itself
//
// These live here rather than in `platform/database/migrations/tests.rs` because they assert about
// this subdomain's tables. That file registers migrations and owns the execution protocol; what a
// particular schema does belongs to whoever owns the schema.
// ---------------------------------------------------------------------------

#[test]
fn migration_86_rebuilds_installations_with_references_and_keeps_their_rows() {
    // Migration 85 created the table without foreign keys and SQLite cannot add one afterwards, so
    // 86 recreates it and copies. The copy is the part worth testing: a rebuild that silently
    // dropped rows would leave every installed extension unfindable while every schema assertion
    // still passed.
    let connection = Connection::open_in_memory().expect("in-memory database");
    migrate(&connection).expect("current schema");

    // Put the database back the way 85 left it: the new tables gone, the migration row removed,
    // and installations in its original shape holding a row.
    connection
        .execute_batch(
            r#"
            DELETE FROM schema_migrations WHERE version = 86;
            DROP TABLE extension_platform_active_runtime_generations;
            DROP TABLE extension_platform_runtime_generations;
            DROP TABLE extension_platform_operation_witnesses;
            DROP TABLE extension_platform_snapshot_contributions;
            DROP TABLE extension_platform_snapshot_dependencies;
            DROP TABLE extension_platform_packages;
            DROP TABLE extension_platform_version_claim_conflicts;
            DROP TABLE extension_platform_version_claims;
            DROP INDEX idx_extension_platform_snapshots_identity;
            DROP TABLE extension_platform_installations;

            CREATE TABLE extension_platform_installations (
                installation_id TEXT PRIMARY KEY,
                extension_id TEXT NOT NULL UNIQUE,
                active_snapshot_id TEXT NOT NULL,
                previous_snapshot_id TEXT,
                revision INTEGER NOT NULL,
                updated_at TEXT NOT NULL
            );

            INSERT INTO extension_platform_snapshots
                (snapshot_id, extension_id, version, package_hash, manifest_digest, created_at)
            VALUES ('snapshot-a', 'acme.git-guardian', '1.2.0',
                    '1111111111111111111111111111111111111111111111111111111111111111',
                    '2222222222222222222222222222222222222222222222222222222222222222',
                    '2026-08-22T00:00:00Z');

            INSERT INTO extension_platform_installations
                (installation_id, extension_id, active_snapshot_id, previous_snapshot_id, revision,
                 updated_at)
            VALUES ('install-a', 'acme.git-guardian', 'snapshot-a', NULL, 3,
                    '2026-08-22T00:00:00Z');
            "#,
        )
        .expect("pre-migration-86 fixture");

    let references_before: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pragma_foreign_key_list('extension_platform_installations')",
            [],
            |row| row.get(0),
        )
        .expect("references before");
    assert_eq!(references_before, 0, "the fixture must start without them");

    migrate(&connection).expect("upgrade migration");

    let (installation, snapshot, revision): (String, String, i64) = connection
        .query_row(
            "SELECT installation_id, active_snapshot_id, revision              FROM extension_platform_installations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("the installation row survives the rebuild");
    assert_eq!(installation, "install-a");
    assert_eq!(snapshot, "snapshot-a");
    assert_eq!(revision, 3, "and its revision is not reset");

    let references_after: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pragma_foreign_key_list('extension_platform_installations')",
            [],
            |row| row.get(0),
        )
        .expect("references after");
    assert_eq!(
        references_after, 2,
        "active and previous snapshot both reference the snapshots table"
    );
}

#[test]
fn migration_86_is_a_no_op_on_a_database_that_already_has_it() {
    // The rebuild is guarded on the references being absent, so re-running must not recreate the
    // table -- a second rebuild would be a second chance to lose rows.
    let connection = Connection::open_in_memory().expect("in-memory database");
    migrate(&connection).expect("current schema");
    connection
        .execute(
            "INSERT INTO extension_platform_packages                  (package_hash, byte_length, signature_state, first_seen_at)              VALUES ('1111111111111111111111111111111111111111111111111111111111111111', 1,                      'verified', '2026-08-22T00:00:00Z')",
            [],
        )
        .expect("evidence row");

    migrate(&connection).expect("second run");

    let packages: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM extension_platform_packages",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(packages, 1);
}
