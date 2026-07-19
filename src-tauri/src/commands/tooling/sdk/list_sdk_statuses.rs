use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::sdk::api::SdkApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_sdk_statuses(api: State<'_, SdkApi>) -> Result<dto::SdkStatusMap, CommandError> {
    api.list_statuses()
        .map(mapper::status_map_to_dto)
        .map_err(map_command_error)
}
