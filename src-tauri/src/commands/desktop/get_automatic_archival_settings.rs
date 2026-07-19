use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::DesktopSettingsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_automatic_archival_settings(
    api: State<'_, DesktopSettingsApi>,
) -> Result<dto::AutomaticArchivalSettings, CommandError> {
    api.get_automatic_archival_settings()
        .map(mapper::archival_to_dto)
        .map_err(map_command_error)
}
