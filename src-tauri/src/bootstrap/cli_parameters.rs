use crate::contexts::operations::api::DiagnosticLogPort;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::cli_parameters::CliParametersApi;
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn assemble_cli_parameters_api(
    database: NativeDatabase,
    fallback_log_directory: PathBuf,
) -> CliParametersApi {
    let logging: Arc<dyn DiagnosticLogPort> =
        Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    CliParametersApi::new(database, logging)
}
