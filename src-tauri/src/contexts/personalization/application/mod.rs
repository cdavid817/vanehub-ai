mod error;
mod manage_memory;
#[cfg(test)]
mod manage_memory_tests;
mod models;
mod ports;

pub(crate) use error::PersonalizationApplicationError;
pub(crate) use manage_memory::{CoordinatedMemory, MemoryApplicationService};
pub(crate) use models::{
    CreateMemoryInput, DeleteMemoryOutcome, ResetCounts, UpdateMemoryPatch,
    WorkspaceIdentityRequest,
};
pub(crate) use ports::{
    CandidateRepository, ClockPort, DerivedIndexPort, MemoryIdGeneratorPort,
    MemoryMaintenanceRepository, MemoryProjectionPort, MemoryRepository, MigrationJournalPort,
    MigrationStatePort, PolicyRepository, RetrievalIndexPort, WorkspaceIdentityPort,
};
