use crate::commands::sessions::events::NativeSessionRecoveryEvents;
use crate::contexts::agent_runtime::application::AgentRegistryRepository;
use crate::contexts::desktop::api::DesktopSettingsApi;
use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::sessions::api::{ArchivalPolicy, SessionsApi};
use crate::contexts::sessions::application::{
    SessionApplicationPorts, SessionRecoveryCoordinator, SessionsApplicationService,
};
use crate::contexts::sessions::infrastructure::{
    AgentSessionRuntimeAdapter, SessionAgentEligibilityAdapter, SessionCreationContextAdapter,
    SessionFileAdapter, SessionOperationAdapter, SqliteSessionChatProfileAdapter,
    SqliteSessionsRepository, SystemSessionClock, UnifiedSessionLoggingAdapter,
    UuidSessionIdentities,
};
use crate::contexts::tooling::cli::application::NativeConfigPort;
use crate::contexts::tooling::cli_parameters::CliParametersApi;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

const SESSION_MAINTENANCE_INTERVAL: Duration = Duration::from_secs(60 * 60);

pub(crate) struct SessionRuntimeDependencies {
    pub(crate) app: AppHandle,
    pub(crate) operations: OperationsApi,
    pub(crate) workspaces: WorkspaceApi,
}

pub(crate) fn assemble_sessions_api(
    database: NativeDatabase,
    runtime: SessionRuntimeDependencies,
    cli_parameters: CliParametersApi,
    native_config: Arc<dyn NativeConfigPort>,
    agent_registry: Arc<dyn AgentRegistryRepository>,
    fallback_log_directory: PathBuf,
) -> (
    SessionsApi,
    AgentSessionRuntimeAdapter,
    SessionRecoveryCoordinator,
) {
    let SessionRuntimeDependencies {
        app,
        operations,
        workspaces,
    } = runtime;
    let repository = Arc::new(SqliteSessionsRepository::new(database.clone()));
    let logging: Arc<dyn DiagnosticLogPort> =
        Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    let runtime_adapter = AgentSessionRuntimeAdapter::new(workspaces.clone(), repository.clone())
        .with_operations(operations.clone());
    let clock = Arc::new(SystemSessionClock);
    let session_logging = Arc::new(UnifiedSessionLoggingAdapter::new(logging.clone()));
    let recovery_events = Arc::new(NativeSessionRecoveryEvents::new(app));
    let recovery = SessionRecoveryCoordinator::new(
        repository.clone(),
        repository.clone(),
        Arc::new(runtime_adapter.clone()),
        clock.clone(),
        session_logging.clone(),
    )
    .with_events(recovery_events.clone());
    let service = SessionsApplicationService::new(SessionApplicationPorts {
        sessions: repository.clone(),
        messages: repository.clone(),
        categories: repository.clone(),
        configurations: repository.clone(),
        usage: repository.clone(),
        accounting: repository.clone(),
        transactions: repository.clone(),
        recovery_reports: repository,
        recovery_events,
        clock,
        identities: Arc::new(UuidSessionIdentities),
        files: Arc::new(SessionFileAdapter::new(workspaces.clone(), logging.clone())),
        operations: Arc::new(SessionOperationAdapter::new(operations)),
        logging: session_logging,
        chat_profiles: Arc::new(SqliteSessionChatProfileAdapter::new(
            database.clone(),
            cli_parameters,
            native_config,
        )),
        creation: Arc::new(SessionCreationContextAdapter::new(
            database.clone(),
            workspaces.clone(),
        )),
        eligibility: Arc::new(SessionAgentEligibilityAdapter::new(agent_registry)),
        runtime: Arc::new(runtime_adapter.clone()),
    });
    (SessionsApi::new(service), runtime_adapter, recovery)
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
