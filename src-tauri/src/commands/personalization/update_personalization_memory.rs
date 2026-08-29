use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// Corrects one memory, refusing an edit made from a stale copy.
#[tauri::command]
pub(crate) fn update_personalization_memory(
    api: State<'_, PersonalizationApi>,
    input: dto::UpdateMemoryCommandInput,
) -> Result<dto::MemoryDetailView, CommandError> {
    let id = mapper::memory_id(&input.id)?;
    let patch = mapper::update_patch(&input)?;
    api.update_memory(&id, input.expected_revision, patch)
        .map(mapper::detail_to_dto)
        .map_err(map_command_error)
}
