use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::cli::api::{CliApi, CliError};
use crate::contexts::tooling::cli::application::{CliApplicationPorts, CliApplicationService};
use crate::contexts::tooling::cli::infrastructure::{
    CliDetectionAdapter, CliExecutableLocatorAdapter, CliMutationAdapter, CliOperationAdapter,
    CliPackageAdapter, SqliteCliStatusRepository, SystemCliClock, UnifiedCliLoggingAdapter,
};
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn assemble_cli_api(
    database: NativeDatabase,
    operations: OperationsApi,
    fallback_log_dir: PathBuf,
) -> CliApi {
    let logging = Arc::new(UnifiedLoggingAdapter::active(fallback_log_dir));
    CliApi::new(CliApplicationService::new(CliApplicationPorts {
        repository: Arc::new(SqliteCliStatusRepository::new(database)),
        detection: Arc::new(CliDetectionAdapter::new()),
        executable_locator: Arc::new(CliExecutableLocatorAdapter::new()),
        packages: Arc::new(CliPackageAdapter::new()),
        operations: Arc::new(CliOperationAdapter::new(operations)),
        logging: Arc::new(UnifiedCliLoggingAdapter::new(logging.clone(), logging)),
        clock: Arc::new(SystemCliClock),
        mutations: Arc::new(CliMutationAdapter::default()),
    }))
}

pub(crate) fn start_initial_cli_refresh(api: CliApi) -> Result<(), CliError> {
    if !api.needs_initial_refresh()? {
        return Ok(());
    }
    let prepared = api.prepare_refresh(None, "Initial CLI detection refresh".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_refresh(prepared);
    });
    Ok(())
}
