//! Who may claim a connector subject id, and what a built-in descriptor is.

use super::{
    all_connector_seed_rejections, builtin_connector_catalog, decide_connector_owner,
    BuiltinConnectorDescriptor, ConnectorDefinitionDigest, ConnectorGlobalId, ConnectorSeedOutcome,
    ConnectorSeedRejection, ConnectorSnapshotRef, OwnerExtensionId, BUILTIN_CONNECTOR_OWNER,
    BUILTIN_CONNECTOR_SNAPSHOT,
};

const DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";

fn connector() -> ConnectorGlobalId {
    ConnectorGlobalId::parse("vanehub.github").expect("connector")
}

fn host() -> OwnerExtensionId {
    BuiltinConnectorDescriptor::owner()
}

fn extension_owner() -> OwnerExtensionId {
    OwnerExtensionId::parse("acme.mailer").expect("owner")
}

#[test]
fn an_unclaimed_id_may_be_seeded() {
    assert!(decide_connector_owner(&connector(), None, &host()).is_ok());
}

#[test]
fn the_same_owner_seeding_again_is_the_ordinary_repeated_launch() {
    assert!(decide_connector_owner(&connector(), Some(&host()), &host()).is_ok());
    assert!(
        decide_connector_owner(&connector(), Some(&extension_owner()), &extension_owner()).is_ok()
    );
}

#[test]
fn ownership_is_refused_in_both_directions() {
    let seed_over_extension =
        decide_connector_owner(&connector(), Some(&extension_owner()), &host())
            .expect_err("seed must not take over");
    let extension_over_builtin =
        decide_connector_owner(&connector(), Some(&host()), &extension_owner())
            .expect_err("extension must not take over");

    assert_eq!(
        seed_over_extension.code(),
        "builtin_connector_owner_conflict"
    );
    assert_eq!(
        extension_over_builtin.code(),
        "builtin_connector_owner_conflict"
    );
    let ConnectorSeedRejection::OwnerConflict {
        stored, offered, ..
    } = seed_over_extension
    else {
        panic!("expected an owner conflict");
    };
    assert_eq!(
        (stored.as_str(), offered.as_str()),
        ("acme.mailer", BUILTIN_CONNECTOR_OWNER),
        "the conflict reports which way round it was"
    );
}

#[test]
fn a_built_in_is_owned_by_a_real_reserved_id_rather_than_by_nothing() {
    // "Owned by the host" is a fact worth being able to query, and a nullable owner column would
    // make the ownership check partial.
    assert_eq!(host().as_str(), BUILTIN_CONNECTOR_OWNER);
    assert!(OwnerExtensionId::parse(BUILTIN_CONNECTOR_OWNER).is_ok());
}

#[test]
fn a_built_in_definition_is_recorded_under_a_reserved_snapshot() {
    let descriptor = BuiltinConnectorDescriptor {
        connector: connector(),
        digest: ConnectorDefinitionDigest::parse(DIGEST).expect("digest"),
    };

    assert_eq!(
        descriptor.snapshot(),
        ConnectorSnapshotRef::parse(BUILTIN_CONNECTOR_SNAPSHOT).expect("snapshot")
    );
    assert!(
        BUILTIN_CONNECTOR_SNAPSHOT.ends_with("-1"),
        "the generation suffix is what makes an upgrade a new revision rather than an edit"
    );
}

#[test]
fn the_shipped_catalog_is_empty_in_this_build() {
    // GitHub, the IM connectors, and the MCP projection each arrive with the Task Group 10 task
    // that brings the driver. Seeding descriptors now would pre-empt their decisions about legacy
    // ids and create rows nothing reads.
    assert!(builtin_connector_catalog().is_empty());
}

#[test]
fn every_outcome_and_rejection_has_a_distinct_stable_code() {
    let mut outcome_codes: Vec<&str> = [
        ConnectorSeedOutcome::Seeded,
        ConnectorSeedOutcome::AlreadySeeded,
        ConnectorSeedOutcome::RevisionAdded,
    ]
    .iter()
    .map(|outcome| outcome.code())
    .collect();
    let outcomes = outcome_codes.len();
    outcome_codes.sort_unstable();
    outcome_codes.dedup();
    assert_eq!(outcome_codes.len(), outcomes);

    let rejections = all_connector_seed_rejections();
    let total = rejections.len();
    let mut codes: Vec<&str> = rejections
        .iter()
        .map(ConnectorSeedRejection::code)
        .collect();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);
}
