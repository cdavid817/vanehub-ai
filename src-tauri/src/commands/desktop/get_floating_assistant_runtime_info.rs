use super::{dto::FloatingAssistantRuntimeInfo, mapper};
use crate::contexts::desktop::api::FloatingAssistantApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_floating_assistant_runtime_info(
    api: State<'_, FloatingAssistantApi>,
) -> FloatingAssistantRuntimeInfo {
    mapper::floating_runtime_to_dto(api.platform())
}
