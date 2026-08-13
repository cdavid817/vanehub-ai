use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_token_usage_details(
    api: State<'_, SessionsApi>,
    input: dto::TokenUsageDetailsInput,
) -> Result<dto::TokenUsageDetailsPage, CommandError> {
    let query = mapper::token_usage_details_query(input).map_err(map_command_error)?;
    api.token_usage_details(&query)
        .map(mapper::token_usage_details_to_dto)
        .map_err(map_command_error)
}
