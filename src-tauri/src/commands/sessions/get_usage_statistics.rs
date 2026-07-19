use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_usage_statistics(
    api: State<'_, SessionsApi>,
    range: dto::UsageStatisticsRange,
) -> Result<dto::UsageStatistics, CommandError> {
    api.usage_statistics(mapper::usage_range(range))
        .map(mapper::usage_statistics_to_dto)
        .map_err(map_command_error)
}
