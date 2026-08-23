//! What recording a connector definition means, and what a second recording of the pair means.

use super::{
    decide_connector_definition, ConnectorDefinitionContentConflict, ConnectorDefinitionDigest,
    ConnectorDefinitionOutcome, ConnectorDefinitionRevision, ConnectorGlobalId,
    ConnectorSnapshotRef, ConnectorSubject, OwnerExtensionId,
};

const FIRST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn revision(snapshot: &str, digest: &str, at: &str) -> ConnectorDefinitionRevision {
    ConnectorDefinitionRevision {
        snapshot: ConnectorSnapshotRef::parse(snapshot).expect("snapshot"),
        connector: ConnectorGlobalId::parse("ext::acme.mailer::smtp").expect("connector"),
        digest: ConnectorDefinitionDigest::parse(digest).expect("digest"),
        recorded_at: at.to_string(),
    }
}

#[test]
fn an_unrecorded_pair_is_bound_by_the_first_revision_to_claim_it() {
    let outcome =
        decide_connector_definition(&revision("snap-a", FIRST, "2026-08-01T00:00:00Z"), None);

    assert_eq!(outcome, ConnectorDefinitionOutcome::Recorded);
    assert!(outcome.admits_connect());
}

#[test]
fn reinstalling_a_snapshot_re_records_its_definitions_idempotently() {
    let recorded = revision("snap-a", FIRST, "2026-08-01T00:00:00Z");
    let again = revision("snap-a", FIRST, "2026-08-20T00:00:00Z");

    let outcome = decide_connector_definition(&again, Some(&recorded));

    assert_eq!(outcome, ConnectorDefinitionOutcome::AlreadyRecorded);
    assert!(outcome.admits_connect());
}

#[test]
fn the_same_pair_with_a_different_definition_is_refused_and_both_digests_are_reported() {
    // Connecting on either would be a guess about which endpoint and which scopes were reviewed.
    let recorded = revision("snap-a", FIRST, "2026-08-01T00:00:00Z");
    let offered = revision("snap-a", SECOND, "2026-08-20T00:00:00Z");

    let outcome = decide_connector_definition(&offered, Some(&recorded));

    assert_eq!(
        outcome,
        ConnectorDefinitionOutcome::Conflict(ConnectorDefinitionContentConflict {
            recorded_digest: ConnectorDefinitionDigest::parse(FIRST).expect("digest"),
            offered_digest: ConnectorDefinitionDigest::parse(SECOND).expect("digest"),
            recorded_at: "2026-08-01T00:00:00Z".to_string(),
        })
    );
    assert!(!outcome.admits_connect());
    assert_eq!(outcome.code(), "connector_definition_content_conflict");
    let ConnectorDefinitionOutcome::Conflict(conflict) = &outcome else {
        panic!("expected a conflict");
    };
    assert_eq!(
        conflict.code(),
        outcome.code(),
        "the conflict and the outcome name the same finding"
    );
}

#[test]
fn two_snapshots_may_each_record_the_same_subject() {
    // The point of versioning by snapshot: an upgrade records beside the old revision rather than
    // over it, so a rollback still has something to connect with.
    let old = revision("snap-a", FIRST, "2026-08-01T00:00:00Z");
    let new = revision("snap-b", SECOND, "2026-08-20T00:00:00Z");

    assert_ne!(old.snapshot, new.snapshot);
    assert_eq!(
        decide_connector_definition(&new, None),
        ConnectorDefinitionOutcome::Recorded
    );
}

#[test]
fn a_subject_records_which_extension_contributes_it() {
    // So an operator looking at an orphaned connector can find the package to uninstall. It is
    // opaque text: `extension_platform` owns extensions, and this carries no reference.
    let subject = ConnectorSubject {
        connector: ConnectorGlobalId::parse("ext::acme.mailer::smtp").expect("connector"),
        owner_extension: OwnerExtensionId::parse("acme.mailer").expect("owner"),
        first_seen_at: "2026-08-01T00:00:00Z".to_string(),
    };

    assert_eq!(subject.owner_extension.as_str(), "acme.mailer");
}

#[test]
fn every_outcome_has_a_distinct_stable_code() {
    let outcomes = [
        ConnectorDefinitionOutcome::Recorded,
        ConnectorDefinitionOutcome::AlreadyRecorded,
        ConnectorDefinitionOutcome::Conflict(ConnectorDefinitionContentConflict {
            recorded_digest: ConnectorDefinitionDigest::parse(FIRST).expect("digest"),
            offered_digest: ConnectorDefinitionDigest::parse(SECOND).expect("digest"),
            recorded_at: String::new(),
        }),
    ];
    let mut codes: Vec<&str> = outcomes
        .iter()
        .map(ConnectorDefinitionOutcome::code)
        .collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);
}
