//! The four states a subject can be in relative to the snapshot the platform is running.

use super::{
    judge_subject, ActiveSnapshot, DefinitionDigest, HookGlobalId, SnapshotRef, SubjectFacts,
    SubjectReadiness, ALL_SUBJECT_READINESS,
};

const FIRST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn digest(value: &str) -> DefinitionDigest {
    DefinitionDigest::parse(value).expect("digest")
}

fn snapshot(value: &str) -> SnapshotRef {
    SnapshotRef::parse(value).expect("snapshot")
}

fn facts(active: ActiveSnapshot, recorded: Option<&str>, has_any: bool) -> SubjectFacts {
    SubjectFacts {
        hook: HookGlobalId::parse("ext::acme.git-guardian::pre-commit").expect("hook"),
        active,
        recorded_at_active: recorded.map(digest),
        has_any_revision: has_any,
    }
}

fn running(snapshot_id: &str, declared: Option<&str>) -> ActiveSnapshot {
    ActiveSnapshot::Running {
        snapshot: snapshot(snapshot_id),
        declared: declared.map(digest),
    }
}

#[test]
fn the_running_snapshot_and_the_recorded_definition_agreeing_is_ready() {
    let verdict = judge_subject(&facts(running("snap-a", Some(FIRST)), Some(FIRST), true));

    assert_eq!(verdict.readiness, SubjectReadiness::Ready);
    assert!(verdict.readiness.admits_dispatch());
    assert_eq!(
        verdict.snapshot,
        Some(snapshot("snap-a")),
        "a ready verdict must name what would run, or nobody can check it"
    );
}

#[test]
fn a_recorded_definition_that_disagrees_with_the_running_snapshot_is_drifted() {
    // Worse than not running: the operator believes the reviewed version is in effect while
    // something else would execute.
    let verdict = judge_subject(&facts(running("snap-a", Some(FIRST)), Some(SECOND), true));

    assert_eq!(verdict.readiness, SubjectReadiness::Drifted);
    assert!(!verdict.readiness.admits_dispatch());
    assert_eq!(verdict.snapshot, Some(snapshot("snap-a")));
}

#[test]
fn a_running_snapshot_that_does_not_declare_the_hook_is_unavailable() {
    // The newer version dropped the Hook. There is nothing to dispatch, and the one thing that
    // must not happen is falling back to a revision from some other snapshot.
    let verdict = judge_subject(&facts(running("snap-b", None), None, true));

    assert_eq!(verdict.readiness, SubjectReadiness::Unavailable);
    assert_eq!(
        verdict.snapshot, None,
        "there is no snapshot this verdict is about"
    );
}

#[test]
fn a_running_snapshot_this_subdomain_has_no_definition_for_is_unavailable() {
    let verdict = judge_subject(&facts(running("snap-a", Some(FIRST)), None, false));

    assert_eq!(verdict.readiness, SubjectReadiness::Unavailable);
}

#[test]
fn no_active_generation_is_unavailable_rather_than_ready() {
    // Installed, not running. The defect this replaced would have answered `ready` here from
    // whatever definition happened to be recorded most recently.
    let verdict = judge_subject(&facts(ActiveSnapshot::NoActiveGeneration, None, true));

    assert_eq!(verdict.readiness, SubjectReadiness::Unavailable);
    assert_eq!(verdict.snapshot, None);
}

#[test]
fn an_unreachable_platform_is_unavailable_and_never_ready() {
    // A subdomain that cannot reach the authority does not get to guess. Every path from `Unknown`
    // is `Unavailable`, including the one where a definition is sitting right there.
    let verdict = judge_subject(&facts(ActiveSnapshot::Unknown, Some(FIRST), true));

    assert_eq!(verdict.readiness, SubjectReadiness::Unavailable);
    assert!(!verdict.readiness.admits_dispatch());
}

#[test]
fn an_uninstalled_extension_leaves_its_hook_orphaned_when_evidence_remains() {
    let verdict = judge_subject(&facts(ActiveSnapshot::NotInstalled, None, true));

    assert_eq!(verdict.readiness, SubjectReadiness::Orphaned);
    assert!(!verdict.readiness.admits_dispatch());
}

#[test]
fn a_subject_with_no_definition_at_all_is_unavailable_rather_than_orphaned() {
    // A subject that exists only because a binding or an execution mentions it. Nothing was ever
    // recorded for it, so nothing was orphaned.
    let verdict = judge_subject(&facts(ActiveSnapshot::NotInstalled, None, false));

    assert_eq!(verdict.readiness, SubjectReadiness::Unavailable);
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
    let subject = facts(running("snap-a", Some(FIRST)), Some(FIRST), true);

    let verdict = judge_subject(&subject);

    assert_eq!(verdict.hook, subject.hook);
    assert_eq!(
        subject.recorded_at_active,
        Some(digest(FIRST)),
        "the facts are unchanged; judging stores nothing"
    );
}

#[test]
fn the_same_input_judges_the_same_way_every_time() {
    // Nothing here may depend on a clock or on iteration order; a verdict that reshuffled would
    // make a stored diagnostic unmatchable.
    let subject = facts(running("snap-a", Some(FIRST)), Some(SECOND), true);

    assert_eq!(judge_subject(&subject), judge_subject(&subject));
}
