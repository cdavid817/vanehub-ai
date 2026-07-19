use super::{
    DesktopClockPort, DesktopLogDirectoryPort, DesktopNetworkProxyPort,
    DesktopSettingsApplicationError, DesktopSettingsRepository, DesktopStartupPort,
};
use crate::contexts::desktop::domain::{
    AutomaticArchivalSettings, DesktopSettingKey, DesktopSettingMutation, DesktopSettings,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct DesktopSettingsApplicationService {
    repository: Arc<dyn DesktopSettingsRepository>,
    clock: Arc<dyn DesktopClockPort>,
    network_proxy: Arc<dyn DesktopNetworkProxyPort>,
    log_directory: Arc<dyn DesktopLogDirectoryPort>,
    startup: Arc<dyn DesktopStartupPort>,
    defaults: DesktopSettings,
}

impl DesktopSettingsApplicationService {
    pub(crate) fn new(
        repository: Arc<dyn DesktopSettingsRepository>,
        clock: Arc<dyn DesktopClockPort>,
        network_proxy: Arc<dyn DesktopNetworkProxyPort>,
        log_directory: Arc<dyn DesktopLogDirectoryPort>,
        startup: Arc<dyn DesktopStartupPort>,
        default_log_directory: impl Into<String>,
    ) -> Self {
        Self {
            repository,
            clock,
            network_proxy,
            log_directory,
            startup,
            defaults: DesktopSettings::defaults(default_log_directory),
        }
    }

    pub(crate) fn get_settings(&self) -> Result<DesktopSettings, DesktopSettingsApplicationError> {
        self.load_effective_settings()
    }

    pub(crate) fn save_setting(
        &self,
        mutation: DesktopSettingMutation,
    ) -> Result<DesktopSettings, DesktopSettingsApplicationError> {
        let key = mutation.key();
        if let DesktopSettingMutation::LogDirectory(path) = &mutation {
            self.log_directory.validate(path)?;
        }
        self.repository.save_setting(&mutation, &self.clock.now())?;
        let settings = self.load_effective_settings()?;
        match key {
            DesktopSettingKey::LogDirectory => {
                self.log_directory.activate(settings.log_directory())?;
            }
            DesktopSettingKey::LaunchOnStartup => {
                self.startup.apply(settings.startup())?;
            }
            _ => {}
        }
        Ok(settings)
    }

    pub(crate) fn get_automatic_archival_settings(
        &self,
    ) -> Result<AutomaticArchivalSettings, DesktopSettingsApplicationError> {
        self.load_effective_settings()
            .map(|settings| settings.automatic_archival())
    }

    pub(crate) fn save_automatic_archival_settings(
        &self,
        enabled: bool,
        inactive_days: i64,
    ) -> Result<AutomaticArchivalSettings, DesktopSettingsApplicationError> {
        let archival = AutomaticArchivalSettings::new(enabled, inactive_days)?;
        self.repository
            .save_automatic_archival(archival, &self.clock.now())?;
        self.load_effective_settings()
            .map(|settings| settings.automatic_archival())
    }

    pub(crate) fn set_launch_on_startup(
        &self,
        enabled: bool,
    ) -> Result<DesktopSettings, DesktopSettingsApplicationError> {
        self.save_setting(DesktopSettingMutation::LaunchOnStartup(enabled))
    }

    pub(crate) fn activate_configured_log_directory(
        &self,
    ) -> Result<(), DesktopSettingsApplicationError> {
        let settings = self.load_effective_settings()?;
        self.log_directory.activate(settings.log_directory())
    }

    pub(crate) fn sync_startup_preference(&self) -> Result<(), DesktopSettingsApplicationError> {
        let settings = self.load_effective_settings()?;
        self.startup.apply(settings.startup())
    }

    fn load_effective_settings(&self) -> Result<DesktopSettings, DesktopSettingsApplicationError> {
        let mut settings = self.defaults.clone();
        for stored in self.repository.load_settings()? {
            if let Ok(mutation) = DesktopSettingMutation::parse_for_key(stored.key, &stored.value) {
                settings.apply(mutation);
            }
        }
        self.network_proxy.apply(settings.network_proxy())?;
        Ok(settings)
    }
}
