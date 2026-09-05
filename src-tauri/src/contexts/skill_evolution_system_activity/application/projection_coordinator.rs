use thiserror::Error;

use crate::contexts::skill_evolution_system_activity::domain::{
    ActivityDomainCheckpoint, ActivityDomainCursor, ActivityGapCode, ActivityProjectionFailureCode,
    EvolutionProjectionSource, EvolutionSourceDomain, ProjectionScanLimit, ProjectionSourceError,
    VerifiedProjectionEvent, MAX_SOURCE_SCAN_ITEMS,
};

pub(crate) const PROJECTION_BATCH_BUDGET_MS: i64 = 2_000;
pub(crate) const STARTUP_CATCH_UP_BUDGET_MS: i64 = 10_000;

#[derive(Debug)]
pub(crate) struct ActivityProjectionBatch {
    pub(crate) events: Vec<VerifiedProjectionEvent>,
    pub(crate) checkpoint: ActivityDomainCheckpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActivityProjectionBatchResult {
    pub(crate) scanned: usize,
    pub(crate) inserted: usize,
    pub(crate) replayed: usize,
    pub(crate) has_more: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ProjectionDomainResult {
    pub(crate) domain: EvolutionSourceDomain,
    pub(crate) outcome: Result<Option<ActivityProjectionBatchResult>, ProjectionCoordinatorError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StartupCatchUpPlan {
    pub(crate) readiness_wait_ms: i64,
    pub(crate) background_budget_ms: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StartupCatchUpResult {
    pub(crate) cycles: usize,
    pub(crate) domains: Vec<ProjectionDomainResult>,
    pub(crate) continuation_required: bool,
}

pub(crate) trait ActivityProjectionStore {
    fn cursor(
        &self,
        domain: EvolutionSourceDomain,
    ) -> Result<Option<ActivityDomainCursor>, ActivityProjectionStoreError>;

    fn commit_batch(
        &self,
        batch: &ActivityProjectionBatch,
    ) -> Result<ActivityProjectionBatchResult, ActivityProjectionStoreError>;

    fn record_failure(
        &self,
        domain: EvolutionSourceDomain,
        gap: Option<ActivityGapCode>,
        failure: ActivityProjectionFailureCode,
        expected_revision: u64,
    ) -> Result<(), ActivityProjectionStoreError>;
}

pub(crate) trait ActivityProjectionClock {
    fn now_ms(&self) -> i64;
}

pub(crate) struct ActivityProjectionCoordinator<'ports> {
    store: &'ports dyn ActivityProjectionStore,
    clock: &'ports dyn ActivityProjectionClock,
}

impl<'ports> ActivityProjectionCoordinator<'ports> {
    pub(crate) fn new(
        store: &'ports dyn ActivityProjectionStore,
        clock: &'ports dyn ActivityProjectionClock,
    ) -> Self {
        Self { store, clock }
    }

    pub(crate) fn process_domain(
        &self,
        source: &dyn EvolutionProjectionSource,
    ) -> Result<Option<ActivityProjectionBatchResult>, ProjectionCoordinatorError> {
        let domain = source.domain();
        let current = self.store.cursor(domain)?;
        if current.as_ref().is_some_and(|cursor| cursor.gap.is_some()) {
            return Ok(None);
        }
        let expected_revision = current.as_ref().map_or(0, |cursor| cursor.revision);
        let after = current
            .as_ref()
            .and_then(|cursor| cursor.opaque_cursor.as_ref());
        let page = match source.scan_committed(
            after,
            ProjectionScanLimit::new(MAX_SOURCE_SCAN_ITEMS)
                .map_err(ProjectionCoordinatorError::Source)?,
        ) {
            Ok(page) => page,
            Err(error) => {
                let (gap, failure) = classify_source_error(&error);
                self.store
                    .record_failure(domain, gap, failure, expected_revision)?;
                return Err(ProjectionCoordinatorError::Source(error));
            }
        };
        if page.events.is_empty() {
            return Ok(None);
        }

        if let Some((expected, actual)) = sequence_gap(&page.events, current.as_ref()) {
            self.store.record_failure(
                domain,
                Some(ActivityGapCode::MissingSequence),
                ActivityProjectionFailureCode::IntegrityFailed,
                expected_revision,
            )?;
            return Err(ProjectionCoordinatorError::SequenceGap {
                domain,
                expected,
                actual,
            });
        }

        let started_at_ms = self.clock.now_ms();
        let take = page
            .events
            .iter()
            .enumerate()
            .take_while(|(index, _)| {
                *index == 0
                    || self.clock.now_ms().saturating_sub(started_at_ms)
                        < PROJECTION_BATCH_BUDGET_MS
            })
            .count();
        let oldest_pending_at_ms = page
            .events
            .get(take)
            .map(|event| event.envelope.committed_at_ms);
        let mut events = page.events;
        let deferred = events.len().saturating_sub(take);
        events.truncate(take);
        let last = events
            .last()
            .ok_or(ProjectionCoordinatorError::EmptyBoundedBatch)?;
        let has_more = deferred > 0 || page.has_more;
        let checkpoint = ActivityDomainCheckpoint {
            source_domain: domain,
            opaque_cursor: last.source_cursor.clone(),
            last_sequence: last.source_sequence,
            last_source_hash: last.source_integrity_hash.clone(),
            retention_floor: page.retention_floor,
            pending_count: u64::try_from(deferred)
                .unwrap_or(u64::MAX)
                .saturating_add(u64::from(page.has_more)),
            oldest_pending_at_ms: oldest_pending_at_ms.filter(|_| has_more),
            last_success_at_ms: self.clock.now_ms(),
            expected_revision,
        };
        let mut result = self
            .store
            .commit_batch(&ActivityProjectionBatch { events, checkpoint })?;
        result.has_more = has_more;
        Ok(Some(result))
    }

    pub(crate) fn process_cycle(
        &self,
        sources: &[&dyn EvolutionProjectionSource],
    ) -> Vec<ProjectionDomainResult> {
        sources
            .iter()
            .map(|source| ProjectionDomainResult {
                domain: source.domain(),
                outcome: self.process_domain(*source),
            })
            .collect()
    }

    /// Callers schedule this only after application readiness; unfinished work is returned to the
    /// ordinary background scheduler instead of extending the startup critical path.
    pub(crate) fn process_startup_catch_up(
        &self,
        sources: &[&dyn EvolutionProjectionSource],
    ) -> StartupCatchUpResult {
        let started_at_ms = self.clock.now_ms();
        let mut cycles = 0;
        let mut domains = Vec::new();
        let continuation_required = loop {
            let cycle = self.process_cycle(sources);
            cycles += 1;
            let needs_continuation = cycle.iter().any(|result| {
                result
                    .outcome
                    .as_ref()
                    .ok()
                    .and_then(|batch| batch.as_ref())
                    .is_some_and(|batch| batch.has_more)
            });
            let made_progress = cycle
                .iter()
                .any(|result| matches!(result.outcome, Ok(Some(_))));
            domains.extend(cycle);
            if !needs_continuation || !made_progress {
                break needs_continuation;
            }
            if self.clock.now_ms().saturating_sub(started_at_ms) >= STARTUP_CATCH_UP_BUDGET_MS {
                break needs_continuation;
            }
        };
        StartupCatchUpResult {
            cycles,
            domains,
            continuation_required,
        }
    }

    pub(crate) const fn startup_plan(&self) -> StartupCatchUpPlan {
        StartupCatchUpPlan {
            readiness_wait_ms: 0,
            background_budget_ms: STARTUP_CATCH_UP_BUDGET_MS,
        }
    }
}

fn sequence_gap(
    events: &[VerifiedProjectionEvent],
    current: Option<&ActivityDomainCursor>,
) -> Option<(u64, u64)> {
    let first = events.first()?;
    if let Some(cursor) = current {
        let expected = cursor.last_sequence.saturating_add(1);
        if first.source_sequence != expected {
            return Some((expected, first.source_sequence));
        }
    }
    events.windows(2).find_map(|pair| {
        let expected = pair[0].source_sequence.saturating_add(1);
        (pair[1].source_sequence != expected).then_some((expected, pair[1].source_sequence))
    })
}

fn classify_source_error(
    error: &ProjectionSourceError,
) -> (Option<ActivityGapCode>, ActivityProjectionFailureCode) {
    match error {
        ProjectionSourceError::InvalidCursor => (
            Some(ActivityGapCode::RetentionFloorAdvanced),
            ActivityProjectionFailureCode::InvalidCursor,
        ),
        ProjectionSourceError::InvalidSequence => (
            Some(ActivityGapCode::MissingSequence),
            ActivityProjectionFailureCode::IntegrityFailed,
        ),
        ProjectionSourceError::IntegrityFailed => (
            Some(ActivityGapCode::SourceHashMismatch),
            ActivityProjectionFailureCode::IntegrityFailed,
        ),
        ProjectionSourceError::InvalidEnvelope(
            crate::contexts::skill_evolution_system_activity::domain::ActivityEnvelopeError::UnsupportedSchemaVersion(_),
        ) => (None, ActivityProjectionFailureCode::UnsupportedVersion),
        ProjectionSourceError::InvalidEnvelope(_) => {
            (None, ActivityProjectionFailureCode::InvalidEnvelope)
        }
        ProjectionSourceError::Unavailable => {
            (None, ActivityProjectionFailureCode::SourceUnavailable)
        }
        _ => (None, ActivityProjectionFailureCode::IntegrityFailed),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum ActivityProjectionStoreError {
    #[error("projection store state changed")]
    Conflict,
    #[error("projection receipt conflicts with committed identity")]
    ReceiptCollision,
    #[error("projection store input is invalid")]
    InvalidInput,
    #[error("projection store is unavailable")]
    Storage,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum ProjectionCoordinatorError {
    #[error(transparent)]
    Source(#[from] ProjectionSourceError),
    #[error(transparent)]
    Store(#[from] ActivityProjectionStoreError),
    #[error("source sequence gap for {domain:?}: expected {expected}, found {actual}")]
    SequenceGap {
        domain: EvolutionSourceDomain,
        expected: u64,
        actual: u64,
    },
    #[error("projection time budget produced an empty batch")]
    EmptyBoundedBatch,
}
