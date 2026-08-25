mod error;
mod legacy_settings_compatibility;
mod manage_memory;
#[cfg(test)]
mod manage_memory_tests;
mod migrate_legacy_memories;
mod migrate_legacy_policy;
#[cfg(test)]
mod migrate_legacy_policy_tests;
mod models;
mod policy_cache;
mod ports;
mod preview_personalization;
mod resolve_policy;
#[cfg(test)]
mod resolve_policy_tests;
mod resolve_workspace_identity;
#[cfg(test)]
mod resolve_workspace_identity_tests;
mod run_startup_maintenance;
mod submit_candidates;
#[cfg(test)]
mod submit_candidates_tests;

pub(crate) use error::PersonalizationApplicationError;
pub(crate) use legacy_settings_compatibility::{
    LegacySettingField, LegacySettingsCompatibility, LegacySettingsView,
};
pub(crate) use manage_memory::{CoordinatedMemory, MemoryApplicationService};
pub(crate) use migrate_legacy_memories::{
    legacy_workspace_request, LegacyMemoryMigrationPorts, LegacyMemoryMigrationService,
};
pub(crate) use migrate_legacy_policy::{
    map_legacy_settings, project_to_legacy_settings, LegacyPersonalizationSettings, MigratedPolicy,
    ONEPIECE_AGENT_ID,
};
pub(crate) use models::{
    CreateMemoryInput, DeleteMemoryOutcome, DiscoveredLegacySource, LegacyMemoryFields,
    MemoryEligibilityCriteria, MigrationRunOutcome, ResetCounts, UpdateMemoryPatch,
    WorkspaceIdentityRequest,
};
pub(crate) use policy_cache::{
    is_transient_read_failure, LastKnownGoodPolicyCache, PolicyCacheKey,
};
pub(crate) use ports::{
    AgentCapabilityPort, CandidateRepository, ClockPort, DerivedIndexPort, LegacyAddressAliasPort,
    LegacyMemorySourcePort, LegacyPersonalizationSettingsPort, LegacyPolicyMigrationPort,
    LegacyRowMigrationPort, MaintenanceGatePort, MaintenanceLease, MemoryHealthPort,
    MemoryIdGeneratorPort, MemoryMaintenanceRepository, MemoryProjectionPort, MemoryRepository,
    MigrationJournalPort, MigrationStatePort, MutationAdmission, PolicyRepository,
    RetrievalIndexPort, SecretRedactionPort, WorkspaceIdentityPort,
};
pub(crate) use preview_personalization::{
    ContextSizeEstimate, EffectivePreview, PersonalizationPreviewService, PreviewInstructionSegment,
};
pub(crate) use resolve_policy::{PolicyResolutionService, ResolutionRequest};
pub(crate) use resolve_workspace_identity::WorkspaceIdentityResolver;
pub(crate) use run_startup_maintenance::{StartupMaintenancePorts, StartupMaintenanceService};
pub(crate) use submit_candidates::{
    CandidateRejection, CandidateSubmission, CandidateSubmissionOutcome,
    CandidateSubmissionService, MAX_CANDIDATES_PER_SUBMISSION,
};
