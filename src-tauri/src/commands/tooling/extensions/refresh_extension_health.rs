use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::extensions::api::ExtensionApi;
use tauri::State;

#[tauri::command]
pub(crate) fn refresh_extension_health(
    api: State<'_, ExtensionApi>,
) -> Result<dto::ExtensionOverview, CommandError> {
    api.refresh_health()
        .map(mapper::overview_to_dto)
        .map_err(map_command_error)
}
