use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// A memory the user is writing themselves.
///
/// Distinct from a proposal on purpose: this is the path where a person is the author, and it
/// produces an active record directly rather than a queue entry.
#[tauri::command]
pub(crate) fn create_personalization_memory(
    api: State<'_, PersonalizationApi>,
    input: dto::CreateMemoryCommandInput,
) -> Result<dto::MemoryDetailView, CommandError> {
    let create = mapper::create_input(input)?;
    api.create_memory(create)
        .map(mapper::detail_to_dto)
        .map_err(map_command_error)
}
