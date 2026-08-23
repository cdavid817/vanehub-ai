use super::dto::{
    CliParameterCommandError, CliParameterProfileDto, ResetCliParameterProfileRequest,
};
use crate::contexts::tooling::cli_parameters::api::CliParameterSettingsApi;
use tauri::State;

/// Reset is a write, so it carries the same optimistic tokens as save. Resetting a profile another
/// window has since edited fails the same way saving over it would.
#[tauri::command]
pub(crate) fn reset_cli_parameter_profile(
    api: State<'_, CliParameterSettingsApi>,
    input: ResetCliParameterProfileRequest,
) -> Result<CliParameterProfileDto, CliParameterCommandError> {
    Ok(CliParameterProfileDto::from(
        api.reset_profile(&input.into())?,
    ))
}
