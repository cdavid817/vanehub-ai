use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DesktopSettingsDomainError {
    InvalidSettingValue(String),
}

impl DesktopSettingsDomainError {
    pub(crate) fn invalid(key: impl Into<String>) -> Self {
        Self::InvalidSettingValue(key.into())
    }
}

impl fmt::Display for DesktopSettingsDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSettingValue(key) => {
                write!(formatter, "Invalid setting value for key '{key}'.")
            }
        }
    }
}

impl std::error::Error for DesktopSettingsDomainError {}
