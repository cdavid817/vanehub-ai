use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::cli::api::CliApi;
use crate::contexts::tooling::cli_parameters::api::{
    CliParameterRuntimeApi, CliParameterSettingsApi,
};
use crate::contexts::tooling::cli_parameters::application::service::CliParameterApplicationService;
use crate::contexts::tooling::cli_parameters::domain::definition::CliParameterPlatform;
use crate::contexts::tooling::cli_parameters::infrastructure::{
    CliLifecycleSnapshotAdapter, EmbeddedCliParameterCatalog, FilesystemDirectoryProbe,
    LifecycleVersionComparator, SqliteCliParameterProfileRepository,
    UnifiedCliParameterDiagnostics,
};
use crate::platform::database::NativeDatabase;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Both CLI-parameter facades, over one application service.
///
/// They share the caller's `NativeDatabase` handle — and therefore its single connection pool — and
/// read compatibility from the CLI lifecycle subdomain's cached detection state, so no second pool
/// and no second detector exist. Sharing the service also means the settings page and the launch
/// path can never disagree about a profile: there is one repository, one catalog, one clock.
///
/// The registry is parsed and validated once here. An invalid registry is reported as a startup
/// diagnostic; profile loading later returns a structured catalog error rather than panicking.
pub(crate) fn assemble_cli_parameter_apis(
    database: NativeDatabase,
    cli: CliApi,
    fallback_log_directory: PathBuf,
) -> (CliParameterRuntimeApi, CliParameterSettingsApi) {
    let logging: Arc<dyn DiagnosticLogPort> = Arc::new(UnifiedLoggingAdapter::active(
        fallback_log_directory.clone(),
    ));
    let service = CliParameterApplicationService {
        catalog: Arc::new(EmbeddedCliParameterCatalog),
        repository: Arc::new(SqliteCliParameterProfileRepository::new(database)),
        installations: Arc::new(CliLifecycleSnapshotAdapter::new(cli)),
        directories: Arc::new(FilesystemDirectoryProbe),
        diagnostics: Arc::new(UnifiedCliParameterDiagnostics::new(logging.clone())),
        comparator: Arc::new(LifecycleVersionComparator),
        platform: CliParameterPlatform::current(),
    };
    let api = CliParameterRuntimeApi::new(service.clone());
    let settings = CliParameterSettingsApi::new(service);
    let (severity, message) = match api.validate_registry() {
        Ok(version) => (
            LogSeverity::Info,
            format!("cli parameter registry loaded at catalog version {version}"),
        ),
        Err(error) => (
            LogSeverity::Error,
            format!(
                "cli parameter registry is invalid: {}",
                error.code().as_str()
            ),
        ),
    };
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity,
        category: "cli.parameter.registry".to_string(),
        message,
        context: BTreeMap::new(),
    });
    (api, settings)
}
