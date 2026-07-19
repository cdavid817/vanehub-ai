pub(crate) use super::application::{
    ClientLogEvent, ClientLogEventKind, DataManagementInformation,
    DesktopEnvironmentApplicationService, DesktopLogLevel, DesktopLoggingPolicy,
    DesktopSettingsApplicationError as DesktopSettingsError, DesktopSettingsApplicationService,
    DesktopSettingsView, DetectedNetworkProxy, NetworkProxyTestResult, NodeInformation,
};
pub(crate) use super::application::{
    DesktopLifecycleApplicationError as DesktopLifecycleError, DesktopLifecycleApplicationService,
    FloatingAssistantApplicationError as FloatingAssistantError,
    FloatingAssistantApplicationService,
};
pub(crate) use super::domain::{
    AutomaticArchivalSettings, FloatingAssistantConfig, FloatingAssistantMainAction,
    FloatingAssistantPlatform, FloatingAssistantSurfaceMode, SurfaceTransition,
};
use super::infrastructure::FolderOpenerService;
pub(crate) use super::infrastructure::{
    FolderOpenerAvailability, FolderOpenerId, FolderOpenerPreferencesView, OpenSessionFolderResult,
    SaveFolderOpenerPreferences,
};
use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct DesktopSettingsApi {
    settings: DesktopSettingsApplicationService,
    environment: DesktopEnvironmentApplicationService,
    folder_openers: FolderOpenerService,
}

impl DesktopSettingsApi {
    pub(crate) fn new(
        settings: DesktopSettingsApplicationService,
        environment: DesktopEnvironmentApplicationService,
        folder_openers: FolderOpenerService,
    ) -> Self {
        Self {
            settings,
            environment,
            folder_openers,
        }
    }

    pub(crate) fn list_folder_openers(&self, refresh: bool) -> Vec<FolderOpenerAvailability> {
        self.folder_openers.list(refresh)
    }

    pub(crate) fn get_folder_opener_preferences(
        &self,
    ) -> Result<FolderOpenerPreferencesView, DesktopSettingsError> {
        self.folder_openers
            .preferences()
            .map_err(DesktopSettingsError::Repository)
    }

    pub(crate) fn save_folder_opener_preferences(
        &self,
        input: SaveFolderOpenerPreferences,
    ) -> Result<FolderOpenerPreferencesView, DesktopSettingsError> {
        self.folder_openers
            .save_preferences(input)
            .map_err(DesktopSettingsError::Repository)
    }

    pub(crate) fn open_session_folder(
        &self,
        session_id: &str,
        path: &std::path::Path,
        opener_id: FolderOpenerId,
    ) -> Result<OpenSessionFolderResult, DesktopSettingsError> {
        self.folder_openers
            .open_path(session_id, path, opener_id)
            .map_err(DesktopSettingsError::Directory)
    }

    pub(crate) fn get_settings(&self) -> Result<DesktopSettingsView, DesktopSettingsError> {
        self.settings
            .get_settings()
            .map(DesktopSettingsView::native)
    }

    pub(crate) fn save_setting(
        &self,
        key: &str,
        value: &str,
    ) -> Result<DesktopSettingsView, DesktopSettingsError> {
        let mutation = super::domain::DesktopSettingMutation::parse(key, value)?;
        self.settings
            .save_setting(mutation)
            .map(DesktopSettingsView::native)
    }

    pub(crate) fn get_automatic_archival_settings(
        &self,
    ) -> Result<AutomaticArchivalSettings, DesktopSettingsError> {
        self.settings.get_automatic_archival_settings()
    }

    pub(crate) fn save_automatic_archival_settings(
        &self,
        enabled: bool,
        inactive_days: i64,
    ) -> Result<AutomaticArchivalSettings, DesktopSettingsError> {
        self.settings
            .save_automatic_archival_settings(enabled, inactive_days)
    }

    pub(crate) fn set_launch_on_startup(
        &self,
        enabled: bool,
    ) -> Result<DesktopSettingsView, DesktopSettingsError> {
        self.settings
            .set_launch_on_startup(enabled)
            .map(DesktopSettingsView::native)
    }

    pub(crate) fn activate_configured_log_directory(&self) -> Result<(), DesktopSettingsError> {
        self.settings.activate_configured_log_directory()
    }

    pub(crate) fn sync_startup_preference(&self) -> Result<(), DesktopSettingsError> {
        self.settings.sync_startup_preference()
    }

    pub(crate) fn data_management_info(
        &self,
    ) -> Result<DataManagementInformation, DesktopSettingsError> {
        self.environment.data_management_info()
    }

    pub(crate) fn open_database_directory(&self) -> Result<(), DesktopSettingsError> {
        self.environment.open_database_directory()
    }

    pub(crate) fn open_log_directory(&self) -> Result<(), DesktopSettingsError> {
        let settings = self.settings.get_settings()?;
        self.environment
            .open_log_directory(settings.log_directory())
    }

    pub(crate) fn node_information(&self) -> NodeInformation {
        self.environment.node_information()
    }

    pub(crate) async fn test_network_proxy(
        &self,
        url: String,
        bypass: String,
    ) -> Result<NetworkProxyTestResult, DesktopSettingsError> {
        self.environment.test_network_proxy(url, bypass).await
    }

    pub(crate) async fn scan_network_proxies(&self) -> Vec<DetectedNetworkProxy> {
        self.environment.scan_network_proxies().await
    }

    pub(crate) fn report_client_log(
        &self,
        event: ClientLogEvent,
    ) -> Result<(), DesktopSettingsError> {
        let settings = self.settings.get_settings()?;
        self.environment
            .report_client_log(settings.log_directory(), event)
    }
}

#[derive(Clone)]
pub(crate) struct FloatingAssistantApi {
    service: FloatingAssistantApplicationService,
    logging: Arc<dyn DiagnosticLogPort>,
}

impl FloatingAssistantApi {
    pub(crate) fn new(
        service: FloatingAssistantApplicationService,
        logging: Arc<dyn DiagnosticLogPort>,
    ) -> Self {
        Self { service, logging }
    }

    pub(crate) fn platform(&self) -> FloatingAssistantPlatform {
        self.service.platform()
    }

    pub(crate) fn get_config(&self) -> Result<FloatingAssistantConfig, FloatingAssistantError> {
        self.service.get_config()
    }

    pub(crate) fn set_enabled(
        &self,
        enabled: bool,
    ) -> Result<FloatingAssistantConfig, FloatingAssistantError> {
        self.service.set_enabled(enabled)
    }

    pub(crate) fn save_anchor(
        &self,
        x: f64,
        y: f64,
        monitor_name: Option<String>,
    ) -> Result<FloatingAssistantConfig, FloatingAssistantError> {
        self.service.save_anchor(x, y, monitor_name)
    }

    pub(crate) fn persist_window_position(
        &self,
    ) -> Result<FloatingAssistantConfig, FloatingAssistantError> {
        self.service.persist_window_position()
    }

    pub(crate) fn set_surface(
        &self,
        mode: FloatingAssistantSurfaceMode,
    ) -> Result<SurfaceTransition, FloatingAssistantError> {
        self.service.set_surface(mode)
    }

    pub(crate) fn initialize(&self) -> Result<(), FloatingAssistantError> {
        self.service.initialize()
    }

    pub(crate) fn start_dragging(&self) -> Result<(), FloatingAssistantError> {
        self.service.start_dragging()
    }

    pub(crate) fn show_main_window(
        &self,
        _action: FloatingAssistantMainAction,
    ) -> Result<(), FloatingAssistantError> {
        self.service.show_main_window()
    }

    pub(crate) fn should_hide_main_on_close(&self) -> Result<bool, FloatingAssistantError> {
        self.service.should_hide_main_on_close()
    }

    pub(crate) fn record_configuration_changed(&self, enabled: bool) {
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity: LogSeverity::Info,
            category: "floating-assistant.configuration".to_string(),
            message: if enabled { "enabled" } else { "disabled" }.to_string(),
            context: BTreeMap::new(),
        });
    }
}

#[derive(Clone)]
pub(crate) struct DesktopLifecycleApi {
    service: DesktopLifecycleApplicationService,
}

impl DesktopLifecycleApi {
    pub(crate) fn new(service: DesktopLifecycleApplicationService) -> Self {
        Self { service }
    }

    pub(crate) fn initialize(&self) -> Result<(), DesktopLifecycleError> {
        self.service.initialize()
    }

    pub(crate) fn request_exit(&self) {
        self.service.request_exit();
    }
}
