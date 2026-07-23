//! OpenTelemetry and SQLite adapters are implemented in this outer layer.

mod composite_telemetry;
mod credential_adapter;
mod lifecycle;
mod observability_repository;
mod otel_telemetry;
mod privacy;
mod queries;
mod random_identity;
mod retention;
mod rows;
mod schema;
mod settings_repository;
mod sqlite_repository;
mod storage_mapping;

pub(crate) use random_identity::RandomExecutionIdentity;
pub(crate) use schema::apply_schema;
pub(crate) use sqlite_repository::SqliteExecutionTimelineRepository;

#[cfg(test)]
mod tests;
pub(crate) use composite_telemetry::CompositeExecutionTelemetry;
pub(crate) use credential_adapter::OsObservabilityCredentialAdapter;
pub(crate) use lifecycle::ExecutionTelemetryLifecycle;
pub(crate) use otel_telemetry::OpenTelemetryExecutionExporter;
