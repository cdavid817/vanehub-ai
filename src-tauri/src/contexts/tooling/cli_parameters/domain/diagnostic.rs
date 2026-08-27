use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum CliParameterDiagnosticCode {
    LegacySelectionMigrated,
    LegacySelectionQuarantined,
    UnsupportedByActiveVersion,
    UnsupportedPlatform,
    UnsupportedValue,
    VersionUnknown,
    CliNotInstalled,
    ActiveInstallationConflict,
    DependencyNotSatisfied,
    ConflictingSelection,
    ModelDependentValue,
    MissingDirectory,
    CatalogReviewRequired,
    RevisionConflict,
    CatalogVersionConflict,
}

impl CliParameterDiagnosticCode {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::LegacySelectionMigrated => "LEGACY_SELECTION_MIGRATED",
            Self::LegacySelectionQuarantined => "LEGACY_SELECTION_QUARANTINED",
            Self::UnsupportedByActiveVersion => "UNSUPPORTED_BY_ACTIVE_VERSION",
            Self::UnsupportedPlatform => "UNSUPPORTED_PLATFORM",
            Self::UnsupportedValue => "UNSUPPORTED_VALUE",
            Self::VersionUnknown => "VERSION_UNKNOWN",
            Self::CliNotInstalled => "CLI_NOT_INSTALLED",
            Self::ActiveInstallationConflict => "ACTIVE_INSTALLATION_CONFLICT",
            Self::DependencyNotSatisfied => "DEPENDENCY_NOT_SATISFIED",
            Self::ConflictingSelection => "CONFLICTING_SELECTION",
            Self::ModelDependentValue => "MODEL_DEPENDENT_VALUE",
            Self::MissingDirectory => "MISSING_DIRECTORY",
            Self::CatalogReviewRequired => "CATALOG_REVIEW_REQUIRED",
            Self::RevisionConflict => "REVISION_CONFLICT",
            Self::CatalogVersionConflict => "CATALOG_VERSION_CONFLICT",
        }
    }

    /// A blocking diagnostic disables profile save; the rest are advisory and never stop a launch.
    pub(crate) fn is_blocking(&self) -> bool {
        matches!(
            self,
            Self::DependencyNotSatisfied
                | Self::ConflictingSelection
                | Self::UnsupportedValue
                | Self::RevisionConflict
                | Self::CatalogVersionConflict
        )
    }

    pub(crate) fn severity(&self) -> CliParameterDiagnosticSeverity {
        match self {
            Self::DependencyNotSatisfied
            | Self::ConflictingSelection
            | Self::UnsupportedValue
            | Self::RevisionConflict
            | Self::CatalogVersionConflict => CliParameterDiagnosticSeverity::Error,
            Self::LegacySelectionMigrated | Self::ModelDependentValue => {
                CliParameterDiagnosticSeverity::Info
            }
            _ => CliParameterDiagnosticSeverity::Warning,
        }
    }

    /// The remediation the settings page offers for this diagnostic.
    pub(crate) fn remediation(&self) -> CliParameterRemediation {
        match self {
            Self::CliNotInstalled | Self::ActiveInstallationConflict | Self::VersionUnknown => {
                CliParameterRemediation::OpenCliManagement
            }
            Self::LegacySelectionQuarantined
            | Self::UnsupportedByActiveVersion
            | Self::UnsupportedPlatform
            | Self::UnsupportedValue => CliParameterRemediation::RepairSelection,
            Self::DependencyNotSatisfied | Self::ConflictingSelection => {
                CliParameterRemediation::AdjustDependency
            }
            Self::MissingDirectory => CliParameterRemediation::ReselectDirectory,
            Self::RevisionConflict | Self::CatalogVersionConflict => {
                CliParameterRemediation::ReloadProfile
            }
            Self::LegacySelectionMigrated
            | Self::ModelDependentValue
            | Self::CatalogReviewRequired => CliParameterRemediation::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CliParameterDiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliParameterRemediation {
    None,
    RepairSelection,
    AdjustDependency,
    ReselectDirectory,
    ReloadProfile,
    OpenCliManagement,
}

/// Details are bounded, non-secret facts (ids, versions, counts). Raw user values never enter a
/// persisted diagnostic; `redacted_detail` is the only way a value-derived fact is recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterDiagnostic {
    pub(crate) code: CliParameterDiagnosticCode,
    pub(crate) severity: CliParameterDiagnosticSeverity,
    pub(crate) agent_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parameter_id: Option<String>,
    pub(crate) message_key: String,
    pub(crate) blocking: bool,
    pub(crate) remediation: CliParameterRemediation,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) details: BTreeMap<String, String>,
}

impl CliParameterDiagnostic {
    pub(crate) fn new(
        code: CliParameterDiagnosticCode,
        agent_id: impl Into<String>,
        parameter_id: Option<String>,
    ) -> Self {
        Self {
            code,
            severity: code.severity(),
            agent_id: agent_id.into(),
            parameter_id,
            message_key: format!("cliParameters.diagnostics.{}.message", code.as_str()),
            blocking: code.is_blocking(),
            remediation: code.remediation(),
            details: BTreeMap::new(),
        }
    }

    pub(crate) fn with_detail(mut self, key: &str, value: impl Into<String>) -> Self {
        self.details.insert(key.to_string(), value.into());
        self
    }

    /// Records a bounded shape of a user value rather than the value itself.
    pub(crate) fn with_redacted_detail(self, key: &str, value: &str) -> Self {
        let length = value.chars().count();
        self.with_detail(key, format!("<redacted len={length}>"))
    }

    pub(crate) fn dedup_key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.agent_id,
            self.code.as_str(),
            self.parameter_id.as_deref().unwrap_or("-")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocking_codes_are_the_ones_that_must_stop_a_save() {
        assert!(CliParameterDiagnosticCode::DependencyNotSatisfied.is_blocking());
        assert!(CliParameterDiagnosticCode::ConflictingSelection.is_blocking());
        assert!(CliParameterDiagnosticCode::RevisionConflict.is_blocking());
        assert!(!CliParameterDiagnosticCode::CliNotInstalled.is_blocking());
        assert!(!CliParameterDiagnosticCode::LegacySelectionQuarantined.is_blocking());
        assert!(!CliParameterDiagnosticCode::MissingDirectory.is_blocking());
    }

    #[test]
    fn every_code_has_a_stable_wire_string_and_message_key() {
        let codes = [
            CliParameterDiagnosticCode::LegacySelectionMigrated,
            CliParameterDiagnosticCode::LegacySelectionQuarantined,
            CliParameterDiagnosticCode::UnsupportedByActiveVersion,
            CliParameterDiagnosticCode::UnsupportedPlatform,
            CliParameterDiagnosticCode::UnsupportedValue,
            CliParameterDiagnosticCode::VersionUnknown,
            CliParameterDiagnosticCode::CliNotInstalled,
            CliParameterDiagnosticCode::ActiveInstallationConflict,
            CliParameterDiagnosticCode::DependencyNotSatisfied,
            CliParameterDiagnosticCode::ConflictingSelection,
            CliParameterDiagnosticCode::ModelDependentValue,
            CliParameterDiagnosticCode::MissingDirectory,
            CliParameterDiagnosticCode::CatalogReviewRequired,
            CliParameterDiagnosticCode::RevisionConflict,
            CliParameterDiagnosticCode::CatalogVersionConflict,
        ];
        for code in codes {
            let encoded = serde_json::to_string(&code).expect("encode");
            assert_eq!(encoded, format!("\"{}\"", code.as_str()));
            let diagnostic = CliParameterDiagnostic::new(code, "claude-code", None);
            assert!(diagnostic
                .message_key
                .starts_with("cliParameters.diagnostics."));
        }
    }

    #[test]
    fn redacted_details_never_carry_the_original_value() {
        let diagnostic = CliParameterDiagnostic::new(
            CliParameterDiagnosticCode::LegacySelectionQuarantined,
            "codex-cli",
            Some("model".to_string()),
        )
        .with_redacted_detail("storedValue", "sk-secret-token");
        let encoded = serde_json::to_string(&diagnostic).expect("encode");
        assert!(!encoded.contains("sk-secret-token"));
        assert!(encoded.contains("<redacted len=15>"));
    }

    #[test]
    fn dedup_key_identifies_agent_code_and_parameter() {
        let diagnostic = CliParameterDiagnostic::new(
            CliParameterDiagnosticCode::UnsupportedByActiveVersion,
            "claude-code",
            Some("screenReader".to_string()),
        );
        assert_eq!(
            diagnostic.dedup_key(),
            "claude-code|UNSUPPORTED_BY_ACTIVE_VERSION|screenReader"
        );
    }
}
