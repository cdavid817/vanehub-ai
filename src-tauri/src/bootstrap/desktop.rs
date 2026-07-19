use crate::contexts::communications::api::CommunicationsApi;
use crate::contexts::desktop::api::{
    DesktopLifecycleApi, DesktopSettingsApi, FloatingAssistantApi,
};
use crate::contexts::desktop::application::{
    DesktopEnvironmentApplicationService, DesktopLifecycleApplicationService,
    DesktopSettingsApplicationService, DesktopShutdownPort, FloatingAssistantApplicationService,
};
use crate::contexts::desktop::infrastructure::{
    DesktopDirectoryAdapter, PlatformNodeInfoAdapter, RuntimeLogDirectoryAdapter,
    RuntimeNetworkProxyActionsAdapter, RuntimeNetworkProxyAdapter, SqliteDesktopSettingsRepository,
    SqliteFloatingAssistantRepository, SystemDesktopClock, TauriDesktopLifecycleAdapter,
    TauriDesktopStartupAdapter, TauriFloatingAssistantWindowAdapter, UnifiedClientLoggingAdapter,
};
use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::platform::database::NativeDatabase;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::AppHandle;

pub(crate) fn assemble_desktop_settings_api(
    database: NativeDatabase,
    app: AppHandle,
) -> DesktopSettingsApi {
    let default_log_directory = database
        .db_path
        .parent()
        .map(crate::platform::logging::default_log_dir)
        .unwrap_or_else(|| crate::platform::logging::default_log_dir(std::path::Path::new(".")))
        .to_string_lossy()
        .to_string();
    let settings = DesktopSettingsApplicationService::new(
        Arc::new(SqliteDesktopSettingsRepository::new(database.clone())),
        Arc::new(SystemDesktopClock),
        Arc::new(RuntimeNetworkProxyAdapter),
        Arc::new(RuntimeLogDirectoryAdapter),
        Arc::new(TauriDesktopStartupAdapter::new(app)),
        default_log_directory,
    );
    let environment = DesktopEnvironmentApplicationService::new(
        Arc::new(DesktopDirectoryAdapter::new(database)),
        Arc::new(PlatformNodeInfoAdapter),
        Arc::new(RuntimeNetworkProxyActionsAdapter),
        Arc::new(UnifiedClientLoggingAdapter),
    );
    DesktopSettingsApi::new(settings, environment)
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

pub(crate) fn assemble_desktop_lifecycle_api(
    app: AppHandle,
    language: &str,
    communications: CommunicationsApi,
    fallback_log_directory: PathBuf,
) -> DesktopLifecycleApi {
    let logging: Arc<dyn DiagnosticLogPort> =
        Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    let lifecycle = TauriDesktopLifecycleAdapter::new(
        app,
        language,
        Arc::new(CommunicationsShutdownAdapter { communications }),
        logging,
    );
    DesktopLifecycleApi::new(DesktopLifecycleApplicationService::new(Arc::new(lifecycle)))
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

struct CommunicationsShutdownAdapter {
    communications: CommunicationsApi,
}

#[async_trait]
impl DesktopShutdownPort for CommunicationsShutdownAdapter {
    async fn shutdown(&self) -> Result<(), String> {
        self.communications
            .shutdown()
            .await
            .map_err(|error| error.safe_code().to_string())
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
