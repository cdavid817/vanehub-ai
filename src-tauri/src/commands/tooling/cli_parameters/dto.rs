//! The Tauri boundary for CLI parameters.
//!
//! Every type here is plain data. The domain types it embeds already carry the wire shape the
//! frontend contract declares, so these structs exist to name the boundary and to keep application
//! models — which have no serde derives and are free to change — out of the IPC surface.
//!
//! Failures cross as [`CliParameterCommandError`], an object with a stable `code`, not as the
//! shared `CommandError`'s prose string. That is the whole point: the page maps a code to localized
//! text and never matches an English message.

use crate::contexts::tooling::cli_parameters::api::{
    CliArgumentSegments, CliInstallationSnapshot, CliLaunchScope, CliParameterApplicationError,
    CliParameterDefinition, CliParameterDiagnostic, CliParameterFieldView, CliParameterPreview,
    CliParameterProfileView, CliParameterSelectionMap, CliParameterSupport,
    PreviewCliParameterProfileInput, ResetCliParameterProfileInput, SaveCliParameterProfileInput,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterFieldDto {
    pub(crate) definition: CliParameterDefinition,
    pub(crate) support: CliParameterSupport,
    pub(crate) option_support: BTreeMap<String, CliParameterSupport>,
}

impl From<CliParameterFieldView> for CliParameterFieldDto {
    fn from(view: CliParameterFieldView) -> Self {
        Self {
            definition: view.definition,
            support: view.support,
            option_support: view.option_support,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterSavedPreviewsDto {
    pub(crate) chat: CliArgumentSegments,
    pub(crate) interactive: CliArgumentSegments,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterProfileDto {
    pub(crate) agent_id: String,
    pub(crate) catalog_version: String,
    pub(crate) revision: i64,
    /// `null` rather than absent: the page distinguishes "never saved" from "saved at an unknown
    /// time", and an absent key would collapse those.
    pub(crate) updated_at: Option<String>,
    pub(crate) installation: CliInstallationSnapshot,
    pub(crate) fields: Vec<CliParameterFieldDto>,
    pub(crate) selections: CliParameterSelectionMap,
    pub(crate) saved_previews: CliParameterSavedPreviewsDto,
    pub(crate) diagnostics: Vec<CliParameterDiagnostic>,
}

impl From<CliParameterProfileView> for CliParameterProfileDto {
    fn from(view: CliParameterProfileView) -> Self {
        Self {
            agent_id: view.agent_id,
            catalog_version: view.catalog_version,
            revision: view.revision,
            updated_at: view.updated_at,
            installation: view.installation,
            fields: view
                .fields
                .into_iter()
                .map(CliParameterFieldDto::from)
                .collect(),
            selections: view.selections,
            saved_previews: CliParameterSavedPreviewsDto {
                chat: view.saved_previews.chat,
                interactive: view.saved_previews.interactive,
            },
            diagnostics: view.diagnostics,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterPreviewDto {
    pub(crate) agent_id: String,
    pub(crate) catalog_version: String,
    pub(crate) scope: CliLaunchScope,
    /// Omitted when the caller sent none, so the contract's optional field stays `undefined`
    /// rather than becoming `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) request_id: Option<String>,
    pub(crate) normalized_selections: CliParameterSelectionMap,
    pub(crate) segments: CliArgumentSegments,
    pub(crate) diagnostics: Vec<CliParameterDiagnostic>,
}

impl From<CliParameterPreview> for CliParameterPreviewDto {
    fn from(preview: CliParameterPreview) -> Self {
        Self {
            agent_id: preview.agent_id,
            catalog_version: preview.catalog_version,
            scope: preview.scope,
            request_id: preview.request_id,
            normalized_selections: preview.normalized_selections,
            segments: preview.segments,
            diagnostics: preview.diagnostics,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PreviewCliParameterProfileRequest {
    pub(crate) agent_id: String,
    pub(crate) catalog_version: String,
    pub(crate) scope: CliLaunchScope,
    pub(crate) selections: CliParameterSelectionMap,
    #[serde(default)]
    pub(crate) request_id: Option<String>,
}

impl From<PreviewCliParameterProfileRequest> for PreviewCliParameterProfileInput {
    fn from(request: PreviewCliParameterProfileRequest) -> Self {
        Self {
            agent_id: request.agent_id,
            catalog_version: request.catalog_version,
            scope: request.scope,
            selections: request.selections,
            request_id: request.request_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveCliParameterProfileRequest {
    pub(crate) agent_id: String,
    pub(crate) expected_revision: i64,
    pub(crate) catalog_version: String,
    pub(crate) selections: CliParameterSelectionMap,
}

impl From<SaveCliParameterProfileRequest> for SaveCliParameterProfileInput {
    fn from(request: SaveCliParameterProfileRequest) -> Self {
        Self {
            agent_id: request.agent_id,
            expected_revision: request.expected_revision,
            catalog_version: request.catalog_version,
            selections: request.selections,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResetCliParameterProfileRequest {
    pub(crate) agent_id: String,
    pub(crate) expected_revision: i64,
    pub(crate) catalog_version: String,
}

impl From<ResetCliParameterProfileRequest> for ResetCliParameterProfileInput {
    fn from(request: ResetCliParameterProfileRequest) -> Self {
        Self {
            agent_id: request.agent_id,
            expected_revision: request.expected_revision,
            catalog_version: request.catalog_version,
        }
    }
}

/// Structured failure. `code` is the stable identifier; `agentId` and `parameterId` locate it on
/// the page. `details` carries only the bounded, non-secret context the application error already
/// decided to publish — a repository cause never reaches it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterCommandError {
    pub(crate) code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) parameter_id: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) details: BTreeMap<String, String>,
}

impl From<CliParameterApplicationError> for CliParameterCommandError {
    fn from(error: CliParameterApplicationError) -> Self {
        Self {
            code: error.code().as_str(),
            agent_id: error.agent_id().map(str::to_string),
            parameter_id: error.parameter_id().map(str::to_string),
            details: error.details(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli_parameters::api::{
        CliParameterSavedPreviews, CliParameterSelection,
    };
    use serde_json::json;

    fn profile_view() -> CliParameterProfileView {
        CliParameterProfileView {
            agent_id: "codex-cli".to_string(),
            catalog_version: "2.0.0".to_string(),
            revision: 4,
            updated_at: None,
            installation: CliInstallationSnapshot::default(),
            fields: Vec::new(),
            selections: CliParameterSelectionMap::from([(
                "model".to_string(),
                CliParameterSelection::text("gpt-5.5"),
            )]),
            saved_previews: CliParameterSavedPreviews::default(),
            diagnostics: Vec::new(),
        }
    }

    #[test]
    fn a_profile_crosses_the_boundary_with_the_contract_field_names() {
        let dto = CliParameterProfileDto::from(profile_view());
        let wire = serde_json::to_value(&dto).expect("serialize profile");

        assert_eq!(wire["agentId"], json!("codex-cli"));
        assert_eq!(wire["catalogVersion"], json!("2.0.0"));
        assert_eq!(wire["revision"], json!(4));
        assert_eq!(wire["savedPreviews"]["chat"]["global"], json!([]));
        assert_eq!(
            wire["selections"]["model"],
            json!({"state": "value", "value": "gpt-5.5"})
        );
    }

    #[test]
    fn a_never_saved_profile_reports_a_null_timestamp_rather_than_omitting_it() {
        let wire = serde_json::to_value(CliParameterProfileDto::from(profile_view()))
            .expect("serialize profile");

        assert!(wire.as_object().expect("object").contains_key("updatedAt"));
        assert_eq!(wire["updatedAt"], json!(null));
    }

    #[test]
    fn an_absent_request_id_stays_absent_rather_than_becoming_null() {
        let preview = CliParameterPreview {
            agent_id: "opencode".to_string(),
            catalog_version: "2.0.0".to_string(),
            scope: CliLaunchScope::Chat,
            request_id: None,
            normalized_selections: CliParameterSelectionMap::new(),
            segments: CliArgumentSegments::default(),
            diagnostics: Vec::new(),
        };

        let wire =
            serde_json::to_value(CliParameterPreviewDto::from(preview)).expect("serialize preview");

        assert!(!wire.as_object().expect("object").contains_key("requestId"));
    }

    #[test]
    fn a_save_request_requires_both_concurrency_tokens() {
        let request: SaveCliParameterProfileRequest = serde_json::from_value(json!({
            "agentId": "claude-code",
            "expectedRevision": 7,
            "catalogVersion": "2.0.0",
            "selections": {"model": {"state": "inherit"}},
        }))
        .expect("deserialize save request");

        assert_eq!(request.expected_revision, 7);
        assert_eq!(request.catalog_version, "2.0.0");

        let missing_revision = serde_json::from_value::<SaveCliParameterProfileRequest>(json!({
            "agentId": "claude-code",
            "catalogVersion": "2.0.0",
            "selections": {},
        }));
        assert!(missing_revision.is_err());
    }

    #[test]
    fn an_unknown_field_is_rejected_rather_than_silently_dropped() {
        let stowaway = serde_json::from_value::<SaveCliParameterProfileRequest>(json!({
            "agentId": "claude-code",
            "expectedRevision": 1,
            "catalogVersion": "2.0.0",
            "selections": {},
            "rawArgs": "--dangerously-skip-permissions",
        }));

        assert!(stowaway.is_err());
    }

    #[test]
    fn an_error_crosses_as_a_code_bearing_object_not_a_sentence() {
        let error =
            CliParameterCommandError::from(CliParameterApplicationError::RevisionConflict {
                agent_id: "gemini-cli".to_string(),
                expected_revision: 2,
                actual_revision: 5,
            });

        let wire = serde_json::to_value(&error).expect("serialize error");
        assert_eq!(wire["code"], json!("CLI_PARAMETER_REVISION_CONFLICT"));
        assert_eq!(wire["agentId"], json!("gemini-cli"));
        assert_eq!(wire["details"]["expectedRevision"], json!("2"));
        assert_eq!(wire["details"]["actualRevision"], json!("5"));
        assert!(!wire
            .as_object()
            .expect("object")
            .contains_key("parameterId"));
    }

    #[test]
    fn a_repository_failure_publishes_its_code_and_nothing_else() {
        let error = CliParameterCommandError::from(CliParameterApplicationError::Repository(
            "no such table: cli_parameter_profiles at C:/Users/someone/db".to_string(),
        ));

        let wire = serde_json::to_value(&error).expect("serialize error");
        assert_eq!(wire["code"], json!("CLI_PARAMETER_REPOSITORY_FAILURE"));
        let object = wire.as_object().expect("object");
        assert_eq!(object.len(), 1);
    }
}
