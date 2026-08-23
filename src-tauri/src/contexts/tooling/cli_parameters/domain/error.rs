use serde::Serialize;
use std::collections::BTreeMap;
use thiserror::Error;

/// Stable machine-readable codes. React maps these to localized text and never parses prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum CliParameterErrorCode {
    UnknownAgent,
    UnknownParameter,
    InvalidValue,
    DependencyUnsatisfied,
    Conflict,
    UnsupportedVersion,
    RevisionConflict,
    CatalogMismatch,
    CatalogInvalid,
    RepositoryFailure,
}

impl CliParameterErrorCode {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::UnknownAgent => "CLI_PARAMETER_UNKNOWN_AGENT",
            Self::UnknownParameter => "CLI_PARAMETER_UNKNOWN_PARAMETER",
            Self::InvalidValue => "CLI_PARAMETER_INVALID_VALUE",
            Self::DependencyUnsatisfied => "CLI_PARAMETER_DEPENDENCY_UNSATISFIED",
            Self::Conflict => "CLI_PARAMETER_CONFLICT",
            Self::UnsupportedVersion => "CLI_PARAMETER_UNSUPPORTED_VERSION",
            Self::RevisionConflict => "CLI_PARAMETER_REVISION_CONFLICT",
            Self::CatalogMismatch => "CLI_PARAMETER_CATALOG_MISMATCH",
            Self::CatalogInvalid => "CLI_PARAMETER_CATALOG_INVALID",
            Self::RepositoryFailure => "CLI_PARAMETER_REPOSITORY_FAILURE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{code}")]
pub(crate) struct CliParameterDomainError {
    pub(crate) code: CliParameterErrorCode,
    pub(crate) agent_id: Option<String>,
    pub(crate) parameter_id: Option<String>,
    pub(crate) details: BTreeMap<String, String>,
}

impl std::fmt::Display for CliParameterErrorCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl CliParameterDomainError {
    pub(crate) fn new(code: CliParameterErrorCode) -> Self {
        Self {
            code,
            agent_id: None,
            parameter_id: None,
            details: BTreeMap::new(),
        }
    }

    pub(crate) fn unknown_agent(agent_id: impl Into<String>) -> Self {
        Self::new(CliParameterErrorCode::UnknownAgent).for_agent(agent_id)
    }

    pub(crate) fn unknown_parameter(
        agent_id: impl Into<String>,
        parameter_id: impl Into<String>,
    ) -> Self {
        Self::new(CliParameterErrorCode::UnknownParameter)
            .for_agent(agent_id)
            .for_parameter(parameter_id)
    }

    pub(crate) fn invalid_value(
        agent_id: impl Into<String>,
        parameter_id: impl Into<String>,
        reason: &str,
    ) -> Self {
        Self::new(CliParameterErrorCode::InvalidValue)
            .for_agent(agent_id)
            .for_parameter(parameter_id)
            .with_detail("reason", reason)
    }

    pub(crate) fn catalog_invalid(reason: impl Into<String>) -> Self {
        Self::new(CliParameterErrorCode::CatalogInvalid).with_detail("reason", reason)
    }

    pub(crate) fn for_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub(crate) fn for_parameter(mut self, parameter_id: impl Into<String>) -> Self {
        self.parameter_id = Some(parameter_id.into());
        self
    }

    pub(crate) fn with_detail(mut self, key: &str, value: impl Into<String>) -> Self {
        self.details.insert(key.to_string(), value.into());
        self
    }

    pub(crate) fn code_str(&self) -> &'static str {
        self.code.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_the_documented_wire_string() {
        let expected = [
            (
                CliParameterErrorCode::UnknownAgent,
                "CLI_PARAMETER_UNKNOWN_AGENT",
            ),
            (
                CliParameterErrorCode::UnknownParameter,
                "CLI_PARAMETER_UNKNOWN_PARAMETER",
            ),
            (
                CliParameterErrorCode::InvalidValue,
                "CLI_PARAMETER_INVALID_VALUE",
            ),
            (
                CliParameterErrorCode::DependencyUnsatisfied,
                "CLI_PARAMETER_DEPENDENCY_UNSATISFIED",
            ),
            (CliParameterErrorCode::Conflict, "CLI_PARAMETER_CONFLICT"),
            (
                CliParameterErrorCode::UnsupportedVersion,
                "CLI_PARAMETER_UNSUPPORTED_VERSION",
            ),
            (
                CliParameterErrorCode::RevisionConflict,
                "CLI_PARAMETER_REVISION_CONFLICT",
            ),
            (
                CliParameterErrorCode::CatalogMismatch,
                "CLI_PARAMETER_CATALOG_MISMATCH",
            ),
            (
                CliParameterErrorCode::CatalogInvalid,
                "CLI_PARAMETER_CATALOG_INVALID",
            ),
            (
                CliParameterErrorCode::RepositoryFailure,
                "CLI_PARAMETER_REPOSITORY_FAILURE",
            ),
        ];
        for (code, wire) in expected {
            assert_eq!(code.as_str(), wire);
        }
    }

    #[test]
    fn a_field_error_identifies_its_agent_and_parameter() {
        let error = CliParameterDomainError::invalid_value("codex-cli", "model", "pattern");
        assert_eq!(error.code_str(), "CLI_PARAMETER_INVALID_VALUE");
        assert_eq!(error.agent_id.as_deref(), Some("codex-cli"));
        assert_eq!(error.parameter_id.as_deref(), Some("model"));
        assert_eq!(
            error.details.get("reason").map(String::as_str),
            Some("pattern")
        );
    }

    #[test]
    fn an_error_message_never_needs_prose_parsing() {
        let error = CliParameterDomainError::unknown_parameter("gemini-cli", "sandbox");
        assert!(error
            .to_string()
            .contains("CLI_PARAMETER_UNKNOWN_PARAMETER"));
    }
}
