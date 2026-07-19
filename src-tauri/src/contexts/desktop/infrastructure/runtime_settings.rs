use crate::contexts::desktop::application::{
    DesktopClockPort, DesktopLogDirectoryPort, DesktopNetworkProxyPort,
    DesktopSettingsApplicationError, DesktopStartupPort,
};
use crate::contexts::desktop::domain::{NetworkProxyPreferences, StartupPreference};
use crate::platform::{clock::SystemClock, network};
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemDesktopClock;

impl DesktopClockPort for SystemDesktopClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RuntimeNetworkProxyAdapter;

impl DesktopNetworkProxyPort for RuntimeNetworkProxyAdapter {
    fn apply(
        &self,
        preferences: &NetworkProxyPreferences,
    ) -> Result<(), DesktopSettingsApplicationError> {
        network::apply(preferences.url(), preferences.bypass())
            .map_err(|error| DesktopSettingsApplicationError::NetworkProxy(error.to_string()))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RuntimeLogDirectoryAdapter;

impl DesktopLogDirectoryPort for RuntimeLogDirectoryAdapter {
    fn validate(&self, path: &str) -> Result<(), DesktopSettingsApplicationError> {
        crate::platform::logging::validate_log_dir(&PathBuf::from(path))
            .map(|_| ())
            .map_err(|error| DesktopSettingsApplicationError::LogDirectory(error.to_string()))
    }

    fn activate(&self, path: &str) -> Result<(), DesktopSettingsApplicationError> {
        let path = crate::platform::logging::validate_log_dir(&PathBuf::from(path))
            .map_err(|error| DesktopSettingsApplicationError::LogDirectory(error.to_string()))?;
        crate::platform::logging::set_active_log_dir(path);
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct TauriDesktopStartupAdapter {
    app: AppHandle,
}

impl TauriDesktopStartupAdapter {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl DesktopStartupPort for TauriDesktopStartupAdapter {
    fn apply(&self, preference: StartupPreference) -> Result<(), DesktopSettingsApplicationError> {
        let autolaunch = self.app.autolaunch();
        let result = if preference.enabled() {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        result.map_err(|error| DesktopSettingsApplicationError::Startup(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::desktop::domain::DesktopSettingMutation;
    use crate::test_support::TempDirectory;

    #[test]
    fn system_clock_uses_the_existing_rfc3339_timestamp_contract() {
        assert!(chrono::DateTime::parse_from_rfc3339(&SystemDesktopClock.now()).is_ok());
    }

    #[test]
    fn runtime_proxy_adapter_applies_normalized_domain_preferences() {
        let preferences =
            NetworkProxyPreferences::new("http://127.0.0.1:7890", " localhost 127.0.0.1 ")
                .expect("preferences");

        RuntimeNetworkProxyAdapter
            .apply(&preferences)
            .expect("proxy apply");

        let state = network::current_state();
        assert_eq!(state.url, "http://127.0.0.1:7890");
        assert_eq!(state.bypass, "localhost,127.0.0.1");
        let defaults = DesktopSettingMutation::parse("networkProxyUrl", "").expect("reset");
        let reset =
            NetworkProxyPreferences::new(defaults.persisted_value(), network::DEFAULT_BYPASS)
                .expect("reset preferences");
        RuntimeNetworkProxyAdapter.apply(&reset).expect("reset");
    }

    #[test]
    fn log_directory_adapter_validates_and_activates_a_real_directory() {
        let directory = TempDirectory::new("desktop-log-directory");
        let path = directory.path().join("logs");
        let path = path.to_string_lossy().to_string();

        RuntimeLogDirectoryAdapter
            .validate(&path)
            .expect("validate directory");
        RuntimeLogDirectoryAdapter
            .activate(&path)
            .expect("activate directory");

        assert!(PathBuf::from(path).is_dir());
    }
}
