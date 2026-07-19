mod installation_adapter;
mod process_adapter;
mod runtime_adapter;
mod runtime_support;
mod sqlite_repository;

pub(crate) use installation_adapter::{ManagedExtensionInstallation, SystemExtensionEnvironment};
pub(crate) use runtime_adapter::OwnedExtensionRuntime;
pub(crate) use runtime_support::{
    ExtensionOperationAdapter, SystemExtensionClock, UnifiedExtensionLoggingAdapter,
};
pub(crate) use sqlite_repository::{apply_schema, SqliteExtensionRepository};
