use crate::contexts::desktop::api::DesktopSettingsApi;
use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::sessions::api::{ArchivalPolicy, SessionsApi};
use crate::contexts::sessions::application::{SessionApplicationPorts, SessionsApplicationService};
use crate::contexts::sessions::infrastructure::{
    AgentSessionRuntimeAdapter, SessionCreationContextAdapter, SessionFileAdapter,
    SessionOperationAdapter, SqliteSessionChatProfileAdapter, SqliteSessionsRepository,
    SystemSessionClock, UnifiedSessionLoggingAdapter, UuidSessionIdentities,
};
use crate::contexts::tooling::cli_parameters::CliParametersApi;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const SESSION_MAINTENANCE_INTERVAL: Duration = Duration::from_secs(60 * 60);

pub(crate) fn assemble_sessions_api(
    database: NativeDatabase,
    operations: OperationsApi,
    workspaces: WorkspaceApi,
    cli_parameters: CliParametersApi,
    fallback_log_directory: PathBuf,
) -> (SessionsApi, AgentSessionRuntimeAdapter) {
    let repository = Arc::new(SqliteSessionsRepository::new(database.clone()));
    let logging: Arc<dyn DiagnosticLogPort> =
        Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    let runtime_adapter = AgentSessionRuntimeAdapter::new(workspaces.clone());
    let service = SessionsApplicationService::new(SessionApplicationPorts {
        sessions: repository.clone(),
        messages: repository.clone(),
        categories: repository.clone(),
        configurations: repository.clone(),
        usage: repository.clone(),
        transactions: repository,
        clock: Arc::new(SystemSessionClock),
        identities: Arc::new(UuidSessionIdentities),
        files: Arc::new(SessionFileAdapter::new(workspaces.clone(), logging.clone())),
        operations: Arc::new(SessionOperationAdapter::new(operations)),
        logging: Arc::new(UnifiedSessionLoggingAdapter::new(logging)),
        chat_profiles: Arc::new(SqliteSessionChatProfileAdapter::new(cli_parameters)),
        creation: Arc::new(SessionCreationContextAdapter::new(
            database.clone(),
            workspaces.clone(),
        )),
        runtime: Arc::new(runtime_adapter.clone()),
    });
    (SessionsApi::new(service), runtime_adapter)
}

pub(crate) fn start_session_maintenance_jobs(
    api: SessionsApi,
    settings: DesktopSettingsApi,
    fallback_log_directory: PathBuf,
) {
    let logging: Arc<dyn DiagnosticLogPort> =
        Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    thread::spawn(move || loop {
        run_session_maintenance_cycle(&api, &settings, logging.as_ref());
        thread::sleep(SESSION_MAINTENANCE_INTERVAL);
    });
}

fn run_session_maintenance_cycle(
    api: &SessionsApi,
    settings: &DesktopSettingsApi,
    logging: &dyn DiagnosticLogPort,
) {
    let policy = match settings.get_automatic_archival_settings() {
        Ok(settings) => ArchivalPolicy {
            enabled: settings.enabled(),
            inactive_days: settings.inactive_days(),
        },
        Err(error) => {
            write_maintenance_error(
                logging,
                format!("Automatic archival settings could not be loaded: {error}"),
            );
            ArchivalPolicy {
                enabled: false,
                inactive_days: 1,
            }
        }
    };
    if let Err(error) = api.run_maintenance(policy) {
        write_maintenance_error(logging, format!("Session maintenance failed: {error}"));
    }
}

fn write_maintenance_error(logging: &dyn DiagnosticLogPort, message: String) {
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity: LogSeverity::Error,
        category: "session.maintenance".to_string(),
        message,
        context: Default::default(),
    });
}
