//! What seeding a built-in connector does, and the user state it must never touch.

use super::{
    SqliteConnectorBindingRepository, SqliteConnectorDefinitionRepository,
    SqliteConnectorInstanceRepository, SqliteConnectorSubjectRepository,
};
use crate::contexts::tooling::connectors::application::{
    seed_builtin_connectors, ConnectorBindingRepository, ConnectorDefinitionRepository,
    ConnectorInstanceRepository, ConnectorSeedReport, ConnectorSubjectRepository,
};
use crate::contexts::tooling::connectors::domain::{
    builtin_connector_catalog, BuiltinConnectorDescriptor, ConnectorDefinitionDigest,
    ConnectorDefinitionRevision, ConnectorGlobalId, ConnectorSnapshotRef, ConnectorSubject,
    ConnectorTarget, CredentialHandle, DisplayLabel, InstanceEdit, InstanceId, OwnerExtensionId,
    PublicConfiguration, ABSENT_REVISION, BUILTIN_CONNECTOR_OWNER, BUILTIN_CONNECTOR_SNAPSHOT,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use std::sync::Arc;

const FIRST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const CONNECTOR: &str = "vanehub.github";
const AT: &str = "2026-08-23T00:00:00Z";

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

fn connector() -> ConnectorGlobalId {
    ConnectorGlobalId::parse(CONNECTOR).expect("connector")
}

fn descriptor(digest: &str) -> BuiltinConnectorDescriptor {
    BuiltinConnectorDescriptor {
        connector: connector(),
        digest: ConnectorDefinitionDigest::parse(digest).expect("digest"),
    }
}

fn seed(
    fixture: &Fixture,
    catalog: &[BuiltinConnectorDescriptor],
) -> Result<ConnectorSeedReport, crate::contexts::tooling::connectors::domain::ConnectorSeedRejection>
{
    seed_builtin_connectors(
        &SqliteConnectorSubjectRepository::new(fixture.database.clone()),
        &SqliteConnectorDefinitionRepository::new(fixture.database.clone()),
        catalog,
        AT,
    )
}

#[test]
fn the_shipped_catalog_seeds_without_error() {
    // Empty in this build: GitHub, the IM connectors, and the MCP projection each arrive with the
    // Task Group 10 task that also brings the driver. This asserts the wiring, and will start
    // asserting content for free.
    let fixture = fixture("connector-seed-shipped");

    let report = seed(&fixture, &builtin_connector_catalog()).expect("seed");

    assert_eq!(report, ConnectorSeedReport::default());
    assert!(!report.changed_anything());
}

#[test]
fn a_first_launch_creates_the_subject_under_the_reserved_owner() {
    let fixture = fixture("connector-seed-first");

    let report = seed(&fixture, &[descriptor(FIRST)]).expect("seed");

    assert_eq!(report.seeded, 1);
    let stored = SqliteConnectorSubjectRepository::new(fixture.database.clone())
        .get(&connector())
        .expect("get")
        .expect("present");
    assert_eq!(stored.owner_extension.as_str(), BUILTIN_CONNECTOR_OWNER);
}

#[test]
fn a_repeated_launch_changes_nothing() {
    let fixture = fixture("connector-seed-repeat");
    seed(&fixture, &[descriptor(FIRST)]).expect("seed");

    let report = seed(&fixture, &[descriptor(FIRST)]).expect("re-seed");

    assert_eq!(report.already_seeded, 1);
    assert!(!report.changed_anything());
    assert_eq!(
        SqliteConnectorSubjectRepository::new(fixture.database.clone())
            .get(&connector())
            .expect("get")
            .expect("present")
            .first_seen_at,
        AT
    );
}

#[test]
fn an_upgrade_adds_a_revision_beside_the_old_one() {
    let fixture = fixture("connector-seed-upgrade");
    seed(&fixture, &[descriptor(FIRST)]).expect("seed");
    let definitions = SqliteConnectorDefinitionRepository::new(fixture.database.clone());

    definitions
        .record(&ConnectorDefinitionRevision {
            snapshot: ConnectorSnapshotRef::parse("builtin-2").expect("snapshot"),
            connector: connector(),
            digest: ConnectorDefinitionDigest::parse(SECOND).expect("digest"),
            recorded_at: AT.to_string(),
        })
        .expect("record the upgraded revision");

    let revisions = definitions.revisions(&connector()).expect("revisions");
    assert_eq!(revisions.len(), 2);
    assert!(revisions
        .iter()
        .any(|revision| revision.snapshot.as_str() == BUILTIN_CONNECTOR_SNAPSHOT));
}

#[test]
fn the_same_built_in_snapshot_with_a_different_definition_is_refused() {
    let fixture = fixture("connector-seed-definition-conflict");
    seed(&fixture, &[descriptor(FIRST)]).expect("seed");

    let error = seed(&fixture, &[descriptor(SECOND)]).expect_err("definition conflict");

    assert_eq!(error.code(), "builtin_connector_definition_conflict");
}

#[test]
fn a_seed_never_takes_over_an_extensions_subject() {
    let fixture = fixture("connector-seed-owner-conflict");
    SqliteConnectorSubjectRepository::new(fixture.database.clone())
        .ensure(&ConnectorSubject {
            connector: connector(),
            owner_extension: OwnerExtensionId::parse("acme.mailer").expect("owner"),
            first_seen_at: AT.to_string(),
        })
        .expect("an extension claimed it first");

    let error = seed(&fixture, &[descriptor(FIRST)]).expect_err("owner conflict");

    assert_eq!(error.code(), "builtin_connector_owner_conflict");
    assert_eq!(
        SqliteConnectorSubjectRepository::new(fixture.database.clone())
            .get(&connector())
            .expect("get")
            .expect("present")
            .owner_extension
            .as_str(),
        "acme.mailer",
        "the stored owner is untouched -- there is no INSERT OR REPLACE in this path"
    );
}

#[test]
fn seeding_creates_no_instance_binding_or_credential_and_preserves_the_ones_a_user_made() {
    // The rule with the most at stake here: an instance carries a credential handle, so a seed
    // that touched instances would be a launch-time process next to a person's secrets.
    let fixture = fixture("connector-seed-user-state");
    seed(&fixture, &[descriptor(FIRST)]).expect("seed");

    let instances = SqliteConnectorInstanceRepository::new(fixture.database.clone());
    assert!(
        instances
            .for_connector(&connector())
            .expect("instances")
            .is_empty(),
        "seeding creates no instance at all"
    );

    // A user configures one, with a credential and a binding.
    let instance = InstanceId::parse("instance-1").expect("instance");
    instances
        .save(&InstanceEdit {
            instance: &instance,
            connector: &connector(),
            label: &DisplayLabel::parse("Work account").expect("label"),
            desired_enabled: false,
            configuration: &PublicConfiguration::of(&[("base_url", "https://ghe.test")])
                .expect("config"),
            expected_revision: ABSENT_REVISION,
            at: AT,
        })
        .expect("configure");
    instances
        .attach_credential(
            &instance,
            Some(&CredentialHandle::parse("cred-1").expect("handle")),
            1,
            AT,
        )
        .expect("attach");
    SqliteConnectorBindingRepository::new(fixture.database.clone())
        .set(
            &crate::contexts::tooling::connectors::domain::BindingId::parse("binding-1")
                .expect("binding"),
            &instance,
            &ConnectorTarget::global(),
            true,
            ABSENT_REVISION,
            AT,
        )
        .expect("bind");

    seed(&fixture, &[descriptor(FIRST)]).expect("re-seed");

    let held = instances.get(&instance).expect("get").expect("present");
    assert!(!held.desired_enabled, "the user's enablement survives");
    assert_eq!(held.display_label.as_str(), "Work account");
    assert!(
        held.credential.is_some(),
        "and so does the credential handle"
    );
    assert_eq!(
        held.revision, 2,
        "the seed did not write, so the revision did not move"
    );
    assert_eq!(
        SqliteConnectorBindingRepository::new(fixture.database.clone())
            .bindings(&instance)
            .expect("bindings")
            .len(),
        1
    );
}

#[test]
fn a_rejection_partway_through_leaves_the_earlier_descriptors_applied() {
    let fixture = fixture("connector-seed-partial");
    SqliteConnectorSubjectRepository::new(fixture.database.clone())
        .ensure(&ConnectorSubject {
            connector: ConnectorGlobalId::parse("vanehub.second").expect("connector"),
            owner_extension: OwnerExtensionId::parse("acme.mailer").expect("owner"),
            first_seen_at: AT.to_string(),
        })
        .expect("an extension owns the second id");

    let error = seed(
        &fixture,
        &[
            descriptor(FIRST),
            BuiltinConnectorDescriptor {
                connector: ConnectorGlobalId::parse("vanehub.second").expect("connector"),
                digest: ConnectorDefinitionDigest::parse(SECOND).expect("digest"),
            },
        ],
    )
    .expect_err("the second descriptor conflicts");

    assert_eq!(error.code(), "builtin_connector_owner_conflict");
    let subjects = SqliteConnectorSubjectRepository::new(fixture.database.clone());
    assert!(
        subjects.get(&connector()).expect("get").is_some(),
        "the descriptor that succeeded stays applied"
    );
}

#[test]
fn two_concurrent_seeds_leave_exactly_one_subject_and_one_definition() {
    // The seed runs at startup, and nothing serialises two processes.
    let fixture = fixture("connector-seed-concurrent");

    let left_database = fixture.database.clone();
    let right_database = fixture.database.clone();
    let left = std::thread::spawn(move || {
        seed_builtin_connectors(
            &SqliteConnectorSubjectRepository::new(left_database.clone()),
            &SqliteConnectorDefinitionRepository::new(left_database),
            &[descriptor(FIRST)],
            AT,
        )
    });
    let right = std::thread::spawn(move || {
        seed_builtin_connectors(
            &SqliteConnectorSubjectRepository::new(right_database.clone()),
            &SqliteConnectorDefinitionRepository::new(right_database),
            &[descriptor(FIRST)],
            AT,
        )
    });

    let outcomes = [left.join().expect("thread"), right.join().expect("thread")];
    for outcome in &outcomes {
        assert!(outcome.is_ok(), "neither seed may fail: {outcomes:?}");
    }
    assert_eq!(
        SqliteConnectorSubjectRepository::new(fixture.database.clone())
            .all()
            .expect("all")
            .len(),
        1
    );
    assert_eq!(
        SqliteConnectorDefinitionRepository::new(fixture.database.clone())
            .revisions(&connector())
            .expect("revisions")
            .len(),
        1
    );
}
