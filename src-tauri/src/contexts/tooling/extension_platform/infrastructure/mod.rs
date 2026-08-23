//! SQLite adapters and schema for the Extension Platform.

mod active_contribution;
#[cfg(test)]
mod active_contribution_tests;
mod package_reader;
#[cfg(test)]
mod package_reader_tests;
mod persistence_schema;
mod reconciler;
#[cfg(test)]
mod reconciler_tests;
mod roots;
#[cfg(test)]
mod roots_tests;
mod schema;
mod snapshot_store;
#[cfg(test)]
mod snapshot_store_tests;
mod sqlite_developer_mode;
#[cfg(test)]
mod sqlite_developer_mode_tests;
mod sqlite_persistence;
#[cfg(test)]
mod sqlite_persistence_tests;
mod sqlite_publisher_keys;
#[cfg(test)]
mod sqlite_publisher_keys_tests;
mod sqlite_repository;
#[cfg(test)]
mod tests;

pub(crate) use active_contribution::SqliteActiveContributionReader;
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use package_reader::{read_extension_package, PackageReadError, ReadPackage};
pub(crate) use persistence_schema::{
    apply_extension_persistence_schema, repair_snapshot_contribution_digest,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use reconciler::{reconcile, referenced_package_hashes};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use roots::{ExtensionRoots, RootError};
pub(crate) use schema::{
    apply_developer_mode_schema, apply_feature_gate_degradation_schema, apply_feature_gate_schema,
    apply_publisher_key_schema, apply_snapshot_schema,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use snapshot_store::{
    read_snapshot, FilesystemSnapshotContentStore, SqliteSnapshotPointerRepository,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use sqlite_developer_mode::{
    SqliteDeveloperModeAuditSink, SqliteDeveloperModeRepository,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use sqlite_persistence::{
    claim_for, record_operation_witness, record_package, record_snapshot_detail,
    RecordedContribution, SqliteRuntimeGenerationRepository, SqliteVersionClaimRepository,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use sqlite_publisher_keys::SqlitePublisherKeyRepository;
pub(crate) use sqlite_repository::{
    FeatureGateSystemClock, SqliteFeatureGateAuditSink, SqliteFeatureGateRepository,
};
