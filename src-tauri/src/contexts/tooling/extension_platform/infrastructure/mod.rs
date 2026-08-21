//! SQLite adapters and schema for the Extension Platform.

mod schema;
mod sqlite_repository;
#[cfg(test)]
mod tests;

pub(crate) use schema::apply_feature_gate_schema;
pub(crate) use sqlite_repository::{
    FeatureGateSystemClock, SqliteFeatureGateAuditSink, SqliteFeatureGateRepository,
};
