use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::DesktopSettingsApi;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub(crate) fn set_launch_on_startup(
    app: AppHandle,
    api: State<'_, DesktopSettingsApi>,
    enabled: bool,
) -> Result<dto::AppSettings, CommandError> {
    let settings = api
        .set_launch_on_startup(enabled)
        .map(mapper::settings_to_dto)
        .map_err(map_command_error)?;
    app.emit(
        "settings:event",
        dto::SettingsStateEvent {
            kind: "settings-changed",
            key: "launchOnStartup".to_string(),
        },
    )
    .map_err(|error| CommandError::storage(error.to_string()))?;
    Ok(settings)
}
