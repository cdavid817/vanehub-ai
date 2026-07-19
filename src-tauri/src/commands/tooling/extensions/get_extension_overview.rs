use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::extensions::api::ExtensionApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_extension_overview(
    api: State<'_, ExtensionApi>,
) -> Result<dto::ExtensionOverview, CommandError> {
    api.overview()
        .map(mapper::overview_to_dto)
        .map_err(map_command_error)
}
