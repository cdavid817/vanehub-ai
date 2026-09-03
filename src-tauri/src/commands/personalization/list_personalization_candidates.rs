use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// Proposals waiting for a decision.
#[tauri::command]
pub(crate) fn list_personalization_candidates(
    api: State<'_, PersonalizationApi>,
    limit: Option<usize>,
) -> Result<Vec<dto::MemoryCandidateView>, CommandError> {
    api.pending_memory_candidates(limit.unwrap_or(50).clamp(1, 200))
        .map(mapper::candidates_to_dto)
        .map_err(map_command_error)
}
