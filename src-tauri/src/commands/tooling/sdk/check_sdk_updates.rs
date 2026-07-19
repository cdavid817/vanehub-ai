use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::sdk::api::SdkApi;
use tauri::State;

#[tauri::command]
pub(crate) fn check_sdk_updates(
    api: State<'_, SdkApi>,
    sdk_id: Option<dto::SdkId>,
) -> Result<dto::SdkUpdateMap, CommandError> {
    api.check_updates(mapper::optional_id_from_dto(sdk_id))
        .map(mapper::update_map_to_dto)
        .map_err(map_command_error)
}
