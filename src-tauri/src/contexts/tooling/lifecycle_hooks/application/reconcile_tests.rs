//! That readiness follows the active snapshot, and that reconciliation writes nothing.
//!
//! The scenarios that matter are the ones "the most recently recorded revision" got wrong: a
//! version recorded but not activated, and a rollback. Both are here, driven through the same
//! definition store an install flow would have written.

use super::{
    reconcile_subject, reconcile_subjects, recorded_revisions, ActiveExtensionSnapshotPort,
    HookDefinitionRepository, HookSubjectRepository,
};
use crate::contexts::tooling::lifecycle_hooks::domain::{
    ActiveSnapshot, DefinitionDigest, DefinitionOutcome, HookDefinitionRevision, HookEvent,
    HookGlobalId, HookOrigin, HookSubject, SnapshotRef, SubjectReadiness,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const V1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const V2: &str = "2222222222222222222222222222222222222222222222222222222222222222";
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

/// Revisions in the order the repository returns them: most recently recorded first.
///
/// That ordering is the trap. Every scenario below records v2 after v1, so a reconciliation that
/// consulted this order rather than the active pointer would answer v2 in all of them.
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

struct FakeActive {
    answer: ActiveSnapshot,
    asked: Mutex<Vec<String>>,
    calls: AtomicUsize,
}

impl FakeActive {
    fn new(answer: ActiveSnapshot) -> Self {
        Self {
            answer,
            asked: Mutex::new(Vec::new()),
            calls: AtomicUsize::new(0),
        }
    }
}

impl ActiveExtensionSnapshotPort for FakeActive {
    fn active_snapshot(&self, hook: &HookGlobalId) -> Result<ActiveSnapshot, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.asked
            .lock()
            .expect("lock")
            .push(hook.as_str().to_string());
        Ok(self.answer.clone())
    }
}

fn revision(snapshot_id: &str, value: &str, at: &str) -> HookDefinitionRevision {
    HookDefinitionRevision {
        hook: hook("ext::acme.git-guardian::pre-commit"),
        snapshot: snapshot(snapshot_id),
        event: HookEvent::PreToolUse,
        digest: digest(value),
        recorded_at: at.to_string(),
    }
}

/// v2 recorded after v1, so the diagnostic ordering leads with v2 in every scenario.
fn both_versions_recorded() -> FakeDefinitions {
    FakeDefinitions {
        revisions: vec![
            revision("snap-v2", V2, "2026-09-01T00:00:00Z"),
            revision("snap-v1", V1, AT),
        ],
    }
}

fn running(snapshot_id: &str, declared: &str) -> ActiveSnapshot {
    ActiveSnapshot::Running {
        snapshot: snapshot(snapshot_id),
        declared: Some(digest(declared)),
    }
}

#[test]
fn a_version_recorded_but_not_activated_does_not_become_the_answer() {
    // The defect this replaced. v1 is running; v2 has been recorded by an install that has not
    // activated. The report must still say v1.
    let definitions = both_versions_recorded();
    let active = FakeActive::new(running("snap-v1", V1));

    let verdict = reconcile_subject(
        &hook("ext::acme.git-guardian::pre-commit"),
        &definitions,
        &active,
    )
    .expect("reconcile");

    assert_eq!(verdict.readiness, SubjectReadiness::Ready);
    assert_eq!(
        verdict.snapshot,
        Some(snapshot("snap-v1")),
        "the recorded-but-unactivated v2 must not win"
    );
}

#[test]
fn activating_the_new_version_moves_the_answer_to_it() {
    let definitions = both_versions_recorded();
    let active = FakeActive::new(running("snap-v2", V2));

    let verdict = reconcile_subject(
        &hook("ext::acme.git-guardian::pre-commit"),
        &definitions,
        &active,
    )
    .expect("reconcile");

    assert_eq!(verdict.readiness, SubjectReadiness::Ready);
    assert_eq!(verdict.snapshot, Some(snapshot("snap-v2")));
}

#[test]
fn rolling_back_to_the_older_version_moves_the_answer_back() {
    // The second way "most recently recorded" was wrong: after a rollback, v2 is still the most
    // recently recorded revision while the platform runs v1.
    let definitions = both_versions_recorded();
    let active = FakeActive::new(running("snap-v1", V1));

    let verdict = reconcile_subject(
        &hook("ext::acme.git-guardian::pre-commit"),
        &definitions,
        &active,
    )
    .expect("reconcile");

    assert_eq!(verdict.readiness, SubjectReadiness::Ready);
    assert_eq!(verdict.snapshot, Some(snapshot("snap-v1")));
    assert_eq!(
        recorded_revisions(&hook("ext::acme.git-guardian::pre-commit"), &definitions)
            .expect("diagnostic")
            .first()
            .map(|revision| revision.snapshot.clone()),
        Some(snapshot("snap-v2")),
        "the diagnostic listing still leads with v2, which is exactly why it cannot decide"
    );
}

#[test]
fn no_active_pointer_is_unavailable_even_with_definitions_recorded() {
    let definitions = both_versions_recorded();
    let active = FakeActive::new(ActiveSnapshot::NoActiveGeneration);

    let verdict = reconcile_subject(
        &hook("ext::acme.git-guardian::pre-commit"),
        &definitions,
        &active,
    )
    .expect("reconcile");

    assert_eq!(verdict.readiness, SubjectReadiness::Unavailable);
    assert_eq!(verdict.snapshot, None);
}

#[test]
fn an_active_snapshot_that_does_not_declare_the_hook_is_unavailable() {
    // v2 dropped the Hook. The v1 revision is still recorded and must not be reached for.
    let definitions = both_versions_recorded();
    let active = FakeActive::new(ActiveSnapshot::Running {
        snapshot: snapshot("snap-v2"),
        declared: None,
    });

    let verdict = reconcile_subject(
        &hook("ext::acme.git-guardian::pre-commit"),
        &definitions,
        &active,
    )
    .expect("reconcile");

    assert_eq!(verdict.readiness, SubjectReadiness::Unavailable);
}

#[test]
fn a_digest_that_disagrees_with_the_active_snapshot_is_drifted() {
    // The platform runs snap-v1 and says it declares V2; this subdomain recorded V1 for that
    // snapshot. The two copies disagree, which is the whole point of keeping two.
    let definitions = both_versions_recorded();
    let active = FakeActive::new(running("snap-v1", V2));

    let verdict = reconcile_subject(
        &hook("ext::acme.git-guardian::pre-commit"),
        &definitions,
        &active,
    )
    .expect("reconcile");

    assert_eq!(verdict.readiness, SubjectReadiness::Drifted);
    assert_eq!(verdict.snapshot, Some(snapshot("snap-v1")));
}

#[test]
fn an_unavailable_port_never_yields_ready() {
    // The conservative fallback. Even with a definition recorded for a snapshot the platform would
    // have named, not being able to ask means not being able to say yes.
    let definitions = both_versions_recorded();
    let active = FakeActive::new(ActiveSnapshot::Unknown);

    let verdict = reconcile_subject(
        &hook("ext::acme.git-guardian::pre-commit"),
        &definitions,
        &active,
    )
    .expect("reconcile");

    assert_eq!(verdict.readiness, SubjectReadiness::Unavailable);
}

#[test]
fn readiness_is_never_computed_without_asking_the_platform() {
    // If this count is ever zero, some path is answering from local state alone -- which is the
    // defect, whatever verdict it happens to produce.
    let definitions = both_versions_recorded();
    let active = FakeActive::new(running("snap-v1", V1));

    reconcile_subject(
        &hook("ext::acme.git-guardian::pre-commit"),
        &definitions,
        &active,
    )
    .expect("reconcile");

    assert_eq!(active.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        *active.asked.lock().expect("lock"),
        vec!["ext::acme.git-guardian::pre-commit".to_string()]
    );
}

#[test]
fn reconciliation_writes_nothing() {
    // The fakes panic on every write method, so a reconciliation that deleted a row, rebound a
    // binding, or recorded a mark would fail here rather than in production six months later.
    let subjects = FakeSubjects {
        subjects: vec![
            HookSubject {
                hook: hook("ext::acme.git-guardian::pre-commit"),
                origin: HookOrigin::Extension,
                first_seen_at: AT.to_string(),
            },
            HookSubject {
                hook: hook("vanehub.session-start"),
                origin: HookOrigin::Builtin,
                first_seen_at: AT.to_string(),
            },
        ],
    };
    let definitions = both_versions_recorded();
    let active = FakeActive::new(ActiveSnapshot::NotInstalled);

    let verdicts = reconcile_subjects(&subjects, &definitions, &active).expect("reconcile");

    assert_eq!(
        verdicts
            .iter()
            .map(|verdict| verdict.readiness)
            .collect::<Vec<_>>(),
        vec![
            // Has recorded revisions and nothing installed contributes it any more.
            SubjectReadiness::Orphaned,
            // Never had one.
            SubjectReadiness::Unavailable,
        ],
    );
}

#[test]
fn a_report_is_ordered_by_subject_so_two_runs_agree() {
    let subjects = FakeSubjects {
        subjects: vec![
            HookSubject {
                hook: hook("acme.one"),
                origin: HookOrigin::Extension,
                first_seen_at: AT.to_string(),
            },
            HookSubject {
                hook: hook("acme.two"),
                origin: HookOrigin::Extension,
                first_seen_at: AT.to_string(),
            },
        ],
    };
    let definitions = FakeDefinitions::default();
    let active = FakeActive::new(ActiveSnapshot::NotInstalled);

    let first = reconcile_subjects(&subjects, &definitions, &active).expect("first");
    let second = reconcile_subjects(&subjects, &definitions, &active).expect("second");

    assert_eq!(first, second);
    assert_eq!(
        first
            .iter()
            .map(|verdict| verdict.hook.as_str().to_string())
            .collect::<Vec<_>>(),
        vec!["acme.one".to_string(), "acme.two".to_string()]
    );
}
