use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// One bounded page of memories.
///
/// Summaries only. A page carrying every body would read the whole store to render a list of
/// names, which is the shape the previous unscoped listing had.
#[tauri::command]
pub(crate) fn query_personalization_memories(
    api: State<'_, PersonalizationApi>,
    input: dto::MemoryQueryInput,
) -> Result<dto::MemoryPageView, CommandError> {
    let query = mapper::memory_query(input)?;
    api.list_memories(&query)
        .map(mapper::page_to_dto)
        .map_err(map_command_error)
}
