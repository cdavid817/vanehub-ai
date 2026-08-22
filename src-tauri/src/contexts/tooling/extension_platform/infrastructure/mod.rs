//! SQLite adapters and schema for the Extension Platform.

mod package_reader;
#[cfg(test)]
mod package_reader_tests;
mod roots;
#[cfg(test)]
mod roots_tests;
mod schema;
mod sqlite_developer_mode;
#[cfg(test)]
mod sqlite_developer_mode_tests;
mod sqlite_publisher_keys;
#[cfg(test)]
mod sqlite_publisher_keys_tests;
mod sqlite_repository;
#[cfg(test)]
mod tests;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use package_reader::{read_extension_package, PackageReadError, ReadPackage};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use roots::{ExtensionRootKind, ExtensionRoots, RootError, ALL_EXTENSION_ROOT_KINDS};
pub(crate) use schema::{
    apply_developer_mode_schema, apply_feature_gate_degradation_schema, apply_feature_gate_schema,
    apply_publisher_key_schema,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use sqlite_developer_mode::{
    SqliteDeveloperModeAuditSink, SqliteDeveloperModeRepository,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use sqlite_publisher_keys::SqlitePublisherKeyRepository;
pub(crate) use sqlite_repository::{
    FeatureGateSystemClock, SqliteFeatureGateAuditSink, SqliteFeatureGateRepository,
};
