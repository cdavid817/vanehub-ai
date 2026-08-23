use crate::contexts::tooling::cli_parameters::domain::compatibility::{
    CliInstallationSnapshot, CliParameterSupport,
};
use crate::contexts::tooling::cli_parameters::domain::definition::{
    CliLaunchScope, CliParameterDefinition,
};
use crate::contexts::tooling::cli_parameters::domain::diagnostic::CliParameterDiagnostic;
use crate::contexts::tooling::cli_parameters::domain::rendering::CliArgumentSegments;
use crate::contexts::tooling::cli_parameters::domain::selection::CliParameterSelectionMap;
use std::collections::BTreeMap;

/// One editable field with its evaluated compatibility. Option-level support is separate so the
/// page can disable a single version-gated value without disabling the whole control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliParameterFieldView {
    pub(crate) definition: CliParameterDefinition,
    pub(crate) support: CliParameterSupport,
    pub(crate) option_support: BTreeMap<String, CliParameterSupport>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CliParameterSavedPreviews {
    pub(crate) chat: CliArgumentSegments,
    pub(crate) interactive: CliArgumentSegments,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliParameterProfileView {
    pub(crate) agent_id: String,
    pub(crate) catalog_version: String,
    pub(crate) revision: i64,
    pub(crate) updated_at: Option<String>,
    pub(crate) installation: CliInstallationSnapshot,
    pub(crate) fields: Vec<CliParameterFieldView>,
    pub(crate) selections: CliParameterSelectionMap,
    pub(crate) saved_previews: CliParameterSavedPreviews,
    pub(crate) diagnostics: Vec<CliParameterDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreviewCliParameterProfileInput {
    pub(crate) agent_id: String,
    pub(crate) catalog_version: String,
    pub(crate) scope: CliLaunchScope,
    pub(crate) selections: CliParameterSelectionMap,
    /// Echoed back so a slower response can be discarded without the domain knowing about UI
    /// timing.
    pub(crate) request_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliParameterPreview {
    pub(crate) agent_id: String,
    pub(crate) catalog_version: String,
    pub(crate) scope: CliLaunchScope,
    pub(crate) request_id: Option<String>,
    pub(crate) normalized_selections: CliParameterSelectionMap,
    pub(crate) segments: CliArgumentSegments,
    pub(crate) diagnostics: Vec<CliParameterDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SaveCliParameterProfileInput {
    pub(crate) agent_id: String,
    pub(crate) expected_revision: i64,
    pub(crate) catalog_version: String,
    pub(crate) selections: CliParameterSelectionMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResetCliParameterProfileInput {
    pub(crate) agent_id: String,
    pub(crate) expected_revision: i64,
    pub(crate) catalog_version: String,
}

/// Bounded launch context. It exists so a resolution diagnostic can be associated with the
/// observable operation that triggered the launch; it carries no prompt, credential, or session
/// value, and an Agent Terminal launch legitimately has no operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CliLaunchExecutionContext {
    pub(crate) operation_id: Option<String>,
}

/// Policy-governed values arrive from the Agent policy projection rather than the saved profile,
/// so a user profile can never take precedence over a governed argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolveCliLaunchParametersInput {
    pub(crate) agent_id: String,
    pub(crate) scope: CliLaunchScope,
    pub(crate) message_overrides: CliParameterSelectionMap,
    pub(crate) policy_overrides: CliParameterSelectionMap,
    pub(crate) execution_context: CliLaunchExecutionContext,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ResolvedCliLaunchParameters {
    pub(crate) global_tokens: Vec<String>,
    pub(crate) invocation_tokens: Vec<String>,
    pub(crate) selections: CliParameterSelectionMap,
    pub(crate) diagnostics: Vec<CliParameterDiagnostic>,
    pub(crate) profile_revision: i64,
    pub(crate) catalog_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplaceCliParameterProfile {
    pub(crate) agent_id: String,
    pub(crate) expected_revision: i64,
    pub(crate) catalog_version: String,
    pub(crate) selections: CliParameterSelectionMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PersistedCliParameterProfile {
    pub(crate) agent_id: String,
    pub(crate) revision: i64,
    pub(crate) catalog_version: String,
    pub(crate) updated_at: String,
}
