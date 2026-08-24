//! Concrete adapters for operation lifecycle and unified diagnostics.

mod log_index_repository;
mod log_index_schema;
mod log_index_support;
mod log_source_reader;
mod mission_control_repository;
mod operation_registry;
mod run_repository;
#[cfg(test)]
pub(crate) use run_repository::persistent_run_service_for_test;
mod unified_logging;

pub(crate) use log_index_repository::SqliteLogIndexRepository;
pub(crate) use log_index_schema::apply_log_query_index_schema;
pub(crate) use log_index_support::{
    BoundedLogIndexDiagnostics, SystemLogIndexClock, TauriBackfillPublisher,
    TauriLogNoticePublisher, UuidLogIndexIds,
};
pub(crate) use log_source_reader::UnifiedLogSourceReader;
pub(crate) use mission_control_repository::SqliteMissionControlRepository;
#[cfg(test)]
pub(crate) use operation_registry::operation_service;
pub(crate) use operation_registry::persistent_operation_service;
pub(crate) use run_repository::{
    apply_run_schema, apply_runner_projection_schema, persistent_run_service,
};
pub(crate) use unified_logging::UnifiedLoggingAdapter;
