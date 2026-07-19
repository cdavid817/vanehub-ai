//! Concrete adapters for operation lifecycle and unified diagnostics.

mod operation_registry;
mod unified_logging;

pub(crate) use operation_registry::operation_service;
pub(crate) use unified_logging::UnifiedLoggingAdapter;
