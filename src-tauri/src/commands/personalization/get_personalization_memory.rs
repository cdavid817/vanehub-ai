use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// One memory in full, for the one the user opened.
#[tauri::command]
pub(crate) fn get_personalization_memory(
    api: State<'_, PersonalizationApi>,
    memory_id: String,
) -> Result<Option<dto::MemoryDetailView>, CommandError> {
    let id = mapper::memory_id(&memory_id)?;
    api.memory_detail(&id)
        .map(|record| record.map(mapper::detail_to_dto))
        .map_err(map_command_error)
}
