use crate::contexts::tooling::cli_parameters::domain::error::{
    CliParameterDomainError, CliParameterErrorCode,
};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum CliParameterApplicationError {
    #[error("{0}")]
    Domain(#[from] CliParameterDomainError),
    #[error("CLI_PARAMETER_REVISION_CONFLICT")]
    RevisionConflict {
        agent_id: String,
        expected_revision: i64,
        actual_revision: i64,
    },
    #[error("CLI_PARAMETER_CATALOG_MISMATCH")]
    CatalogMismatch {
        agent_id: String,
        expected_catalog_version: String,
        actual_catalog_version: String,
    },
    #[error("CLI_PARAMETER_REPOSITORY_FAILURE")]
    Repository(String),
}

impl CliParameterApplicationError {
    pub(crate) fn code(&self) -> CliParameterErrorCode {
        match self {
            Self::Domain(error) => error.code,
            Self::RevisionConflict { .. } => CliParameterErrorCode::RevisionConflict,
            Self::CatalogMismatch { .. } => CliParameterErrorCode::CatalogMismatch,
            Self::Repository(_) => CliParameterErrorCode::RepositoryFailure,
        }
    }

    pub(crate) fn agent_id(&self) -> Option<&str> {
        match self {
            Self::Domain(error) => error.agent_id.as_deref(),
            Self::RevisionConflict { agent_id, .. } | Self::CatalogMismatch { agent_id, .. } => {
                Some(agent_id)
            }
            Self::Repository(_) => None,
        }
    }

    pub(crate) fn parameter_id(&self) -> Option<&str> {
        match self {
            Self::Domain(error) => error.parameter_id.as_deref(),
            _ => None,
        }
    }

    /// Bounded, non-secret context the frontend uses to explain the failure. Repository causes are
    /// deliberately excluded: they can carry filesystem or SQL detail.
    pub(crate) fn details(&self) -> BTreeMap<String, String> {
        match self {
            Self::Domain(error) => error.details.clone(),
            Self::RevisionConflict {
                expected_revision,
                actual_revision,
                ..
            } => BTreeMap::from([
                (
                    "expectedRevision".to_string(),
                    expected_revision.to_string(),
                ),
                ("actualRevision".to_string(), actual_revision.to_string()),
            ]),
            Self::CatalogMismatch {
                expected_catalog_version,
                actual_catalog_version,
                ..
            } => BTreeMap::from([
                (
                    "expectedCatalogVersion".to_string(),
                    expected_catalog_version.clone(),
                ),
                (
                    "actualCatalogVersion".to_string(),
                    actual_catalog_version.clone(),
                ),
            ]),
            Self::Repository(_) => BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_domain_error_keeps_its_code_and_field_context() {
        let error = CliParameterApplicationError::from(CliParameterDomainError::invalid_value(
            "codex-cli",
            "model",
            "pattern",
        ));
        assert_eq!(error.code().as_str(), "CLI_PARAMETER_INVALID_VALUE");
        assert_eq!(error.agent_id(), Some("codex-cli"));
        assert_eq!(error.parameter_id(), Some("model"));
    }

    #[test]
    fn a_revision_conflict_reports_both_revisions() {
        let error = CliParameterApplicationError::RevisionConflict {
            agent_id: "claude-code".to_string(),
            expected_revision: 2,
            actual_revision: 5,
        };
        assert_eq!(error.code().as_str(), "CLI_PARAMETER_REVISION_CONFLICT");
        let details = error.details();
        assert_eq!(
            details.get("expectedRevision").map(String::as_str),
            Some("2")
        );
        assert_eq!(details.get("actualRevision").map(String::as_str), Some("5"));
    }

    #[test]
    fn a_repository_failure_never_leaks_its_cause_to_the_frontend() {
        let error = CliParameterApplicationError::Repository(
            "no such table: cli_parameter_profiles at /home/user/db".to_string(),
        );
        assert!(error.details().is_empty());
        assert_eq!(error.to_string(), "CLI_PARAMETER_REPOSITORY_FAILURE");
    }
}
