use crate::contexts::desktop::domain::DesktopSettingsDomainError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DesktopSettingsApplicationError {
    Domain(DesktopSettingsDomainError),
    Repository(String),
    NetworkProxy(String),
    LogDirectory(String),
    Startup(String),
    Directory(String),
    ClientLogging(String),
}

impl fmt::Display for DesktopSettingsApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::Repository(message) => write!(formatter, "settings repository error: {message}"),
            Self::NetworkProxy(message) => write!(formatter, "network proxy error: {message}"),
            Self::LogDirectory(message) => write!(formatter, "log directory error: {message}"),
            Self::Startup(message) => write!(formatter, "startup preference error: {message}"),
            Self::Directory(message) => write!(formatter, "directory action error: {message}"),
            Self::ClientLogging(message) => write!(formatter, "client logging error: {message}"),
        }
    }
}

impl std::error::Error for DesktopSettingsApplicationError {}

impl From<DesktopSettingsDomainError> for DesktopSettingsApplicationError {
    fn from(error: DesktopSettingsDomainError) -> Self {
        Self::Domain(error)
    }
}
