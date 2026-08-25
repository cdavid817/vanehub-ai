mod candidate;
mod error;
mod legacy_identity;
#[cfg(test)]
mod legacy_identity_tests;
mod maintenance;
#[cfg(test)]
mod maintenance_tests;
mod memory;
#[cfg(test)]
mod memory_tests;
mod policy;
#[cfg(test)]
mod policy_tests;
mod query;
mod resolution;
#[cfg(test)]
mod resolution_tests;
mod scope;
#[cfg(test)]
mod scope_tests;
mod snapshot;
mod workspace_identity;
#[cfg(test)]
mod workspace_identity_tests;

pub(crate) use candidate::{
    ArchiveMemoryCandidate, CandidateReviewStatus, CreateMemoryCandidate, MemoryCandidate,
    MemoryCandidateOperation, ReviewAction, ReviewOutcome, UpdateMemoryCandidate,
};
pub(crate) use error::{IdentityRejection, PersonalizationDomainError};
pub(crate) use legacy_identity::{
    LegacyAddressKey, LegacySourceFingerprint, LegacySourceId, LegacySourceLocator,
    LegacyTableKind, MigrationJournalEntry, MigrationStage, NormalizedLegacyPath,
};
pub(crate) use maintenance::{
    MaintenanceFailure, MaintenancePhase, MemoryRuntimeHealth, MigrationPhase, MigrationState,
    OwnedEntryClassification, ReconcileMemoryOutcome, ResetConfirmationToken, ResetMemoryOutcome,
    ResetMemoryPreview, ResetMemoryRequest, ResetRefusal, StorageEntry, RESET_CONFIRMATION_PHRASE,
    RESET_TOKEN_TTL_SECONDS,
};
pub(crate) use memory::{
    content_hash, eligibility, LegacyMemorySaveSource, MemoryAudience, MemoryId, MemoryProvenance,
    MemoryRecord, MemoryScope, MemorySensitivity, MemorySource, MemoryStatus, MemoryType,
    MEMORY_AUDIENCE_MAX_AGENTS, MEMORY_CONTENT_MAX_CHARS, MEMORY_DESCRIPTION_MAX_CHARS,
    MEMORY_LEGACY_FIELD_MAX_CHARS, MEMORY_NAME_MAX_CHARS,
};
pub(crate) use policy::{
    InstructionMergeMode, PatchPolicyResult, PersonalizationPolicyPatch,
    PersonalizationPolicyRecord, PolicyToggle, RevisionConflict, SessionPersonalizationMode,
    DEFAULT_POLICY_SET_ID, INSTRUCTION_FIELD_MAX_CHARS,
};
pub(crate) use query::{
    MemoryCursor, MemoryOrder, MemoryPage, MemoryQuery, MemoryScopeFilter, MemorySummary,
    MEMORY_PAGE_DEFAULT_SIZE, MEMORY_PAGE_MAX_SIZE,
};
pub(crate) use resolution::{
    resolve, MaintenanceState, PersonalizationLayers, PolicyLayerState, PolicyResolutionBundle,
};
pub(crate) use scope::{
    AgentId, AgentRuntimeKind, PersonalizationPolicyScope, SessionId, WorkspaceIdentity,
    WorkspaceKey, WorkspaceKind,
};
pub(crate) use snapshot::{
    EffectiveMemoryAccess, EffectivePersonalizationSnapshot, ExcludedInstructionSegment,
    InstructionExclusionReason, InstructionField, InstructionMergeAction, MemoryBlockReason,
    MemoryDeliveryMode, MemoryEligibilitySummary, MemoryExclusionCount, MemorySaveConstraint,
    MemoryScopeAllowance, PersonalizationExclusion, PersonalizationExclusionReason,
    PersonalizationResolutionContext, PersonalizationRuntimeCapabilities, PersonalizationWarning,
    PersonalizationWarningCode, ResolvedInstructionSegment, SnapshotMemoryRef,
    FAIL_CLOSED_REVISION_TOKEN,
};
pub(crate) use workspace_identity::{
    local_paths_fold_case, normalize_local_root, WorkspaceIdentitySource,
};
