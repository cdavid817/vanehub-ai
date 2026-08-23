mod derived_index;
mod markdown_memory_repository;
#[cfg(test)]
mod markdown_memory_repository_tests;
mod memory_document;
mod memory_id_generator;
mod schema;
mod sqlite_candidate_repository;
#[cfg(test)]
mod sqlite_candidate_repository_tests;
mod sqlite_memory_projection;
#[cfg(test)]
mod sqlite_memory_projection_tests;
mod sqlite_migration_state;
mod sqlite_policy_repository;
#[cfg(test)]
mod sqlite_policy_repository_tests;

pub(crate) use derived_index::MarkdownDerivedIndex;
pub(crate) use markdown_memory_repository::{MarkdownMemoryRepository, DERIVED_INDEX_FILE_NAME};
pub(crate) use memory_document::{
    compose, content_hash, normalize_body, parse, MEMORY_SCHEMA_VERSION,
};
pub(crate) use memory_id_generator::UuidMemoryIdGenerator;
pub(crate) use schema::apply_schema;
pub(crate) use sqlite_candidate_repository::SqliteCandidateRepository;
pub(crate) use sqlite_memory_projection::SqliteMemoryProjection;
pub(crate) use sqlite_migration_state::SqliteMigrationState;
pub(crate) use sqlite_policy_repository::SqlitePolicyRepository;
