use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::extension_platform::api::ExtensionPlatformApi;
use tauri::State;

/// The actor recorded for gate changes made through the desktop UI. Gate mutation is a local
/// operator action; there is no separate account model to attribute it to.
const DESKTOP_ACTOR: &str = "desktop";

#[tauri::command]
pub(crate) fn set_extension_feature_gate(
    api: State<'_, ExtensionPlatformApi>,
    request: dto::SetFeatureGateRequest,
) -> Result<dto::FeatureGateOverviewDto, CommandError> {
    api.set_feature_desired_state(
        mapper::feature_from_dto(request.feature),
        request.desired_enabled,
        request.expected_revision,
        DESKTOP_ACTOR,
        request.reason,
    )
    .map(|snapshot| mapper::snapshot_to_dto(&snapshot))
    .map_err(map_command_error)
}
