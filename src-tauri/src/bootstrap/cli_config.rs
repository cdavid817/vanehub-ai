use crate::contexts::operations::api::DiagnosticLogPort;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::cli_config::infrastructure::{
    NativeCliGlobalConfigAdapter, OsCliConfigCredentialAdapter, SqliteCliConfigRepository,
};
use crate::contexts::tooling::cli_config::CliConfigApi;
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn assemble_cli_config_api(
    database: NativeDatabase,
    fallback_log_directory: PathBuf,
) -> Result<CliConfigApi, String> {
    let logging: Arc<dyn DiagnosticLogPort> =
        Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    Ok(CliConfigApi::new(
        Arc::new(SqliteCliConfigRepository::new(database)),
        Arc::new(OsCliConfigCredentialAdapter::new()),
        Arc::new(NativeCliGlobalConfigAdapter::new().map_err(|error| error.to_string())?),
        logging,
    ))
}
