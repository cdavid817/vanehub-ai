//! Concrete adapters for operation lifecycle and unified diagnostics.

mod mission_control_repository;
mod operation_registry;
mod run_repository;
mod unified_logging;

pub(crate) use mission_control_repository::SqliteMissionControlRepository;
#[cfg(test)]
pub(crate) use operation_registry::operation_service;
pub(crate) use operation_registry::persistent_operation_service;
pub(crate) use run_repository::{apply_run_schema, persistent_run_service};
pub(crate) use unified_logging::UnifiedLoggingAdapter;
