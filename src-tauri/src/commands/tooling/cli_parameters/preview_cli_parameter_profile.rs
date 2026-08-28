use super::dto::{
    CliParameterCommandError, CliParameterPreviewDto, PreviewCliParameterProfileRequest,
};
use crate::contexts::tooling::cli_parameters::api::CliParameterSettingsApi;
use tauri::State;

/// Read-only. It renders the draft the page currently holds and touches neither the stored
/// selections nor the revision, so typing in a text field cannot bump the value that save's
/// optimistic check compares against.
#[tauri::command]
pub(crate) fn preview_cli_parameter_profile(
    api: State<'_, CliParameterSettingsApi>,
    input: PreviewCliParameterProfileRequest,
) -> Result<CliParameterPreviewDto, CliParameterCommandError> {
    Ok(CliParameterPreviewDto::from(
        api.preview_profile(&input.into())?,
    ))
}
