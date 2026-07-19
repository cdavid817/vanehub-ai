use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::FloatingAssistantApi;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub(crate) async fn set_floating_assistant_enabled(
    app: AppHandle,
    api: State<'_, FloatingAssistantApi>,
    enabled: bool,
) -> Result<dto::FloatingAssistantConfig, CommandError> {
    let config = api
        .set_enabled(enabled)
        .map(mapper::floating_config_to_dto)
        .map_err(map_command_error)?;
    app.emit(
        "floating-assistant:event",
        dto::FloatingAssistantEvent::ConfigurationChanged {
            config: config.clone(),
        },
    )
    .map_err(|error| CommandError::storage(error.to_string()))?;
    api.record_configuration_changed(enabled);
    Ok(config)
}
