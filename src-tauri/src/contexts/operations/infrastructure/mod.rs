//! Concrete adapters for operation lifecycle and unified diagnostics.

mod operation_registry;
mod unified_logging;

#[cfg(test)]
pub(crate) use operation_registry::operation_service;
pub(crate) use operation_registry::persistent_operation_service;
pub(crate) use unified_logging::UnifiedLoggingAdapter;
