use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PluginIntegrationDomainError {
    UnknownIntegration(String),
}

impl PluginIntegrationDomainError {
    #[cfg(test)]
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::UnknownIntegration(_) => "plugin-integration.unknown",
        }
    }
}

impl fmt::Display for PluginIntegrationDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownIntegration(value) => {
                write!(formatter, "Unknown plugin integration: {value}")
            }
        }
    }
}

impl std::error::Error for PluginIntegrationDomainError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_integration_error_has_stable_code_and_safe_message() {
        let error = PluginIntegrationDomainError::UnknownIntegration("other".to_string());
        assert_eq!(error.code(), "plugin-integration.unknown");
        assert_eq!(error.to_string(), "Unknown plugin integration: other");
    }
}
