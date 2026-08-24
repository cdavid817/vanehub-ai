//! CLI parameter outbound adapters: the embedded canonical registry, the SQLite profile
//! repository, the cached CLI lifecycle snapshot, and the unified-logging/clock/filesystem edges.

mod catalog_loader;
#[cfg(test)]
mod generated_contract_tests;
mod lifecycle_snapshot_adapter;
mod runtime_adapters;
mod sqlite_profile_repository;
#[cfg(test)]
mod sqlite_profile_repository_tests;

pub(crate) use catalog_loader::EmbeddedCliParameterCatalog;
pub(crate) use lifecycle_snapshot_adapter::{
    CliLifecycleSnapshotAdapter, LifecycleVersionComparator,
};
pub(crate) use runtime_adapters::{FilesystemDirectoryProbe, UnifiedCliParameterDiagnostics};
pub(crate) use sqlite_profile_repository::{apply_schema, SqliteCliParameterProfileRepository};
