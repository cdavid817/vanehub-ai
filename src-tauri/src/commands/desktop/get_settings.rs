use super::{dto::AppSettings, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::DesktopSettingsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_settings(
    api: State<'_, DesktopSettingsApi>,
) -> Result<AppSettings, CommandError> {
    api.get_settings()
        .map(mapper::settings_to_dto)
        .map_err(map_command_error)
}
