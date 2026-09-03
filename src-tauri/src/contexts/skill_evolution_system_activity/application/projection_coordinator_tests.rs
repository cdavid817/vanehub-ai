use std::{
    collections::{BTreeMap, VecDeque},
    sync::Mutex,
};

use super::*;
use crate::contexts::skill_evolution_system_activity::domain::*;

struct StaticSource {
    page: ProjectionSourcePage,
}

struct UnavailableSource(EvolutionSourceDomain);

impl EvolutionProjectionSource for UnavailableSource {
    fn domain(&self) -> EvolutionSourceDomain {
        self.0
    }

    fn scan_committed(
        &self,
        _after: Option<&OpaqueDomainCursor>,
        _limit: ProjectionScanLimit,
    ) -> Result<ProjectionSourcePage, ProjectionSourceError> {
        Err(ProjectionSourceError::Unavailable)
    }
}

impl EvolutionProjectionSource for StaticSource {
    fn domain(&self) -> EvolutionSourceDomain {
        self.page.source_domain
    }

    fn scan_committed(
        &self,
        _after: Option<&OpaqueDomainCursor>,
        _limit: ProjectionScanLimit,
    ) -> Result<ProjectionSourcePage, ProjectionSourceError> {
        Ok(self.page.clone())
    }
}

#[derive(Default)]
struct RecordingStore {
    cursor: Option<ActivityDomainCursor>,
    committed_sequences: Mutex<Vec<Vec<u64>>>,
    failure: Mutex<Option<(EvolutionSourceDomain, ActivityGapCode)>>,
}

impl ActivityProjectionStore for RecordingStore {
    fn cursor(
        &self,
        _domain: EvolutionSourceDomain,
    ) -> Result<Option<ActivityDomainCursor>, ActivityProjectionStoreError> {
        Ok(self.cursor.clone())
    }

    fn commit_batch(
        &self,
        batch: &ActivityProjectionBatch,
    ) -> Result<ActivityProjectionBatchResult, ActivityProjectionStoreError> {
        self.committed_sequences
            .lock()
            .expect("recording lock")
            .push(
                batch
                    .events
                    .iter()
                    .map(|event| event.source_sequence)
                    .collect(),
            );
        Ok(ActivityProjectionBatchResult {
            scanned: batch.events.len(),
            inserted: batch.events.len(),
            replayed: 0,
            has_more: false,
        })
    }

    fn record_failure(
        &self,
        domain: EvolutionSourceDomain,
        gap: Option<ActivityGapCode>,
        _failure: ActivityProjectionFailureCode,
        _expected_revision: u64,
    ) -> Result<(), ActivityProjectionStoreError> {
        *self.failure.lock().expect("failure lock") = gap.map(|code| (domain, code));
        Ok(())
    }
}

struct SequenceClock(Mutex<VecDeque<i64>>);

impl SequenceClock {
    fn new(values: impl IntoIterator<Item = i64>) -> Self {
        Self(Mutex::new(values.into_iter().collect()))
    }
}

impl ActivityProjectionClock for SequenceClock {
    fn now_ms(&self) -> i64 {
        self.0
            .lock()
            .expect("clock lock")
            .pop_front()
            .expect("clock value")
    }
}

#[test]
fn batch_stops_at_two_seconds_and_checkpoints_the_last_included_cursor() {
    let store = RecordingStore::default();
    let clock = SequenceClock::new([0, 1_000, 2_000, 2_010]);
    let source = StaticSource {
        page: page(3, true),
    };

    let result = ActivityProjectionCoordinator::new(&store, &clock)
        .process_domain(&source)
        .expect("bounded batch")
        .expect("batch result");

    assert_eq!(result.scanned, 2);
    assert!(result.has_more);
    assert_eq!(
        *store.committed_sequences.lock().expect("recording lock"),
        vec![vec![1, 2]]
    );
}

#[test]
fn batch_never_scans_more_than_one_hundred_envelopes() {
    let store = RecordingStore::default();
    let clock = SequenceClock::new(vec![0; 101]);
    let source = StaticSource {
        page: page(100, true),
    };

    let result = ActivityProjectionCoordinator::new(&store, &clock)
        .process_domain(&source)
        .expect("bounded batch")
        .expect("batch result");

    assert_eq!(result.scanned, 100);
    assert!(result.has_more);
}

#[test]
fn existing_cursor_gap_stops_only_that_domain_before_commit() {
    let store = RecordingStore {
        cursor: Some(ActivityDomainCursor {
            source_domain: EvolutionSourceDomain::Evidence,
            opaque_cursor: Some(cursor(4)),
            last_sequence: 4,
            last_source_hash: Some("hash:4".into()),
            retention_floor: None,
            pending_count: 0,
            oldest_pending_at_ms: None,
            gap: None,
            failure_code: None,
            revision: 3,
        }),
        ..RecordingStore::default()
    };
    let mut source_page = page(1, false);
    source_page.events[0] = event(6);
    source_page.next_cursor = Some(cursor(6));
    let source = StaticSource { page: source_page };
    let clock = SequenceClock::new([]);

    let error = ActivityProjectionCoordinator::new(&store, &clock)
        .process_domain(&source)
        .expect_err("sequence gap");

    assert!(matches!(
        error,
        ProjectionCoordinatorError::SequenceGap {
            expected: 5,
            actual: 6,
            ..
        }
    ));
    assert_eq!(
        *store.failure.lock().expect("failure lock"),
        Some((
            EvolutionSourceDomain::Evidence,
            ActivityGapCode::MissingSequence
        ))
    );
    assert!(store
        .committed_sequences
        .lock()
        .expect("recording lock")
        .is_empty());
}

#[test]
fn failed_domain_does_not_prevent_unrelated_domain_progress() {
    let store = RecordingStore::default();
    let clock = SequenceClock::new([0, 1]);
    let unavailable = UnavailableSource(EvolutionSourceDomain::Evidence);
    let mut healthy_page = page(1, false);
    healthy_page.source_domain = EvolutionSourceDomain::Assessment;
    healthy_page.events[0].envelope.source_domain = "assessment".into();
    healthy_page.events[0].envelope = healthy_page.events[0]
        .envelope
        .clone()
        .seal()
        .expect("resealed domain");
    let healthy = StaticSource { page: healthy_page };

    let results =
        ActivityProjectionCoordinator::new(&store, &clock).process_cycle(&[&unavailable, &healthy]);

    assert_eq!(results.len(), 2);
    assert!(matches!(
        results[0].outcome,
        Err(ProjectionCoordinatorError::Source(
            ProjectionSourceError::Unavailable
        ))
    ));
    assert!(matches!(results[1].outcome, Ok(Some(_))));
    assert_eq!(
        *store.committed_sequences.lock().expect("recording lock"),
        vec![vec![1]]
    );
}

#[test]
fn startup_plan_never_waits_for_readiness_and_returns_background_continuation() {
    let store = RecordingStore::default();
    let clock = SequenceClock::new([0, 0, 1, 5_000, 5_000, 5_001, 10_000]);
    let source = StaticSource {
        page: page(1, true),
    };
    let coordinator = ActivityProjectionCoordinator::new(&store, &clock);

    assert_eq!(
        coordinator.startup_plan(),
        StartupCatchUpPlan {
            readiness_wait_ms: 0,
            background_budget_ms: 10_000,
        }
    );
    let result = coordinator.process_startup_catch_up(&[&source]);
    assert_eq!(result.cycles, 2);
    assert!(result.continuation_required);
}

fn page(count: u64, has_more: bool) -> ProjectionSourcePage {
    ProjectionSourcePage {
        source_domain: EvolutionSourceDomain::Evidence,
        events: (1..=count).map(event).collect(),
        next_cursor: Some(cursor(count)),
        retention_floor: Some(cursor(1)),
        has_more,
    }
}

fn event(sequence: u64) -> VerifiedProjectionEvent {
    VerifiedProjectionEvent {
        source_cursor: cursor(sequence),
        source_sequence: sequence,
        source_integrity_hash: format!("hash:{sequence}"),
        envelope: EvolutionActivityEnvelopeV1 {
            schema_version: ACTIVITY_SCHEMA_VERSION_V1,
            event_id: format!("event-{sequence}"),
            event_code: ActivityEventCode::EvidenceReady,
            source_domain: "evidence".into(),
            source_id: format!("source-{sequence}"),
            source_revision: "revision-1".into(),
            source_sequence: sequence,
            scope_kind: ActivityScopeKind::Workspace,
            canonical_scope_id: "workspace-1".into(),
            occurred_at_ms: sequence as i64,
            committed_at_ms: sequence as i64,
            severity: ActivitySeverity::Info,
            status: ActivityStatus::Succeeded,
            attention_kind: ActivityAttentionKind::None,
            safe_actor_kind: ActivityActorKind::System,
            safe_identities: Vec::new(),
            metrics: BTreeMap::new(),
            reason_codes: Vec::new(),
            navigation: None,
            supersedes_event_id: None,
            payload: None,
            projection_policy_version: 1,
            content_hash: String::new(),
        }
        .seal()
        .expect("envelope"),
    }
}

fn cursor(sequence: u64) -> OpaqueDomainCursor {
    OpaqueDomainCursor::parse(format!("cursor:{sequence}")).expect("cursor")
}
