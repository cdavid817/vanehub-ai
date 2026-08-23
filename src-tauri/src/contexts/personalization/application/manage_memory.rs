use std::sync::Arc;

use super::error::PersonalizationApplicationError;
use super::models::{CreateMemoryInput, DeleteMemoryOutcome, ResetCounts, UpdateMemoryPatch};
use super::ports::{
    ClockPort, DerivedIndexPort, MemoryMaintenanceRepository, MemoryProjectionPort,
    MemoryRepository, RetrievalIndexPort,
};
use crate::contexts::personalization::domain::{
    MaintenanceFailure, MaintenancePhase, MemoryId, MemoryPage, MemoryQuery, MemoryRecord,
    MemoryScopeFilter, MemoryStatus, ReconcileMemoryOutcome, ResetMemoryOutcome,
    ResetMemoryRequest,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Coordinates the authoritative file with its three derived views.
///
/// The ordering rule everywhere below is the same: the authoritative Markdown file changes first,
/// and derived state follows. If a derived update then fails, the memory is still correct on disk
/// and the outcome carries a repair-required failure — as opposed to the alternative, where a
/// derived write succeeds against a file change that did not, and the index advertises a memory
/// that does not exist.
pub(crate) struct MemoryApplicationService {
    repository: Arc<dyn MemoryRepository>,
    maintenance: Arc<dyn MemoryMaintenanceRepository>,
    projection: Arc<dyn MemoryProjectionPort>,
    derived_index: Arc<dyn DerivedIndexPort>,
    retrieval_index: Arc<dyn RetrievalIndexPort>,
    clock: Arc<dyn ClockPort>,
}

impl MemoryApplicationService {
    pub(crate) fn new(
        repository: Arc<dyn MemoryRepository>,
        maintenance: Arc<dyn MemoryMaintenanceRepository>,
        projection: Arc<dyn MemoryProjectionPort>,
        derived_index: Arc<dyn DerivedIndexPort>,
        retrieval_index: Arc<dyn RetrievalIndexPort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            repository,
            maintenance,
            projection,
            derived_index,
            retrieval_index,
            clock,
        }
    }

    /// List pages come from the projection, never from the files. This is the only place that
    /// choice is made, so no caller can accidentally take the N+1 path.
    pub(crate) fn list(&self, query: &MemoryQuery) -> Result<MemoryPage> {
        self.projection.list_page(query)
    }

    /// Detail comes from the authoritative file, because the projection deliberately holds no body.
    pub(crate) fn detail(&self, id: &MemoryId) -> Result<Option<MemoryRecord>> {
        self.repository.get(id)
    }

    pub(crate) fn create(&self, input: CreateMemoryInput) -> Result<CoordinatedMemory> {
        let now = self.clock.now();
        let record = self.repository.create(input, now)?;
        let failures = self.publish_derived(&record);
        Ok(CoordinatedMemory { record, failures })
    }

    pub(crate) fn update(
        &self,
        id: &MemoryId,
        expected_revision: u64,
        patch: UpdateMemoryPatch,
    ) -> Result<CoordinatedMemory> {
        let now = self.clock.now();
        let record = self.repository.update(id, expected_revision, patch, now)?;
        let failures = self.publish_derived(&record);
        Ok(CoordinatedMemory { record, failures })
    }

    pub(crate) fn delete(
        &self,
        id: &MemoryId,
        expected_revision: Option<u64>,
    ) -> Result<DeleteMemoryOutcome> {
        let mut outcome = self.repository.delete(id, expected_revision)?;
        if !outcome.deleted_file {
            // Nothing was there. Still reconcile derived state: an orphaned projection row or
            // retrieval entry is exactly the condition that would otherwise keep a deleted memory
            // recallable.
            self.revoke_derived(id, &mut outcome);
            return Ok(outcome);
        }
        self.revoke_derived(id, &mut outcome);
        match self.rebuild_index() {
            Ok(()) => outcome.removed_index_line = true,
            Err(_) => outcome.failures.push(MaintenanceFailure {
                memory_id: Some(id.clone()),
                phase: MaintenancePhase::DerivedIndex,
            }),
        }
        Ok(outcome)
    }

    pub(crate) fn preview_reset(
        &self,
        scope: &MemoryScopeFilter,
        statuses: &[MemoryStatus],
    ) -> Result<ResetCounts> {
        // Counted from complete filesystem enumeration rather than from the projection, because a
        // malformed file has no projection row and is precisely what the previous implementation
        // undercounted.
        self.maintenance.count_for_reset(scope, statuses)
    }

    pub(crate) fn reset(&self, request: &ResetMemoryRequest) -> Result<ResetMemoryOutcome> {
        let now = self.clock.now();
        let projected_before = self.projection.projected_ids().unwrap_or_default();
        let mut outcome = self.maintenance.reset(request, now)?;

        // Derived state is rebuilt from what survived rather than deleted per record: after a bulk
        // removal, "what should exist" is cheaper and more reliable to recompute than to diff.
        let surviving = self.surviving_records();
        for id in projected_before {
            if surviving.iter().any(|record| record.id == id) {
                continue;
            }
            match self.projection.remove(&id) {
                Ok(true) => outcome.deleted_projection_rows += 1,
                Ok(false) => {}
                Err(_) => outcome.failures.push(MaintenanceFailure {
                    memory_id: Some(id.clone()),
                    phase: MaintenancePhase::SqliteProjection,
                }),
            }
            match self.retrieval_index.revoke(&id) {
                Ok(()) => outcome.revoked_retrieval_entries += 1,
                Err(_) => outcome.failures.push(MaintenanceFailure {
                    memory_id: Some(id),
                    phase: MaintenancePhase::RetrievalIndex,
                }),
            }
        }

        if self.derived_index.rebuild(&surviving).is_err() {
            outcome.failures.push(MaintenanceFailure {
                memory_id: None,
                phase: MaintenancePhase::DerivedIndex,
            });
        }
        Ok(outcome)
    }

    /// Rebuilds every derived view from the authoritative files and revokes orphans.
    pub(crate) fn reconcile(&self) -> Result<ReconcileMemoryOutcome> {
        let now = self.clock.now();
        let mut outcome = self.maintenance.reconcile(now)?;
        let surviving = self.surviving_records();

        for record in &surviving {
            match self.projection.upsert(record, &record.content_hash()) {
                Ok(()) => outcome.rebuilt_projection_rows += 1,
                Err(_) => outcome.failures.push(MaintenanceFailure {
                    memory_id: Some(record.id.clone()),
                    phase: MaintenancePhase::SqliteProjection,
                }),
            }
        }

        // A projection row or retrieval entry with no authoritative file behind it is the shape
        // that keeps a deleted memory recallable, so both are swept here rather than only on the
        // delete path.
        for id in self.projection.projected_ids().unwrap_or_default() {
            if surviving.iter().any(|record| record.id == id) {
                continue;
            }
            if self.projection.remove(&id).is_err() {
                outcome.failures.push(MaintenanceFailure {
                    memory_id: Some(id),
                    phase: MaintenancePhase::SqliteProjection,
                });
            }
        }
        for id in self.retrieval_index.indexed_ids().unwrap_or_default() {
            let still_eligible = surviving
                .iter()
                .any(|record| record.id == id && matches!(record.status, MemoryStatus::Active));
            if still_eligible {
                continue;
            }
            match self.retrieval_index.revoke(&id) {
                Ok(()) => outcome.revoked_orphan_retrieval_entries += 1,
                Err(_) => outcome.failures.push(MaintenanceFailure {
                    memory_id: Some(id),
                    phase: MaintenancePhase::RetrievalIndex,
                }),
            }
        }

        match self.derived_index.rebuild(&surviving) {
            Ok(count) => outcome.rebuilt_index_lines = count,
            Err(_) => outcome.failures.push(MaintenanceFailure {
                memory_id: None,
                phase: MaintenancePhase::DerivedIndex,
            }),
        }
        Ok(outcome)
    }

    /// Publishes one record to every derived view, collecting rather than propagating failures.
    ///
    /// The authoritative write already succeeded by the time this runs. Turning a projection error
    /// into an `Err` here would tell the caller their memory was not saved, which is false, and
    /// would invite a retry that creates a second copy.
    fn publish_derived(&self, record: &MemoryRecord) -> Vec<MaintenanceFailure> {
        let mut failures = Vec::new();
        if self
            .projection
            .upsert(record, &record.content_hash())
            .is_err()
        {
            failures.push(MaintenanceFailure {
                memory_id: Some(record.id.clone()),
                phase: MaintenancePhase::SqliteProjection,
            });
        }

        // Only an active record participates in retrieval and in the index. A candidate reaching
        // either would be an unreviewed proposal presented as an established fact.
        if matches!(record.status, MemoryStatus::Active) {
            if self.retrieval_index.upsert(record).is_err() {
                failures.push(MaintenanceFailure {
                    memory_id: Some(record.id.clone()),
                    phase: MaintenancePhase::RetrievalIndex,
                });
            }
        } else if self.retrieval_index.revoke(&record.id).is_err() {
            failures.push(MaintenanceFailure {
                memory_id: Some(record.id.clone()),
                phase: MaintenancePhase::RetrievalIndex,
            });
        }

        if self.rebuild_index().is_err() {
            failures.push(MaintenanceFailure {
                memory_id: Some(record.id.clone()),
                phase: MaintenancePhase::DerivedIndex,
            });
        }
        failures
    }

    fn revoke_derived(&self, id: &MemoryId, outcome: &mut DeleteMemoryOutcome) {
        match self.projection.remove(id) {
            Ok(removed) => outcome.deleted_projection_row = removed,
            Err(_) => outcome.failures.push(MaintenanceFailure {
                memory_id: Some(id.clone()),
                phase: MaintenancePhase::SqliteProjection,
            }),
        }
        match self.retrieval_index.revoke(id) {
            Ok(()) => outcome.revoked_retrieval_entry = true,
            Err(_) => outcome.failures.push(MaintenanceFailure {
                memory_id: Some(id.clone()),
                phase: MaintenancePhase::RetrievalIndex,
            }),
        }
    }

    fn rebuild_index(&self) -> Result<()> {
        self.derived_index.rebuild(&self.surviving_records())?;
        Ok(())
    }

    /// Every record still readable from the authoritative store.
    ///
    /// Reads bodies, so it belongs to maintenance paths only — never to a list page.
    fn surviving_records(&self) -> Vec<MemoryRecord> {
        let Ok(entries) = self.maintenance.enumerate_owned_entries() else {
            return Vec::new();
        };
        entries
            .into_iter()
            .filter_map(|entry| entry.memory_id)
            .filter_map(|id| self.repository.get(&id).ok().flatten())
            .collect()
    }
}

/// What a coordinated write produced, plus anything derived that did not keep up.
///
/// Failures travel beside the record rather than replacing it, because the record is genuinely
/// saved; the caller surfaces a repair-required state without telling the user their edit was lost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CoordinatedMemory {
    pub(crate) record: MemoryRecord,
    pub(crate) failures: Vec<MaintenanceFailure>,
}

impl CoordinatedMemory {
    pub(crate) fn requires_repair(&self) -> bool {
        !self.failures.is_empty()
    }
}
