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

/// Read-through and write-through to the dedicated personalization policy.
///
/// A port rather than the concrete API because `desktop` must not depend on `personalization`, and
/// bound after construction because personalization is assembled later — it needs the settings this
/// very type owns in order to migrate them.
pub(crate) trait PersonalizationSettingsBridge: Send + Sync {
    /// The five legacy personalization values as the policy currently holds them, with the revision
    /// they were read at.
    fn view(&self) -> Result<PersonalizationSettingsSnapshot, String>;

    /// Applies one legacy key, refusing a stale expected revision.
    fn save(
        &self,
        key: &str,
        value: &str,
        expected_revision: u64,
    ) -> Result<PersonalizationSettingsSnapshot, PersonalizationSaveRejection>;

    /// Whether this key belongs to the policy rather than to the settings table.
    fn owns(&self, key: &str) -> bool;
}

/// The legacy-shaped personalization values plus the revision behind them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PersonalizationSettingsSnapshot {
    pub(crate) about_user: String,
    pub(crate) style_rules: String,
    pub(crate) custom_instructions_enabled: bool,
    pub(crate) memory_enabled: bool,
    pub(crate) tool_assisted_extraction_enabled: bool,
    pub(crate) revision: u64,
}

/// Why a personalization save was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PersonalizationSaveRejection {
    Conflict { expected: u64, current: u64 },
    Unavailable(String),
}

#[derive(Clone)]
pub(crate) struct DesktopSettingsApi {
    settings: DesktopSettingsApplicationService,
    environment: DesktopEnvironmentApplicationService,
    folder_openers: FolderOpenerService,
    /// Empty until the composition root binds it. Until then the legacy rows answer, which keeps
    /// the settings page working through the window where personalization has not been assembled.
    personalization: Arc<std::sync::OnceLock<Arc<dyn PersonalizationSettingsBridge>>>,
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
            personalization: Arc::new(std::sync::OnceLock::new()),
        }
    }

    /// Makes the dedicated policy the source of truth for the personalization fields.
    ///
    /// Idempotent, and one-way: once bound, the legacy rows are never read or written for those
    /// fields again. Two sources of truth for one setting is the state this change exists to end,
    /// so there is deliberately no way to unbind.
    pub(crate) fn bind_personalization(&self, bridge: Arc<dyn PersonalizationSettingsBridge>) {
        let _ = self.personalization.set(bridge);
    }

    fn personalization_bridge(&self) -> Option<&Arc<dyn PersonalizationSettingsBridge>> {
        self.personalization.get()
    }

    /// Overlays the policy's values onto a settings view read from the legacy rows.
    ///
    /// The rows still hold stale copies of these five fields — they stay deserializable for the
    /// compatibility window — so the overlay is what makes the policy authoritative without
    /// migrating every reader at once.
    fn overlay_personalization(&self, view: DesktopSettingsView) -> DesktopSettingsView {
        let Some(bridge) = self.personalization_bridge() else {
            return view;
        };
        let Ok(snapshot) = bridge.view() else {
            // An unreadable policy leaves the legacy values standing for display, and every save is
            // refused below — so nothing is written on top of a policy nobody could read.
            return view;
        };
        let mut view = view;
        for (key, value) in [
            ("customInstructionsAboutUser", snapshot.about_user.clone()),
            ("customInstructionsStyleRules", snapshot.style_rules.clone()),
            (
                "customInstructionsEnabled",
                snapshot.custom_instructions_enabled.to_string(),
            ),
            ("memoryEnabled", snapshot.memory_enabled.to_string()),
            (
                "memoryToolAssistedChatsEnabled",
                snapshot.tool_assisted_extraction_enabled.to_string(),
            ),
        ] {
            if let Ok(mutation) = super::domain::DesktopSettingMutation::parse(key, &value) {
                view.settings.apply(mutation);
            }
        }
        view.with_personalization_revision(snapshot.revision)
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
            .map(|view| self.overlay_personalization(view))
    }

    /// Saves one setting, routing the personalization keys to the dedicated policy.
    ///
    /// `expected_personalization_revision` is consulted only for a key the policy owns, and it is
    /// required for those: a caller that cannot say which revision it was looking at is not making
    /// an informed edit, and accepting one anyway is last-response-wins.
    pub(crate) fn save_setting(
        &self,
        key: &str,
        value: &str,
        expected_personalization_revision: Option<u64>,
    ) -> Result<DesktopSettingsView, DesktopSettingsError> {
        if let Some(bridge) = self.personalization_bridge() {
            if bridge.owns(key) {
                let expected = expected_personalization_revision.ok_or_else(|| {
                    DesktopSettingsError::Personalization(
                        "a personalization save must state the revision it was made against"
                            .to_string(),
                    )
                })?;
                // Parsed first, so an invalid value is refused by the same rule as before rather
                // than reaching the policy in a shape the legacy page could never have produced.
                super::domain::DesktopSettingMutation::parse(key, value)?;
                return match bridge.save(key, value, expected) {
                    Ok(_) => self.get_settings(),
                    Err(PersonalizationSaveRejection::Conflict { expected, current }) => {
                        Err(DesktopSettingsError::PersonalizationConflict { expected, current })
                    }
                    Err(PersonalizationSaveRejection::Unavailable(message)) => {
                        Err(DesktopSettingsError::Personalization(message))
                    }
                };
            }
        }
        let mutation = super::domain::DesktopSettingMutation::parse(key, value)?;
        self.settings
            .save_setting(mutation)
            .map(DesktopSettingsView::native)
            .map(|view| self.overlay_personalization(view))
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
