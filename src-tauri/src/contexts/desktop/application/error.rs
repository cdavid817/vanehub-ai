use crate::contexts::desktop::domain::DesktopSettingsDomainError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DesktopSettingsApplicationError {
    Domain(DesktopSettingsDomainError),
    Repository(String),
    NetworkProxy(String),
    LogDirectory(String),
    Startup(String),
    NativeLocale(String),
    Directory(String),
    ClientLogging(String),
    /// A personalization save was rejected because the screen it came from was rendered from an
    /// older revision. Typed rather than folded into `Repository` because the caller's correct
    /// response is entirely different: keep the draft, show the stored value, let the user decide.
    PersonalizationConflict {
        expected: u64,
        current: u64,
    },
    /// The dedicated personalization policy could not be read or written.
    Personalization(String),
}

impl fmt::Display for DesktopSettingsApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::Repository(message) => write!(formatter, "settings repository error: {message}"),
            Self::NetworkProxy(message) => write!(formatter, "network proxy error: {message}"),
            Self::LogDirectory(message) => write!(formatter, "log directory error: {message}"),
            Self::Startup(message) => write!(formatter, "startup preference error: {message}"),
            Self::NativeLocale(message) => write!(formatter, "native locale error: {message}"),
            Self::Directory(message) => write!(formatter, "directory action error: {message}"),
            Self::ClientLogging(message) => write!(formatter, "client logging error: {message}"),
            Self::PersonalizationConflict { expected, current } => write!(
                formatter,
                "personalization changed since it was loaded (expected revision {expected}, current {current})"
            ),
            Self::Personalization(message) => {
                write!(formatter, "personalization policy error: {message}")
            }
        }
    }
}

impl std::error::Error for DesktopSettingsApplicationError {}

impl From<DesktopSettingsDomainError> for DesktopSettingsApplicationError {
    fn from(error: DesktopSettingsDomainError) -> Self {
        Self::Domain(error)
    }
}
