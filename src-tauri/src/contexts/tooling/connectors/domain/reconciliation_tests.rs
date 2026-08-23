//! The four states a connector subject can be in relative to what the platform is running.

use super::{
    judge_connector, ActiveConnectorSnapshot, ConnectorDefinitionDigest, ConnectorFacts,
    ConnectorGlobalId, ConnectorReadiness, ConnectorSnapshotRef, ALL_CONNECTOR_READINESS,
};

const FIRST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn digest(value: &str) -> ConnectorDefinitionDigest {
    ConnectorDefinitionDigest::parse(value).expect("digest")
}

fn snapshot(value: &str) -> ConnectorSnapshotRef {
    ConnectorSnapshotRef::parse(value).expect("snapshot")
}

fn facts(active: ActiveConnectorSnapshot, recorded: Option<&str>, has_any: bool) -> ConnectorFacts {
    ConnectorFacts {
        connector: ConnectorGlobalId::parse("ext::acme.mailer::smtp").expect("connector"),
        active,
        recorded_at_active: recorded.map(digest),
        has_any_revision: has_any,
    }
}

fn running(snapshot_id: &str, declared: Option<&str>) -> ActiveConnectorSnapshot {
    ActiveConnectorSnapshot::Running {
        snapshot: snapshot(snapshot_id),
        declared: declared.map(digest),
    }
}

#[test]
fn the_running_snapshot_and_the_recorded_definition_agreeing_is_ready() {
    let verdict = judge_connector(&facts(running("snap-a", Some(FIRST)), Some(FIRST), true));

    assert_eq!(verdict.readiness, ConnectorReadiness::Ready);
    assert!(verdict.readiness.admits_connect());
    assert_eq!(verdict.snapshot, Some(snapshot("snap-a")));
}

#[test]
fn a_recorded_definition_that_disagrees_with_the_running_snapshot_is_drifted() {
    // Connecting on a drifted definition dials whatever the stale one said, while the operator
    // believes the reviewed endpoint and scopes are in effect.
    let verdict = judge_connector(&facts(running("snap-a", Some(FIRST)), Some(SECOND), true));

    assert_eq!(verdict.readiness, ConnectorReadiness::Drifted);
    assert!(!verdict.readiness.admits_connect());
    assert_eq!(verdict.snapshot, Some(snapshot("snap-a")));
}

#[test]
fn a_running_snapshot_that_does_not_declare_the_connector_is_unavailable() {
    let verdict = judge_connector(&facts(running("snap-b", None), None, true));

    assert_eq!(verdict.readiness, ConnectorReadiness::Unavailable);
    assert_eq!(verdict.snapshot, None);
}

#[test]
fn a_running_snapshot_this_subdomain_has_no_definition_for_is_unavailable() {
    let verdict = judge_connector(&facts(running("snap-a", Some(FIRST)), None, false));

    assert_eq!(verdict.readiness, ConnectorReadiness::Unavailable);
}

#[test]
fn no_active_generation_is_unavailable_rather_than_ready() {
    let verdict = judge_connector(&facts(
        ActiveConnectorSnapshot::NoActiveGeneration,
        None,
        true,
    ));

    assert_eq!(verdict.readiness, ConnectorReadiness::Unavailable);
    assert_eq!(verdict.snapshot, None);
}

#[test]
fn an_unreachable_platform_is_unavailable_and_never_ready() {
    // Not knowing is not the same as being ready. Every path from `Unknown` is `Unavailable`,
    // including the one where a definition is sitting right there.
    let verdict = judge_connector(&facts(ActiveConnectorSnapshot::Unknown, Some(FIRST), true));

    assert_eq!(verdict.readiness, ConnectorReadiness::Unavailable);
    assert!(!verdict.readiness.admits_connect());
}

#[test]
fn an_uninstalled_extension_leaves_its_connector_orphaned_when_evidence_remains() {
    let verdict = judge_connector(&facts(ActiveConnectorSnapshot::NotInstalled, None, true));

    assert_eq!(verdict.readiness, ConnectorReadiness::Orphaned);
    assert!(!verdict.readiness.admits_connect());
}

#[test]
fn a_subject_with_no_definition_at_all_is_unavailable_rather_than_orphaned() {
    let verdict = judge_connector(&facts(ActiveConnectorSnapshot::NotInstalled, None, false));

    assert_eq!(verdict.readiness, ConnectorReadiness::Unavailable);
}

#[test]
fn only_ready_admits_a_connect() {
    // Readiness gates *new* connects and nothing else. It never removes an instance, a binding, or
    // a credential handle -- see `instance.rs`.
    for readiness in ALL_CONNECTOR_READINESS.iter().copied() {
        assert_eq!(
            readiness.admits_connect(),
            readiness == ConnectorReadiness::Ready,
            "{readiness:?}"
        );
    }
}

#[test]
fn every_readiness_spelling_is_distinct() {
    let mut spellings: Vec<&str> = ALL_CONNECTOR_READINESS
        .iter()
        .map(|readiness| readiness.as_str())
        .collect();
    let total = spellings.len();
    spellings.sort_unstable();
    spellings.dedup();
    assert_eq!(spellings.len(), total);
}

#[test]
fn judging_is_a_read_and_the_same_input_judges_the_same_way_every_time() {
    // Nothing here deletes, rebinds, or activates, and nothing depends on a clock -- a verdict
    // that reshuffled would make a stored diagnostic unmatchable.
    let subject = facts(running("snap-a", Some(FIRST)), Some(SECOND), true);

    let verdict = judge_connector(&subject);

    assert_eq!(verdict.connector, subject.connector);
    assert_eq!(judge_connector(&subject), verdict);
    assert_eq!(
        subject.recorded_at_active,
        Some(digest(SECOND)),
        "the facts are unchanged; judging stores nothing"
    );
}
