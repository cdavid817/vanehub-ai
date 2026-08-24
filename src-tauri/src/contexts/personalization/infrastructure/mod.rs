mod derived_index;
mod markdown_memory_repository;
#[cfg(test)]
mod markdown_memory_repository_tests;
mod memory_directory_lock;
#[cfg(test)]
mod memory_directory_lock_tests;
mod memory_document;
mod memory_id_generator;
mod schema;
mod sqlite_candidate_repository;
#[cfg(test)]
mod sqlite_candidate_repository_tests;
mod sqlite_legacy_policy_migration;
#[cfg(test)]
mod sqlite_legacy_policy_migration_tests;
mod sqlite_memory_projection;
#[cfg(test)]
mod sqlite_memory_projection_tests;
mod sqlite_migration_journal;
mod sqlite_migration_state;
mod sqlite_policy_repository;
#[cfg(test)]
mod sqlite_policy_repository_tests;

pub(crate) use derived_index::MarkdownDerivedIndex;
pub(crate) use markdown_memory_repository::{MarkdownMemoryRepository, DERIVED_INDEX_FILE_NAME};
pub(crate) use memory_directory_lock::{
    is_lock_file, MemoryDirectoryGuard, MemoryDirectoryLock, MemoryLockRejection,
    MEMORY_LOCK_FILE_NAME,
};
pub(crate) use memory_document::{
    compose, content_hash, normalize_body, parse, MEMORY_SCHEMA_VERSION,
};
pub(crate) use memory_id_generator::UuidMemoryIdGenerator;
pub(crate) use schema::apply_schema;
pub(crate) use sqlite_candidate_repository::SqliteCandidateRepository;
pub(crate) use sqlite_legacy_policy_migration::SqliteLegacyPolicyMigration;
pub(crate) use sqlite_memory_projection::SqliteMemoryProjection;
pub(crate) use sqlite_migration_journal::SqliteMigrationJournal;
pub(crate) use sqlite_migration_state::SqliteMigrationState;
pub(crate) use sqlite_policy_repository::SqlitePolicyRepository;
