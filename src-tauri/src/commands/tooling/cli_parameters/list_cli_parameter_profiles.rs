use super::dto::{CliParameterCommandError, CliParameterProfileDto};
use crate::contexts::tooling::cli_parameters::api::CliParameterSettingsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_cli_parameter_profiles(
    api: State<'_, CliParameterSettingsApi>,
) -> Result<Vec<CliParameterProfileDto>, CliParameterCommandError> {
    Ok(api
        .list_profiles()?
        .into_iter()
        .map(CliParameterProfileDto::from)
        .collect())
}
