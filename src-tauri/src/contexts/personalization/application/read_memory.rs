use std::sync::Arc;

use super::models::{
    AuthorizedMemoryRelation, EligibilityEnumerationBudget, EmbeddingEgressDecision,
    IndexMaintenanceRecord, MemoryEligibilityCriteria, MemoryReadRefusal, PinnedMemoryBody,
};
use super::ports::{
    MemoryHealthPort, MemoryIdGeneratorPort, MemoryMaintenanceRepository, MemoryProjectionPort,
    MemoryRepository,
};
use crate::contexts::personalization::domain::{
    EffectivePersonalizationSnapshot, MemoryAudience, MemoryId, MemoryReadContext,
    MemoryReadHandle, MemoryRecord, MemoryRuntimeHealth, MemoryScope, MemoryStatus,
    SessionPersonalizationMode, SnapshotMemoryRef, WorkspaceBinding,
};

/// One index-page ref after authoritative re-validation, with the handle a later body read pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedMemoryRef {
    pub(crate) snapshot_ref: SnapshotMemoryRef,
    pub(crate) handle: MemoryReadHandle,
}

/// The one governed read path every Agent-facing memory surface goes through.
///
/// Three capabilities live here and are deliberately separate methods rather than one with an
/// `Option<context>`: reads for an Agent require an authentic `MemoryReadContext`; index
/// maintenance enumerates every valid active record for the host-wide keyword index and takes no
/// context at all, because it is not an Agent read; egress decisions consult the authoritative
/// record for the background embedder. Owner management stays on `MemoryApplicationService`.
///
/// Every Agent read re-reads the authoritative file and checks the pinned handle against it, so
/// a stale projection row, an external edit that kept the revision, or a scope change that left
/// the body alone all fail the same way: the record is dropped, never substituted.
pub(crate) struct GovernedMemoryReadService {
    repository: Arc<dyn MemoryRepository>,
    maintenance: Arc<dyn MemoryMaintenanceRepository>,
    projection: Arc<dyn MemoryProjectionPort>,
    health: Arc<dyn MemoryHealthPort>,
    /// Random per process. A context fingerprint is bound to it, so a value assembled anywhere
    /// but here, or carried across an application restart, never authenticates.
    epoch: String,
}

impl GovernedMemoryReadService {
    pub(crate) fn new(
        repository: Arc<dyn MemoryRepository>,
        maintenance: Arc<dyn MemoryMaintenanceRepository>,
        projection: Arc<dyn MemoryProjectionPort>,
        health: Arc<dyn MemoryHealthPort>,
        ids: &dyn MemoryIdGeneratorPort,
    ) -> Self {
        Self {
            repository,
            maintenance,
            projection,
            health,
            epoch: ids.generate().as_str().to_string(),
        }
    }

    /// Freezes the read context for one generation from its resolved snapshot.
    ///
    /// The only constructor a runtime can reach. The workspace binding comes from the session
    /// owner, never from the model, and an unresolved binding is preserved as such so the frozen
    /// context denies every read rather than degrading into a workspace-less standard session.
    pub(crate) fn freeze_context(
        &self,
        snapshot: &EffectivePersonalizationSnapshot,
        workspace: WorkspaceBinding,
        generation_id: &str,
        seat_id: Option<&str>,
    ) -> MemoryReadContext {
        let generation = match self.health.health() {
            MemoryRuntimeHealth::Ready { generation } => generation,
            _ => 0,
        };
        MemoryReadContext::freeze(
            snapshot,
            workspace,
            generation_id,
            seat_id,
            generation,
            &self.epoch,
        )
    }

    /// Every check a read must pass before touching a record: authenticity, a permitted read, and
    /// a memory store still in the generation the context was frozen under.
    pub(crate) fn admit(&self, context: &MemoryReadContext) -> Result<(), MemoryReadRefusal> {
        if !context.is_authentic(&self.epoch) {
            return Err(MemoryReadRefusal::Inauthentic);
        }
        if !context.permits_any_read() {
            return Err(MemoryReadRefusal::ReadDenied);
        }
        match self.health.health() {
            MemoryRuntimeHealth::Ready { generation }
                if generation == context.maintenance_generation =>
            {
                Ok(())
            }
            _ => Err(MemoryReadRefusal::Unhealthy),
        }
    }

    /// The complete eligible metadata relation for this context, or a typed refusal.
    ///
    /// Complete or nothing: an enumeration that stopped at its budget is reported as incomplete
    /// rather than returned as a smaller relation a caller might search over.
    pub(crate) fn open_query(
        &self,
        context: &MemoryReadContext,
        budget: EligibilityEnumerationBudget,
    ) -> Result<AuthorizedMemoryRelation, MemoryReadRefusal> {
        self.admit(context)?;
        let relation = self
            .projection
            .eligible_authority(&criteria_for(context), budget)
            .map_err(|error| MemoryReadRefusal::Storage(error.to_string()))?;
        if !relation.complete {
            return Err(MemoryReadRefusal::Incomplete);
        }
        Ok(relation)
    }

    /// Re-validates a snapshot's index refs against the authoritative files.
    ///
    /// Names and descriptions are memory content too, so an index line is only delivered once
    /// the file behind it still exists, still parses, is still active and admitted under this
    /// context, and still carries the pinned revision and hash. Anything else is omitted.
    pub(crate) fn verify_refs(
        &self,
        context: &MemoryReadContext,
        refs: &[SnapshotMemoryRef],
    ) -> Vec<VerifiedMemoryRef> {
        if self.admit(context).is_err() {
            return Vec::new();
        }
        refs.iter()
            .filter_map(|entry| {
                let record = self.admitted_record(context, &entry.id)?;
                let handle = MemoryReadHandle::of(&record);
                (handle.revision == entry.revision && handle.content_hash == entry.content_hash)
                    .then(|| VerifiedMemoryRef {
                        snapshot_ref: entry.clone(),
                        handle,
                    })
            })
            .collect()
    }

    /// Bodies for pinned handles, in handle order, dropping any that no longer match.
    ///
    /// A handle whose record moved is absent rather than newer: the caller was shown one version
    /// and must not silently receive another. Only bounded candidate sets reach here, so the cost
    /// is one file per surviving candidate rather than the whole pool.
    pub(crate) fn read_pinned(
        &self,
        context: &MemoryReadContext,
        handles: &[MemoryReadHandle],
    ) -> Result<Vec<PinnedMemoryBody>, MemoryReadRefusal> {
        self.admit(context)?;
        Ok(handles
            .iter()
            .filter_map(|handle| {
                let record = self.admitted_record(context, &handle.id)?;
                handle.matches(&record).then(|| PinnedMemoryBody {
                    handle: handle.clone(),
                    name: record.name.clone(),
                    description: record.description.clone(),
                    memory_type: record.memory_type,
                    content: record.content.clone(),
                    scope_hint: scope_hint(&record.scope),
                    updated_at: record.updated_at,
                    created_at: record.created_at,
                })
            })
            .collect())
    }

    /// Every valid active record, for the host-wide local index. Not an Agent read.
    ///
    /// Storage failures propagate rather than degrading to an empty list: reconciliation treats an
    /// empty snapshot as "everything was deleted", and a failed enumeration must never say that.
    pub(crate) fn index_maintenance_records(
        &self,
    ) -> Result<Vec<IndexMaintenanceRecord>, MemoryReadRefusal> {
        if !self.health.health().allows_memory_use() {
            return Err(MemoryReadRefusal::Unhealthy);
        }
        let entries = self
            .maintenance
            .enumerate_owned_entries()
            .map_err(|error| MemoryReadRefusal::Storage(error.to_string()))?;
        let mut records = Vec::new();
        for id in entries.into_iter().filter_map(|entry| entry.memory_id) {
            let record = self
                .repository
                .get(&id)
                .map_err(|error| MemoryReadRefusal::Storage(error.to_string()))?;
            let Some(record) = record.filter(is_indexable) else {
                continue;
            };
            records.push(IndexMaintenanceRecord {
                handle: MemoryReadHandle::of(&record),
                content: record.content.clone(),
                created_at: record.created_at,
                egress_restricted: egress_restricted(&record),
            });
        }
        Ok(records)
    }

    /// Whether each queued body may still be sent to the remote embedder, decided from the
    /// authoritative file at dispatch time rather than from the row that queued it.
    ///
    /// `queued` pairs each immutable id with the content hash the index row holds. A body whose
    /// hash no longer matches the file is refused too: the queue would be sending text the
    /// authoritative record no longer contains.
    pub(crate) fn embedding_egress(
        &self,
        queued: &[(MemoryId, String)],
    ) -> Vec<EmbeddingEgressDecision> {
        let healthy = self.health.health().allows_memory_use();
        queued
            .iter()
            .map(|(id, content_hash)| EmbeddingEgressDecision {
                id: id.clone(),
                permitted: healthy
                    && self
                        .repository
                        .get(id)
                        .ok()
                        .flatten()
                        .is_some_and(|record| {
                            is_indexable(&record)
                                && !egress_restricted(&record)
                                && record.content_hash() == *content_hash
                        }),
            })
            .collect()
    }

    fn admitted_record(&self, context: &MemoryReadContext, id: &MemoryId) -> Option<MemoryRecord> {
        let record = self.repository.get(id).ok().flatten()?;
        if record.validate().is_err() || context.admits(&record).is_err() {
            return None;
        }
        Some(record)
    }
}

fn criteria_for(context: &MemoryReadContext) -> MemoryEligibilityCriteria {
    MemoryEligibilityCriteria {
        agent_id: context.subject.agent_id.clone(),
        allow_global: context.global_allowed,
        workspace: context.workspace_allowed.clone(),
        project_only: matches!(
            context.session_mode,
            SessionPersonalizationMode::ProjectOnly
        ),
        limit: 0,
    }
}

fn is_indexable(record: &MemoryRecord) -> bool {
    matches!(record.status, MemoryStatus::Active) && record.validate().is_ok()
}

/// No consent capability exists for sending a scoped or audience-restricted body to a remote
/// embedder, so anything but a global all-Agent record stays keyword-only.
fn egress_restricted(record: &MemoryRecord) -> bool {
    !matches!(record.scope, MemoryScope::Global)
        || !matches!(record.audience, MemoryAudience::AllAgents)
}

fn scope_hint(scope: &MemoryScope) -> String {
    scope
        .workspace_key()
        .map(|key| key.as_str().to_string())
        .unwrap_or_else(|| scope.kind_str().to_string())
}
