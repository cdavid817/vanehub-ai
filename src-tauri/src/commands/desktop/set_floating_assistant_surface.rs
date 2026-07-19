use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::FloatingAssistantApi;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub(crate) fn set_floating_assistant_surface(
    app: AppHandle,
    api: State<'_, FloatingAssistantApi>,
    mode: dto::FloatingAssistantSurfaceMode,
) -> Result<(), CommandError> {
    let mode = mapper::floating_surface_to_domain(mode);
    api.set_surface(mode).map_err(map_command_error)?;
    app.emit(
        "floating-assistant:event",
        dto::FloatingAssistantEvent::SurfaceChanged {
            mode: mapper::floating_surface_to_dto(mode),
        },
    )
    .map_err(|error| CommandError::storage(error.to_string()))
}
