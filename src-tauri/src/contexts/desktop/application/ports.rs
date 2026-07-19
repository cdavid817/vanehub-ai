use super::{
    ClientLogEvent, DataManagementInformation, DesktopSettingsApplicationError,
    DetectedNetworkProxy, NetworkProxyTestResult, NodeInformation, StoredDesktopSetting,
};
use crate::contexts::desktop::domain::{
    AutomaticArchivalSettings, DesktopSettingMutation, NetworkProxyPreferences, StartupPreference,
};
use async_trait::async_trait;

pub(crate) trait DesktopSettingsRepository: Send + Sync {
    fn load_settings(&self) -> Result<Vec<StoredDesktopSetting>, DesktopSettingsApplicationError>;

    fn save_setting(
        &self,
        mutation: &DesktopSettingMutation,
        updated_at: &str,
    ) -> Result<(), DesktopSettingsApplicationError>;

    fn save_automatic_archival(
        &self,
        settings: AutomaticArchivalSettings,
        updated_at: &str,
    ) -> Result<(), DesktopSettingsApplicationError>;
}

pub(crate) trait DesktopClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait DesktopNetworkProxyPort: Send + Sync {
    fn apply(
        &self,
        preferences: &NetworkProxyPreferences,
    ) -> Result<(), DesktopSettingsApplicationError>;
}

pub(crate) trait DesktopLogDirectoryPort: Send + Sync {
    fn validate(&self, path: &str) -> Result<(), DesktopSettingsApplicationError>;

    fn activate(&self, path: &str) -> Result<(), DesktopSettingsApplicationError>;
}

pub(crate) trait DesktopStartupPort: Send + Sync {
    fn apply(&self, preference: StartupPreference) -> Result<(), DesktopSettingsApplicationError>;
}

pub(crate) trait DesktopDirectoryPort: Send + Sync {
    fn data_management_info(
        &self,
    ) -> Result<DataManagementInformation, DesktopSettingsApplicationError>;

    fn open_database_directory(&self) -> Result<(), DesktopSettingsApplicationError>;

    fn open_log_directory(&self, path: &str) -> Result<(), DesktopSettingsApplicationError>;
}

pub(crate) trait DesktopNodeInfoPort: Send + Sync {
    fn inspect(&self) -> NodeInformation;
}

#[async_trait]
pub(crate) trait DesktopNetworkProxyActionsPort: Send + Sync {
    async fn test(
        &self,
        preferences: &NetworkProxyPreferences,
    ) -> Result<NetworkProxyTestResult, DesktopSettingsApplicationError>;

    async fn scan(&self) -> Vec<DetectedNetworkProxy>;
}

pub(crate) trait DesktopClientLoggingPort: Send + Sync {
    fn record(
        &self,
        log_directory: &str,
        event: ClientLogEvent,
    ) -> Result<(), DesktopSettingsApplicationError>;
}
