//! Translating the platform's answer into this subdomain's vocabulary.
//!
//! Assembled through `bootstrap`, not by hand: the point of the adapter is that it goes through
//! the published API, and a test that constructed the reader directly would prove the translation
//! while skipping the seam it exists to cross.

use super::{ExtensionPlatformActiveSnapshot, UnknownActiveSnapshot};
use crate::bootstrap::assemble_extension_platform_api;
use crate::contexts::tooling::extension_platform::application::{
    RuntimeGenerationRepository, SnapshotPointerRepository,
};
use crate::contexts::tooling::extension_platform::domain::{
    ExtensionId, InstallationId, ManifestDigest, PackageHash, RuntimeGenerationId,
    RuntimeGenerationRecord, SnapshotId, SnapshotRecord,
};
use crate::contexts::tooling::extension_platform::infrastructure::{
    record_snapshot_detail, RecordedContribution, SqliteRuntimeGenerationRepository,
    SqliteSnapshotPointerRepository,
};
use crate::contexts::tooling::lifecycle_hooks::application::ActiveExtensionSnapshotPort;
use crate::contexts::tooling::lifecycle_hooks::domain::{
    ActiveSnapshot, DefinitionDigest, HookGlobalId, SnapshotRef,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use semver::Version;
use std::sync::Arc;

const DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const HOOK: &str = "ext::acme.git-guardian::pre-commit";
const AT: &str = "2026-08-22T00:00:00Z";

fn hook() -> HookGlobalId {
    HookGlobalId::parse(HOOK).expect("hook")
}

struct Fixture {
    _directory: TempDirectory,
    database: Arc<NativeDatabase>,
    port: ExtensionPlatformActiveSnapshot,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDirectory::new(label);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let port =
        ExtensionPlatformActiveSnapshot::new(assemble_extension_platform_api(database.clone()));
    Fixture {
        _directory: directory,
        database: Arc::new(database),
        port,
    }
}

fn publish_and_activate(fixture: &Fixture, declared: Option<&str>) {
    let installation = InstallationId::parse("install-a").expect("installation");
    SqliteSnapshotPointerRepository::new(fixture.database.clone(), installation.clone())
        .point_at(
            &SnapshotRecord {
                snapshot: SnapshotId::parse("snap-v1").expect("snapshot"),
                extension: ExtensionId::parse("acme.git-guardian").expect("extension"),
                version: Version::parse("1.0.0").expect("version"),
                package_hash: PackageHash::parse(DIGEST).expect("hash"),
                manifest_digest: ManifestDigest::parse(DIGEST).expect("digest"),
                created_at: AT.to_string(),
            },
            0,
        )
        .expect("publish");
    record_snapshot_detail(
        &fixture.database,
        &SnapshotId::parse("snap-v1").expect("snapshot"),
        &[],
        &[RecordedContribution {
            global_id: HOOK.to_string(),
            kind: "hook".to_string(),
            local_id: "pre-commit".to_string(),
            declared_digest: declared.map(str::to_string),
        }],
    )
    .expect("detail");

    let generations = SqliteRuntimeGenerationRepository::new(fixture.database.clone());
    let generation = RuntimeGenerationId::parse("generation-1").expect("generation");
    generations
        .record(&RuntimeGenerationRecord {
            generation: generation.clone(),
            installation: installation.clone(),
            snapshot: SnapshotId::parse("snap-v1").expect("snapshot"),
            started_at: AT.to_string(),
        })
        .expect("record");
    generations
        .activate(&installation, &generation, 0, AT)
        .expect("activate");
}

#[test]
fn a_running_contribution_arrives_as_a_parsed_snapshot_and_digest() {
    let fixture = fixture("hooks-port-running");
    publish_and_activate(&fixture, Some(DIGEST));

    let answer = fixture.port.active_snapshot(&hook()).expect("read");

    assert_eq!(
        answer,
        ActiveSnapshot::Running {
            snapshot: SnapshotRef::parse("snap-v1").expect("snapshot"),
            declared: Some(DefinitionDigest::parse(DIGEST).expect("digest")),
        }
    );
}

#[test]
fn nothing_installed_arrives_as_not_installed() {
    let fixture = fixture("hooks-port-absent");

    assert_eq!(
        fixture.port.active_snapshot(&hook()).expect("read"),
        ActiveSnapshot::NotInstalled
    );
}

#[test]
fn a_declaration_this_subdomain_cannot_parse_is_discarded_rather_than_compared() {
    // Comparing an unparsed string would make `drifted` depend on byte equality of something
    // neither side validated -- a digest with a stray space would read as drift forever.
    let fixture = fixture("hooks-port-unparseable");
    publish_and_activate(&fixture, Some("not a digest"));

    assert_eq!(
        fixture.port.active_snapshot(&hook()).expect("read"),
        ActiveSnapshot::Running {
            snapshot: SnapshotRef::parse("snap-v1").expect("snapshot"),
            declared: None,
        }
    );
}

#[test]
fn the_conservative_port_never_reports_anything_running() {
    // The answer when the platform cannot be asked at all. Every path from `Unknown` is
    // `Unavailable`, so a subsystem that is not wired yet cannot make a Hook look ready.
    assert_eq!(
        UnknownActiveSnapshot
            .active_snapshot(&hook())
            .expect("read"),
        ActiveSnapshot::Unknown
    );
}
