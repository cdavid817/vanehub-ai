//! That reconciliation reads, reports, and writes nothing.

use super::{
    reconcile_subject, reconcile_subjects, HookDefinitionRepository, HookSubjectRepository,
    SnapshotProjectionPort,
};
use crate::contexts::tooling::lifecycle_hooks::domain::{
    DefinitionDigest, DefinitionOutcome, HookDefinitionRevision, HookEvent, HookGlobalId,
    HookOrigin, HookSubject, SnapshotFact, SnapshotRef, SubjectReadiness,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const FIRST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const AT: &str = "2026-08-01T00:00:00Z";

fn hook(name: &str) -> HookGlobalId {
    HookGlobalId::parse(name).expect("hook")
}

fn digest(value: &str) -> DefinitionDigest {
    DefinitionDigest::parse(value).expect("digest")
}

fn snapshot(value: &str) -> SnapshotRef {
    SnapshotRef::parse(value).expect("snapshot")
}

#[derive(Default)]
struct FakeSubjects {
    subjects: Vec<HookSubject>,
}

impl HookSubjectRepository for FakeSubjects {
    fn ensure(&self, _subject: &HookSubject) -> Result<(), String> {
        panic!("reconciliation must not write a subject");
    }

    fn get(&self, hook: &HookGlobalId) -> Result<Option<HookSubject>, String> {
        Ok(self
            .subjects
            .iter()
            .find(|subject| subject.hook == *hook)
            .cloned())
    }

    fn all(&self) -> Result<Vec<HookSubject>, String> {
        Ok(self.subjects.clone())
    }
}

#[derive(Default)]
struct FakeDefinitions {
    revisions: Vec<HookDefinitionRevision>,
}

impl HookDefinitionRepository for FakeDefinitions {
    fn record(&self, _revision: &HookDefinitionRevision) -> Result<DefinitionOutcome, String> {
        panic!("reconciliation must not record a definition");
    }

    fn recorded(
        &self,
        hook: &HookGlobalId,
        snapshot: &SnapshotRef,
    ) -> Result<Option<HookDefinitionRevision>, String> {
        Ok(self
            .revisions
            .iter()
            .find(|revision| revision.hook == *hook && revision.snapshot == *snapshot)
            .cloned())
    }

    fn revisions(&self, hook: &HookGlobalId) -> Result<Vec<HookDefinitionRevision>, String> {
        Ok(self
            .revisions
            .iter()
            .filter(|revision| revision.hook == *hook)
            .cloned()
            .collect())
    }
}

#[derive(Default)]
struct FakeProjection {
    facts: Vec<SnapshotFact>,
    lookups: AtomicUsize,
    asked: Mutex<Vec<String>>,
}

impl SnapshotProjectionPort for FakeProjection {
    fn fact(
        &self,
        _hook: &HookGlobalId,
        snapshot: &SnapshotRef,
    ) -> Result<Option<SnapshotFact>, String> {
        self.lookups.fetch_add(1, Ordering::SeqCst);
        self.asked
            .lock()
            .expect("lock")
            .push(snapshot.as_str().to_string());
        Ok(self
            .facts
            .iter()
            .find(|fact| fact.snapshot == *snapshot)
            .cloned())
    }
}

fn revision(name: &str, snapshot_id: &str, value: &str) -> HookDefinitionRevision {
    HookDefinitionRevision {
        hook: hook(name),
        snapshot: snapshot(snapshot_id),
        event: HookEvent::PreToolUse,
        digest: digest(value),
        recorded_at: AT.to_string(),
    }
}

fn subject(name: &str) -> HookSubject {
    HookSubject {
        hook: hook(name),
        origin: HookOrigin::Extension,
        first_seen_at: AT.to_string(),
    }
}

#[test]
fn a_subject_whose_snapshot_agrees_with_it_is_ready() {
    let definitions = FakeDefinitions {
        revisions: vec![revision("acme.one", "snap-a", FIRST)],
    };
    let projection = FakeProjection {
        facts: vec![SnapshotFact {
            snapshot: snapshot("snap-a"),
            hook_digest: Some(digest(FIRST)),
        }],
        ..FakeProjection::default()
    };

    let verdict =
        reconcile_subject(&hook("acme.one"), &definitions, &projection).expect("reconcile");

    assert_eq!(verdict.readiness, SubjectReadiness::Ready);
}

#[test]
fn a_subject_whose_snapshot_disagrees_is_drifted_and_the_projection_was_consulted() {
    let definitions = FakeDefinitions {
        revisions: vec![revision("acme.one", "snap-a", FIRST)],
    };
    let projection = FakeProjection {
        facts: vec![SnapshotFact {
            snapshot: snapshot("snap-a"),
            hook_digest: Some(digest(SECOND)),
        }],
        ..FakeProjection::default()
    };

    let verdict =
        reconcile_subject(&hook("acme.one"), &definitions, &projection).expect("reconcile");

    assert_eq!(verdict.readiness, SubjectReadiness::Drifted);
    assert_eq!(
        *projection.asked.lock().expect("lock"),
        vec!["snap-a".to_string()],
        "the snapshot fact comes from the port, never from a read of another subdomain's tables"
    );
}

#[test]
fn a_subject_with_no_revision_does_not_reach_the_projection_at_all() {
    // There is nothing to look up: the domain reads "no revision" as `Unavailable` without the
    // projection's opinion, so asking would be a query made for no reason.
    let definitions = FakeDefinitions::default();
    let projection = FakeProjection::default();

    let verdict =
        reconcile_subject(&hook("acme.one"), &definitions, &projection).expect("reconcile");

    assert_eq!(verdict.readiness, SubjectReadiness::Unavailable);
    assert_eq!(projection.lookups.load(Ordering::SeqCst), 0);
}

#[test]
fn reconciliation_writes_nothing() {
    // The fakes panic on every write method, so a reconciliation that deleted a row, rebound a
    // binding, or recorded a mark would fail here rather than in production six months later.
    let subjects = FakeSubjects {
        subjects: vec![subject("acme.one"), subject("acme.two")],
    };
    let definitions = FakeDefinitions {
        revisions: vec![revision("acme.one", "snap-gone", FIRST)],
    };
    let projection = FakeProjection::default();

    let verdicts = reconcile_subjects(&subjects, &definitions, &projection).expect("reconcile");

    assert_eq!(
        verdicts
            .iter()
            .map(|verdict| verdict.readiness)
            .collect::<Vec<_>>(),
        vec![SubjectReadiness::Orphaned, SubjectReadiness::Unavailable],
    );
}

#[test]
fn a_report_is_ordered_by_subject_so_two_runs_agree() {
    let subjects = FakeSubjects {
        subjects: vec![subject("acme.one"), subject("acme.two")],
    };
    let definitions = FakeDefinitions::default();
    let projection = FakeProjection::default();

    let first = reconcile_subjects(&subjects, &definitions, &projection).expect("first");
    let second = reconcile_subjects(&subjects, &definitions, &projection).expect("second");

    assert_eq!(first, second);
    assert_eq!(
        first
            .iter()
            .map(|verdict| verdict.hook.as_str().to_string())
            .collect::<Vec<_>>(),
        vec!["acme.one".to_string(), "acme.two".to_string()]
    );
}

#[test]
fn an_upgrade_is_judged_on_the_revision_it_would_dispatch_from() {
    // An upgrade records beside the old revision rather than over it. The verdict answers "would
    // this Hook run right now", so it is about the most recently recorded one.
    let definitions = FakeDefinitions {
        revisions: vec![
            HookDefinitionRevision {
                recorded_at: "2026-09-01T00:00:00Z".to_string(),
                ..revision("acme.one", "snap-b", SECOND)
            },
            revision("acme.one", "snap-a", FIRST),
        ],
    };
    let projection = FakeProjection {
        facts: vec![SnapshotFact {
            snapshot: snapshot("snap-b"),
            hook_digest: Some(digest(SECOND)),
        }],
        ..FakeProjection::default()
    };

    let verdict =
        reconcile_subject(&hook("acme.one"), &definitions, &projection).expect("reconcile");

    assert_eq!(verdict.readiness, SubjectReadiness::Ready);
    assert_eq!(verdict.snapshot, Some(snapshot("snap-b")));
}
