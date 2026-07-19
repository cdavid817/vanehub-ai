use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::DesktopSettingsApi;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub(crate) fn save_automatic_archival_settings(
    app: AppHandle,
    api: State<'_, DesktopSettingsApi>,
    input: dto::AutomaticArchivalSettings,
) -> Result<dto::AutomaticArchivalSettings, CommandError> {
    let saved = api
        .save_automatic_archival_settings(input.enabled, input.inactive_days)
        .map(mapper::archival_to_dto)
        .map_err(map_command_error)?;
    app.emit(
        "settings:event",
        dto::SettingsStateEvent {
            kind: "settings-changed",
            key: "automaticArchival".to_string(),
        },
    )
    .map_err(|error| CommandError::storage(error.to_string()))?;
    Ok(saved)
}
