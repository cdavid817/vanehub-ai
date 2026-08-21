use super::{dto, mapper};
use crate::commands::error::CommandError;
use crate::contexts::tooling::extension_platform::api::ExtensionPlatformApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_extension_feature_gates(
    api: State<'_, ExtensionPlatformApi>,
) -> Result<dto::FeatureGateOverviewDto, CommandError> {
    Ok(mapper::snapshot_to_dto(&api.feature_gates()))
}
