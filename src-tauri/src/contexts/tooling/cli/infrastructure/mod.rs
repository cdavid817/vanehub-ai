mod candidates;
mod detection_adapter;
mod executable_locator;
mod native_config_reader;
mod package_adapter;
mod process_adapter;
mod runtime_adapters;
mod sqlite_repository;
mod support;

pub(crate) use detection_adapter::CliDetectionAdapter;
pub(crate) use executable_locator::CliExecutableLocatorAdapter;
pub(crate) use native_config_reader::NativeConfigReader;
pub(crate) use package_adapter::CliPackageAdapter;
pub(crate) use runtime_adapters::{
    CliMutationAdapter, CliOperationAdapter, SystemCliClock, UnifiedCliLoggingAdapter,
};
pub(crate) use sqlite_repository::SqliteCliStatusRepository;
