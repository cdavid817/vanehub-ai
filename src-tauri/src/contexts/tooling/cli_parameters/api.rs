//! The published CLI-parameter contracts.
//!
//! Two facades, deliberately separate:
//!
//! * [`CliParameterRuntimeApi`] is the launch-facing contract re-exported through
//!   `contexts::tooling::api`. It exposes resolution and read-only projections only — no save, no
//!   reset, no repository, no catalog loader, no private domain module.
//! * [`CliParameterSettingsApi`] is the settings-facing facade, held as Tauri state and reached
//!   only by the four CLI-parameter commands. It is not re-exported through `tooling::api`, so no
//!   launch path can write.
//!
//! Both wrap the same application service, so the page and the launch cannot disagree about a
//! profile.

use super::application::service::CliParameterApplicationService;

pub(crate) use super::application::error::CliParameterApplicationError;
/// Part of `CliParameterProfileView`'s shape rather than a standalone contract: production reads
/// it through the view, and only a test that builds a view from scratch names the type.
#[cfg(test)]
pub(crate) use super::application::models::CliParameterSavedPreviews;
pub(crate) use super::application::models::{
    CliLaunchExecutionContext, CliParameterFieldView, CliParameterPreview, CliParameterProfileView,
    PreviewCliParameterProfileInput, ResetCliParameterProfileInput,
    ResolveCliLaunchParametersInput, ResolvedCliLaunchParameters, SaveCliParameterProfileInput,
};
pub(crate) use super::domain::compatibility::{CliInstallationSnapshot, CliParameterSupport};
pub(crate) use super::domain::definition::{CliLaunchScope, CliParameterDefinition};
pub(crate) use super::domain::diagnostic::CliParameterDiagnostic;
pub(crate) use super::domain::rendering::CliArgumentSegments;
pub(crate) use super::domain::selection::{
    CliParameterSelection, CliParameterSelectionMap, CliParameterValue,
};

/// Launch-facing contract. Every managed CLI process — chat, resume, and Agent Terminal — resolves
/// its user-profile argv through this and nothing else.
#[derive(Clone)]
pub(crate) struct CliParameterRuntimeApi {
    service: CliParameterApplicationService,
}

impl CliParameterRuntimeApi {
    pub(crate) fn new(service: CliParameterApplicationService) -> Self {
        Self { service }
    }

    /// Validates the registry once so an invalid catalog is a startup-visible diagnostic rather
    /// than a first-launch surprise. Production never panics on a bad registry.
    pub(crate) fn validate_registry(&self) -> Result<String, CliParameterApplicationError> {
        self.service.catalog_version()
    }

    /// Resolves the launch segments for one managed CLI, dual-reading legacy and v2 rows.
    pub(crate) fn resolve_cli_launch_segments(
        &self,
        input: &ResolveCliLaunchParametersInput,
    ) -> Result<ResolvedCliLaunchParameters, CliParameterApplicationError> {
        self.service.resolve_launch_parameters(input)
    }

    /// Validated explicit selections, used by Sessions to derive chat defaults. Inherited entries
    /// stay inherited; the caller decides its own fallback.
    pub(crate) fn resolved_selections(
        &self,
        agent_id: &str,
    ) -> Result<CliParameterSelectionMap, CliParameterApplicationError> {
        Ok(self.service.load_profile(agent_id)?.view.selections)
    }
}

/// Settings-facing facade. Retained for the settings cutover; not published through
/// `contexts::tooling::api`, so no v2 save or reset path is reachable from a command yet.
#[derive(Clone)]
pub(crate) struct CliParameterSettingsApi {
    service: CliParameterApplicationService,
}

impl CliParameterSettingsApi {
    pub(crate) fn new(service: CliParameterApplicationService) -> Self {
        Self { service }
    }

    pub(crate) fn list_profiles(
        &self,
    ) -> Result<Vec<CliParameterProfileView>, CliParameterApplicationError> {
        self.service.list_profiles()
    }

    pub(crate) fn preview_profile(
        &self,
        input: &PreviewCliParameterProfileInput,
    ) -> Result<CliParameterPreview, CliParameterApplicationError> {
        self.service.preview_profile(input)
    }

    pub(crate) fn save_profile(
        &self,
        input: &SaveCliParameterProfileInput,
    ) -> Result<CliParameterProfileView, CliParameterApplicationError> {
        self.service.save_profile(input)
    }

    pub(crate) fn reset_profile(
        &self,
        input: &ResetCliParameterProfileInput,
    ) -> Result<CliParameterProfileView, CliParameterApplicationError> {
        self.service.reset_profile(input)
    }
}
