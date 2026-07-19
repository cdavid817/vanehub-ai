use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::extensions::api::ExtensionApi;
use crate::contexts::tooling::extensions::application::ExtensionApplicationService;
use crate::contexts::tooling::extensions::infrastructure::{
    ExtensionOperationAdapter, ManagedExtensionInstallation, OwnedExtensionRuntime,
    SqliteExtensionRepository, SystemExtensionClock, SystemExtensionEnvironment,
    UnifiedExtensionLoggingAdapter,
};
use crate::platform::database::NativeDatabase;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) fn assemble_extension_api(
    database: NativeDatabase,
    operations: OperationsApi,
    fallback_log_dir: PathBuf,
) -> ExtensionApi {
    let logging = Arc::new(UnifiedLoggingAdapter::active(fallback_log_dir));
    let root = database
        .db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("extensions");
    let runtime = Arc::new(OwnedExtensionRuntime::new(root.clone()));
    ExtensionApi::new(ExtensionApplicationService::new(
        Arc::new(SqliteExtensionRepository::new(database)),
        Arc::new(SystemExtensionEnvironment::new()),
        Arc::new(ManagedExtensionInstallation::new(root)),
        runtime.clone(),
        runtime,
        Arc::new(ExtensionOperationAdapter::new(operations)),
        Arc::new(UnifiedExtensionLoggingAdapter::new(logging)),
        Arc::new(SystemExtensionClock),
    ))
}
