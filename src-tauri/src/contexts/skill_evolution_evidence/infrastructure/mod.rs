mod deletion;
mod feedback;
mod feedback_authorization;
mod feedback_storage;
mod governance;
mod processor;
mod purge;
mod queries;
mod query_models;
mod schema;
mod seed_builder;
mod seed_repository;
mod sqlite_repository;
mod storage_values;

pub(crate) use feedback::*;
pub(crate) use feedback_authorization::*;
pub(crate) use governance::*;
pub(crate) use processor::*;
pub(crate) use purge::*;
pub(crate) use queries::*;
pub(crate) use query_models::*;
pub(crate) use schema::apply_schema;
pub(crate) use seed_repository::SeedRebuildOutcome;
pub(crate) use sqlite_repository::{
    EvidenceRepositoryError, PersistSignalOutcome, SqliteEvolutionEvidenceRepository,
};

#[cfg(test)]
mod feedback_tests;
#[cfg(test)]
mod governance_tests;
#[cfg(test)]
mod query_tests;
#[cfg(test)]
mod seed_tests;
#[cfg(test)]
mod tests;
