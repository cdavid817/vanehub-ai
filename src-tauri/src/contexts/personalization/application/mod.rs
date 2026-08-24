mod error;
mod manage_memory;
#[cfg(test)]
mod manage_memory_tests;
mod migrate_legacy_policy;
#[cfg(test)]
mod migrate_legacy_policy_tests;
mod models;
mod ports;
mod resolve_workspace_identity;
#[cfg(test)]
mod resolve_workspace_identity_tests;

pub(crate) use error::PersonalizationApplicationError;
pub(crate) use manage_memory::{CoordinatedMemory, MemoryApplicationService};
pub(crate) use migrate_legacy_policy::{
    map_legacy_settings, project_to_legacy_settings, LegacyPersonalizationSettings, MigratedPolicy,
    ONEPIECE_AGENT_ID,
};
pub(crate) use models::{
    CreateMemoryInput, DeleteMemoryOutcome, ResetCounts, UpdateMemoryPatch,
    WorkspaceIdentityRequest,
};
pub(crate) use ports::{
    CandidateRepository, ClockPort, DerivedIndexPort, LegacyPolicyMigrationPort,
    MemoryIdGeneratorPort, MemoryMaintenanceRepository, MemoryProjectionPort, MemoryRepository,
    MigrationJournalPort, MigrationStatePort, PolicyRepository, RetrievalIndexPort,
    WorkspaceIdentityPort,
};
pub(crate) use resolve_workspace_identity::WorkspaceIdentityResolver;
