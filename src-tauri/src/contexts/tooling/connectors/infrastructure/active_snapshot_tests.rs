//! Translating the platform's answer into this subdomain's vocabulary, and the rollback case.
//!
//! Assembled through `bootstrap` rather than by hand: the point of the adapter is that it goes
//! through the published API, and a test that constructed the reader directly would prove the
//! translation while skipping the seam it exists to cross.

use super::{ExtensionPlatformActiveConnector, UnknownActiveConnector};
use crate::bootstrap::assemble_extension_platform_api;
use crate::contexts::tooling::connectors::application::ActiveConnectorSnapshotPort;
use crate::contexts::tooling::connectors::domain::{
    ActiveConnectorSnapshot, ConnectorDefinitionDigest, ConnectorGlobalId, ConnectorSnapshotRef,
};
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
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use semver::Version;
use std::sync::Arc;

const V1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const V2: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const CONNECTOR: &str = "ext::acme.mailer::smtp";
const AT: &str = "2026-08-23T00:00:00Z";

fn connector() -> ConnectorGlobalId {
    ConnectorGlobalId::parse(CONNECTOR).expect("connector")
}

struct Fixture {
    _directory: TempDirectory,
    database: Arc<NativeDatabase>,
    port: ExtensionPlatformActiveConnector,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDirectory::new(label);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let port =
        ExtensionPlatformActiveConnector::new(assemble_extension_platform_api(database.clone()));
    Fixture {
        _directory: directory,
        database: Arc::new(database),
        port,
    }
}

/// Publishes one version of the extension and records what its snapshot contributes.
fn publish(fixture: &Fixture, snapshot: &str, version: &str, revision: i64, declared: &str) {
    SqliteSnapshotPointerRepository::new(
        fixture.database.clone(),
        InstallationId::parse("install-a").expect("installation"),
    )
    .point_at(
        &SnapshotRecord {
            snapshot: SnapshotId::parse(snapshot).expect("snapshot"),
            extension: ExtensionId::parse("acme.mailer").expect("extension"),
            version: Version::parse(version).expect("version"),
            package_hash: PackageHash::parse(V1).expect("hash"),
            manifest_digest: ManifestDigest::parse(V1).expect("digest"),
            created_at: AT.to_string(),
        },
        revision,
    )
    .expect("publish");
    record_snapshot_detail(
        &fixture.database,
        &SnapshotId::parse(snapshot).expect("snapshot"),
        &[],
        &[RecordedContribution {
            global_id: CONNECTOR.to_string(),
            kind: "connector".to_string(),
            local_id: "smtp".to_string(),
            declared_digest: Some(declared.to_string()),
        }],
    )
    .expect("detail");
}

fn activate(fixture: &Fixture, generation: &str, snapshot: &str, revision: i64) {
    let repository = SqliteRuntimeGenerationRepository::new(fixture.database.clone());
    let installation = InstallationId::parse("install-a").expect("installation");
    repository
        .record(&RuntimeGenerationRecord {
            generation: RuntimeGenerationId::parse(generation).expect("generation"),
            installation: installation.clone(),
            snapshot: SnapshotId::parse(snapshot).expect("snapshot"),
            started_at: AT.to_string(),
        })
        .expect("record");
    repository
        .activate(
            &installation,
            &RuntimeGenerationId::parse(generation).expect("generation"),
            revision,
            AT,
        )
        .expect("activate");
}

#[test]
fn a_running_contribution_arrives_as_a_parsed_snapshot_and_digest() {
    let fixture = fixture("connector-port-running");
    publish(&fixture, "snap-v1", "1.0.0", 0, V1);
    activate(&fixture, "generation-1", "snap-v1", 0);

    assert_eq!(
        fixture.port.active_snapshot(&connector()).expect("read"),
        ActiveConnectorSnapshot::Running {
            snapshot: ConnectorSnapshotRef::parse("snap-v1").expect("snapshot"),
            declared: Some(ConnectorDefinitionDigest::parse(V1).expect("digest")),
        }
    );
}

#[test]
fn a_rollback_moves_the_answer_back_to_the_older_snapshot() {
    // The reconciliation case that recording order gets wrong: v2 is published and its
    // contribution recorded, so it leads every "most recent" ordering -- and the pointer says v1.
    let fixture = fixture("connector-port-rollback");
    publish(&fixture, "snap-v1", "1.0.0", 0, V1);
    publish(&fixture, "snap-v2", "2.0.0", 1, V2);
    activate(&fixture, "generation-1", "snap-v1", 0);
    activate(&fixture, "generation-2", "snap-v2", 1);

    assert_eq!(
        fixture.port.active_snapshot(&connector()).expect("read"),
        ActiveConnectorSnapshot::Running {
            snapshot: ConnectorSnapshotRef::parse("snap-v2").expect("snapshot"),
            declared: Some(ConnectorDefinitionDigest::parse(V2).expect("digest")),
        }
    );

    activate(&fixture, "generation-3", "snap-v1", 2);

    assert_eq!(
        fixture.port.active_snapshot(&connector()).expect("read"),
        ActiveConnectorSnapshot::Running {
            snapshot: ConnectorSnapshotRef::parse("snap-v1").expect("snapshot"),
            declared: Some(ConnectorDefinitionDigest::parse(V1).expect("digest")),
        },
        "the rollback moves the answer even though v2 is still the newest recorded"
    );
}

#[test]
fn an_installation_with_no_active_generation_is_installed_but_not_running() {
    let fixture = fixture("connector-port-no-generation");
    publish(&fixture, "snap-v1", "1.0.0", 0, V1);

    assert_eq!(
        fixture.port.active_snapshot(&connector()).expect("read"),
        ActiveConnectorSnapshot::NoActiveGeneration
    );
}

#[test]
fn nothing_installed_arrives_as_not_installed() {
    let fixture = fixture("connector-port-absent");

    assert_eq!(
        fixture.port.active_snapshot(&connector()).expect("read"),
        ActiveConnectorSnapshot::NotInstalled
    );
}

#[test]
fn a_declaration_this_subdomain_cannot_parse_is_discarded_rather_than_compared() {
    // Comparing an unparsed string would make `drifted` depend on byte equality of something
    // neither side validated.
    let fixture = fixture("connector-port-unparseable");
    publish(&fixture, "snap-v1", "1.0.0", 0, "not a digest");
    activate(&fixture, "generation-1", "snap-v1", 0);

    assert_eq!(
        fixture.port.active_snapshot(&connector()).expect("read"),
        ActiveConnectorSnapshot::Running {
            snapshot: ConnectorSnapshotRef::parse("snap-v1").expect("snapshot"),
            declared: None,
        }
    );
}

#[test]
fn the_conservative_port_never_reports_anything_running() {
    assert_eq!(
        UnknownActiveConnector
            .active_snapshot(&connector())
            .expect("read"),
        ActiveConnectorSnapshot::Unknown
    );
}
