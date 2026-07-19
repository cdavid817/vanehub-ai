use crate::contexts::tooling::plugin_integrations::domain::PluginIntegrationDomainError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PluginIntegrationApplicationError {
    Domain(PluginIntegrationDomainError),
    Logging(String),
}

impl fmt::Display for PluginIntegrationApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::Logging(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for PluginIntegrationApplicationError {}

impl From<PluginIntegrationDomainError> for PluginIntegrationApplicationError {
    fn from(error: PluginIntegrationDomainError) -> Self {
        Self::Domain(error)
    }
}
