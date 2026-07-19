use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::sdk::api::SdkApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_sdk_operation_logs(
    api: State<'_, SdkApi>,
    sdk_id: Option<dto::SdkId>,
) -> Result<Vec<dto::SdkOperationLog>, CommandError> {
    api.operation_logs(mapper::optional_id_from_dto(sdk_id))
        .map(mapper::operation_logs_to_dto)
        .map_err(map_command_error)
}
