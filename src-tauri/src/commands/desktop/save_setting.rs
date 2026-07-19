use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::DesktopSettingsApi;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub(crate) fn save_setting(
    app: AppHandle,
    api: State<'_, DesktopSettingsApi>,
    input: dto::SaveSettingInput,
) -> Result<dto::AppSettings, CommandError> {
    let (key, value) = mapper::setting_input(input)?;
    let settings = api
        .save_setting(&key, &value)
        .map(mapper::settings_to_dto)
        .map_err(map_command_error)?;
    emit_settings_changed(&app, key)?;
    Ok(settings)
}

fn emit_settings_changed(app: &AppHandle, key: String) -> Result<(), CommandError> {
    app.emit(
        "settings:event",
        dto::SettingsStateEvent {
            kind: "settings-changed",
            key,
        },
    )
    .map_err(|error| CommandError::storage(error.to_string()))
}
