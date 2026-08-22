//! SQLite adapters and schema for the Extension Platform.

mod schema;
mod sqlite_publisher_keys;
#[cfg(test)]
mod sqlite_publisher_keys_tests;
mod sqlite_repository;
#[cfg(test)]
mod tests;

pub(crate) use schema::{
    apply_feature_gate_degradation_schema, apply_feature_gate_schema, apply_publisher_key_schema,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use sqlite_publisher_keys::SqlitePublisherKeyRepository;
pub(crate) use sqlite_repository::{
    FeatureGateSystemClock, SqliteFeatureGateAuditSink, SqliteFeatureGateRepository,
};
