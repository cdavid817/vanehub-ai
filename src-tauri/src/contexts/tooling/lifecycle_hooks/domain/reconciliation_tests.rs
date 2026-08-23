//! The four states a subject can be in relative to the snapshot its definition names.

use super::{
    judge_subject, DefinitionDigest, HookGlobalId, SnapshotFact, SnapshotRef, SubjectProjection,
    SubjectReadiness, ALL_SUBJECT_READINESS,
};

const FIRST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn digest(value: &str) -> DefinitionDigest {
    DefinitionDigest::parse(value).expect("digest")
}

fn snapshot() -> SnapshotRef {
    SnapshotRef::parse("snap-a").expect("snapshot")
}

fn projection(revision: Option<(SnapshotRef, DefinitionDigest)>) -> SubjectProjection {
    SubjectProjection {
        hook: HookGlobalId::parse("ext::acme.git-guardian::pre-commit").expect("hook"),
        revision,
    }
}

#[test]
fn a_definition_whose_snapshot_agrees_with_it_is_ready() {
    let verdict = judge_subject(
        &projection(Some((snapshot(), digest(FIRST)))),
        Some(&SnapshotFact {
            snapshot: snapshot(),
            hook_digest: Some(digest(FIRST)),
        }),
    );

    assert_eq!(verdict.readiness, SubjectReadiness::Ready);
    assert!(verdict.readiness.admits_dispatch());
    assert_eq!(verdict.snapshot, Some(snapshot()));
}

#[test]
fn a_definition_whose_snapshot_is_gone_is_orphaned() {
    let verdict = judge_subject(&projection(Some((snapshot(), digest(FIRST)))), None);

    assert_eq!(verdict.readiness, SubjectReadiness::Orphaned);
    assert!(!verdict.readiness.admits_dispatch());
    assert_eq!(
        verdict.snapshot,
        Some(snapshot()),
        "the verdict must name the snapshot that went missing, or nobody can find out which"
    );
}

#[test]
fn a_subject_with_no_revision_at_all_is_unavailable_rather_than_an_error() {
    // An extension installed but not activated looks exactly like this, and so does a subject that
    // exists only because a binding or an execution mentions it. Neither is a fault.
    let verdict = judge_subject(&projection(None), None);

    assert_eq!(verdict.readiness, SubjectReadiness::Unavailable);
    assert_eq!(verdict.snapshot, None);
}

#[test]
fn a_snapshot_that_no_longer_contributes_the_hook_is_unavailable() {
    let verdict = judge_subject(
        &projection(Some((snapshot(), digest(FIRST)))),
        Some(&SnapshotFact {
            snapshot: snapshot(),
            hook_digest: None,
        }),
    );

    assert_eq!(verdict.readiness, SubjectReadiness::Unavailable);
    assert!(!verdict.readiness.admits_dispatch());
}

#[test]
fn a_definition_that_disagrees_with_its_snapshot_is_drifted_and_does_not_dispatch() {
    // Dispatching a drifted definition runs something other than what was installed, which is
    // worse than not running at all: the operator believes the reviewed version is in effect.
    let verdict = judge_subject(
        &projection(Some((snapshot(), digest(FIRST)))),
        Some(&SnapshotFact {
            snapshot: snapshot(),
            hook_digest: Some(digest(SECOND)),
        }),
    );

    assert_eq!(verdict.readiness, SubjectReadiness::Drifted);
    assert!(!verdict.readiness.admits_dispatch());
}

#[test]
fn only_ready_dispatches() {
    for readiness in ALL_SUBJECT_READINESS.iter().copied() {
        assert_eq!(
            readiness.admits_dispatch(),
            readiness == SubjectReadiness::Ready,
            "{readiness:?}"
        );
    }
}

#[test]
fn every_readiness_spelling_is_distinct() {
    let mut spellings: Vec<&str> = ALL_SUBJECT_READINESS
        .iter()
        .map(|readiness| readiness.as_str())
        .collect();
    let total = spellings.len();
    spellings.sort_unstable();
    spellings.dedup();
    assert_eq!(spellings.len(), total);
}

#[test]
fn judging_is_a_read_and_the_verdict_carries_the_subject_it_is_about() {
    // Nothing here deletes, rebinds, or activates. The verdict is the entire output, so a caller
    // that wanted to act on it has to do so explicitly rather than by having called this.
    let subject = projection(Some((snapshot(), digest(FIRST))));

    let verdict = judge_subject(&subject, None);

    assert_eq!(verdict.hook, subject.hook);
    assert_eq!(
        subject.revision,
        Some((snapshot(), digest(FIRST))),
        "the projection is unchanged; judging stores nothing"
    );
}

#[test]
fn the_same_input_judges_the_same_way_every_time() {
    // Nothing here may depend on a clock or on iteration order; a verdict that reshuffled would
    // make a stored diagnostic unmatchable.
    let subject = projection(Some((snapshot(), digest(FIRST))));
    let fact = SnapshotFact {
        snapshot: snapshot(),
        hook_digest: Some(digest(SECOND)),
    };

    assert_eq!(
        judge_subject(&subject, Some(&fact)),
        judge_subject(&subject, Some(&fact))
    );
}
