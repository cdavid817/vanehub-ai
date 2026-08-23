//! That readiness follows the active snapshot, and that reconciliation writes nothing.
//!
//! The scenarios that matter are the ones "the most recently recorded revision" gets wrong: a
//! version recorded but not activated, and a rollback.

use super::{
    reconcile_connector, reconcile_connectors, recorded_revisions, ActiveConnectorSnapshotPort,
    ConnectorDefinitionRepository, ConnectorSubjectRepository,
};
use crate::contexts::tooling::connectors::domain::{
    ActiveConnectorSnapshot, ConnectorDefinitionDigest, ConnectorDefinitionOutcome,
    ConnectorDefinitionRevision, ConnectorGlobalId, ConnectorReadiness, ConnectorSnapshotRef,
    ConnectorSubject, OwnerExtensionId,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const V1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const V2: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const CONNECTOR: &str = "ext::acme.mailer::smtp";
const AT: &str = "2026-08-23T00:00:00Z";

fn connector(name: &str) -> ConnectorGlobalId {
    ConnectorGlobalId::parse(name).expect("connector")
}

fn digest(value: &str) -> ConnectorDefinitionDigest {
    ConnectorDefinitionDigest::parse(value).expect("digest")
}

fn snapshot(value: &str) -> ConnectorSnapshotRef {
    ConnectorSnapshotRef::parse(value).expect("snapshot")
}

#[derive(Default)]
struct FakeSubjects {
    subjects: Vec<ConnectorSubject>,
}

impl ConnectorSubjectRepository for FakeSubjects {
    fn ensure(&self, _subject: &ConnectorSubject) -> Result<(), String> {
        panic!("reconciliation must not write a subject");
    }

    fn get(&self, connector: &ConnectorGlobalId) -> Result<Option<ConnectorSubject>, String> {
        Ok(self
            .subjects
            .iter()
            .find(|subject| subject.connector == *connector)
            .cloned())
    }

    fn all(&self) -> Result<Vec<ConnectorSubject>, String> {
        Ok(self.subjects.clone())
    }
}

/// Revisions in the order the repository returns them: most recently recorded first.
///
/// That ordering is the trap. Every scenario below records v2 after v1, so a reconciliation that
/// consulted this order rather than the active pointer would answer v2 in all of them.
#[derive(Default)]
struct FakeDefinitions {
    revisions: Vec<ConnectorDefinitionRevision>,
}

impl ConnectorDefinitionRepository for FakeDefinitions {
    fn record(
        &self,
        _revision: &ConnectorDefinitionRevision,
    ) -> Result<ConnectorDefinitionOutcome, String> {
        panic!("reconciliation must not record a definition");
    }

    fn recorded(
        &self,
        connector: &ConnectorGlobalId,
        snapshot: &ConnectorSnapshotRef,
    ) -> Result<Option<ConnectorDefinitionRevision>, String> {
        Ok(self
            .revisions
            .iter()
            .find(|revision| revision.connector == *connector && revision.snapshot == *snapshot)
            .cloned())
    }

    fn revisions(
        &self,
        connector: &ConnectorGlobalId,
    ) -> Result<Vec<ConnectorDefinitionRevision>, String> {
        Ok(self
            .revisions
            .iter()
            .filter(|revision| revision.connector == *connector)
            .cloned()
            .collect())
    }
}

struct FakeActive {
    answer: ActiveConnectorSnapshot,
    asked: Mutex<Vec<String>>,
    calls: AtomicUsize,
}

impl FakeActive {
    fn new(answer: ActiveConnectorSnapshot) -> Self {
        Self {
            answer,
            asked: Mutex::new(Vec::new()),
            calls: AtomicUsize::new(0),
        }
    }
}

impl ActiveConnectorSnapshotPort for FakeActive {
    fn active_snapshot(
        &self,
        connector: &ConnectorGlobalId,
    ) -> Result<ActiveConnectorSnapshot, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.asked
            .lock()
            .expect("lock")
            .push(connector.as_str().to_string());
        Ok(self.answer.clone())
    }
}

fn revision(snapshot_id: &str, value: &str, at: &str) -> ConnectorDefinitionRevision {
    ConnectorDefinitionRevision {
        snapshot: snapshot(snapshot_id),
        connector: connector(CONNECTOR),
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

fn running(snapshot_id: &str, declared: &str) -> ActiveConnectorSnapshot {
    ActiveConnectorSnapshot::Running {
        snapshot: snapshot(snapshot_id),
        declared: Some(digest(declared)),
    }
}

#[test]
fn a_version_recorded_but_not_activated_does_not_become_the_answer() {
    let definitions = both_versions_recorded();
    let active = FakeActive::new(running("snap-v1", V1));

    let verdict =
        reconcile_connector(&connector(CONNECTOR), &definitions, &active).expect("reconcile");

    assert_eq!(verdict.readiness, ConnectorReadiness::Ready);
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

    let verdict =
        reconcile_connector(&connector(CONNECTOR), &definitions, &active).expect("reconcile");

    assert_eq!(verdict.readiness, ConnectorReadiness::Ready);
    assert_eq!(verdict.snapshot, Some(snapshot("snap-v2")));
}

#[test]
fn rolling_back_to_the_older_version_moves_the_answer_back() {
    // After a rollback, v2 is still the most recently *recorded* revision while the platform runs
    // v1. The diagnostic listing still leads with v2, which is exactly why it cannot decide.
    let definitions = both_versions_recorded();
    let active = FakeActive::new(running("snap-v1", V1));

    let verdict =
        reconcile_connector(&connector(CONNECTOR), &definitions, &active).expect("reconcile");

    assert_eq!(verdict.snapshot, Some(snapshot("snap-v1")));
    assert_eq!(
        recorded_revisions(&connector(CONNECTOR), &definitions)
            .expect("diagnostic")
            .first()
            .map(|revision| revision.snapshot.clone()),
        Some(snapshot("snap-v2"))
    );
}

#[test]
fn no_active_generation_is_unavailable_even_with_definitions_recorded() {
    let definitions = both_versions_recorded();
    let active = FakeActive::new(ActiveConnectorSnapshot::NoActiveGeneration);

    let verdict =
        reconcile_connector(&connector(CONNECTOR), &definitions, &active).expect("reconcile");

    assert_eq!(verdict.readiness, ConnectorReadiness::Unavailable);
    assert_eq!(verdict.snapshot, None);
}

#[test]
fn an_active_snapshot_that_does_not_declare_the_connector_is_unavailable() {
    // v2 dropped the connector. The v1 revision is still recorded and must not be reached for.
    let definitions = both_versions_recorded();
    let active = FakeActive::new(ActiveConnectorSnapshot::Running {
        snapshot: snapshot("snap-v2"),
        declared: None,
    });

    let verdict =
        reconcile_connector(&connector(CONNECTOR), &definitions, &active).expect("reconcile");

    assert_eq!(verdict.readiness, ConnectorReadiness::Unavailable);
}

#[test]
fn a_digest_that_disagrees_with_the_active_snapshot_is_drifted() {
    let definitions = both_versions_recorded();
    let active = FakeActive::new(running("snap-v1", V2));

    let verdict =
        reconcile_connector(&connector(CONNECTOR), &definitions, &active).expect("reconcile");

    assert_eq!(verdict.readiness, ConnectorReadiness::Drifted);
    assert_eq!(verdict.snapshot, Some(snapshot("snap-v1")));
}

#[test]
fn an_unavailable_port_never_yields_ready() {
    let definitions = both_versions_recorded();
    let active = FakeActive::new(ActiveConnectorSnapshot::Unknown);

    let verdict =
        reconcile_connector(&connector(CONNECTOR), &definitions, &active).expect("reconcile");

    assert_eq!(verdict.readiness, ConnectorReadiness::Unavailable);
}

#[test]
fn the_platform_is_asked_before_anything_is_keyed_on_its_answer() {
    // The read order is the consistency argument: the platform first, everything else keyed on the
    // snapshot it named. If this count is ever zero, some path is answering from local state
    // alone -- which is the defect, whatever verdict it happens to produce.
    let definitions = both_versions_recorded();
    let active = FakeActive::new(running("snap-v1", V1));

    reconcile_connector(&connector(CONNECTOR), &definitions, &active).expect("reconcile");

    assert_eq!(active.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        *active.asked.lock().expect("lock"),
        vec![CONNECTOR.to_string()]
    );
}

#[test]
fn reconciliation_writes_nothing() {
    // The fakes panic on every write method, so a reconciliation that deleted an instance, dropped
    // a binding, or cleared a credential handle would fail here rather than in production.
    let subjects = FakeSubjects {
        subjects: vec![
            ConnectorSubject {
                connector: connector(CONNECTOR),
                owner_extension: OwnerExtensionId::parse("acme.mailer").expect("owner"),
                first_seen_at: AT.to_string(),
            },
            ConnectorSubject {
                connector: connector("vanehub.github"),
                owner_extension: OwnerExtensionId::parse("vanehub.core").expect("owner"),
                first_seen_at: AT.to_string(),
            },
        ],
    };
    let definitions = both_versions_recorded();
    let active = FakeActive::new(ActiveConnectorSnapshot::NotInstalled);

    let verdicts = reconcile_connectors(&subjects, &definitions, &active).expect("reconcile");

    assert_eq!(
        verdicts
            .iter()
            .map(|verdict| verdict.readiness)
            .collect::<Vec<_>>(),
        vec![
            // Has recorded revisions and nothing installed contributes it any more.
            ConnectorReadiness::Orphaned,
            // Never had one.
            ConnectorReadiness::Unavailable,
        ],
    );
}

#[test]
fn a_report_is_ordered_by_connector_so_two_runs_agree() {
    let subjects = FakeSubjects {
        subjects: vec![
            ConnectorSubject {
                connector: connector("acme.one"),
                owner_extension: OwnerExtensionId::parse("acme.one").expect("owner"),
                first_seen_at: AT.to_string(),
            },
            ConnectorSubject {
                connector: connector("acme.two"),
                owner_extension: OwnerExtensionId::parse("acme.two").expect("owner"),
                first_seen_at: AT.to_string(),
            },
        ],
    };
    let definitions = FakeDefinitions::default();
    let active = FakeActive::new(ActiveConnectorSnapshot::NotInstalled);

    let first = reconcile_connectors(&subjects, &definitions, &active).expect("first");
    let second = reconcile_connectors(&subjects, &definitions, &active).expect("second");

    assert_eq!(first, second);
    assert_eq!(
        first
            .iter()
            .map(|verdict| verdict.connector.as_str().to_string())
            .collect::<Vec<_>>(),
        vec!["acme.one".to_string(), "acme.two".to_string()]
    );
}
