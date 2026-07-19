use super::DesktopSettingsDomainError;

pub(crate) const DEFAULT_NETWORK_PROXY_BYPASS: &str = "localhost,127.0.0.1,::1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApplicationLanguage {
    ChineseSimplified,
    English,
}

impl ApplicationLanguage {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "zh-CN" => Some(Self::ChineseSimplified),
            "en" => Some(Self::English),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ChineseSimplified => "zh-CN",
            Self::English => "en",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesktopFontSize {
    Px12,
    Px14,
    Px16,
    Px18,
}

impl DesktopFontSize {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "12px" => Some(Self::Px12),
            "14px" => Some(Self::Px14),
            "16px" => Some(Self::Px16),
            "18px" => Some(Self::Px18),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Px12 => "12px",
            Self::Px14 => "14px",
            Self::Px16 => "16px",
            Self::Px18 => "18px",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesktopTheme {
    Futuristic,
    Minimal,
}

impl DesktopTheme {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "futuristic" => Some(Self::Futuristic),
            "minimal" => Some(Self::Minimal),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Futuristic => "futuristic",
            Self::Minimal => "minimal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkProxyPreferences {
    url: String,
    bypass: String,
}

impl NetworkProxyPreferences {
    pub(crate) fn new(
        url: impl Into<String>,
        bypass: impl Into<String>,
    ) -> Result<Self, DesktopSettingsDomainError> {
        Ok(Self {
            url: normalize_proxy_url(&url.into())?,
            bypass: normalize_proxy_bypass(&bypass.into())?,
        })
    }

    fn defaults() -> Self {
        Self {
            url: String::new(),
            bypass: DEFAULT_NETWORK_PROXY_BYPASS.to_string(),
        }
    }

    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    pub(crate) fn bypass(&self) -> &str {
        &self.bypass
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AutomaticArchivalSettings {
    enabled: bool,
    inactive_days: i64,
}

impl AutomaticArchivalSettings {
    pub(crate) fn new(
        enabled: bool,
        inactive_days: i64,
    ) -> Result<Self, DesktopSettingsDomainError> {
        if !(1..=3650).contains(&inactive_days) {
            return Err(DesktopSettingsDomainError::invalid(
                DesktopSettingKey::AutomaticArchivalInactiveDays.as_str(),
            ));
        }
        Ok(Self {
            enabled,
            inactive_days,
        })
    }

    pub(crate) fn enabled(self) -> bool {
        self.enabled
    }

    pub(crate) fn inactive_days(self) -> i64 {
        self.inactive_days
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StartupPreference(bool);

impl StartupPreference {
    pub(crate) fn new(enabled: bool) -> Self {
        Self(enabled)
    }

    pub(crate) fn enabled(self) -> bool {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum DesktopSettingKey {
    ApplicationLanguage,
    FontSize,
    Theme,
    DefaultFolderPath,
    LogDirectory,
    NetworkProxyUrl,
    NetworkProxyBypass,
    AutomaticArchivalEnabled,
    AutomaticArchivalInactiveDays,
    LaunchOnStartup,
}

impl DesktopSettingKey {
    pub(crate) fn parse(value: &str) -> Result<Self, DesktopSettingsDomainError> {
        match value {
            "applicationLanguage" => Ok(Self::ApplicationLanguage),
            "fontSize" => Ok(Self::FontSize),
            "theme" => Ok(Self::Theme),
            "defaultFolderPath" => Ok(Self::DefaultFolderPath),
            "logDirectory" => Ok(Self::LogDirectory),
            "networkProxyUrl" => Ok(Self::NetworkProxyUrl),
            "networkProxyBypass" => Ok(Self::NetworkProxyBypass),
            "automaticArchivalEnabled" => Ok(Self::AutomaticArchivalEnabled),
            "automaticArchivalInactiveDays" => Ok(Self::AutomaticArchivalInactiveDays),
            "launchOnStartup" => Ok(Self::LaunchOnStartup),
            _ => Err(DesktopSettingsDomainError::invalid(value)),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ApplicationLanguage => "applicationLanguage",
            Self::FontSize => "fontSize",
            Self::Theme => "theme",
            Self::DefaultFolderPath => "defaultFolderPath",
            Self::LogDirectory => "logDirectory",
            Self::NetworkProxyUrl => "networkProxyUrl",
            Self::NetworkProxyBypass => "networkProxyBypass",
            Self::AutomaticArchivalEnabled => "automaticArchivalEnabled",
            Self::AutomaticArchivalInactiveDays => "automaticArchivalInactiveDays",
            Self::LaunchOnStartup => "launchOnStartup",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DesktopSettingMutation {
    ApplicationLanguage(ApplicationLanguage),
    FontSize(DesktopFontSize),
    Theme(DesktopTheme),
    DefaultFolderPath(String),
    LogDirectory(String),
    NetworkProxyUrl(String),
    NetworkProxyBypass(String),
    AutomaticArchivalEnabled(bool),
    AutomaticArchivalInactiveDays(i64),
    LaunchOnStartup(bool),
}

impl DesktopSettingMutation {
    pub(crate) fn parse(key: &str, value: &str) -> Result<Self, DesktopSettingsDomainError> {
        Self::parse_for_key(DesktopSettingKey::parse(key)?, value)
    }

    pub(crate) fn parse_for_key(
        key: DesktopSettingKey,
        value: &str,
    ) -> Result<Self, DesktopSettingsDomainError> {
        let invalid = || DesktopSettingsDomainError::invalid(key.as_str());
        match key {
            DesktopSettingKey::ApplicationLanguage => ApplicationLanguage::parse(value)
                .map(Self::ApplicationLanguage)
                .ok_or_else(invalid),
            DesktopSettingKey::FontSize => DesktopFontSize::parse(value)
                .map(Self::FontSize)
                .ok_or_else(invalid),
            DesktopSettingKey::Theme => DesktopTheme::parse(value)
                .map(Self::Theme)
                .ok_or_else(invalid),
            DesktopSettingKey::DefaultFolderPath => Ok(Self::DefaultFolderPath(value.to_string())),
            DesktopSettingKey::LogDirectory if !value.trim().is_empty() => {
                Ok(Self::LogDirectory(value.to_string()))
            }
            DesktopSettingKey::NetworkProxyUrl => normalize_proxy_url(value)
                .map(Self::NetworkProxyUrl)
                .map_err(|_| invalid()),
            DesktopSettingKey::NetworkProxyBypass => normalize_proxy_bypass(value)
                .map(Self::NetworkProxyBypass)
                .map_err(|_| invalid()),
            DesktopSettingKey::AutomaticArchivalEnabled => parse_bool(value)
                .map(Self::AutomaticArchivalEnabled)
                .ok_or_else(invalid),
            DesktopSettingKey::AutomaticArchivalInactiveDays => value
                .parse::<i64>()
                .ok()
                .filter(|days| (1..=3650).contains(days))
                .map(Self::AutomaticArchivalInactiveDays)
                .ok_or_else(invalid),
            DesktopSettingKey::LaunchOnStartup => parse_bool(value)
                .map(Self::LaunchOnStartup)
                .ok_or_else(invalid),
            DesktopSettingKey::LogDirectory => Err(invalid()),
        }
    }

    pub(crate) fn key(&self) -> DesktopSettingKey {
        match self {
            Self::ApplicationLanguage(_) => DesktopSettingKey::ApplicationLanguage,
            Self::FontSize(_) => DesktopSettingKey::FontSize,
            Self::Theme(_) => DesktopSettingKey::Theme,
            Self::DefaultFolderPath(_) => DesktopSettingKey::DefaultFolderPath,
            Self::LogDirectory(_) => DesktopSettingKey::LogDirectory,
            Self::NetworkProxyUrl(_) => DesktopSettingKey::NetworkProxyUrl,
            Self::NetworkProxyBypass(_) => DesktopSettingKey::NetworkProxyBypass,
            Self::AutomaticArchivalEnabled(_) => DesktopSettingKey::AutomaticArchivalEnabled,
            Self::AutomaticArchivalInactiveDays(_) => {
                DesktopSettingKey::AutomaticArchivalInactiveDays
            }
            Self::LaunchOnStartup(_) => DesktopSettingKey::LaunchOnStartup,
        }
    }

    pub(crate) fn persisted_value(&self) -> String {
        match self {
            Self::ApplicationLanguage(value) => value.as_str().to_string(),
            Self::FontSize(value) => value.as_str().to_string(),
            Self::Theme(value) => value.as_str().to_string(),
            Self::DefaultFolderPath(value)
            | Self::LogDirectory(value)
            | Self::NetworkProxyUrl(value)
            | Self::NetworkProxyBypass(value) => value.clone(),
            Self::AutomaticArchivalEnabled(value) | Self::LaunchOnStartup(value) => {
                value.to_string()
            }
            Self::AutomaticArchivalInactiveDays(value) => value.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopSettings {
    application_language: ApplicationLanguage,
    font_size: DesktopFontSize,
    theme: DesktopTheme,
    default_folder_path: String,
    log_directory: String,
    network_proxy: NetworkProxyPreferences,
    automatic_archival: AutomaticArchivalSettings,
    startup: StartupPreference,
}

impl DesktopSettings {
    pub(crate) fn defaults(default_log_directory: impl Into<String>) -> Self {
        Self {
            application_language: ApplicationLanguage::ChineseSimplified,
            font_size: DesktopFontSize::Px14,
            theme: DesktopTheme::Futuristic,
            default_folder_path: String::new(),
            log_directory: default_log_directory.into(),
            network_proxy: NetworkProxyPreferences::defaults(),
            automatic_archival: AutomaticArchivalSettings {
                enabled: true,
                inactive_days: 10,
            },
            startup: StartupPreference::new(false),
        }
    }

    pub(crate) fn apply(&mut self, mutation: DesktopSettingMutation) {
        match mutation {
            DesktopSettingMutation::ApplicationLanguage(value) => {
                self.application_language = value;
            }
            DesktopSettingMutation::FontSize(value) => self.font_size = value,
            DesktopSettingMutation::Theme(value) => self.theme = value,
            DesktopSettingMutation::DefaultFolderPath(value) => self.default_folder_path = value,
            DesktopSettingMutation::LogDirectory(value) => self.log_directory = value,
            DesktopSettingMutation::NetworkProxyUrl(value) => self.network_proxy.url = value,
            DesktopSettingMutation::NetworkProxyBypass(value) => {
                self.network_proxy.bypass = value;
            }
            DesktopSettingMutation::AutomaticArchivalEnabled(value) => {
                self.automatic_archival.enabled = value;
            }
            DesktopSettingMutation::AutomaticArchivalInactiveDays(value) => {
                self.automatic_archival.inactive_days = value;
            }
            DesktopSettingMutation::LaunchOnStartup(value) => {
                self.startup = StartupPreference::new(value);
            }
        }
    }

    pub(crate) fn application_language(&self) -> ApplicationLanguage {
        self.application_language
    }

    pub(crate) fn font_size(&self) -> DesktopFontSize {
        self.font_size
    }

    pub(crate) fn theme(&self) -> DesktopTheme {
        self.theme
    }

    pub(crate) fn default_folder_path(&self) -> &str {
        &self.default_folder_path
    }

    pub(crate) fn log_directory(&self) -> &str {
        &self.log_directory
    }

    pub(crate) fn network_proxy(&self) -> &NetworkProxyPreferences {
        &self.network_proxy
    }

    pub(crate) fn automatic_archival(&self) -> AutomaticArchivalSettings {
        self.automatic_archival
    }

    pub(crate) fn startup(&self) -> StartupPreference {
        self.startup
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn normalize_proxy_url(value: &str) -> Result<String, DesktopSettingsDomainError> {
    let key = DesktopSettingKey::NetworkProxyUrl.as_str();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if trimmed != value || trimmed.chars().any(char::is_control) {
        return Err(DesktopSettingsDomainError::invalid(key));
    }
    let url = url::Url::parse(trimmed).map_err(|_| DesktopSettingsDomainError::invalid(key))?;
    if !matches!(url.scheme(), "http" | "https" | "socks5" | "socks5h") || url.host_str().is_none()
    {
        return Err(DesktopSettingsDomainError::invalid(key));
    }
    Ok(trimmed.to_string())
}

fn normalize_proxy_bypass(value: &str) -> Result<String, DesktopSettingsDomainError> {
    let key = DesktopSettingKey::NetworkProxyBypass.as_str();
    if value.chars().any(char::is_control) {
        return Err(DesktopSettingsDomainError::invalid(key));
    }
    Ok(value
        .split(|character: char| character == ',' || character.is_whitespace())
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>()
        .join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_preserve_the_existing_native_settings_contract() {
        let settings = DesktopSettings::defaults("D:/data/logs");

        assert_eq!(settings.application_language().as_str(), "zh-CN");
        assert_eq!(settings.font_size().as_str(), "14px");
        assert_eq!(settings.theme().as_str(), "futuristic");
        assert_eq!(settings.default_folder_path(), "");
        assert_eq!(settings.log_directory(), "D:/data/logs");
        assert_eq!(settings.network_proxy().url(), "");
        assert_eq!(settings.network_proxy().bypass(), "localhost,127.0.0.1,::1");
        assert_eq!(
            settings.automatic_archival(),
            AutomaticArchivalSettings::new(true, 10).expect("archival defaults")
        );
        assert!(!settings.startup().enabled());
    }

    #[test]
    fn setting_keys_and_values_keep_exact_storage_names_and_allowed_values() {
        let cases = [
            ("applicationLanguage", "en"),
            ("fontSize", "18px"),
            ("theme", "minimal"),
            ("defaultFolderPath", "D:/work"),
            ("logDirectory", "D:/logs"),
            ("automaticArchivalEnabled", "false"),
            ("automaticArchivalInactiveDays", "3650"),
            ("launchOnStartup", "true"),
        ];

        for (key, value) in cases {
            let mutation = DesktopSettingMutation::parse(key, value).expect("valid setting");
            assert_eq!(mutation.key().as_str(), key);
            assert_eq!(mutation.persisted_value(), value);
        }

        let error = DesktopSettingMutation::parse("fontSize", "20px").expect_err("font size");
        assert_eq!(
            error.to_string(),
            "Invalid setting value for key 'fontSize'."
        );
        assert!(DesktopSettingMutation::parse("unknownSetting", "value").is_err());
    }

    #[test]
    fn proxy_values_are_normalized_without_accepting_controls_or_unknown_schemes() {
        let bypass =
            DesktopSettingMutation::parse("networkProxyBypass", " localhost, 127.0.0.1 ::1 ")
                .expect("bypass");
        assert_eq!(bypass.persisted_value(), "localhost,127.0.0.1,::1");
        assert!(DesktopSettingMutation::parse(
            "networkProxyUrl",
            "http://user:pass@127.0.0.1:7890"
        )
        .is_ok());
        assert!(DesktopSettingMutation::parse("networkProxyUrl", "ftp://127.0.0.1:21").is_err());
        assert!(DesktopSettingMutation::parse("networkProxyBypass", "localhost\nbad").is_err());
    }

    #[test]
    fn archival_and_startup_rules_are_explicit_domain_values() {
        assert_eq!(
            AutomaticArchivalSettings::new(true, 0),
            Err(DesktopSettingsDomainError::invalid(
                "automaticArchivalInactiveDays"
            ))
        );
        assert_eq!(
            AutomaticArchivalSettings::new(false, 3651),
            Err(DesktopSettingsDomainError::invalid(
                "automaticArchivalInactiveDays"
            ))
        );
        assert!(StartupPreference::new(true).enabled());
    }

    #[test]
    fn aggregate_applies_only_already_validated_setting_mutations() {
        let mut settings = DesktopSettings::defaults("D:/logs");
        settings
            .apply(DesktopSettingMutation::parse("applicationLanguage", "en").expect("language"));
        settings.apply(
            DesktopSettingMutation::parse("networkProxyBypass", "localhost 10.0.0.1")
                .expect("bypass"),
        );
        settings.apply(DesktopSettingMutation::parse("launchOnStartup", "true").expect("startup"));

        assert_eq!(
            settings.application_language(),
            ApplicationLanguage::English
        );
        assert_eq!(settings.network_proxy().bypass(), "localhost,10.0.0.1");
        assert!(settings.startup().enabled());
    }
}
