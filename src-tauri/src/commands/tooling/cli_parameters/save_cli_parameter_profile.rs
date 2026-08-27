use super::dto::{
    CliParameterCommandError, CliParameterProfileDto, SaveCliParameterProfileRequest,
};
use crate::contexts::tooling::cli_parameters::api::CliParameterSettingsApi;
use tauri::State;

/// The write path. `expectedRevision` and `catalogVersion` are both required by the request shape,
/// so a caller cannot save without declaring what it believed it was editing.
#[tauri::command]
pub(crate) fn save_cli_parameter_profile(
    api: State<'_, CliParameterSettingsApi>,
    input: SaveCliParameterProfileRequest,
) -> Result<CliParameterProfileDto, CliParameterCommandError> {
    Ok(CliParameterProfileDto::from(
        api.save_profile(&input.into())?,
    ))
}
