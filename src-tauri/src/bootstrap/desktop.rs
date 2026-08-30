use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::code_intelligence::api::CodeIntelligenceApi;
use crate::contexts::communications::api::CommunicationsApi;
use crate::contexts::desktop::api::{
    DesktopLifecycleApi, DesktopSettingsApi, FloatingAssistantApi,
};
use crate::contexts::desktop::application::{
    DesktopEnvironmentApplicationService, DesktopLifecycleApplicationService,
    DesktopSettingsApplicationService, DesktopShutdownPort, FloatingAssistantApplicationService,
};
use crate::contexts::desktop::infrastructure::{
    DesktopDirectoryAdapter, DesktopLocaleBridge, FolderOpenerService, PlatformNodeInfoAdapter,
    RuntimeLogDirectoryAdapter, RuntimeNetworkProxyActionsAdapter, RuntimeNetworkProxyAdapter,
    SqliteDesktopSettingsRepository, SqliteFloatingAssistantRepository, SystemDesktopClock,
    TauriDesktopLifecycleAdapter, TauriDesktopStartupAdapter, TauriFloatingAssistantWindowAdapter,
    UnifiedClientLoggingAdapter,
};
use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::skill_evolution_orchestration::infrastructure::EvolutionBackgroundLifecycle;
use crate::platform::database::NativeDatabase;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tauri::AppHandle;

pub(crate) fn assemble_desktop_settings_api(
    database: NativeDatabase,
    app: AppHandle,
) -> (DesktopSettingsApi, DesktopLocaleBridge) {
    let default_log_directory = database
        .db_path
        .parent()
        .map(crate::platform::logging::default_log_dir)
        .unwrap_or_else(|| crate::platform::logging::default_log_dir(std::path::Path::new(".")))
        .to_string_lossy()
        .to_string();
    let settings_repository = SqliteDesktopSettingsRepository::new(database.clone());
    let locale_bridge = DesktopLocaleBridge::default();
    let settings = DesktopSettingsApplicationService::new(
        Arc::new(settings_repository.clone()),
        Arc::new(SystemDesktopClock),
        Arc::new(RuntimeNetworkProxyAdapter),
        Arc::new(RuntimeLogDirectoryAdapter),
        Arc::new(TauriDesktopStartupAdapter::new(app)),
        Arc::new(locale_bridge.clone()),
        default_log_directory,
    );
    let environment = DesktopEnvironmentApplicationService::new(
        Arc::new(DesktopDirectoryAdapter::new(database.clone())),
        Arc::new(PlatformNodeInfoAdapter),
        Arc::new(RuntimeNetworkProxyActionsAdapter),
        Arc::new(UnifiedClientLoggingAdapter),
    );
    (
        DesktopSettingsApi::new(
            settings,
            environment,
            FolderOpenerService::new(settings_repository),
        ),
        locale_bridge,
    )
}

pub(crate) fn assemble_floating_assistant_api(
    database: NativeDatabase,
    app: AppHandle,
    fallback_log_directory: PathBuf,
) -> FloatingAssistantApi {
    let logging: Arc<dyn DiagnosticLogPort> =
        Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    FloatingAssistantApi::new(
        FloatingAssistantApplicationService::new(
            Arc::new(SqliteFloatingAssistantRepository::new(database)),
            Arc::new(TauriFloatingAssistantWindowAdapter::new(
                app,
                logging.clone(),
            )),
            Arc::new(SystemDesktopClock),
        ),
        logging,
    )
}

pub(crate) struct DesktopLifecycleDependencies<'a> {
    pub(crate) app: AppHandle,
    pub(crate) language: &'a str,
    pub(crate) agents: AgentRuntimeApi,
    pub(crate) communications: CommunicationsApi,
    pub(crate) code_intelligence: CodeIntelligenceApi,
    pub(crate) evolution_background: EvolutionBackgroundLifecycle,
    pub(crate) locale_bridge: DesktopLocaleBridge,
    pub(crate) fallback_log_directory: PathBuf,
}

pub(crate) fn assemble_desktop_lifecycle_api(
    dependencies: DesktopLifecycleDependencies<'_>,
) -> Result<DesktopLifecycleApi, String> {
    let logging: Arc<dyn DiagnosticLogPort> = Arc::new(UnifiedLoggingAdapter::active(
        dependencies.fallback_log_directory,
    ));
    let lifecycle = Arc::new(TauriDesktopLifecycleAdapter::new(
        dependencies.app,
        dependencies.language,
        Arc::new(RuntimeShutdownAdapter {
            agents: dependencies.agents,
            communications: dependencies.communications,
            code_intelligence: dependencies.code_intelligence,
            evolution_background: dependencies.evolution_background,
        }),
        logging,
    ));
    dependencies.locale_bridge.attach(lifecycle.clone())?;
    Ok(DesktopLifecycleApi::new(
        DesktopLifecycleApplicationService::new(lifecycle),
    ))
}

pub(crate) fn initialize_desktop_runtime(
    lifecycle: &DesktopLifecycleApi,
    floating_assistant: &FloatingAssistantApi,
    fallback_log_directory: PathBuf,
) {
    let logging = UnifiedLoggingAdapter::active(fallback_log_directory);
    if let Err(error) = lifecycle.initialize() {
        record_initialization_error(&logging, "desktop.lifecycle", "tray", &error.to_string());
    }
    if let Err(error) = floating_assistant.initialize() {
        record_initialization_error(
            &logging,
            "floating-assistant.initialize",
            "window",
            &error.to_string(),
        );
    }
}

struct RuntimeShutdownAdapter {
    agents: AgentRuntimeApi,
    communications: CommunicationsApi,
    code_intelligence: CodeIntelligenceApi,
    evolution_background: EvolutionBackgroundLifecycle,
}

#[async_trait]
impl DesktopShutdownPort for RuntimeShutdownAdapter {
    async fn shutdown(&self, deadline: Instant) -> Result<(), String> {
        // Runs before every fallible shutdown so a failure there cannot leave background command
        // trees behind (`add-background-shell-execution`) — including a panicked evolution
        // maintenance worker, whose join error must not abort the reap.
        self.agents.reap_all_background_commands();
        let evolution = self.evolution_background.shutdown();
        self.agents
            .shutdown_generations()
            .map_err(|_| "agent-runner-shutdown-failed".to_string())?;
        let agents = self
            .agents
            .shutdown_agent_terminals()
            .map_err(|_| "agent-terminal-shutdown-failed".to_string());
        let (communications, code_intelligence) = tokio::join!(
            self.communications.shutdown(),
            self.code_intelligence.shutdown(deadline),
        );
        agents?;
        communications.map_err(|error| error.safe_code().to_string())?;
        code_intelligence.map_err(|_| "lsp-shutdown-failed".to_string())?;
        evolution
    }
}

fn record_initialization_error(
    logging: &dyn DiagnosticLogPort,
    category: &str,
    operation: &str,
    error: &str,
) {
    let mut context = BTreeMap::new();
    context.insert("operation".to_string(), operation.to_string());
    context.insert("error".to_string(), error.to_string());
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity: LogSeverity::Warn,
        category: category.to_string(),
        message: "Desktop runtime initialization failed; fallback behavior remains active"
            .to_string(),
        context,
    });
}
