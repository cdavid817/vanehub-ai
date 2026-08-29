use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// Decides one proposal.
///
/// The one path from a proposal to an active record. A target that moved since the proposal was
/// written comes back as a conflict rather than an overwrite.
#[tauri::command]
pub(crate) fn review_personalization_candidate(
    api: State<'_, PersonalizationApi>,
    input: dto::ReviewCandidateInput,
) -> Result<dto::ReviewOutcomeView, CommandError> {
    let request = mapper::review_request(input)?;
    api.review_memory_candidate(request)
        .map(mapper::review_outcome_to_dto)
        .map_err(map_command_error)
}
